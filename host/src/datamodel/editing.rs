//! A `TextBox`'s editing model: where its cursor may sit, what a word is, and
//! the state a focused box keeps between two keystrokes.
//!
//! POSITIONS ARE THE ENGINE'S. `CursorPosition` and `SelectionStart` are 1
//! based and sit BEFORE the character at that index, so a cursor ranges over
//! `1..=#Text + 1`, and either reads -1 when there is nothing to report. The
//! index is a UTF-8 byte offset plus one, the unit `string.sub` and `#` use,
//! and it never lands inside a codepoint: every value this module hands out is
//! snapped to a character boundary. What the engine counts for non-ASCII text
//! has not been measured.
//!
//! NOTHING HERE TOUCHES THE TREE OR FIRES ANYTHING. `input.rs` decides what a
//! key or a click means and fires the signals; this file answers questions
//! about strings and holds the session, so the rules can be tested without a
//! VM.

use std::time::{Duration, Instant};

/// How long the caret stays shown, and then hidden, in one blink.
pub const BLINK: Duration = Duration::from_millis(500);

/// The undo levels a focused box keeps. Older states fall off the bottom.
pub const UNDO_DEPTH: usize = 100;

/// Two presses closer together than this, in time and in pixels, are a double
/// click. The Windows defaults.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(500);
pub const DOUBLE_CLICK_SLOP: f32 = 4.0;

/// The engine's selection highlight and the colour of the glyphs inside it,
/// read off a Studio screenshot.
pub const HIGHLIGHT: (u8, u8, u8) = (107, 161, 249);
pub const SELECTED_TEXT: (u8, u8, u8) = (255, 255, 255);

/// Whether the caret is in the shown half of its blink, `since` the cycle
/// last restarted.
pub fn caret_shown(since: Duration) -> bool {
    (since.as_millis() / BLINK.as_millis()).is_multiple_of(2)
}

/// The state of the one focused `TextBox`, kept on the `Dom` so the renderer
/// can read it and `input.rs` can change it.
///
/// THE CURSOR IS NOT HERE. `CursorPosition` and `SelectionStart` are real
/// properties on the instance, and a second copy would be a second answer.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: usize,
    /// When the blink cycle last restarted: focus, and every move or edit.
    pub since: Instant,
    /// What the last painted frame showed, so the frame loop repaints only
    /// when the blink flips. `None` before the first frame.
    pub painted_shown: Option<bool>,
    /// Whether the last painted frame drew this box at all. An invisible box
    /// (Aether's hidden proxy) is never drawn, so its blink owes no repaint.
    pub drawn: bool,
    /// How far an unwrapped line wider than its box is scrolled left, in
    /// pixels, to keep the caret in view. Updated by the renderer.
    pub scroll: f32,
    /// Earlier states, newest last: the text, and the cursor that text was
    /// entered with.
    pub undo: Vec<(String, i64)>,
    /// The cursor the current text was entered with: where focus put it, or
    /// where the last edit left it. What an undo back to this text restores.
    pub entered_cursor: i64,
    /// The last edit was typing, so the next typed character joins its undo
    /// step rather than starting one.
    pub typing: bool,
    /// Where a mouse drag selection is anchored, while the button is held.
    pub drag_anchor: Option<i64>,
}

impl Session {
    pub fn new(id: usize, cursor: i64) -> Session {
        Session {
            id,
            since: Instant::now(),
            painted_shown: None,
            drawn: false,
            scroll: 0.0,
            undo: Vec::new(),
            entered_cursor: cursor,
            typing: false,
            drag_anchor: None,
        }
    }

    /// Show the caret now and restart its cycle.
    pub fn restart_blink(&mut self) {
        self.since = Instant::now();
    }

    /// Whether the caret should be shown at `now`.
    pub fn shown_at(&self, now: Instant) -> bool {
        caret_shown(now.saturating_duration_since(self.since))
    }

    /// Remember `text` before an edit replaces it.
    pub fn remember(&mut self, text: &str) {
        self.undo.push((text.to_string(), self.entered_cursor));
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
    }
}

/// The largest character boundary at or before byte `at`.
pub fn floor_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// A cursor clamped into `1..=#text + 1` and snapped to a character boundary.
pub fn clamp_cursor(text: &str, cursor: i64) -> i64 {
    let byte = (cursor - 1).clamp(0, text.len() as i64) as usize;
    floor_boundary(text, byte) as i64 + 1
}

/// A value a script assigned to `CursorPosition` or `SelectionStart`: -1 for
/// anything below 1, otherwise clamped and snapped like [`clamp_cursor`].
pub fn clamp_script(text: &str, value: i64) -> i64 {
    if value < 1 {
        -1
    } else {
        clamp_cursor(text, value)
    }
}

/// The byte offset of a cursor.
pub fn byte_of(text: &str, cursor: i64) -> usize {
    (clamp_cursor(text, cursor) - 1) as usize
}

/// One character left of byte `at`, or `at` at the start.
pub fn prev_char(text: &str, at: usize) -> usize {
    text[..at].char_indices().next_back().map_or(0, |(i, _)| i)
}

/// One character right of byte `at`, or `at` at the end.
pub fn next_char(text: &str, at: usize) -> usize {
    text[at..].chars().next().map_or(at, |c| at + c.len_utf8())
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Where Ctrl+Left lands from `at`: back over spaces and punctuation, then to
/// the start of the word before them.
pub fn word_left(text: &str, at: usize) -> usize {
    let mut at = at;
    while at > 0 && !text[..at].chars().next_back().is_some_and(is_word) {
        at = prev_char(text, at);
    }
    while at > 0 && text[..at].chars().next_back().is_some_and(is_word) {
        at = prev_char(text, at);
    }
    at
}

/// Where Ctrl+Right lands from `at`: past the rest of this word and the
/// spaces after it, to the start of the next word, as Windows edit controls
/// do.
pub fn word_right(text: &str, at: usize) -> usize {
    let mut at = at;
    let starts_in_word = text[at..].chars().next().is_some_and(is_word);
    if starts_in_word {
        while text[at..].chars().next().is_some_and(is_word) {
            at = next_char(text, at);
        }
    } else if let Some(c) = text[at..].chars().next() {
        if !c.is_whitespace() {
            at = next_char(text, at);
        }
    }
    while text[at..].chars().next().is_some_and(char::is_whitespace) {
        at = next_char(text, at);
    }
    at
}

/// The byte range a double click at byte boundary `at` selects: the word
/// under it, the run of spaces under it, or the one character.
///
/// THE CHARACTER UNDER A BOUNDARY is the one after it, unless that is not a
/// word character and the one before is: a click just past the end of a word
/// selects the word.
pub fn word_at(text: &str, at: usize) -> (usize, usize) {
    if text.is_empty() {
        return (0, 0);
    }
    let after = text[at..].chars().next();
    let before = text[..at].chars().next_back();
    let pivot = match (before, after) {
        (Some(b), Some(a)) if !is_word(a) && is_word(b) => prev_char(text, at),
        (Some(_), None) => prev_char(text, at),
        _ => at,
    };
    let c = text[pivot..]
        .chars()
        .next()
        .expect("a character at the pivot");
    let same = |x: char| {
        if is_word(c) {
            is_word(x)
        } else if c.is_whitespace() {
            x.is_whitespace()
        } else {
            false
        }
    };
    let mut start = pivot;
    while start > 0 && text[..start].chars().next_back().is_some_and(same) {
        start = prev_char(text, start);
    }
    let mut end = next_char(text, pivot);
    while text[end..].chars().next().is_some_and(same) {
        end = next_char(text, end);
    }
    (start, end)
}

/// What a paste inserts: carriage returns removed, and newlines too unless
/// the box is `MultiLine`. Other control characters are dropped as typing
/// would drop them.
pub fn paste_filter(text: &str, multi_line: bool) -> String {
    text.chars()
        .filter(|&c| (c == '\n' && multi_line) || c == '\t' || !c.is_control())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_is_clamped_and_never_splits_a_codepoint() {
        assert_eq!(clamp_cursor("hello", 0), 1);
        assert_eq!(clamp_cursor("hello", 99), 6);
        // "é" is two bytes: positions 2 and 3 straddle it, 3 snaps to 2.
        assert_eq!(clamp_cursor("\u{e9}x", 2), 1);
        assert_eq!(clamp_cursor("\u{e9}x", 3), 3);
        assert_eq!(clamp_script("hello", 0), -1);
        assert_eq!(clamp_script("hello", -5), -1);
        assert_eq!(clamp_script("hello", 3), 3);
    }

    #[test]
    fn word_moves_skip_spaces_then_a_word() {
        let t = "hello world  again";
        assert_eq!(word_left(t, t.len()), 13);
        assert_eq!(word_left(t, 13), 6);
        assert_eq!(word_left(t, 6), 0);
        assert_eq!(word_right(t, 0), 6);
        assert_eq!(word_right(t, 6), 13);
        assert_eq!(word_right(t, 13), t.len());
    }

    #[test]
    fn a_double_click_selects_the_word_under_it() {
        let t = "hello world";
        assert_eq!(word_at(t, 8), (6, 11));
        assert_eq!(word_at(t, 6), (6, 11));
        assert_eq!(word_at(t, 11), (6, 11));
        // Just past "hello", on the space: the word before wins.
        assert_eq!(word_at(t, 5), (0, 5));
        assert_eq!(word_at("a   b", 2), (1, 4));
    }

    #[test]
    fn the_blink_is_500_shown_then_500_hidden() {
        let ms = Duration::from_millis;
        assert!(caret_shown(ms(0)));
        assert!(caret_shown(ms(499)));
        assert!(!caret_shown(ms(500)));
        assert!(!caret_shown(ms(999)));
        assert!(caret_shown(ms(1000)));
    }

    #[test]
    fn a_paste_drops_newlines_unless_multi_line() {
        assert_eq!(paste_filter("a\r\nb", false), "ab");
        assert_eq!(paste_filter("a\r\nb", true), "a\nb");
    }

    #[test]
    fn undo_keeps_a_bounded_history() {
        let mut s = Session::new(1, 1);
        for i in 0..(UNDO_DEPTH + 5) {
            s.remember(&i.to_string());
        }
        assert_eq!(s.undo.len(), UNDO_DEPTH);
        assert_eq!(s.undo[0].0, "5");
    }
}
