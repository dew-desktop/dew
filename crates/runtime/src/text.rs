//! The text line model: which lines a string breaks into, where each one sits,
//! and what `TextBounds` and `TextFits` read.
//!
//! EVERY TEXT PATH ASKS THIS MODULE. AutomaticSize and `desktop.Text.Measure`
//! call [`measure`], TextScaled calls [`scaled_size`], and the display list
//! carries the [`TextLayout`] that [`lay_out`] returns, which the painter draws
//! line by line and the host reports as `TextBounds` and `TextFits`. The painter
//! once chose its own top edge, one way for wrapped text and another for
//! unwrapped, and a single line of text moved when `TextWrapped` changed. With
//! one function there is no second answer to drift from the first.
//!
//! WHAT THE ENGINE DOES, measured in Studio at TextSize 14, 20 and 32 for three
//! families, wrapped and unwrapped:
//!
//! - Every line sits in a line box one EFFECTIVE EM tall. The effective em is
//!   TextSize times the face's em scale, which is 1.5 for the Legacy families
//!   and 1.0 for the rest.
//! - Wrapped and unwrapped text with the same lines place identically.
//! - The glyphs are centred in the line box by their ascent plus descent
//!   ([`Face::glyph_top`]).
//! - A wrapped line that does not fit the box height is not drawn, and
//!   `TextBounds` counts only the lines that are.
//!
//! THE FACE IS A PARAMETER. Which file a label draws with, its em scale and its
//! metrics are decided by whoever resolves fonts; this module takes the answer
//! and never looks a face up itself.

use crate::frame::{Align, Rect};

/// Glyph advances for one font: the only thing the model asks of a font file.
pub trait Advance {
    /// Width of `text` drawn with glyphs `px` pixels tall, or `None` when the
    /// font cannot answer. `None` is a failure, never a zero width: a zero
    /// width collapses whatever asked.
    fn advance(&self, px: f32, text: &str) -> Option<f32>;

    /// The rasteriser's id for this font, which a [`TextLayout`] carries so the
    /// painter draws in the face the lines were measured in. `None` for a font
    /// that is not in the rasteriser's store.
    fn raster_id(&self) -> Option<u32> {
        None
    }

    /// Whether the font draws `ch` with a glyph of its own. A font that cannot
    /// say is treated as lacking it.
    fn has_glyph(&self, _ch: char) -> bool {
        false
    }
}

#[cfg(feature = "raster")]
impl Advance for dew_raster::Font {
    fn advance(&self, px: f32, text: &str) -> Option<f32> {
        self.width(px, text)
    }

    fn has_glyph(&self, ch: char) -> bool {
        dew_raster::Font::has_glyph(*self, ch)
    }

    fn raster_id(&self) -> Option<u32> {
        Some(self.id())
    }
}

/// A face, as the line model needs it.
///
/// `font` answers advances. The other four numbers turn a TextSize into a line
/// box and a glyph size, and say where the glyphs sit in that box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face<F> {
    pub font: F,
    /// Effective em per unit of TextSize. The line box is
    /// `TextSize * em_scale` tall.
    pub em_scale: f32,
    /// Glyph size per pixel of effective em: what is handed to the rasteriser
    /// as the size of a run.
    pub glyph_per_em: f32,
    /// The font's ascent per pixel of glyph size, as `fill_text` uses it to
    /// find the baseline.
    pub ascent: f32,
    /// The font's descent per pixel of glyph size, positive.
    pub descent: f32,
}

/// The em scale of the Legacy families, LegacyArial among them, which is the
/// default `FontFace` of every new text object.
pub const LEGACY_EM_SCALE: f32 = 1.5;

impl<F> Face<F> {
    /// Height of one line box.
    pub fn effective_em(&self, text_size: f32) -> f32 {
        text_size * self.em_scale
    }

    /// The size glyphs are drawn at.
    pub fn glyph_px(&self, text_size: f32) -> f32 {
        self.effective_em(text_size) * self.glyph_per_em
    }

    /// Where the top of a run goes, for a line box starting at `line_top`.
    ///
    /// THE PLACEMENT RULE, AND THE ONLY COPY OF IT: the face's ascent plus
    /// descent is centred in the line box. The alternative fitted against the
    /// same engine screenshots, a baseline at ascent over ascent plus descent
    /// of the line height, is the same rule whenever the glyphs exactly fill
    /// the box and lands up to 3 px lower when they do not.
    ///
    /// A TOP, NOT A BASELINE, because `fill_text` takes a top edge and adds
    /// the ascent itself. Handing it a baseline draws every run a line low.
    pub fn glyph_top(&self, line_top: f32, line_height: f32, glyph_px: f32) -> f32 {
        line_top + (line_height - (self.ascent + self.descent) * glyph_px) / 2.0
    }
}

#[cfg(feature = "raster")]
impl Face<dew_raster::Font> {
    /// A face file drawn at `em_scale`, with its metrics read from the file.
    ///
    /// THE GLYPHS FILL THE LINE BOX. The engine scales a face so that its
    /// ascent plus descent is one effective em: Arimo's "Hamburgefonts" at
    /// TextSize 20 is 123 px wide where drawing it 20 px to the em would make
    /// it 137, and LegacyArial's is 1.5 times that, 184. So the glyph size
    /// per effective em is one over the face's ascent plus descent, and a
    /// legacy family differs from a modern one only in `em_scale`.
    pub fn from_font(font: dew_raster::Font, em_scale: f32) -> Self {
        let ascent = font.ascent(1.0);
        let descent = font.descent(1.0);
        let extent = ascent + descent;
        Face {
            font,
            em_scale,
            glyph_per_em: if extent > 0.0 { 1.0 / extent } else { 1.0 },
            ascent,
            descent,
        }
    }
}

/// What to lay out, and into what.
#[derive(Debug, Clone, Copy)]
pub struct Block<'a> {
    pub text: &'a str,
    /// The TextSize to draw at, after TextScaled.
    pub text_size: f32,
    /// The label's box inset by its UIPadding.
    pub content: Rect,
    pub wrap: bool,
    pub align_x: Align,
    pub align_y: Align,
    /// `TextTruncate` is `AtEnd`: a line wider than the content box, and the
    /// last line drawn when wrapped lines were dropped, end in an ellipsis.
    pub truncate: bool,
    /// `LineHeight`, as a multiple of the line box: the step from one line's
    /// top to the next.
    pub line_height: f32,
    /// Whether each line should carry its caret [`Line::stops`]. Only an
    /// edited `TextBox` asks, because finding them measures every prefix of
    /// the line.
    pub stops: bool,
}

/// One visible line, placed.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    /// The top-left `fill_text` takes.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    /// The top of this line's box, which `y` is centred in. A caret and a
    /// selection highlight span the box, not the glyphs.
    pub top: f32,
    /// The byte range of the laid out string this line was broken from. A
    /// wrapped line leaves out the spaces it broke at, so consecutive lines
    /// need not touch.
    pub start: usize,
    pub end: usize,
    /// Every character boundary from `start` to `end`, as a byte offset into
    /// the laid out string and the pen position there, measured from `x`.
    /// Empty unless [`Block::stops`] asked for them. These are the same
    /// advances the line was measured with, so a caret placed from them sits
    /// where the painter put the glyphs.
    pub stops: Vec<(usize, f32)>,
}

impl Line {
    /// The pen position of byte offset `at`, measured from `x`: the nearest
    /// stop at or before it, the line's start before it, its end past it.
    pub fn stop_x(&self, at: usize) -> f32 {
        let mut x = 0.0;
        for &(offset, sx) in &self.stops {
            if offset > at {
                break;
            }
            x = sx;
        }
        x
    }

    /// The byte offset of the stop nearest to `x`, measured from the line's
    /// own `x`.
    pub fn nearest_stop(&self, x: f32) -> usize {
        let mut best = (self.start, f32::INFINITY);
        for &(offset, sx) in &self.stops {
            let d = (sx - x).abs();
            if d < best.1 {
                best = (offset, d);
            }
        }
        best.0
    }
}

/// A laid-out label: what is drawn, and what `TextBounds` and `TextFits` say.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    /// The rasteriser id of the face the lines were broken and measured in,
    /// from [`Advance::raster_id`]. The painter draws in this face, not one
    /// of its own.
    pub face: Option<u32>,
    /// The TextSize this was laid out at, after TextScaled.
    pub text_size: f32,
    /// The size every line is drawn at.
    pub glyph_px: f32,
    pub line_height: f32,
    /// The lines that are drawn, top to bottom. Lines that did not fit are
    /// not here.
    pub lines: Vec<Line>,
    /// `TextBounds`: the widest drawn line by the height of the drawn lines.
    pub bounds: (f32, f32),
    /// `TextFits`: every line drawn, and all of it inside the content box.
    pub fits: bool,
}

/// How far a size may exceed its box and still fit. Widths and heights are
/// sums of floats, and an AutomaticSize box is exactly as large as the text
/// that grew it.
const SLACK: f32 = 0.01;

/// One broken line: its text, its width and the byte range of `text` it
/// came from.
struct Span {
    text: String,
    width: f32,
    start: usize,
    end: usize,
}

/// Break `text` into lines with their widths.
///
/// Paragraphs split on `\n`, wrapped or not. With a width, words are packed
/// greedily against it, and a word wider than the width on its own is broken
/// between characters rather than left to overflow.
#[cfg(test)]
fn break_lines<F: Advance>(
    font: &F,
    text: &str,
    px: f32,
    max_width: Option<f32>,
) -> Option<Vec<(String, f32)>> {
    Some(
        break_spans(font, text, px, max_width)?
            .into_iter()
            .map(|span| (span.text, span.width))
            .collect(),
    )
}

/// The lines `text` breaks into, keeping where in `text` each came from.
fn break_spans<F: Advance>(
    font: &F,
    text: &str,
    px: f32,
    max_width: Option<f32>,
) -> Option<Vec<Span>> {
    let Some(max_width) = max_width.filter(|w| *w > 0.0) else {
        let mut offset = 0;
        let mut out = Vec::new();
        for line in text.split('\n') {
            out.push(Span {
                text: line.to_string(),
                width: font.advance(px, line)?,
                start: offset,
                end: offset + line.len(),
            });
            offset += line.len() + 1;
        }
        return Some(out);
    };

    let mut lines: Vec<Span> = Vec::new();
    let mut paragraph_at = 0;
    for paragraph in text.split('\n') {
        let before = lines.len();
        let mut current = String::new();
        let mut current_w = 0.0_f32;
        let mut current_start = paragraph_at;
        let mut current_end = paragraph_at;

        let mut word_at = paragraph_at;
        for word in paragraph.split(' ') {
            let at = word_at;
            word_at += word.len() + 1;
            if word.is_empty() {
                continue;
            }
            let word_w = font.advance(px, word)?;
            if word_w > max_width {
                if !current.is_empty() {
                    lines.push(Span {
                        text: std::mem::take(&mut current),
                        width: current_w,
                        start: current_start,
                        end: current_end,
                    });
                }
                for (i, ch) in word.char_indices() {
                    let ch_at = at + i;
                    let candidate = format!("{current}{ch}");
                    let candidate_w = font.advance(px, &candidate)?;
                    if current.is_empty() || candidate_w <= max_width {
                        if current.is_empty() {
                            current_start = ch_at;
                        }
                        current = candidate;
                        current_w = candidate_w;
                    } else {
                        lines.push(Span {
                            text: std::mem::take(&mut current),
                            width: current_w,
                            start: current_start,
                            end: current_end,
                        });
                        current = ch.to_string();
                        current_w = font.advance(px, &current)?;
                        current_start = ch_at;
                    }
                    current_end = ch_at + ch.len_utf8();
                }
                continue;
            }

            if current.is_empty() {
                current = word.to_string();
                current_w = word_w;
                current_start = at;
                current_end = at + word.len();
                continue;
            }
            let candidate = format!("{current} {word}");
            let candidate_w = font.advance(px, &candidate)?;
            if candidate_w <= max_width {
                current = candidate;
                current_w = candidate_w;
                current_end = at + word.len();
            } else {
                lines.push(Span {
                    text: std::mem::take(&mut current),
                    width: current_w,
                    start: current_start,
                    end: current_end,
                });
                current = word.to_string();
                current_w = word_w;
                current_start = at;
                current_end = at + word.len();
            }
        }

        // A paragraph that never pushed a line still owes one: an empty
        // paragraph between two newlines is a blank line, not nothing.
        if !current.is_empty() || lines.len() == before {
            if current.is_empty() {
                current_start = paragraph_at;
                current_end = paragraph_at;
            }
            lines.push(Span {
                text: current,
                width: current_w,
                start: current_start,
                end: current_end,
            });
        }
        paragraph_at += paragraph.len() + 1;
    }
    Some(lines)
}

/// The caret stops of one line: every character boundary of `source` from
/// `start` to `end`, with the pen position of the drawn `text` there.
///
/// THE DRAWN TEXT IS WALKED BESIDE THE SOURCE, because they differ in two
/// ways. A wrapped line keeps one space where the source had several, so an
/// extra source space takes no width; and a truncated line ends early in an
/// ellipsis, so every boundary past the cut sits at the cut.
fn stops_of<F: Advance>(
    font: &F,
    px: f32,
    source: &str,
    start: usize,
    end: usize,
    text: &str,
) -> Vec<(usize, f32)> {
    let mut stops = vec![(start, 0.0)];
    let mut drawn = text.char_indices().peekable();
    let mut x = 0.0;
    let mut matching = true;
    for (i, ch) in source[start..end].char_indices() {
        if matching {
            match drawn.peek() {
                Some(&(j, d)) if d == ch => {
                    drawn.next();
                    x = font.advance(px, &text[..j + d.len_utf8()]).unwrap_or(x);
                }
                _ if ch == ' ' => {}
                _ => matching = false,
            }
        }
        stops.push((start + i + ch.len_utf8(), x));
    }
    stops
}

/// The size `text` takes with every line drawn: the widest line, and the
/// height of all of them. What AutomaticSize grows a box to.
///
/// `wrap_width` breaks lines against a width; `None` breaks only at `\n`.
pub fn measure<F: Advance>(
    face: &Face<F>,
    text: &str,
    text_size: f32,
    wrap_width: Option<f32>,
) -> Option<(f32, f32)> {
    measure_spaced(face, text, text_size, wrap_width, 1.0)
}

/// [`measure`], with line spacing given by `line_height_mul`.
pub fn measure_spaced<F: Advance>(
    face: &Face<F>,
    text: &str,
    text_size: f32,
    wrap_width: Option<f32>,
    line_height_mul: f32,
) -> Option<(f32, f32)> {
    let lines = break_spans(&face.font, text, face.glyph_px(text_size), wrap_width)?;
    let width = lines.iter().fold(0.0_f32, |w, span| w.max(span.width));
    let base_h = face.effective_em(text_size);
    let height = if lines.is_empty() {
        0.0
    } else {
        base_h + (lines.len() - 1) as f32 * (base_h * line_height_mul)
    };
    Some((width, height))
}

/// What a truncated line ends in: the engine draws one HORIZONTAL ELLIPSIS
/// (U+2026), so a face with that glyph ends in it, and only a face without it
/// falls back to three full stops rather than drawing a box.
fn ellipsis<F: Advance>(font: &F) -> &'static str {
    if font.has_glyph('\u{2026}') {
        "\u{2026}"
    } else {
        "..."
    }
}

/// Lay a label's text out in its content box.
pub fn lay_out<F: Advance>(face: &Face<F>, block: &Block) -> Option<TextLayout> {
    let base_line_h = face.effective_em(block.text_size);
    let glyph_px = face.glyph_px(block.text_size);
    let content = block.content;
    let all = break_spans(
        &face.font,
        block.text,
        glyph_px,
        block.wrap.then_some(content.w),
    )?;

    // WRAPPED LINES THAT DO NOT FIT ARE DROPPED WHOLE, not clipped and not
    // left to overflow. The first line always stays: a label shorter than its
    // own line still shows it.
    let step = base_line_h * block.line_height;
    let visible = if block.wrap && base_line_h > 0.0 {
        if content.h + SLACK < base_line_h {
            1
        } else if step > 0.0 {
            let extra = ((content.h + SLACK - base_line_h) / step).floor() as usize;
            all.len().min(1 + extra)
        } else {
            all.len()
        }
    } else {
        all.len()
    };

    let block_h = if visible == 0 {
        0.0
    } else {
        base_line_h + (visible - 1) as f32 * step
    };
    let top = match block.align_y {
        Align::Start => content.y,
        Align::Center => content.y + (content.h - block_h) / 2.0,
        Align::End => content.y + content.h - block_h,
    };

    // A CUT LINE ENDS IN THE ELLIPSIS: one wider than the box, and the last
    // line drawn when wrapped lines below it were dropped. That last line
    // keeps as much of itself as fits beside the ellipsis.
    let ellipsis = ellipsis(&face.font);
    let dropped = visible < all.len();
    let mut truncated_any = false;
    let lines: Vec<Line> = all
        .iter()
        .take(visible)
        .enumerate()
        .map(|(i, span)| {
            let mut line_text = span.text.clone();
            let mut line_width = span.width;
            let cut = line_width > content.w + SLACK || (dropped && i + 1 == visible);
            if block.truncate && cut && content.w > 0.0 {
                if let Some(ellipsis_w) = face.font.advance(glyph_px, ellipsis) {
                    if ellipsis_w <= content.w + SLACK {
                        let target_w = (content.w - ellipsis_w).max(0.0);
                        let mut prefix = String::new();
                        let mut prefix_w = 0.0;
                        for ch in line_text.chars() {
                            let candidate = format!("{prefix}{ch}");
                            if let Some(w) = face.font.advance(glyph_px, &candidate) {
                                if w <= target_w + SLACK {
                                    prefix = candidate;
                                    prefix_w = w;
                                } else {
                                    break;
                                }
                            }
                        }
                        line_text = format!("{prefix}{ellipsis}");
                        line_width = prefix_w + ellipsis_w;
                        truncated_any = true;
                    }
                }
            }
            let stops = if block.stops {
                stops_of(
                    &face.font, glyph_px, block.text, span.start, span.end, &line_text,
                )
            } else {
                Vec::new()
            };
            Line {
                text: line_text,
                x: match block.align_x {
                    Align::Start => content.x,
                    Align::Center => content.x + (content.w - line_width) / 2.0,
                    Align::End => content.x + content.w - line_width,
                },
                y: face.glyph_top(top + i as f32 * step, base_line_h, glyph_px),
                width: line_width,
                top: top + i as f32 * step,
                start: span.start,
                end: span.end,
                stops,
            }
        })
        .collect();

    let drawn_w = lines.iter().fold(0.0_f32, |w, line| w.max(line.width));
    let widest = all.iter().fold(0.0_f32, |w, span| w.max(span.width));
    let fits = !truncated_any
        && visible == all.len()
        && widest <= content.w + SLACK
        && block_h <= content.h + SLACK;

    Some(TextLayout {
        face: face.font.raster_id(),
        text_size: block.text_size,
        glyph_px,
        line_height: base_line_h,
        lines,
        bounds: (drawn_w, block_h),
        fits,
    })
}

/// The largest TextSize at which `text` fits a `width` by `height` box: what
/// TextScaled draws at.
pub fn scaled_size<F: Advance>(
    face: &Face<F>,
    text: &str,
    width: f32,
    height: f32,
    wrap: bool,
) -> Option<f32> {
    if !wrap {
        // Widths and heights are linear in the size, so one measurement at
        // size 1 answers directly.
        let (w1, h1) = measure(face, text, 1.0, None)?;
        let by_w = if w1 > 0.0 { width / w1 } else { f32::INFINITY };
        let by_h = if h1 > 0.0 { height / h1 } else { f32::INFINITY };
        return Some(by_w.min(by_h));
    }

    // Wrapping is not linear, because the breaks move with the size, so the
    // size is searched for.
    let mut low = 1.0_f32;
    let mut high = (height / face.em_scale).max(1.0);
    for _ in 0..16 {
        let mid = (low + high) / 2.0;
        let (w, h) = measure(face, text, mid, Some(width))?;
        if w <= width && h <= height {
            low = mid;
        } else {
            high = mid;
        }
    }
    Some(low)
}

/// `ContentText` of a label with `RichText` on: the text with its markup
/// removed and its escapes decoded.
///
/// ONLY THE ENGINE'S OWN TAGS ARE MARKUP. Anything else between angle
/// brackets is text and stays as written. `<br />` is a line break, and a
/// comment is removed whole. This strips; it does not render, and a RichText
/// label is still drawn with its tags.
pub fn strip_markup(text: &str) -> String {
    const TAGS: &[&str] = &[
        "b",
        "i",
        "u",
        "s",
        "br",
        "font",
        "stroke",
        "mark",
        "sc",
        "smallcaps",
        "uc",
        "uppercase",
    ];
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        if let Some(body) = tail.strip_prefix("<!--") {
            if let Some(end) = body.find("-->") {
                rest = &body[end + 3..];
                continue;
            }
        }
        if let Some(end) = tail.find('>') {
            let name = tail[1..end]
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("");
            if TAGS.contains(&name) {
                if name == "br" {
                    out.push('\n');
                }
                rest = &tail[end + 1..];
                continue;
            }
        }
        out.push('<');
        rest = &tail[1..];
    }
    out.push_str(rest);
    // `&amp;` LAST, so an escaped escape decodes once and no further.
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tags the engine reads are removed, and nothing else is.
    #[test]
    fn markup_is_stripped_and_text_is_kept() {
        assert_eq!(strip_markup("<b>bold</b> plain"), "bold plain");
        assert_eq!(
            strip_markup(r#"<font color="rgb(255,0,0)">red</font><br/>next"#),
            "red\nnext"
        );
        assert_eq!(strip_markup("a <!-- note --> b"), "a  b");
        assert_eq!(strip_markup("1 < 2 and <x>"), "1 < 2 and <x>");
        assert_eq!(strip_markup("&lt;b&gt; &amp;lt;"), "<b> &lt;");
    }

    /// A monospace font with no file behind it: every character is 0.6 of the
    /// glyph size wide, so line breaks are known in advance.
    #[derive(Debug, Clone, Copy)]
    struct Mono;

    impl Advance for Mono {
        fn advance(&self, px: f32, text: &str) -> Option<f32> {
            Some(text.chars().count() as f32 * 0.6 * px)
        }
    }

    /// The faces the engine was measured with: name, whether it is drawn as a
    /// Legacy family, and ascent, descent and cap height per em of glyph size
    /// from the font's own tables. LegacyArial is Arimo drawn the Legacy way.
    const FACES: [(&str, bool, f32, f32, f32); 4] = [
        ("LegacyArial", true, 0.905, 0.212, 0.688),
        ("Arimo", false, 0.905, 0.212, 0.688),
        ("Builder Sans", false, 0.980, 0.280, 0.700),
        ("Source Sans Pro", false, 0.984, 0.273, 0.660),
    ];

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn face(em_scale: f32, glyph_per_em: f32, ascent: f32, descent: f32) -> Face<Mono> {
        Face {
            font: Mono,
            em_scale,
            glyph_per_em,
            ascent,
            descent,
        }
    }

    fn stand_in() -> Face<Mono> {
        face(1.5, 1.0 / 1.5, 1.079, 0.251)
    }

    fn block(text: &str, size: f32, content: Rect, wrap: bool, y: Align) -> Block<'_> {
        Block {
            text,
            text_size: size,
            content,
            wrap,
            align_x: Align::Start,
            align_y: y,
            truncate: false,
            line_height: 1.0,
            stops: false,
        }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The single line of a wrapped label and of an unwrapped one land on the
    /// same pixels, at every alignment and size. The painter once disagreed by
    /// 5 px at TextSize 20, centred.
    #[test]
    fn wrapped_and_unwrapped_place_one_line_identically() {
        let f = stand_in();
        for size in [14.0, 20.0, 32.0, 48.0, 64.0] {
            for y in [Align::Start, Align::Center, Align::End] {
                let content = rect(10.0, 20.0, 400.0, 104.0);
                let plain = lay_out(&f, &block("H", size, content, false, y)).unwrap();
                let wrapped = lay_out(&f, &block("H", size, content, true, y)).unwrap();
                assert_eq!(plain.lines, wrapped.lines, "size {size}, {y:?}");
                assert_eq!(plain.bounds, wrapped.bounds);
            }
        }
    }

    /// The line box is one effective em: TextSize times the em scale, 1.5 for
    /// the face standing in for LegacyArial and 1.0 for a modern family.
    #[test]
    fn a_line_is_one_effective_em_tall() {
        let legacy = stand_in();
        let modern = face(1.0, 1.0 / 1.26, 0.98, 0.28);
        let roomy = rect(0.0, 0.0, 400.0, 400.0);
        for size in [14.0, 20.0, 32.0] {
            let l = lay_out(&legacy, &block("H", size, roomy, false, Align::Start)).unwrap();
            let m = lay_out(&modern, &block("H", size, roomy, false, Align::Start)).unwrap();
            assert!(close(l.bounds.1, size * 1.5));
            assert!(close(m.bounds.1, size));
            assert!(close(
                measure(&legacy, "H\nH", size, None).unwrap().1,
                size * 3.0
            ));
        }
    }

    /// The engine's cap centre sits within 1.5 px of the centre of its line
    /// box, for every family measured. With the faces' own metrics, the rule
    /// reproduces that at every size from 14 to 64 and every alignment.
    #[test]
    fn the_cap_centre_is_within_a_pixel_and_a_half_of_the_line_box_centre() {
        let content = rect(0.0, 0.0, 400.0, 200.0);
        for (name, legacy, ascent, descent, cap) in FACES {
            // Every family's glyphs are scaled so that ascent plus descent
            // fills the line, as `Face::from_font` builds them; a Legacy
            // family's line is 1.5 times taller.
            let em_scale = if legacy { LEGACY_EM_SCALE } else { 1.0 };
            let f = face(em_scale, 1.0 / (ascent + descent), ascent, descent);
            for size in [14.0, 20.0, 32.0, 48.0, 64.0] {
                for y in [Align::Start, Align::Center, Align::End] {
                    let laid = lay_out(&f, &block("H", size, content, false, y)).unwrap();
                    let line_top = match y {
                        Align::Start => 0.0,
                        Align::Center => (content.h - laid.line_height) / 2.0,
                        Align::End => content.h - laid.line_height,
                    };
                    let baseline = laid.lines[0].y + ascent * laid.glyph_px;
                    let cap_centre = baseline - cap * laid.glyph_px / 2.0;
                    let box_centre = line_top + laid.line_height / 2.0;
                    let off = cap_centre - box_centre;
                    assert!(
                        off.abs() <= 1.5,
                        "{name} at {size}, em scale {em_scale}, {y:?}: cap centre {off:+.2} px from the line box centre"
                    );
                }
            }
        }
    }

    /// Padding is applied by the caller as the content box. Moving the content
    /// box moves every line by exactly as much, wrapped or not.
    #[test]
    fn insetting_the_content_box_moves_the_lines_by_the_inset() {
        let f = stand_in();
        for wrap in [false, true] {
            for y in [Align::Start, Align::Center, Align::End] {
                let bare = rect(0.0, 0.0, 200.0, 64.0);
                // UIPadding left 16 and top 16, right and bottom 0.
                let padded = rect(16.0, 16.0, 184.0, 48.0);
                let a = lay_out(&f, &block("H", 20.0, bare, wrap, y)).unwrap();
                let b = lay_out(&f, &block("H", 20.0, padded, wrap, y)).unwrap();
                let dy = match y {
                    Align::Start => 16.0,
                    Align::Center => 8.0,
                    Align::End => 0.0,
                };
                assert!(close(b.lines[0].x - a.lines[0].x, 16.0));
                assert!(close(b.lines[0].y - a.lines[0].y, dy), "{y:?}");
            }
        }
    }

    /// Three wrapped lines of 30 in a box 64 tall: two are drawn, the third is
    /// not, `TextBounds` is two lines and `TextFits` is false. In a box 90
    /// tall all three fit. Unwrapped lines are never dropped.
    #[test]
    fn wrapped_lines_that_do_not_fit_are_dropped() {
        let f = stand_in();
        // 0.6 x 20 = 12 px a character: "one two" is 84, "three four" 120.
        let text = "one two three four five six";
        let short = lay_out(
            &f,
            &block(text, 20.0, rect(0.0, 0.0, 120.0, 64.0), true, Align::Start),
        )
        .unwrap();
        assert_eq!(short.lines.len(), 2);
        assert_eq!(short.lines[0].text, "one two");
        assert_eq!(short.lines[1].text, "three four");
        assert!(close(short.bounds.0, 120.0) && close(short.bounds.1, 60.0));
        assert!(!short.fits);

        let tall = lay_out(
            &f,
            &block(text, 20.0, rect(0.0, 0.0, 120.0, 90.0), true, Align::Start),
        )
        .unwrap();
        assert_eq!(tall.lines.len(), 3);
        assert!(close(tall.bounds.1, 90.0));
        assert!(tall.fits);

        // A box shorter than one line still shows the first.
        let tiny = lay_out(
            &f,
            &block(text, 20.0, rect(0.0, 0.0, 120.0, 10.0), true, Align::Start),
        )
        .unwrap();
        assert_eq!(tiny.lines.len(), 1);
        assert!(!tiny.fits);

        let unwrapped = lay_out(
            &f,
            &block(
                "a\nb\nc",
                20.0,
                rect(0.0, 0.0, 120.0, 64.0),
                false,
                Align::Start,
            ),
        )
        .unwrap();
        assert_eq!(unwrapped.lines.len(), 3);
        assert!(!unwrapped.fits);
    }

    /// `TextBounds` is what is drawn: the widest drawn line, not the box, and
    /// not the string. `TextFits` is false when the text is wider than the
    /// box even though nothing is dropped.
    #[test]
    fn text_bounds_and_text_fits() {
        let f = stand_in();
        let fits = lay_out(
            &f,
            &block(
                "Hello",
                20.0,
                rect(0.0, 0.0, 200.0, 64.0),
                false,
                Align::Center,
            ),
        )
        .unwrap();
        assert!(close(fits.bounds.0, 60.0) && close(fits.bounds.1, 30.0));
        assert!(fits.fits);

        let wide = lay_out(
            &f,
            &block(
                "Hello",
                20.0,
                rect(0.0, 0.0, 50.0, 64.0),
                false,
                Align::Center,
            ),
        )
        .unwrap();
        assert!(close(wide.bounds.0, 60.0));
        assert!(!wide.fits);

        // Exactly the size AutomaticSize would grow the box to.
        let (w, h) = measure(&f, "Hello", 20.0, None).unwrap();
        let snug = lay_out(
            &f,
            &block("Hello", 20.0, rect(0.0, 0.0, w, h), false, Align::Center),
        )
        .unwrap();
        assert!(snug.fits);
        assert_eq!(snug.bounds, (w, h));

        // Empty text is one empty line.
        let empty = lay_out(
            &f,
            &block("", 20.0, rect(0.0, 0.0, 200.0, 64.0), false, Align::Center),
        )
        .unwrap();
        assert_eq!(empty.bounds.0, 0.0);
        assert!(close(empty.bounds.1, 30.0));
    }

    /// TextScaled grows the text until one axis is full, through the same
    /// measurement the layout uses.
    #[test]
    fn scaled_size_fills_the_tighter_axis() {
        let f = stand_in();
        // "Hi" is 1.2 px wide and 1.5 tall per unit of TextSize.
        assert!(close(
            scaled_size(&f, "Hi", 120.0, 300.0, false).unwrap(),
            100.0
        ));
        assert!(close(
            scaled_size(&f, "Hi", 1200.0, 30.0, false).unwrap(),
            20.0
        ));
        let wrapped = scaled_size(&f, "Hi there", 120.0, 300.0, true).unwrap();
        let (w, h) = measure(&f, "Hi there", wrapped, Some(120.0)).unwrap();
        assert!(w <= 120.0 && h <= 300.0);
    }

    /// Mono, with a glyph for every character it is asked about, U+2026
    /// included.
    #[derive(Debug, Clone, Copy)]
    struct MonoWithEllipsis;

    impl Advance for MonoWithEllipsis {
        fn advance(&self, px: f32, text: &str) -> Option<f32> {
            Mono.advance(px, text)
        }

        fn has_glyph(&self, _ch: char) -> bool {
            true
        }
    }

    /// A truncated line ends in the engine's single ellipsis glyph, U+2026,
    /// when the face has it, and in three full stops only when it does not.
    /// Either way the prefix and the ellipsis together fit the box, and
    /// `TextBounds` is their width.
    #[test]
    fn truncation_ends_in_one_ellipsis_glyph_when_the_face_has_it() {
        // 12 px a character at TextSize 20: 8 characters fit in 100.
        let content = rect(0.0, 0.0, 100.0, 64.0);
        let mut b = block("A long label", 20.0, content, false, Align::Start);
        b.truncate = true;

        let s = stand_in();
        let with = Face {
            font: MonoWithEllipsis,
            em_scale: s.em_scale,
            glyph_per_em: s.glyph_per_em,
            ascent: s.ascent,
            descent: s.descent,
        };
        let laid = lay_out(&with, &b).unwrap();
        assert_eq!(laid.lines[0].text, "A long \u{2026}");
        assert!(close(laid.bounds.0, 96.0), "{}", laid.bounds.0);
        assert!(!laid.fits);

        let without = lay_out(&stand_in(), &b).unwrap();
        assert_eq!(without.lines[0].text, "A lon...");
        assert!(close(without.bounds.0, 96.0), "{}", without.bounds.0);
        assert!(!without.fits);
    }

    /// A wrapped label whose lower lines were dropped ends its last drawn line
    /// in the ellipsis: whole when the line and the ellipsis fit the box, cut
    /// back a character at a time when they do not. Without TextTruncate the
    /// line is drawn as it broke.
    #[test]
    fn a_wrapped_cut_ends_its_last_drawn_line_in_the_ellipsis() {
        let s = stand_in();
        let face = Face {
            font: MonoWithEllipsis,
            em_scale: s.em_scale,
            glyph_per_em: s.glyph_per_em,
            ascent: s.ascent,
            descent: s.descent,
        };
        // 12 px a character, lines 30 tall: one line of 8 characters fits.
        let content = rect(0.0, 0.0, 100.0, 30.0);

        let mut b = block("ab cd efgh ij", 20.0, content, true, Align::Start);
        b.truncate = true;
        let laid = lay_out(&face, &b).unwrap();
        assert_eq!(laid.lines.len(), 1);
        assert_eq!(laid.lines[0].text, "ab cd\u{2026}");
        assert!(close(laid.bounds.0, 72.0), "{}", laid.bounds.0);
        assert!(!laid.fits);

        let mut b = block("ab cd ef gh", 20.0, content, true, Align::Start);
        b.truncate = true;
        let laid = lay_out(&face, &b).unwrap();
        assert_eq!(laid.lines[0].text, "ab cd e\u{2026}");
        assert!(close(laid.bounds.0, 96.0), "{}", laid.bounds.0);

        let plain = block("ab cd efgh ij", 20.0, content, true, Align::Start);
        assert_eq!(lay_out(&face, &plain).unwrap().lines[0].text, "ab cd");
    }

    /// The wrap breaks between words, then between characters for a word
    /// wider than the box, and keeps blank paragraphs.
    #[test]
    fn lines_break_between_words_then_characters() {
        let px = 10.0; // 6 px a character
        let lines = break_lines(&Mono, "ab cd\n\nabcdefgh", px, Some(30.0)).unwrap();
        let texts: Vec<&str> = lines.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(texts, ["ab cd", "", "abcde", "fgh"]);
    }
}
