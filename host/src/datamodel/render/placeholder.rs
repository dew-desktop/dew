//! A `TextBox` whose `Text` is empty measures and draws its `PlaceholderText`,
//! held to the engine's rows as equivalences: a placeholder measures exactly
//! as the same string set as `Text` does, in the same box. Dew's advances run
//! a few percent narrower than the engine's, so engine widths are not asserted.
//! These skip on a machine with no font.

use super::*;
use crate::datamodel::{install, install_vocabulary, SharedDom};
use dew_runtime::{Painter, RasterPainter};
use mlua::prelude::*;

const W: u32 = 320;
const H: u32 = 240;

const PH: &str = "Hamburgefonts";

/// A TextBox set up as every probe row starts: 260 by 30, LegacyArial at
/// TextSize 20, left aligned, with empty `Text`.
const PRELUDE: &str = r#"
    local function box(name, y)
        local o = Instance.new("TextBox")
        o.Name = name
        o.Size = UDim2.fromOffset(260, 30)
        o.Position = UDim2.fromOffset(20, y)
        o.BackgroundColor3 = Color3.new(0, 0, 0)
        o.TextColor3 = Color3.new(1, 1, 1)
        o.Font = Enum.Font.Arial
        o.TextSize = 20
        o.TextXAlignment = Enum.TextXAlignment.Left
        o.ClearTextOnFocus = false
        o.Text = ""
        o.Parent = root
        return o
    end
"#;

/// Build `src` under a `root` global after [`PRELUDE`], lay it out and draw
/// it. Returns the Luau state and the painted pixels, BGRA.
fn draw(src: &str) -> Option<(Lua, Vec<u8>)> {
    let face = crate::services::default_face()?;
    let lua = Lua::new();
    let dom = SharedDom::default();
    install(&lua, &dom).expect("install");
    install_vocabulary(&lua).expect("vocabulary");
    let root = dom
        .lock()
        .expect("dom")
        .insert("Folder".into(), "Root".into());
    lua.globals()
        .set(
            "root",
            crate::datamodel::handle(&lua, &dom, root).expect("root handle"),
        )
        .expect("root");
    lua.load(format!("{PRELUDE}\n{src}")).exec().expect("guest");
    let frame = frame_of(&dom, root, W as f32, H as f32);
    let mut painter = RasterPainter::new(W, H, dew_raster::Backend::VelloCpu)
        .expect("surface")
        .with_face(face);
    painter.paint_frame(&frame, Some(Rgb(0, 0, 0)));
    let bgra = painter.canvas_mut().bgra().expect("pixels").to_vec();
    Some((lua, bgra))
}

#[derive(Debug, Clone, PartialEq)]
struct Read {
    bounds: (f32, f32),
    fits: bool,
    content: String,
    size: (f32, f32),
}

/// What a guest reads back from the global `name` after a frame.
fn read(lua: &Lua, name: &str) -> Read {
    let (bx, by, fits, content, sx, sy): (f32, f32, bool, String, f32, f32) = lua
        .load(format!(
            "return {name}.TextBounds.X, {name}.TextBounds.Y, {name}.TextFits, \
             {name}.ContentText, {name}.AbsoluteSize.X, {name}.AbsoluteSize.Y"
        ))
        .eval()
        .expect("members");
    Read {
        bounds: (bx, by),
        fits,
        content,
        size: (sx, sy),
    }
}

/// Row 1. Empty `Text`: `TextBounds` is the placeholder's, 125 by 20 in the
/// engine (its `GetTextSize`), `TextFits` is true and `ContentText` is empty.
#[test]
fn an_empty_text_box_measures_its_placeholder() {
    let Some((lua, _)) = draw(&format!(
        r#"shown = box("Shown", 10)
        shown.PlaceholderText = "{PH}"
        typed = box("Typed", 50)
        typed.Text = "{PH}""#
    )) else {
        return;
    };
    let shown = read(&lua, "shown");
    let typed = read(&lua, "typed");
    eprintln!("row 1: placeholder {shown:?}, as Text {typed:?}");
    assert!(shown.bounds.0 > 0.0, "nothing measured: {shown:?}");
    assert_eq!(shown.bounds, typed.bounds);
    assert!(shown.fits);
    assert_eq!(shown.content, "");
}

/// Row 2. Typed text hides the placeholder: `TextBounds` is the text's, 49
/// by 20 in the engine, and the placeholder is still used once it is empty.
#[test]
fn typed_text_is_measured_instead_of_the_placeholder() {
    let Some((lua, _)) = draw(&format!(
        r#"shown = box("Shown", 10)
        shown.PlaceholderText = "{PH}"
        shown.Text = "Typed"
        typed = box("Typed", 50)
        typed.Text = "Typed"
        empty = box("Empty", 90)
        empty.PlaceholderText = "{PH}"
        plain = box("Plain", 130)
        plain.Text = "{PH}""#
    )) else {
        return;
    };
    let shown = read(&lua, "shown");
    let typed = read(&lua, "typed");
    eprintln!("row 2: {shown:?}");
    assert_eq!(shown.bounds, typed.bounds);
    assert_eq!(shown.content, "Typed");
    assert_eq!(read(&lua, "empty").bounds, read(&lua, "plain").bounds);
}

/// Row 3. One space is text, 5 by 20 in the engine: the placeholder is used
/// only when `Text` is exactly empty.
#[test]
fn a_single_space_is_text_and_hides_the_placeholder() {
    let Some((lua, _)) = draw(&format!(
        r#"shown = box("Shown", 10)
        shown.PlaceholderText = "{PH}"
        shown.Text = " "
        typed = box("Typed", 50)
        typed.Text = " "
        empty = box("Empty", 90)
        empty.PlaceholderText = "{PH}"
        plain = box("Plain", 130)
        plain.Text = "{PH}""#
    )) else {
        return;
    };
    let shown = read(&lua, "shown");
    let typed = read(&lua, "typed");
    let empty = read(&lua, "empty");
    eprintln!("row 3: {shown:?}");
    assert_eq!(shown.bounds, typed.bounds);
    assert_eq!(shown.content, " ");
    assert_eq!(empty.bounds, read(&lua, "plain").bounds);
    assert!(
        shown.bounds.0 < empty.bounds.0,
        "{shown:?} against {empty:?}"
    );
}

/// Row 9. A long placeholder wraps and truncates as `Text` would: 243 by 20
/// in the engine, and `TextFits` false.
#[test]
fn a_long_placeholder_wraps_and_truncates_as_text_would() {
    let long = "A placeholder far too long to fit on one line here";
    let Some((lua, _)) = draw(&format!(
        r#"shown = box("Shown", 10)
        shown.PlaceholderText = "{long}"
        shown.TextWrapped = true
        shown.TextTruncate = Enum.TextTruncate.AtEnd
        typed = box("Typed", 50)
        typed.Text = "{long}"
        typed.TextWrapped = true
        typed.TextTruncate = Enum.TextTruncate.AtEnd"#
    )) else {
        return;
    };
    let shown = read(&lua, "shown");
    let typed = read(&lua, "typed");
    eprintln!("row 9: placeholder {shown:?}, as Text {typed:?}");
    assert!(shown.bounds.0 > 0.0, "nothing measured: {shown:?}");
    assert_eq!(shown.bounds, typed.bounds);
    assert!(!shown.fits);
    assert_eq!(shown.content, "");
}

/// Row 10. AutomaticSize X from a width of 0 grows to the placeholder: 125
/// by 30 in the engine.
#[test]
fn automatic_size_grows_to_the_placeholder() {
    let Some((lua, _)) = draw(&format!(
        r#"shown = box("Shown", 10)
        shown.PlaceholderText = "{PH}"
        shown.Size = UDim2.fromOffset(0, 30)
        shown.AutomaticSize = Enum.AutomaticSize.X
        typed = box("Typed", 50)
        typed.Text = "{PH}"
        typed.Size = UDim2.fromOffset(0, 30)
        typed.AutomaticSize = Enum.AutomaticSize.X"#
    )) else {
        return;
    };
    let shown = read(&lua, "shown");
    let typed = read(&lua, "typed");
    eprintln!("row 10: placeholder {shown:?}, as Text {typed:?}");
    assert!(shown.size.0 > 0.0, "did not grow: {shown:?}");
    assert_eq!(shown.size, typed.size);
}

/// Row 11. With `RichText` on, a placeholder is read as `Text` would be.
/// The engine reads 140 by 20, which is the markup measured as written (Dew
/// 136) rather than stripped (Dew 70); see `shows_placeholder`.
#[test]
fn a_rich_text_placeholder_is_read_as_text_would_be() {
    let markup = "<b>Bold</b> hint";
    let Some((lua, _)) = draw(&format!(
        r#"shown = box("Shown", 10)
        shown.RichText = true
        shown.PlaceholderText = "{markup}"
        typed = box("Typed", 50)
        typed.RichText = true
        typed.Text = "{markup}"
        raw = box("Raw", 90)
        raw.Text = "{markup}"
        stripped = box("Stripped", 130)
        stripped.Text = "Bold hint""#
    )) else {
        return;
    };
    let shown = read(&lua, "shown");
    let typed = read(&lua, "typed");
    eprintln!(
        "row 11: placeholder {shown:?}; markup as written {:?}; markup stripped {:?}",
        read(&lua, "raw").bounds,
        read(&lua, "stripped").bounds
    );
    assert!(shown.bounds.0 > 0.0, "nothing measured: {shown:?}");
    assert_eq!(shown.bounds, typed.bounds);
    assert_eq!(shown.content, "");
}

/// A shown placeholder is painted in `PlaceholderColor3`, not `TextColor3`:
/// row 8, red placeholder colour and blue text colour, on black.
#[test]
fn the_placeholder_is_painted_in_placeholder_color3() {
    let Some((_, bgra)) = draw(&format!(
        r#"shown = box("Shown", 10)
        shown.PlaceholderText = "{PH}"
        shown.PlaceholderColor3 = Color3.new(1, 0, 0)
        shown.TextColor3 = Color3.new(0, 0, 1)"#
    )) else {
        return;
    };
    let red = bgra.chunks(4).filter(|p| p[2] > 128 && p[0] < 64).count();
    let blue = bgra.chunks(4).filter(|p| p[0] > 128 && p[2] < 64).count();
    assert!(red > 50, "{red} red pixels");
    assert_eq!(blue, 0, "{blue} blue pixels");
}

/// The engine's defaults: an empty `PlaceholderText` and a grey
/// `PlaceholderColor3` of 128, 128, 128.
#[test]
fn placeholder_defaults_match_the_engine() {
    let lua = Lua::new();
    install(&lua, &SharedDom::default()).expect("install");
    install_vocabulary(&lua).expect("vocabulary");
    let (text, r, g, b): (String, f32, f32, f32) = lua
        .load(
            r#"local t = Instance.new("TextBox")
            local c = t.PlaceholderColor3
            return t.PlaceholderText, c.R, c.G, c.B"#,
        )
        .eval()
        .expect("defaults");
    assert_eq!(text, "");
    for channel in [r, g, b] {
        assert_eq!((channel * 255.0).round(), 128.0, "{r}, {g}, {b}");
    }
}
