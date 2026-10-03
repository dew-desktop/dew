//! A `TextBox` edited the way the engine edits one, held to rows measured in
//! Studio's Play mode: a 300 by 40 box, Arial at TextSize 20, left aligned,
//! `Text` "hello world", `ClearTextOnFocus` false.
//!
//! EVERY CLICK LANDS ON A BOUNDARY READ FROM THE LAYOUT, not on a pixel
//! guessed from the engine's screenshot, because Dew's advances run a few
//! percent narrower than the engine's. What is asserted is the engine's
//! answer to "click between `hello` and ` world`", not its x.
//!
//! These skip on a machine with no font.

use super::input::{Button, Pointer, Surface};
use super::{handle, install, install_vocabulary, SharedDom};
use mlua::prelude::*;

/// What these tests reach that is not a guest's to reach.
mod shim {
    use super::super::input::{Clipboard, Mods, Pointer, Surface};
    use super::super::{render, Dom};
    use mlua::prelude::*;
    use std::time::{Duration, Instant};

    pub fn key(p: &mut Pointer, s: &Surface, name: &str, shift: bool, ctrl: bool) -> LuaResult<()> {
        p.key(s, name, Mods { shift, ctrl })
    }

    pub fn set_shift(p: &mut Pointer, shift: bool) {
        p.mods.shift = shift;
    }

    pub fn use_memory_clipboard(p: &mut Pointer, text: &str) {
        p.clipboard = Clipboard::Memory(text.to_string());
    }

    pub fn clipboard(p: &Pointer) -> String {
        match &p.clipboard {
            Clipboard::Memory(text) => text.clone(),
            Clipboard::System { .. } => String::new(),
        }
    }

    /// The screen point of byte boundary `at` in box `id`'s drawn layout,
    /// at the middle of its line.
    pub fn point_of(dom: &Dom, root: usize, size: (f32, f32), id: usize, at: usize) -> (f32, f32) {
        let placed = render::display_list(dom, root, size.0, size.1)
            .into_iter()
            .find(|p| p.id == id)
            .expect("placed");
        let laid = render::edit_layout(dom, id, placed.rect).expect("layout");
        let line = laid
            .lines
            .iter()
            .find(|l| at >= l.start && at <= l.end)
            .expect("line");
        (line.x + line.stop_x(at), line.top + laid.line_height / 2.0)
    }

    /// Pretend the focused box's caret cycle began `ms` ago.
    pub fn age_blink(dom: &mut Dom, ms: u64) {
        if let Some(e) = dom.editing.as_mut() {
            e.since = Instant::now() - Duration::from_millis(ms);
        }
    }

    /// Whether the frame loop would repaint `ms` from now for the caret alone.
    pub fn caret_due_in(dom: &Dom, ms: u64) -> bool {
        dom.caret_due(Instant::now() + Duration::from_millis(ms))
    }
}

const SIZE: (f32, f32) = (400.0, 200.0);

/// The probe's box, as the global `tb`, logging every signal the probe logged
/// into the global `log` as `tag cursor selection "text"`.
const PROBE: &str = r#"
    tb = Instance.new("TextBox")
    tb.Size = UDim2.fromOffset(300, 40)
    tb.Position = UDim2.fromOffset(40, 120)
    tb.BackgroundColor3 = Color3.fromRGB(255, 255, 255)
    tb.TextColor3 = Color3.fromRGB(0, 0, 0)
    tb.Font = Enum.Font.Arial
    tb.TextSize = 20
    tb.TextXAlignment = Enum.TextXAlignment.Left
    tb.ClearTextOnFocus = false
    tb.Text = "hello world"
    tb.Parent = root
    local function state(tag)
        table.insert(log, string.format("%s %d %d %q", tag, tb.CursorPosition, tb.SelectionStart, tb.Text))
    end
    tb:GetPropertyChangedSignal("CursorPosition"):Connect(function() state("cursor") end)
    tb:GetPropertyChangedSignal("SelectionStart"):Connect(function() state("selection") end)
    tb:GetPropertyChangedSignal("Text"):Connect(function() state("text") end)
    tb.Focused:Connect(function() state("Focused") end)
    tb.FocusLost:Connect(function(enter) state("FocusLost " .. tostring(enter)) end)
"#;

struct Harness {
    lua: Lua,
    dom: SharedDom,
    root: usize,
    pointer: Pointer,
}

impl Harness {
    /// `None` on a machine with no font, where nothing can be laid out.
    fn new(src: &str) -> Option<Harness> {
        crate::services::default_face()?;
        let lua = Lua::new();
        let dom = SharedDom::default();
        install(&lua, &dom).expect("install");
        install_vocabulary(&lua).expect("vocabulary");
        let root = dom
            .lock()
            .expect("dom")
            .insert("ScreenGui".into(), "DewRoot".into());
        lua.globals()
            .set("root", handle(&lua, &dom, root).expect("root handle"))
            .expect("root");
        lua.globals()
            .set("log", lua.create_table().expect("table"))
            .expect("log");
        lua.load(src).exec().expect("guest");
        let mut pointer = Pointer::default();
        shim::use_memory_clipboard(&mut pointer, "");
        Some(Harness {
            lua,
            dom,
            root,
            pointer,
        })
    }

    fn drive(&mut self, f: impl FnOnce(&mut Pointer, &Surface) -> LuaResult<()>) {
        let Harness {
            lua,
            dom,
            root,
            pointer,
        } = self;
        let surface = Surface {
            lua,
            dom,
            root: *root,
            size: SIZE,
        };
        f(pointer, &surface).expect("input");
    }

    fn key(&mut self, name: &str) {
        self.drive(|p, s| shim::key(p, s, name, false, false));
    }

    fn shift(&mut self, name: &str) {
        self.drive(|p, s| shim::key(p, s, name, true, false));
    }

    fn ctrl(&mut self, name: &str) {
        self.drive(|p, s| shim::key(p, s, name, false, true));
    }

    fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.drive(|p, s| p.char(s, c));
        }
    }

    fn down(&mut self, x: f32, y: f32) {
        self.drive(|p, s| p.down(s, Button::Left, x, y));
    }

    fn up(&mut self, x: f32, y: f32) {
        self.drive(|p, s| p.up(s, Button::Left, x, y));
    }

    fn moved(&mut self, x: f32, y: f32) {
        self.drive(|p, s| p.moved(s, x, y));
    }

    fn click(&mut self, x: f32, y: f32) {
        self.down(x, y);
        self.up(x, y);
    }

    /// The point of byte boundary `at` in the global box `name`.
    fn at(&self, name: &str, at: usize) -> (f32, f32) {
        let id = self.id(name);
        let guard = self.dom.lock().expect("dom");
        shim::point_of(&guard, self.root, SIZE, id, at)
    }

    /// A click on byte boundary `at` of `tb`.
    fn click_at(&mut self, at: usize) {
        let (x, y) = self.at("tb", at);
        self.click(x, y);
    }

    fn id(&self, name: &str) -> usize {
        let found: LuaAnyUserData = self.lua.globals().get(name).expect("global");
        found
            .borrow::<super::InstanceRef>()
            .expect("an instance")
            .id
    }

    /// `CursorPosition`, `SelectionStart`, `Text` of `tb`.
    fn state(&self) -> (i64, i64, String) {
        self.lua
            .load("return tb.CursorPosition, tb.SelectionStart, tb.Text")
            .eval()
            .expect("state")
    }

    /// Everything logged since the last call, and clear it.
    fn take_log(&self) -> Vec<String> {
        let lines: Vec<String> = self
            .lua
            .load("local out = table.clone(log); table.clear(log); return out")
            .eval()
            .expect("log");
        lines
    }

    /// Lay out and paint, white background, BGRA.
    fn paint(&self) -> Vec<u8> {
        super::render::paint_white(&self.dom, self.root, SIZE.0, SIZE.1).expect("pixels")
    }
}

/// The colour at (x, y) of a BGRA buffer the size of the surface, as RGB.
fn rgb(px: &[u8], x: i32, y: i32) -> (u8, u8, u8) {
    let i = ((y as usize) * SIZE.0 as usize + x as usize) * 4;
    (px[i + 2], px[i + 1], px[i])
}

fn dark((r, g, b): (u8, u8, u8)) -> bool {
    r < 40 && g < 40 && b < 40
}

fn near((r, g, b): (u8, u8, u8), (er, eg, eb): (u8, u8, u8)) -> bool {
    r.abs_diff(er) <= 3 && g.abs_diff(eg) <= 3 && b.abs_diff(eb) <= 3
}

// ── The measured rows ──────────────────────────────────────────────────────

/// Never focused: `CursorPosition` 1, `SelectionStart` -1.
#[test]
fn a_box_never_focused_reads_cursor_1_and_no_selection() {
    let Some(h) = Harness::new(PROBE) else { return };
    assert_eq!(h.state(), (1, -1, "hello world".into()));
}

/// A click between "hello" and " world": `CursorPosition` 6, fired before
/// `Focused`.
#[test]
fn a_click_places_the_cursor_and_then_focuses() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    assert_eq!(h.state(), (6, -1, "hello world".into()));
    assert_eq!(
        h.take_log(),
        [
            r#"cursor 6 -1 "hello world""#,
            r#"Focused 6 -1 "hello world""#
        ]
    );
}

/// Left, Left: 5, 4. Home: 1. End: 12.
#[test]
fn arrows_home_and_end_move_the_cursor() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.key("Left");
    assert_eq!(h.state().0, 5);
    h.key("Left");
    assert_eq!(h.state().0, 4);
    h.key("Home");
    assert_eq!(h.state().0, 1);
    h.key("End");
    assert_eq!(h.state().0, 12);
}

/// Shift+Left three times from 12: 11, 10, 9, anchored at 12.
#[test]
fn shift_left_extends_the_selection_from_its_anchor() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.key("End");
    for expected in [11, 10, 9] {
        h.shift("Left");
        assert_eq!(h.state(), (expected, 12, "hello world".into()));
    }
}

/// Ctrl+A: `CursorPosition` 12, `SelectionStart` 1.
#[test]
fn ctrl_a_selects_everything() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.ctrl("A");
    assert_eq!(h.state(), (12, 1, "hello world".into()));
}

/// Typing "x" over the selection: `Text` "x", `CursorPosition` 2,
/// `SelectionStart` -1, fired as `CursorPosition`, `SelectionStart`,
/// `CursorPosition`, `Text`.
#[test]
fn typing_over_a_selection_replaces_it_in_the_engines_order() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.ctrl("A");
    h.take_log();
    h.type_text("x");
    assert_eq!(h.state(), (2, -1, "x".into()));
    let order: Vec<String> = h
        .take_log()
        .iter()
        .map(|l| l.split(' ').next().unwrap().to_string())
        .collect();
    assert_eq!(order, ["cursor", "selection", "cursor", "text"]);
}

/// Ctrl+Z after that: `Text` back to "hello world" and `CursorPosition` 6,
/// where the first click put it.
#[test]
fn ctrl_z_restores_the_text_and_the_cursor_it_was_entered_with() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.key("Left");
    h.key("Left");
    h.key("Home");
    h.key("End");
    h.shift("Left");
    h.ctrl("A");
    h.type_text("x");
    h.ctrl("Z");
    assert_eq!(h.state(), (6, -1, "hello world".into()));
}

/// A double click on "world": `SelectionStart` 7, `CursorPosition` 12.
#[test]
fn a_double_click_selects_the_word() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    let (x, y) = h.at("tb", 8);
    h.click(x, y);
    h.click(x, y);
    assert_eq!(h.state(), (12, 7, "hello world".into()));
}

/// A click outside: `CursorPosition` -1, `SelectionStart` -1, then
/// `FocusLost(false)`, in that order.
#[test]
fn a_click_outside_clears_the_cursor_then_loses_focus() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.ctrl("A");
    h.take_log();
    h.click(5.0, 5.0);
    assert_eq!(
        h.take_log(),
        [
            r#"cursor -1 1 "hello world""#,
            r#"selection -1 -1 "hello world""#,
            r#"FocusLost false -1 -1 "hello world""#
        ]
    );
}

/// Clicking the box again: the cursor lands where clicked, 8, not where it
/// was.
#[test]
fn refocusing_by_a_click_puts_the_cursor_where_clicked() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.click(5.0, 5.0);
    h.click_at(7);
    assert_eq!(h.state(), (8, -1, "hello world".into()));
}

// ── Editing beyond the measured rows ───────────────────────────────────────

/// Backspace removes the character before the cursor and Delete the one
/// after it; typing inserts at the cursor.
#[test]
fn backspace_delete_and_typing_work_at_the_cursor() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.key("Backspace");
    assert_eq!(h.state(), (5, -1, "hell world".into()));
    h.key("Delete");
    assert_eq!(h.state(), (5, -1, "hellworld".into()));
    h.type_text("o, ");
    assert_eq!(h.state(), (8, -1, "hello, world".into()));
    h.ctrl("Z");
    assert_eq!(h.state().2, "hellworld", "typing is one undo step");
}

/// Ctrl+Left and Ctrl+Right move by word, Ctrl+Backspace deletes one.
#[test]
fn ctrl_moves_and_deletes_by_word() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.key("End");
    h.ctrl("Left");
    assert_eq!(h.state().0, 7);
    h.ctrl("Left");
    assert_eq!(h.state().0, 1);
    h.ctrl("Right");
    assert_eq!(h.state().0, 7);
    h.key("End");
    h.ctrl("Backspace");
    assert_eq!(h.state(), (7, -1, "hello ".into()));
}

/// Ctrl+C copies the selection, Ctrl+X cuts it, Ctrl+V pastes at the cursor
/// with newlines dropped.
#[test]
fn the_clipboard_copies_cuts_and_pastes() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.key("End");
    for _ in 0..5 {
        h.shift("Left");
    }
    h.ctrl("C");
    assert_eq!(shim::clipboard(&h.pointer), "world");
    h.ctrl("X");
    assert_eq!(h.state(), (7, -1, "hello ".into()));
    shim::use_memory_clipboard(&mut h.pointer, "big\r\nwide");
    h.ctrl("V");
    assert_eq!(h.state(), (14, -1, "hello bigwide".into()));
}

/// A drag from inside the focused box selects up to the pointer, and a
/// Shift+click extends the selection.
#[test]
fn a_drag_and_a_shift_click_select() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    let (x0, y) = h.at("tb", 0);
    let (x1, _) = h.at("tb", 5);
    // Far enough apart in time that this is not a double click.
    h.pointer = {
        let mut p = Pointer::default();
        shim::use_memory_clipboard(&mut p, "");
        p
    };
    h.down(x0, y);
    h.moved(x1, y);
    h.up(x1, y);
    assert_eq!(h.state(), (6, 1, "hello world".into()));

    let (x2, _) = h.at("tb", 8);
    shim::set_shift(&mut h.pointer, true);
    h.click(x2, y);
    shim::set_shift(&mut h.pointer, false);
    assert_eq!(h.state(), (9, 1, "hello world".into()));
}

/// A script's `CursorPosition` is clamped to the text and kept by
/// `CaptureFocus`; below 1 it reads -1.
#[test]
fn a_scripted_cursor_is_clamped_and_kept_by_capture_focus() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    let (c, s): (i64, i64) = h
        .lua
        .load("tb.CursorPosition = 99; tb.SelectionStart = 0; return tb.CursorPosition, tb.SelectionStart")
        .eval()
        .expect("clamp");
    assert_eq!((c, s), (12, -1));
    h.lua
        .load("tb.CursorPosition = 3; tb:CaptureFocus()")
        .exec()
        .expect("focus");
    h.type_text("_");
    assert_eq!(h.state(), (4, -1, "he_llo world".into()));
}

/// Return still releases focus as `FocusLost(true)`, after the cursor and
/// selection clear.
#[test]
fn return_releases_focus_with_enter_pressed() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    h.take_log();
    h.key("Return");
    assert_eq!(
        h.take_log(),
        [
            r#"cursor -1 -1 "hello world""#,
            r#"FocusLost true -1 -1 "hello world""#
        ]
    );
}

/// Aether's proxy: an invisible box focused by script still edits at the
/// cursor the script gave it.
#[test]
fn an_invisible_proxy_still_edits_at_its_scripted_cursor() {
    let Some(mut h) = Harness::new(
        r#"
        proxy = Instance.new("TextBox")
        proxy.Visible = false
        proxy.Text = "abc"
        proxy.Parent = root
        proxy.SelectionStart = -1
        proxy.CursorPosition = 2
        proxy:CaptureFocus()
    "#,
    ) else {
        return;
    };
    h.type_text("X");
    h.key("Backspace");
    h.key("Backspace");
    let (text, cursor): (String, i64) = h
        .lua
        .load("return proxy.Text, proxy.CursorPosition")
        .eval()
        .expect("proxy");
    assert_eq!((text.as_str(), cursor), ("bc", 1));
}

// ── Pixels ─────────────────────────────────────────────────────────────────

/// The caret: one pixel wide, `TextColor3`, as tall as the line (20 at
/// TextSize 20), on the line's own span.
#[test]
fn the_caret_is_one_pixel_of_text_colour_the_height_of_the_line() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    let px = h.paint();
    let (cx, cy) = h.at("tb", 5);
    let x = cx.round() as i32;
    let top = (cy - 10.0).round() as i32;
    for y in top..top + 20 {
        assert!(
            dark(rgb(&px, x, y)),
            "caret pixel ({x}, {y}) is {:?}",
            rgb(&px, x, y)
        );
    }
    assert!(
        !dark(rgb(&px, x, top - 1)),
        "the caret is 20 tall, not taller"
    );
    assert!(
        !dark(rgb(&px, x, top + 20)),
        "the caret is 20 tall, not taller"
    );
    // The top row is above every glyph, so its neighbours are background.
    assert!(
        near(rgb(&px, x - 1, top), (255, 255, 255)),
        "one pixel wide"
    );
    assert!(
        near(rgb(&px, x + 1, top), (255, 255, 255)),
        "one pixel wide"
    );
}

/// The selection: (107, 161, 249) behind the glyphs, the line's height, and
/// the glyphs inside it white.
#[test]
fn the_selection_is_highlighted_line_tall_with_white_glyphs() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    let (x, y) = h.at("tb", 8);
    h.click(x, y);
    h.click(x, y);
    // Let the caret blink off, so only the highlight is at its edge.
    shim::age_blink(&mut h.dom.lock().expect("dom"), 600);
    let px = h.paint();
    let (x0, cy) = h.at("tb", 6);
    let (x1, _) = h.at("tb", 11);
    let top = (cy - 10.0).round() as i32;
    let (x0, x1) = (x0.round() as i32, x1.round() as i32);
    let highlight = (107, 161, 249);
    assert!(
        near(rgb(&px, x0 + 1, top), highlight),
        "{:?}",
        rgb(&px, x0 + 1, top)
    );
    assert!(
        near(rgb(&px, x1 - 1, top + 19), highlight),
        "the full line tall"
    );
    assert!(
        near(rgb(&px, x0 + 1, top - 1), (255, 255, 255)),
        "and no taller"
    );
    assert!(
        near(rgb(&px, x0 + 1, top + 20), (255, 255, 255)),
        "and no taller"
    );
    assert!(
        near(rgb(&px, x0 - 2, top), (255, 255, 255)),
        "starts at the word"
    );
    let mut white = 0;
    let mut black = 0;
    for yy in top..top + 20 {
        for xx in x0..x1 {
            let c = rgb(&px, xx, yy);
            white += near(c, (255, 255, 255)) as usize;
            black += dark(c) as usize;
        }
    }
    assert!(
        white > 20,
        "selected glyphs are white: {white} white pixels"
    );
    assert_eq!(
        black, 0,
        "no glyph inside the selection is drawn in TextColor3"
    );
}

/// The caret blinks 500 ms shown and 500 ms hidden, and the frame loop is
/// owed a repaint only when it flips.
#[test]
fn the_caret_blinks_and_repaints_only_on_a_flip() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    let (cx, cy) = h.at("tb", 5);
    let (x, y) = (cx.round() as i32, (cy - 10.0).round() as i32);

    let shown = h.paint();
    assert!(dark(rgb(&shown, x, y)), "shown straight after the click");
    {
        let guard = h.dom.lock().expect("dom");
        assert!(
            !shim::caret_due_in(&guard, 0),
            "nothing owed while it stays shown"
        );
        assert!(shim::caret_due_in(&guard, 600), "owed once it hides");
    }

    shim::age_blink(&mut h.dom.lock().expect("dom"), 600);
    let hidden = h.paint();
    assert!(!dark(rgb(&hidden, x, y)), "hidden 600 ms in");

    shim::age_blink(&mut h.dom.lock().expect("dom"), 1100);
    let again = h.paint();
    assert!(dark(rgb(&again, x, y)), "shown again 1100 ms in");

    // A move restarts the cycle: shown at once.
    shim::age_blink(&mut h.dom.lock().expect("dom"), 600);
    h.key("Left");
    h.key("Right");
    let moved = h.paint();
    assert!(dark(rgb(&moved, x, y)), "a move shows the caret");
}

/// An invisible box is never painted with a caret, and owes no repaint.
#[test]
fn an_invisible_focused_box_paints_no_caret_and_owes_no_repaint() {
    let Some(h) = Harness::new(
        r#"
        holder = Instance.new("Frame")
        holder.Visible = false
        holder.Parent = root
        proxy = Instance.new("TextBox")
        proxy.Size = UDim2.fromOffset(300, 40)
        proxy.Text = "secret"
        proxy.Parent = holder
        proxy:CaptureFocus()
    "#,
    ) else {
        return;
    };
    let px = h.paint();
    assert!(px
        .chunks(4)
        .all(|p| p[0] == 255 && p[1] == 255 && p[2] == 255));
    let guard = h.dom.lock().expect("dom");
    for ms in [0, 600, 1100] {
        assert!(!shim::caret_due_in(&guard, ms));
    }
}

/// An unwrapped line wider than its box scrolls to keep the caret inside.
#[test]
fn a_long_line_scrolls_to_keep_the_caret_in_view() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.lua
        .load(r#"tb.Text = string.rep("wide ", 30)"#)
        .exec()
        .expect("text");
    h.click_at(5);
    h.key("End");
    let px = h.paint();
    let caret_columns: Vec<i32> = (40..340)
        .filter(|&x| (130..150).all(|y| dark(rgb(&px, x, y))))
        .collect();
    assert_eq!(
        caret_columns.len(),
        1,
        "one caret column inside the box: {caret_columns:?}"
    );
    assert!(
        caret_columns[0] > 300,
        "at the right edge: {caret_columns:?}"
    );
    // Nothing is drawn past the box.
    for x in 341..400 {
        assert!(
            near(rgb(&px, x, 140), (255, 255, 255)),
            "text escaped the box at {x}"
        );
    }
    h.key("Home");
    let px = h.paint();
    assert!(
        (130..150).all(|y| dark(rgb(&px, 40, y))),
        "back at the start"
    );
}

/// Idle, a focused box costs one repaint per blink flip: 180 frames at 60 Hz
/// over three seconds paint the first frame and five flips, and nothing else.
/// This is the frame loop's own test, `take_dirty` or `caret_due`, run on a
/// simulated clock.
#[test]
fn an_idle_focused_box_repaints_only_when_the_caret_flips() {
    let Some(mut h) = Harness::new(PROBE) else {
        return;
    };
    h.click_at(5);
    let mut painted = 0;
    for frame in 0..180u64 {
        let due = {
            let mut guard = h.dom.lock().expect("dom");
            shim::age_blink(&mut guard, frame * 1000 / 60);
            let caret = shim::caret_due_in(&guard, 0);
            guard.take_dirty() || caret
        };
        if due {
            painted += 1;
            super::render::frame_of(&h.dom, h.root, SIZE.0, SIZE.1);
        }
    }
    eprintln!("180 frames over 3 s at 60 Hz, painted {painted}");
    assert_eq!(painted, 6, "the first frame and five flips");
}
