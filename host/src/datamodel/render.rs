//! The DataModel becomes pixels.
//!
//! WHY THIS IS THE PIECE THAT MATTERED
//! Until this file, `SharedDom` was created, installed into every guest VM, and
//! never read again. A guest could call `Instance.new("Frame")`, set 136 of 138
//! properties on it and build a tree, and nothing drew any of it: the frame was
//! driven by `Driver::new(session, ..)` where the session is Aether's. The
//! property surface reading 136 of 138 measured a language nothing spoke back.
//!
//! This gives `dom` its first reader, which is also the precondition the scheme
//! registry and the permission gate in Sprint 5 were waiting on: there was no
//! point gating a fetch when nothing consumed the result.
//!
//! WHAT IT BYPASSES, AND WHY THAT IS CORRECT
//! Not `Driver`, and not `Session`. Those are Aether's: a `Session` is a handle
//! onto Luau objects that Aether's `Live.luau` maintains. Dew's DataModel is a
//! Rust arena and owes nothing to that path. What both share is the far end --
//! `Frame`, `Node` and the `Painter` trait -- which is exactly the seam
//! `hosts/runtime` says has survived four rasterisers without the framework
//! changing a line. Building a `Frame` here and handing it to the same painter is
//! using that seam as intended rather than going around it.
//!
//! THIS IS THE DEMO PATH, NOT THE PARITY PATH, and the distinction is the whole
//! reason it is affordable now. It resolves offset and scale against the parent,
//! applies `AnchorPoint`, honours `Visible`, `ZIndex`, `ClipsDescendants`,
//! `UICorner` and `UIStroke`, draws text, and draws an image. It does NOT do
//! `AutomaticSize`, `UIListLayout`, gradients, or any of the constraints. Those
//! are `conformance/LAYOUT.md`'s subject and they arrive with the Rust
//! conformance runner that can hold them to the engine's own answers --
//! `AutomaticSize` alone cost four wrong rules in Aether, each fitting every case
//! that existed when it was written, and reimplementing it here from memory would
//! be the fifth.
//!
//! WHAT AN IMAGE HONOURS, STATED THE SAME WAY. `Image`, `ImageContent`,
//! `ImageColor3`, `ImageTransparency`, `ImageRectOffset`, `ImageRectSize`, and
//! three of `Enum.ScaleType`'s five members -- `Stretch`, `Fit` and `Crop`.
//!
//! IT DOES NOT DO `Slice` OR `Tile`, and with them `SliceCenter`, `SliceScale`
//! and `TileSize`. Both change what a drawn image looks like, and both need
//! something the painter has no shape for yet: nine draws from one source for a
//! nine-patch, and a repeat for a tile. They are resolved to `Stretch` -- and
//! REPORTED BY NAME the first time an element asks for one, through
//! `crate::assets::Assets::note_once`, because silently drawing everything as `Stretch`
//! is exactly the failure this paragraph exists to prevent. An author whose
//! nine-patch renders as a smear should be told which of the two of us decided
//! that.
//!
//! So: enough to put a real tree on screen, and no claim beyond that.

use dew_runtime::frame::{
    Align, AlphaStop, Frame, Gradient, GradientKind, Image, Node, Rect, Rgb, Scale, Stop, Stroke,
};
use rbx_types::{Variant, Vector2};

use super::{Dom, SharedDom};

/// A rectangle in absolute screen coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box2 {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Classes that are MODIFIERS rather than things to draw.
///
/// A `UICorner` is a property of its parent expressed as a child, which is
/// Roblox's shape and not ours to argue with. Drawing one as a rectangle would
/// put a black square behind every rounded card.
fn is_modifier(class: &str) -> bool {
    class.starts_with("UI")
}

/// Classes that draw text.
fn draws_text(class: &str) -> bool {
    matches!(
        class,
        "TextLabel" | "TextButton" | "TextBox" | "InputActionLabel"
    )
}

/// Classes that draw an image.
///
/// TWO, AND THE BACKLOG PAIRS MORE. `ScrollingFrame` has `TopImage`, `MidImage`
/// and `BottomImage` for its scrollbar and `ImageButton` has `HoverImage` and
/// `PressedImage`; each is the same asset naming under a different property, and
/// each needs a state this pass does not have -- a scroll position, a hover, a
/// press. `ImageLabel` and `ImageButton` need none of that, which is why they are
/// the two that arrive first.
fn draws_image(class: &str) -> bool {
    matches!(class, "ImageLabel" | "ImageButton" | "InputActionLabel")
}

fn udim(dom: &Dom, id: usize, key: &str) -> (f32, f32) {
    match dom.property(id, key) {
        Some(Variant::UDim(v)) => (v.scale, v.offset as f32),
        _ => (0.0, 0.0),
    }
}

fn udim2(dom: &Dom, id: usize, key: &str) -> (f32, f32, f32, f32) {
    match dom.property(id, key) {
        Some(Variant::UDim2(v)) => (v.x.scale, v.x.offset as f32, v.y.scale, v.y.offset as f32),
        _ => (0.0, 0.0, 0.0, 0.0),
    }
}

/// The UIPadding on a node, as four offsets: (left, top, right, bottom).
fn padding_of(dom: &Dom, id: usize) -> (f32, f32, f32, f32) {
    for child in dom.children(id) {
        if dom.class_of(child).as_deref() == Some("UIPadding") {
            let (_, l) = udim(dom, child, "PaddingLeft");
            let (_, t) = udim(dom, child, "PaddingTop");
            let (_, r) = udim(dom, child, "PaddingRight");
            let (_, b) = udim(dom, child, "PaddingBottom");
            return (l, t, r, b);
        }
    }
    (0.0, 0.0, 0.0, 0.0)
}

/// The content box a container offers its children: its own rect, inset by any UIPadding.
pub(crate) fn content_box(dom: &Dom, id: usize, own: Box2) -> Box2 {
    let (l, t, r, b) = padding_of(dom, id);
    Box2 {
        x: own.x + l,
        y: own.y + t,
        w: (own.w - l - r).max(0.0),
        h: (own.h - t - b).max(0.0),
    }
}

fn vector2(dom: &Dom, id: usize, key: &str) -> Vector2 {
    match dom.property(id, key) {
        Some(Variant::Vector2(v)) => v,
        _ => Vector2::new(0.0, 0.0),
    }
}

fn number(dom: &Dom, id: usize, key: &str) -> Option<f32> {
    match dom.property(id, key) {
        Some(Variant::Float32(v)) => Some(v),
        Some(Variant::Float64(v)) => Some(v as f32),
        Some(Variant::Int32(v)) => Some(v as f32),
        Some(Variant::Int64(v)) => Some(v as f32),
        _ => None,
    }
}

fn boolean(dom: &Dom, id: usize, key: &str) -> Option<bool> {
    match dom.property(id, key) {
        Some(Variant::Bool(v)) => Some(v),
        _ => None,
    }
}

fn text(dom: &Dom, id: usize, key: &str) -> Option<String> {
    match dom.property(id, key) {
        Some(Variant::String(v)) => Some(v),
        _ => None,
    }
}

fn text_for(dom: &Dom, id: usize, class: &str) -> Option<String> {
    if class == "InputActionLabel" {
        text(dom, id, "InputAction")
            .filter(|t| !t.is_empty())
            .or_else(|| text(dom, id, "Text").filter(|t| !t.is_empty()))
    } else {
        text(dom, id, "Text").filter(|t| !t.is_empty())
    }
}

/// A `Color3` is 0 to 1 per channel; the display list is 0 to 255.
fn colour(dom: &Dom, id: usize, key: &str) -> Option<Rgb> {
    match dom.property(id, key) {
        Some(Variant::Color3(c)) => Some(Rgb(
            (c.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (c.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (c.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        )),
        _ => None,
    }
}

/// `Transparency` inverted into the alpha the display list carries.
///
/// The display list is opaque at 1 and Roblox is opaque at 0. `Live.luau` inverts
/// at the source for exactly one reason, stated there: three painters each
/// inverted it themselves and one forgot.
fn alpha_from(dom: &Dom, id: usize, key: &str) -> f32 {
    1.0 - number(dom, id, key).unwrap_or(0.0).clamp(0.0, 1.0)
}

fn align(dom: &Dom, id: usize, key: &str, enum_name: &str) -> Option<Align> {
    let Some(Variant::Enum(raw)) = dom.property(id, key) else {
        return None;
    };
    let item = super::enums::item_by_value(enum_name, raw.to_u32())?;
    Some(match item.name {
        "Left" | "Top" => Align::Start,
        "Right" | "Bottom" => Align::End,
        _ => Align::Center,
    })
}

/// The UIListLayout child on a node, if any.
fn list_layout_of(dom: &Dom, id: usize) -> Option<usize> {
    dom.children(id)
        .into_iter()
        .find(|&child| dom.class_of(child).as_deref() == Some("UIListLayout"))
}

/// Resolve one element against the box its parent offers.
///
/// THE COORDINATE MODEL, and it is `conformance/LAYOUT.md` section 1 rather than
/// an invention: a `UDim` is `scale * available + offset` per axis, and
/// `AnchorPoint` then shifts the element by a fraction of ITS OWN resolved size.
/// The ordering is size first, then anchor, which is the thing
/// `anchor_point_after_automatic_size` was opened in Studio to confirm.
///
/// `laid_out` marks an element positioned by a layout container (`UIListLayout`).
/// Its own `Position` is ignored per LAYOUT.md section 4.
fn solve_rect(dom: &Dom, id: usize, parent: Box2, laid_out: bool) -> Box2 {
    let (sxs, sxo, sys, syo) = udim2(dom, id, "Size");
    let (pxs, pxo, pys, pyo) = if laid_out {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        udim2(dom, id, "Position")
    };
    let anchor = vector2(dom, id, "AnchorPoint");

    let w = sxs * parent.w + sxo;
    let h = sys * parent.h + syo;
    Box2 {
        x: parent.x + pxs * parent.w + pxo - anchor.x * w,
        y: parent.y + pys * parent.h + pyo - anchor.y * h,
        w: w.max(0.0),
        h: h.max(0.0),
    }
}

/// The corner radius a `UICorner` child asks for, in pixels.
///
/// SCALE IS IGNORED, and that is a stated gap rather than an oversight: a scale
/// radius resolves against the smaller axis of the element and this pass has no
/// case verifying which. An offset radius is what every mod in the tree uses.
fn corner_radius(dom: &Dom, id: usize) -> f32 {
    for child in dom.children(id) {
        if dom.class_of(child).as_deref() == Some("UICorner") {
            if let Some(Variant::UDim(u)) = dom.property(child, "CornerRadius") {
                return u.offset as f32;
            }
        }
    }
    0.0
}

/// What this element names as its image, whichever generation it used.
///
/// TWO GENERATIONS OF ONE IDEA, AND A HOST TAKES BOTH. `Image` is the legacy
/// `ContentId` -- a bare string -- and `ImageContent` is the modern `Content`, a
/// URI; the backlog pairs them all the way down (`TopImage`/`TopImageContent`,
/// `HoverImage`/`HoverImageContent`) and both name the same asset. A guest
/// written this year assigns the second, a guest written five years ago assigns
/// the first, and neither is wrong.
///
/// `ImageContent` WINS WHEN BOTH ARE SET, for one reason: it is the one the
/// engine keeps. Roblox's own migration writes through `Image` into
/// `ImageContent`, so the modern property is the more recent statement of intent
/// whenever they disagree -- and an application that sets only `Image` never
/// reaches the tie at all.
fn image_uri(dom: &Dom, id: usize) -> Option<String> {
    if let Some(Variant::Content(content)) = dom.property(id, "ImageContent") {
        if let Some(uri) = content.as_uri() {
            if !uri.is_empty() {
                return Some(uri.to_string());
            }
        }
    }
    match dom.property(id, "Image") {
        Some(Variant::ContentId(legacy)) if !legacy.as_str().is_empty() => {
            Some(legacy.as_str().to_string())
        }
        _ => None,
    }
}

/// `ImageRectOffset` and `ImageRectSize` as one source rectangle.
///
/// A ZERO `ImageRectSize` MEANS THE WHOLE IMAGE, which is the engine's own rule
/// and also its default -- so the common case, where nobody set either property,
/// arrives here as (0, 0) and must not be read as "sample nothing". Getting this
/// backwards would blank every image in the tree while every other property
/// looked right.
fn image_source(dom: &Dom, id: usize) -> Option<Rect> {
    let size = vector2(dom, id, "ImageRectSize");
    if size.x <= 0.0 || size.y <= 0.0 {
        return None;
    }
    let offset = vector2(dom, id, "ImageRectOffset");
    Some(Rect {
        x: offset.x,
        y: offset.y,
        w: size.x,
        h: size.y,
    })
}

/// `Enum.ScaleType`, as much of it as the painter can draw.
///
/// The two it cannot are reported by name rather than mapped in silence; see
/// this module's own header for what that costs and why it is not free.
fn image_scale(dom: &mut Dom, id: usize) -> Scale {
    let Some(Variant::Enum(raw)) = dom.property(id, "ScaleType") else {
        return Scale::Stretch;
    };
    let Some(item) = super::enums::item_by_value("ScaleType", raw.to_u32()) else {
        return Scale::Stretch;
    };
    match item.name {
        "Fit" => Scale::Fit,
        "Crop" => Scale::Crop,
        "Stretch" => Scale::Stretch,
        other => {
            let name = dom.name_of(id).unwrap_or_default();
            dom.assets.note_once(
                format!("ScaleType.{other}"),
                &format!(
                    "{name}: Enum.ScaleType.{other} is not drawn yet and is being stretched \
                     instead — with it, SliceCenter, SliceScale and TileSize are ignored"
                ),
            );
            Scale::Stretch
        }
    }
}

/// The image this element draws, resolved to pixels where they could be found.
///
/// `None` MEANS THE ELEMENT NAMES NO IMAGE, and that is a different thing from an
/// image that did not resolve. An `ImageLabel` with an empty `Image` is a plain
/// rectangle -- the same as it is in Roblox -- and drawing a "missing" marker on
/// it would put one on every image element in every tree before its asset was
/// assigned. An element that DID name something the host could not produce keeps
/// its `Image` with `bitmap: None`, reaches the painter, and is drawn as missing.
fn image_of(dom: &mut Dom, id: usize) -> Option<Image> {
    let uri = image_uri(dom, id)?;
    let tint = colour(dom, id, "ImageColor3");
    let alpha = alpha_from(dom, id, "ImageTransparency");
    let source = image_source(dom, id);
    let scale = image_scale(dom, id);
    let bitmap = dom.assets.resolve(&uri);
    Some(Image {
        bitmap,
        uri,
        tint,
        alpha,
        source,
        scale,
    })
}

fn stroke_of(dom: &Dom, id: usize) -> Option<Stroke> {
    for child in dom.children(id) {
        if dom.class_of(child).as_deref() == Some("UIStroke") {
            return Some(Stroke {
                colour: colour(dom, child, "Color"),
                thickness: number(dom, child, "Thickness").unwrap_or(1.0),
                alpha: alpha_from(dom, child, "Transparency"),
            });
        }
    }
    None
}

fn gradient_kind(dom: &Dom, id: usize) -> GradientKind {
    if let Some(prop) = dom.property(id, "Type") {
        match prop {
            Variant::Enum(raw) => {
                if let Some(item) = super::enums::item_by_value("GradientType", raw.to_u32()) {
                    if item.name.eq_ignore_ascii_case("radial") {
                        return GradientKind::Radial;
                    }
                }
                if raw.to_u32() == 1 {
                    return GradientKind::Radial;
                }
            }
            Variant::String(s) if s.eq_ignore_ascii_case("radial") => {
                return GradientKind::Radial;
            }
            _ => {}
        }
    }
    if let Some(Variant::String(s)) = dom.property(id, "Shape") {
        if s.eq_ignore_ascii_case("radial") {
            return GradientKind::Radial;
        }
    }
    GradientKind::Linear
}

fn gradient_of(dom: &Dom, id: usize) -> Option<Gradient> {
    for child in dom.children(id) {
        if dom.class_of(child).as_deref() == Some("UIGradient") {
            if boolean(dom, child, "Enabled") == Some(false) {
                continue;
            }
            let rotation = number(dom, child, "Rotation").unwrap_or(0.0);
            let kind = gradient_kind(dom, child);
            let mut stops = Vec::new();
            if let Some(Variant::ColorSequence(cs)) = dom.property(child, "Color") {
                for kp in &cs.keypoints {
                    stops.push(Stop {
                        at: kp.time,
                        colour: Rgb(
                            (kp.color.r.clamp(0.0, 1.0) * 255.0).round() as u8,
                            (kp.color.g.clamp(0.0, 1.0) * 255.0).round() as u8,
                            (kp.color.b.clamp(0.0, 1.0) * 255.0).round() as u8,
                        ),
                    });
                }
            }
            let mut alpha_stops = Vec::new();
            if let Some(Variant::NumberSequence(ns)) = dom.property(child, "Transparency") {
                for kp in &ns.keypoints {
                    alpha_stops.push(AlphaStop {
                        at: kp.time,
                        alpha: 1.0 - kp.value.clamp(0.0, 1.0),
                    });
                }
            }

            if stops.is_empty() && alpha_stops.is_empty() {
                return None;
            }

            return Some(Gradient {
                kind,
                stops,
                alpha_stops,
                rotation,
            });
        }
    }
    None
}

/// One element, resolved, in paint order.
///
/// THE UNIT BOTH DRAWING AND HIT TESTING CONSUME, and that is the whole reason
/// this type exists rather than the walk building `Node`s directly. Sprint 9
/// needed to know which instance is at (x, y), and the tempting shape is a second
/// recursive walk that resolves geometry again. It would drift -- not at once,
/// but the first time `resolve` learns about `AutomaticSize` or a layout
/// modifier and only one of the two callers is updated. The symptom is a button
/// that works everywhere except where it looks like it should, which is close to
/// unattributable once it ships.
///
/// So there is ONE placement pass. `frame` turns these into `Node`s and
/// `input::hit` reads the same list backwards.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
    /// The instance this box belongs to. `Node` cannot carry it -- `Node` is
    /// Aether's type and its `id` is a paint sequence number, not an arena id --
    /// and it is the one thing a hit test needs and drawing does not.
    pub id: usize,
    pub rect: Box2,
    /// The corner radius of that clipping box, 0.0 when it is square.
    ///
    /// Set from the `UICorner` on the ancestor that does the clipping, because
    /// that is the shape Roblox masks against. Beside the box rather than inside
    /// it: both are written at one site and the compiler checks every reader.
    pub clip_radius: f32,
    /// The clipping box inherited from the nearest `ClipsDescendants` ancestor.
    ///
    /// APPLIES TO HIT TESTING AS WELL AS DRAWING. A child outside a clipping
    /// parent is not visible and therefore not clickable, and honouring it in
    /// the painter alone is the easy half to do and the easy half to forget.
    pub clip: Option<Box2>,
}

/// The AutomaticSize axes requested by a node: (grow_x, grow_y).
fn automatic_axes(dom: &Dom, id: usize) -> (bool, bool) {
    let Some(Variant::Enum(raw)) = dom.property(id, "AutomaticSize") else {
        return (false, false);
    };
    let Some(item) = super::enums::item_by_value("AutomaticSize", raw.to_u32()) else {
        return (false, false);
    };
    match item.name {
        "X" => (true, false),
        "Y" => (false, true),
        "XY" => (true, true),
        _ => (false, false),
    }
}

/// The AutomaticCanvasSize axes requested by a ScrollingFrame: (grow_x, grow_y).
pub(crate) fn automatic_canvas_axes(dom: &Dom, id: usize) -> (bool, bool) {
    let Some(Variant::Enum(raw)) = dom.property(id, "AutomaticCanvasSize") else {
        return (false, false);
    };
    let Some(item) = super::enums::item_by_value("AutomaticSize", raw.to_u32()) else {
        return (false, false);
    };
    match item.name {
        "X" => (true, false),
        "Y" => (false, true),
        "XY" => (true, true),
        _ => (false, false),
    }
}

/// The resolved canvas size and content box of a ScrollingFrame: (canvas_w, canvas_h, frame_w, frame_h).
pub(crate) fn scrolling_frame_bounds(dom: &Dom, id: usize, own_box: Box2) -> (f32, f32, f32, f32) {
    let (_, _, pad_r, pad_b) = padding_of(dom, id);
    let (cs_sx, cs_ox, cs_sy, cs_oy) = udim2(dom, id, "CanvasSize");
    let mut canvas_w = (cs_sx * own_box.w + cs_ox).max(0.0);
    let mut canvas_h = (cs_sy * own_box.h + cs_oy).max(0.0);
    let (auto_canvas_x, auto_canvas_y) = automatic_canvas_axes(dom, id);
    if auto_canvas_x || auto_canvas_y {
        for child in dom.children(id) {
            let Some(class) = dom.class_of(child) else {
                continue;
            };
            if is_modifier(&class) {
                continue;
            }
            let (pxs, pxo, pys, pyo) = udim2(dom, child, "Position");
            let (sxs, sxo, sys, syo) = udim2(dom, child, "Size");
            let cw = sxs * canvas_w + sxo;
            let ch = sys * canvas_h + syo;
            let cx = pxs * canvas_w + pxo;
            let cy = pys * canvas_h + pyo;
            if auto_canvas_x {
                canvas_w = canvas_w.max(cx + cw + pad_r);
            }
            if auto_canvas_y {
                canvas_h = canvas_h.max(cy + ch + pad_b);
            }
        }
    }
    (canvas_w, canvas_h, own_box.w, own_box.h)
}

/// Intersect an inherited clip rectangle with an element's own rectangle.
fn intersect_clip(clip: Option<Box2>, rect: Box2) -> Box2 {
    match clip {
        Some(c) => {
            let x = c.x.max(rect.x);
            let y = c.y.max(rect.y);
            let right = (c.x + c.w).min(rect.x + rect.w);
            let bottom = (c.y + c.h).min(rect.y + rect.h);
            Box2 {
                x,
                y,
                w: (right - x).max(0.0),
                h: (bottom - y).max(0.0),
            }
        }
        None => rect,
    }
}

fn note_once_layout(key: &str, message: &str) {
    static SAID: std::sync::Mutex<Option<std::collections::BTreeSet<String>>> =
        std::sync::Mutex::new(None);
    let mut guard = SAID.lock().expect("said");
    if guard
        .get_or_insert_with(std::collections::BTreeSet::new)
        .insert(key.to_string())
    {
        eprintln!("[dew] {message}");
    }
}

fn report_auto(dom: &Dom, id: usize, want_x: bool, want_y: bool, did_x: bool, did_y: bool) {
    if (want_x && did_x || !want_x) && (want_y && did_y || !want_y) {
        return;
    }
    let mut has_real_child = false;
    for child in dom.children(id) {
        if let Some(class) = dom.class_of(child) {
            if !is_modifier(&class) {
                has_real_child = true;
                break;
            }
        }
    }
    if !has_real_child {
        return;
    }
    let name = dom.name_of(id).unwrap_or_default();
    if want_x && !did_x {
        note_once_layout(
            &format!("unmeasurable_x_{id}"),
            &format!(
                "{name}: AutomaticSize.X measured nothing -- every descendant is sized by scale on X, \
                 so it kept its authored width"
            ),
        );
    }
    if want_y && !did_y {
        note_once_layout(
            &format!("unmeasurable_y_{id}"),
            &format!(
                "{name}: AutomaticSize.Y measured nothing -- every descendant is sized by scale on Y, \
                 so it kept its authored height"
            ),
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SolvedItem {
    /// The corner radius of `clip`, 0.0 when square.
    pub clip_radius: f32,
    pub id: usize,
    pub rect: Box2,
    pub clip: Option<Box2>,
    pub depth: usize,
    pub z_index: i32,
}

#[allow(clippy::too_many_arguments)]
fn grow(
    dom: &Dom,
    node: usize,
    mut entry_rect: Box2,
    out: &[SolvedItem],
    from: usize,
    box_rect: Box2,
    grow_x: bool,
    grow_y: bool,
    pad_l: f32,
    pad_t: f32,
    pad_r: f32,
    pad_b: f32,
    offered: Box2,
) -> (Box2, bool, bool) {
    if !grow_x && !grow_y {
        return (entry_rect, false, false);
    }
    let child_depth = if out.len() > from { out[from].depth } else { 0 };
    let has_layout = list_layout_of(dom, node).is_some();

    let sweep = |base_x: Option<f32>, base_y: Option<f32>| -> (f32, f32) {
        let mut right = -f32::INFINITY;
        let mut bottom = -f32::INFINITY;
        let mut inherit_x: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        let mut inherit_y: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        inherit_x.insert(child_depth, 0.0);
        inherit_y.insert(child_depth, 0.0);

        for i in from..out.len() {
            let item = &out[i];
            let (sxs, sxo, sys, syo) = udim2(dom, item.id, "Size");
            let r = item.rect;
            let d = item.depth;
            let got_x = inherit_x.get(&d).copied().unwrap_or(0.0);
            let got_y = inherit_y.get(&d).copied().unwrap_or(0.0);
            let (_, _, own_r, own_b) = padding_of(dom, item.id);
            let has_content = i + 1 < out.len() && out[i + 1].depth > d;

            if grow_x {
                if sxs == 0.0 {
                    right = right.max(r.x + r.w + got_x);
                    inherit_x.insert(d + 1, got_x);
                } else if d == child_depth && sxs < 1.0 && (has_layout || !has_content) {
                    let resolved = if let Some(bx) = base_x {
                        bx * sxs + sxo
                    } else {
                        sxo
                    };
                    right = right.max(r.x + resolved + got_x);
                    inherit_x.insert(d + 1, got_x);
                } else {
                    inherit_x.insert(d + 1, got_x + own_r);
                }
            }

            if grow_y {
                if sys == 0.0 {
                    bottom = bottom.max(r.y + r.h + got_y);
                    inherit_y.insert(d + 1, got_y);
                } else if d == child_depth && sys < 1.0 && (has_layout || !has_content) {
                    let resolved = if let Some(by) = base_y {
                        by * sys + syo
                    } else {
                        syo
                    };
                    bottom = bottom.max(r.y + resolved + got_y);
                    inherit_y.insert(d + 1, got_y);
                } else {
                    inherit_y.insert(d + 1, got_y + own_b);
                }
            }
        }
        (right, bottom)
    };

    let (right, bottom) = if has_layout {
        sweep(Some(offered.w), Some(offered.h))
    } else {
        let (off_x, off_y) = sweep(None, None);
        sweep(
            if off_x != -f32::INFINITY {
                Some((off_x - box_rect.x).max(0.0))
            } else {
                Some(0.0)
            },
            if off_y != -f32::INFINITY {
                Some((off_y - box_rect.y).max(0.0))
            } else {
                Some(0.0)
            },
        )
    };

    let mut did_x = false;
    let mut did_y = false;
    if grow_x && right != -f32::INFINITY {
        let measured_w = (right - box_rect.x) + pad_l + pad_r;
        entry_rect.w = entry_rect.w.max(measured_w);
        did_x = true;
    }
    if grow_y && bottom != -f32::INFINITY {
        let measured_h = (bottom - box_rect.y) + pad_t + pad_b;
        entry_rect.h = entry_rect.h.max(measured_h);
        did_y = true;
    }

    let class = dom.class_of(node).unwrap_or_default();
    if draws_text(&class) {
        let text_content = text_for(dom, node, &class).unwrap_or_default();
        let text_size = number(dom, node, "TextSize").unwrap_or(14.0);
        let wrapped = boolean(dom, node, "TextWrapped").unwrap_or(false);
        let measured = if wrapped {
            let avail_w = if entry_rect.w > 0.0 {
                (entry_rect.w - pad_l - pad_r).max(0.0)
            } else if offered.w > 0.0 {
                (offered.w - pad_l - pad_r).max(0.0)
            } else {
                0.0
            };
            if avail_w > 0.0 {
                crate::services::measure_wrapped(&text_content, text_size, avail_w)
            } else {
                crate::services::measure(&text_content, text_size)
            }
        } else {
            crate::services::measure(&text_content, text_size)
        };
        if let Ok((tw, th)) = measured {
            if grow_x {
                let needed_w = tw + pad_l + pad_r;
                entry_rect.w = entry_rect.w.max(needed_w);
                did_x = true;
            }
            if grow_y {
                let needed_h = th + pad_t + pad_b;
                entry_rect.h = entry_rect.h.max(needed_h);
                did_y = true;
            }
        }
    }

    (entry_rect, did_x, did_y)
}

#[allow(clippy::too_many_arguments)]
fn visit(
    dom: &Dom,
    id: usize,
    parent_box: Box2,
    clip: Option<Box2>,
    clip_radius: f32,
    depth: usize,
    laid_out: bool,
    offered: Box2,
    out: &mut Vec<SolvedItem>,
) {
    if boolean(dom, id, "Visible") == Some(false) {
        return;
    }

    let rect = solve_rect(dom, id, parent_box, laid_out);
    let z = number(dom, id, "ZIndex").unwrap_or(1.0) as i32;
    let entry_idx = out.len();
    out.push(SolvedItem {
        id,
        rect,
        clip,
        depth,
        clip_radius,
        z_index: z,
    });

    let is_scrolling_frame = dom.class_of(id).as_deref() == Some("ScrollingFrame");

    let box_rect = content_box(dom, id, rect);
    // A ROUNDED PARENT MASKS ITS DESCENDANTS TO THE ROUNDING. Roblox does; Dew
    // clipped to a rectangle and leaked the corner pixels, which milestone 3
    // recorded as an observable divergence rather than fixing.
    //
    // THE ROUNDER OF THE TWO WINS on nesting, matching `ar_clip_push_rounded`: a
    // square clip inside a rounded one is still inside the rounded one.
    //
    // A SCROLLINGFRAME ALWAYS CLIPS ITS CONTENT TO ITS OWN BOX, matching Roblox.
    let (child_clip, child_clip_radius) =
        if is_scrolling_frame || boolean(dom, id, "ClipsDescendants") == Some(true) {
            (
                Some(intersect_clip(clip, rect)),
                corner_radius(dom, id).max(clip_radius),
            )
        } else {
            (clip, clip_radius)
        };

    let (pad_l, pad_t, pad_r, pad_b) = padding_of(dom, id);
    let (grow_x, grow_y) = automatic_axes(dom, id);
    let children_from = out.len();

    let (canvas_w, canvas_h) = if is_scrolling_frame {
        let (auto_canvas_x, auto_canvas_y) = automatic_canvas_axes(dom, id);
        let (cs_sx, cs_ox, cs_sy, cs_oy) = udim2(dom, id, "CanvasSize");
        let mut cw = (cs_sx * box_rect.w + cs_ox).max(0.0);
        let mut ch = (cs_sy * box_rect.h + cs_oy).max(0.0);

        let canvas_box = Box2 {
            x: box_rect.x,
            y: box_rect.y,
            w: cw,
            h: ch,
        };
        place_children(
            dom,
            id,
            canvas_box,
            child_clip,
            child_clip_radius,
            depth,
            out,
        );

        if auto_canvas_x || auto_canvas_y {
            let mut right = -f32::INFINITY;
            let mut bottom = -f32::INFINITY;
            for item in &out[children_from..] {
                if item.depth == depth + 1 {
                    right = right.max(item.rect.x + item.rect.w);
                    bottom = bottom.max(item.rect.y + item.rect.h);
                }
            }
            let mut grew_cw = false;
            let mut grew_ch = false;
            if auto_canvas_x && right != -f32::INFINITY {
                let needed_w = (right - box_rect.x) + pad_r;
                if needed_w > cw {
                    cw = needed_w;
                    grew_cw = true;
                }
            }
            if auto_canvas_y && bottom != -f32::INFINITY {
                let needed_h = (bottom - box_rect.y) + pad_b;
                if needed_h > ch {
                    ch = needed_h;
                    grew_ch = true;
                }
            }

            if grew_cw || grew_ch {
                let mut dependent = false;
                for item in &out[children_from..] {
                    let (sxs, _, sys, _) = udim2(dom, item.id, "Size");
                    if (grew_cw && sxs != 0.0) || (grew_ch && sys != 0.0) {
                        dependent = true;
                        break;
                    }
                }
                if dependent {
                    out.truncate(children_from);
                    let grown_canvas_box = Box2 {
                        x: box_rect.x,
                        y: box_rect.y,
                        w: cw,
                        h: ch,
                    };
                    place_children(
                        dom,
                        id,
                        grown_canvas_box,
                        child_clip,
                        child_clip_radius,
                        depth,
                        out,
                    );
                }
            }
        }
        (cw, ch)
    } else {
        place_children(dom, id, box_rect, child_clip, child_clip_radius, depth, out);
        (0.0, 0.0)
    };

    let before_w = out[entry_idx].rect.w;
    let before_h = out[entry_idx].rect.h;

    let (new_rect, did_x, did_y) = grow(
        dom,
        id,
        out[entry_idx].rect,
        out,
        children_from,
        box_rect,
        grow_x,
        grow_y,
        pad_l,
        pad_t,
        pad_r,
        pad_b,
        offered,
    );
    out[entry_idx].rect = new_rect;

    let grew_w = out[entry_idx].rect.w > before_w;
    let grew_h = out[entry_idx].rect.h > before_h;

    // AnchorPoint post-growth adjustment
    if grew_w || grew_h {
        let anchor = vector2(dom, id, "AnchorPoint");
        let dx = if grew_w {
            (out[entry_idx].rect.w - before_w) * anchor.x
        } else {
            0.0
        };
        let dy = if grew_h {
            (out[entry_idx].rect.h - before_h) * anchor.y
        } else {
            0.0
        };
        if dx != 0.0 || dy != 0.0 {
            out[entry_idx].rect.x -= dx;
            out[entry_idx].rect.y -= dy;
            for item in &mut out[children_from..] {
                item.rect.x -= dx;
                item.rect.y -= dy;
            }
        }
    }

    // Re-placement: if any descendant depends on an axis that grew via scale
    if grew_w || grew_h {
        let mut dependent = false;
        for item in &out[children_from..] {
            let (sxs, _, sys, _) = udim2(dom, item.id, "Size");
            if (grew_w && sxs != 0.0) || (grew_h && sys != 0.0) {
                dependent = true;
                break;
            }
        }
        if dependent {
            out.truncate(children_from);
            let grown_box = content_box(dom, id, out[entry_idx].rect);
            place_children(
                dom,
                id,
                grown_box,
                child_clip,
                child_clip_radius,
                depth,
                out,
            );
        }
    }

    // ScrollingFrame scroll shift: applied AFTER children have resolved their sizes
    // and layout/growth. Shifts all descendants by -CanvasPosition (clamped to scrollable range).
    if is_scrolling_frame {
        let final_box = content_box(dom, id, out[entry_idx].rect);
        let canvas_pos = vector2(dom, id, "CanvasPosition");
        let max_scroll_x = (canvas_w - final_box.w).max(0.0);
        let max_scroll_y = (canvas_h - final_box.h).max(0.0);
        let scroll_x = canvas_pos.x.clamp(0.0, max_scroll_x);
        let scroll_y = canvas_pos.y.clamp(0.0, max_scroll_y);

        if scroll_x != 0.0 || scroll_y != 0.0 {
            for item in &mut out[children_from..] {
                item.rect.x -= scroll_x;
                item.rect.y -= scroll_y;
            }
        }
    }

    report_auto(dom, id, grow_x, grow_y, did_x, did_y);
}

/// Which end of the cross axis a `UIListLayout` gathers its children against.
///
/// `Left` and `Top` are the same answer on different axes, and Roblox spells
/// them differently for the two enums, so both map onto one three-way.
#[derive(Clone, Copy, PartialEq)]
enum CrossAlign {
    Start,
    Center,
    End,
}

fn alignment_of(dom: &Dom, layout_id: usize, property: &str) -> CrossAlign {
    let Some(Variant::Enum(raw)) = dom.property(layout_id, property) else {
        return CrossAlign::Start;
    };
    match super::enums::item_by_value(property, raw.to_u32()).map(|item| item.name) {
        Some("Center") => CrossAlign::Center,
        Some("Right") | Some("Bottom") => CrossAlign::End,
        // `Left`, `Top`, and anything a newer build adds that this does not know.
        _ => CrossAlign::Start,
    }
}

/// How far to move a placed child along the cross axis to satisfy the alignment.
fn cross_shift(align: CrossAlign, slot_start: f32, slot_len: f32, at: f32, len: f32) -> f32 {
    let target = match align {
        CrossAlign::Start => slot_start,
        CrossAlign::Center => slot_start + (slot_len - len) / 2.0,
        CrossAlign::End => slot_start + slot_len - len,
    };
    target - at
}

fn place_children(
    dom: &Dom,
    id: usize,
    box_rect: Box2,
    child_clip: Option<Box2>,
    child_clip_radius: f32,
    depth: usize,
    out: &mut Vec<SolvedItem>,
) {
    if let Some(layout_id) = list_layout_of(dom, id) {
        let (_, pad_offset) = udim(dom, layout_id, "Padding");
        let is_horizontal =
            if let Some(Variant::Enum(raw)) = dom.property(layout_id, "FillDirection") {
                super::enums::item_by_value("FillDirection", raw.to_u32())
                    .map(|item| item.name == "Horizontal")
                    .unwrap_or(false)
            } else {
                false
            };

        let align_x = alignment_of(dom, layout_id, "HorizontalAlignment");
        let align_y = alignment_of(dom, layout_id, "VerticalAlignment");

        let mut kids: Vec<(usize, i32, usize)> = Vec::new();
        for (idx, child) in dom.children(id).iter().copied().enumerate() {
            let Some(class) = dom.class_of(child) else {
                continue;
            };
            if is_modifier(&class) {
                continue;
            }
            let order = number(dom, child, "LayoutOrder").unwrap_or(0.0) as i32;
            kids.push((child, order, idx));
        }
        kids.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2)));

        let mut cursor = 0.0;
        for (child, _, _) in kids {
            let slot = if is_horizontal {
                Box2 {
                    x: box_rect.x + cursor,
                    y: box_rect.y,
                    w: box_rect.w,
                    h: box_rect.h,
                }
            } else {
                Box2 {
                    x: box_rect.x,
                    y: box_rect.y + cursor,
                    w: box_rect.w,
                    h: box_rect.h,
                }
            };
            let before = out.len();
            visit(
                dom,
                child,
                slot,
                child_clip,
                child_clip_radius,
                depth + 1,
                true,
                box_rect,
                out,
            );
            if out.len() > before {
                // ALIGNMENT SHIFTS THE RUN ON THE CROSS AXIS, after the child has
                // resolved its own size against the full slot. Doing it before
                // would change what a `Scale` size resolves against, which is a
                // different behaviour wearing the same name.
                //
                // The host accepted `HorizontalAlignment` and `VerticalAlignment`
                // and the solver read neither, so a centred list drew flush to
                // the corner. Found by the gallery's differential pass.
                let placed_rect = out[before].rect;
                let cross = if is_horizontal {
                    cross_shift(
                        align_y,
                        box_rect.y,
                        box_rect.h,
                        placed_rect.y,
                        placed_rect.h,
                    )
                } else {
                    cross_shift(
                        align_x,
                        box_rect.x,
                        box_rect.w,
                        placed_rect.x,
                        placed_rect.w,
                    )
                };
                if cross != 0.0 {
                    for item in out[before..].iter_mut() {
                        if is_horizontal {
                            item.rect.y += cross;
                        } else {
                            item.rect.x += cross;
                        }
                    }
                }
                let placed_rect = out[before].rect;
                cursor += (if is_horizontal {
                    placed_rect.w
                } else {
                    placed_rect.h
                }) + pad_offset;
            }
        }
    } else {
        for child in dom.children(id) {
            let Some(class) = dom.class_of(child) else {
                continue;
            };
            if is_modifier(&class) {
                continue;
            }
            visit(
                dom,
                child,
                box_rect,
                child_clip,
                child_clip_radius,
                depth + 1,
                false,
                box_rect,
                out,
            );
        }
    }
}

pub fn solve_layout(dom: &Dom, root: usize, surface: Box2) -> Vec<SolvedItem> {
    let mut out: Vec<SolvedItem> = Vec::new();
    let root_box = content_box(dom, root, surface);
    place_children(dom, root, root_box, None, 0.0, 0, &mut out);
    out
}

/// Resolve everything under `root` into paint order, back to front.
///
/// `root` itself is the surface and is not placed; its children are laid out
/// against the box the surface offers, which is how a `ScreenGui` behaves.
///
/// PAINT ORDER IS ZINDEX, THEN DEPTH, THEN DECLARATION, per LAYOUT.md section 6.
/// A STABLE sort on ZIndex alone gives the other two for free, because the walk
/// is already depth-first in declaration order -- and `sort_by_key` on a `Vec`
/// is stable, which is load bearing rather than incidental here.
pub fn display_list(dom: &Dom, root: usize, width: f32, height: f32) -> Vec<Placed> {
    let surface = Box2 {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    };
    let solved = solve_layout(dom, root, surface);
    let mut collected: Vec<(i32, Placed)> = solved
        .into_iter()
        .filter(|item| item.rect.w > 0.0 && item.rect.h > 0.0)
        .map(|item| {
            (
                item.z_index,
                Placed {
                    id: item.id,
                    rect: item.rect,
                    clip: item.clip,
                    clip_radius: item.clip_radius,
                },
            )
        })
        .collect();
    collected.sort_by_key(|(z, _)| *z);
    collected.into_iter().map(|(_, placed)| placed).collect()
}

fn resolved_text_size(dom: &Dom, id: usize, placed_rect: Box2) -> f32 {
    let authored_size = number(dom, id, "TextSize").unwrap_or(14.0);
    if boolean(dom, id, "TextScaled") != Some(true) {
        return authored_size;
    }

    let class = dom.class_of(id).unwrap_or_default();
    let Some(text_content) = text_for(dom, id, &class) else {
        return authored_size;
    };
    if text_content.is_empty() {
        return authored_size;
    }

    let (pad_l, pad_t, pad_r, pad_b) = padding_of(dom, id);
    let box_w = (placed_rect.w - pad_l - pad_r).max(0.0);
    let box_h = (placed_rect.h - pad_t - pad_b).max(0.0);
    if box_w <= 0.0 || box_h <= 0.0 {
        return authored_size;
    }

    let wrapped = boolean(dom, id, "TextWrapped").unwrap_or(false);
    if !wrapped {
        if let Ok((w1, _)) = crate::services::measure(&text_content, 1.0) {
            let lines = text_content.split('\n').count().max(1) as f32;
            let h1 = lines * 1.0 * 1.5;
            let scale_w = if w1 > 0.0 { box_w / w1 } else { f32::INFINITY };
            let scale_h = if h1 > 0.0 { box_h / h1 } else { f32::INFINITY };
            return scale_w.min(scale_h);
        }
    } else {
        let mut low = 1.0_f32;
        let mut high = (box_h / 1.5).max(1.0);
        for _ in 0..16 {
            let mid = (low + high) / 2.0;
            if let Ok((mw, mh)) = crate::services::measure_wrapped(&text_content, mid, box_w) {
                if mw <= box_w && mh <= box_h {
                    low = mid;
                } else {
                    high = mid;
                }
            } else {
                break;
            }
        }
        return low;
    }

    authored_size
}

/// Turn one placed element into the display list node the painter consumes.
///
/// `&mut Dom` FOR ONE REASON, AND IT IS WORTH THE SIGNATURE. Resolving an image
/// reads a file, decodes it, and remembers the answer; without the remembering,
/// a mod with an icon would decode that icon on every frame it is drawn. The
/// cache lives on the DOM because it is per-mod, so building a node writes to it.
/// `display_list` is still `&Dom` and `input::hit` still reads the same
/// placement, which is the property that mattered.
fn node(dom: &mut Dom, placed: &Placed, sequence: u64) -> Node {
    let id = placed.id;
    let class = dom.class_of(id).unwrap_or_default();
    let image = if draws_image(&class) {
        image_of(dom, id)
    } else {
        None
    };
    let dom = &*dom;
    Node {
        id: sequence,
        name: dom.name_of(id).unwrap_or_default(),
        rect: Rect {
            x: placed.rect.x,
            y: placed.rect.y,
            w: placed.rect.w,
            h: placed.rect.h,
        },
        fill: colour(dom, id, "BackgroundColor3"),
        alpha: alpha_from(dom, id, "BackgroundTransparency"),
        radius: corner_radius(dom, id),
        clip_radius: placed.clip_radius,
        clip: placed.clip.map(|c| Rect {
            x: c.x,
            y: c.y,
            w: c.w,
            h: c.h,
        }),
        stroke: stroke_of(dom, id),
        gradient: gradient_of(dom, id),
        text: if draws_text(&class) {
            text_for(dom, id, &class)
        } else {
            None
        },
        text_size: if draws_text(&class) {
            resolved_text_size(dom, id, placed.rect)
        } else {
            number(dom, id, "TextSize").unwrap_or(14.0)
        },
        text_align_x: align(dom, id, "TextXAlignment", "TextXAlignment"),
        text_align_y: align(dom, id, "TextYAlignment", "TextYAlignment"),
        text_colour: colour(dom, id, "TextColor3"),
        // TRANSPARENCY IS THE INVERSE OF ALPHA, as it is everywhere in this
        // vocabulary: Roblox counts how see-through a thing is and the painter
        // counts how solid it is.
        text_alpha: 1.0
            - number(dom, id, "TextTransparency")
                .unwrap_or(0.0)
                .clamp(0.0, 1.0),
        image,
    }
}

/// Build the display list for the subtree under `root`.
pub fn frame(dom: &mut Dom, root: usize, width: f32, height: f32) -> Frame {
    // THE SEQUENCE NUMBER IS ASSIGNED AFTER THE SORT, and it was assigned before
    // it when this walk built `Node`s inline. Nothing read it, so nothing broke;
    // it is now what it claims to be, a paint index.
    //
    // THE PLACEMENT PASS IS COLLECTED BEFORE THE NODES ARE BUILT, which it was
    // anyway, and now has to be: building a node may resolve an image and so
    // needs the DOM mutably, while `display_list` reads it. One pass then the
    // other keeps both borrows to themselves without a second walk.
    let placed = display_list(dom, root, width, height);
    let nodes = placed
        .iter()
        .enumerate()
        .map(|(i, placed)| node(dom, placed, i as u64 + 1))
        .collect();

    Frame {
        width,
        height,
        nodes,
        focused: false,
    }
}

/// Compute geometry and write it back so a guest can read it.
///
/// `AbsolutePosition` and `AbsoluteSize` are what an application reads to make
/// decisions, and until they are computed they read the reflection database's
/// default of zero -- which is a plausible number and therefore worse than an
/// error. They are written directly into the arena rather than through the
/// assignment path, which refuses them as read-only, and that is correct on both
/// counts: read-only to a guest, written by the host that computed them.
pub fn commit_geometry(dom: &mut Dom, root: usize, width: f32, height: f32) {
    let surface = Box2 {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    };
    let solved = solve_layout(dom, root, surface);
    for item in solved {
        dom.set_internal(
            item.id,
            "AbsolutePosition",
            Variant::Vector2(Vector2::new(item.rect.x, item.rect.y)),
        );
        dom.set_internal(
            item.id,
            "AbsoluteSize",
            Variant::Vector2(Vector2::new(item.rect.w, item.rect.h)),
        );
    }
}

/// Render whatever is under `root` in a shared DOM.
pub fn frame_of(dom: &SharedDom, root: usize, width: f32, height: f32) -> Frame {
    let mut guard = dom.lock().expect("dom");
    commit_geometry(&mut guard, root, width, height);
    frame(&mut guard, root, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamodel::{install, install_vocabulary, SharedDom};
    use mlua::prelude::*;

    /// Build a tree from Luau, then render it. The guest half is real, so these
    /// exercise the same path a mod takes rather than a Rust-only fixture.
    fn render(src: &str, width: f32, height: f32) -> Frame {
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
        lua.load(src).exec().expect("guest");
        frame_of(&dom, root, width, height)
    }

    /// The same, with a directory of real assets for `mod://` to resolve against.
    ///
    /// A REAL FILE ON A REAL DISK, DECODED BY THE REAL DECODER. Handing the DOM a
    /// pre-built `Bitmap` would test the display list and skip the half this
    /// sprint added — the scheme, the path check, and turning bytes into pixels.
    /// The files are written and removed here so the suite carries no fixture
    /// that could drift from what the tests believe it contains.
    fn render_with_assets(src: &str, files: &[(&str, Vec<u8>)], width: f32, height: f32) -> Frame {
        let dir = std::env::temp_dir().join(format!(
            "dew-render-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        for (name, bytes) in files {
            std::fs::write(dir.join(name), bytes).expect("asset");
        }

        let lua = Lua::new();
        let dom = SharedDom::default();
        install(&lua, &dom).expect("install");
        install_vocabulary(&lua).expect("vocabulary");
        let root = {
            let mut guard = dom.lock().expect("dom");
            guard.assets.set_root(dir.clone());
            guard.insert("Folder".into(), "Root".into())
        };
        lua.globals()
            .set(
                "root",
                crate::datamodel::handle(&lua, &dom, root).expect("root handle"),
            )
            .expect("root");
        lua.load(src).exec().expect("guest");
        let frame = frame_of(&dom, root, width, height);
        let _ = std::fs::remove_dir_all(&dir);
        frame
    }

    /// A solid PNG of a known size and colour, encoded rather than committed.
    fn png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let pixels: Vec<u8> = rgba
            .iter()
            .copied()
            .cycle()
            .take((width * height * 4) as usize)
            .collect();
        let img = image::RgbaImage::from_raw(width, height, pixels).expect("raw");
        let mut buffer = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .expect("encode");
        buffer.into_inner()
    }

    /// The host accepted `TextTransparency` and the display list had nowhere to
    /// put it, so every run painted solid. Found by the gallery's differential
    /// pass, which reported that changing the property moved no pixels.
    /// Roblox masks descendants against the clipping parent's `UICorner`. The
    /// display list carried a bare rectangle, so corner pixels leaked --
    /// milestone 3 recorded it as an observable divergence rather than fixing it.
    #[test]
    fn a_clipping_parent_passes_its_corner_radius_to_its_children() {
        let f = render(
            r#"
            local card = Instance.new("Frame")
            card.Size = UDim2.new(0, 120, 0, 60)
            card.ClipsDescendants = true
            card.Parent = root
            local corner = Instance.new("UICorner")
            corner.CornerRadius = UDim.new(0, 14)
            corner.Parent = card
            local bar = Instance.new("Frame")
            bar.Size = UDim2.new(1, 0, 0, 20)
            bar.Parent = card
        "#,
            200.0,
            100.0,
        );
        let bar = f
            .nodes
            .iter()
            .find(|n| n.rect.h == 20.0)
            .expect("the titlebar is in the display list");
        assert_eq!(
            bar.clip_radius, 14.0,
            "the clip radius did not reach the child"
        );
        // The card itself is not clipped BY itself.
        let card = f
            .nodes
            .iter()
            .find(|n| n.rect.h == 60.0)
            .expect("the card is in the display list");
        assert_eq!(card.clip_radius, 0.0);
    }

    #[test]
    fn text_transparency_reaches_the_display_list() {
        let f = render(
            r#"
            local t = Instance.new("TextLabel")
            t.Size = UDim2.new(0, 100, 0, 20)
            t.Text = "hello"
            t.TextTransparency = 0.25
            t.Parent = root
        "#,
            200.0,
            100.0,
        );
        assert_eq!(f.nodes.len(), 1);
        assert!(
            (f.nodes[0].text_alpha - 0.75).abs() < 0.001,
            "text_alpha was {}",
            f.nodes[0].text_alpha
        );
    }

    #[test]
    fn text_with_no_transparency_is_solid() {
        let f = render(
            r#"
            local t = Instance.new("TextLabel")
            t.Size = UDim2.new(0, 100, 0, 20)
            t.Text = "hello"
            t.Parent = root
        "#,
            200.0,
            100.0,
        );
        assert_eq!(f.nodes[0].text_alpha, 1.0);
    }

    /// The solver read neither alignment enum, so a centred list drew flush to
    /// the corner. Also found by the differential pass.
    #[test]
    fn a_list_layout_centres_its_children_on_the_cross_axis() {
        let f = render(
            r#"
            local panel = Instance.new("Frame")
            panel.Size = UDim2.new(0, 200, 0, 100)
            panel.Parent = root
            local layout = Instance.new("UIListLayout")
            layout.HorizontalAlignment = Enum.HorizontalAlignment.Center
            layout.Parent = panel
            local row = Instance.new("Frame")
            row.Size = UDim2.new(0, 100, 0, 20)
            row.Parent = panel
        "#,
            200.0,
            100.0,
        );
        let row = f
            .nodes
            .iter()
            .find(|n| n.rect.w == 100.0 && n.rect.h == 20.0)
            .expect("the row is in the display list");
        // 200 wide panel, 100 wide row, centred -> x = 50.
        assert_eq!(row.rect.x, 50.0, "row was not centred");
    }

    #[test]
    fn a_list_layout_defaults_to_the_start_of_the_cross_axis() {
        let f = render(
            r#"
            local panel = Instance.new("Frame")
            panel.Size = UDim2.new(0, 200, 0, 100)
            panel.Parent = root
            local layout = Instance.new("UIListLayout")
            layout.Parent = panel
            local row = Instance.new("Frame")
            row.Size = UDim2.new(0, 100, 0, 20)
            row.Parent = panel
        "#,
            200.0,
            100.0,
        );
        let row = f
            .nodes
            .iter()
            .find(|n| n.rect.w == 100.0 && n.rect.h == 20.0)
            .expect("the row is in the display list");
        assert_eq!(
            row.rect.x, 0.0,
            "an unset alignment should not move a child"
        );
    }

    #[test]
    fn offset_and_scale_resolve_against_the_parent() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0.5, 10, 0, 40)
            a.Position = UDim2.new(0, 20, 0.5, 0)
            a.Parent = root
        "#,
            200.0,
            100.0,
        );
        assert_eq!(f.nodes.len(), 1);
        let r = f.nodes[0].rect;
        // 0.5 * 200 + 10 = 110 wide, 40 tall; x = 20, y = 0.5 * 100 = 50.
        assert_eq!((r.x, r.y, r.w, r.h), (20.0, 50.0, 110.0, 40.0));
    }

    #[test]
    fn anchor_point_shifts_by_a_fraction_of_the_resolved_size() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 80, 0, 40)
            a.Position = UDim2.new(0.5, 0, 0.5, 0)
            a.AnchorPoint = Vector2.new(0.5, 0.5)
            a.Parent = root
        "#,
            200.0,
            100.0,
        );
        let r = f.nodes[0].rect;
        // Centred on (100, 50), so the top-left is (100 - 40, 50 - 20).
        assert_eq!((r.x, r.y), (60.0, 30.0));
    }

    #[test]
    fn a_child_resolves_against_its_parent_not_the_surface() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 100, 0, 100)
            a.Position = UDim2.new(0, 50, 0, 0)
            a.Parent = root
            local b = Instance.new("Frame")
            b.Size = UDim2.new(0.5, 0, 0.5, 0)
            b.Parent = a
        "#,
            200.0,
            200.0,
        );
        let child = f.nodes.iter().find(|n| n.rect.w == 50.0).expect("child");
        assert_eq!((child.rect.x, child.rect.y), (50.0, 0.0));
    }

    #[test]
    fn an_invisible_element_hides_its_subtree() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 10, 0, 10)
            a.Visible = false
            a.Parent = root
            local b = Instance.new("Frame")
            b.Size = UDim2.new(0, 10, 0, 10)
            b.Parent = a
        "#,
            100.0,
            100.0,
        );
        assert!(f.nodes.is_empty());
    }

    #[test]
    fn a_zero_area_element_is_absent_but_still_positions_its_children() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 0, 0, 0)
            a.Position = UDim2.new(0, 30, 0, 30)
            a.Parent = root
            local b = Instance.new("Frame")
            b.Size = UDim2.new(0, 10, 0, 10)
            b.Parent = a
        "#,
            100.0,
            100.0,
        );
        assert_eq!(f.nodes.len(), 1);
        assert_eq!((f.nodes[0].rect.x, f.nodes[0].rect.y), (30.0, 30.0));
    }

    #[test]
    fn zindex_decides_paint_order_over_declaration() {
        let f = render(
            r#"
            local function box(name, z)
                local n = Instance.new("Frame")
                n.Name = name
                n.Size = UDim2.new(0, 10, 0, 10)
                n.ZIndex = z
                n.Parent = root
            end
            box("last", 1)
            box("first", 5)
        "#,
            100.0,
            100.0,
        );
        let order: Vec<&str> = f.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(order, vec!["last", "first"]);
    }

    #[test]
    fn a_modifier_is_a_property_of_its_parent_and_not_a_rectangle() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 10, 0, 10)
            a.Parent = root
            local c = Instance.new("UICorner")
            c.CornerRadius = UDim.new(0, 14)
            c.Parent = a
        "#,
            100.0,
            100.0,
        );
        assert_eq!(f.nodes.len(), 1, "the UICorner must not be drawn");
        assert_eq!(f.nodes[0].radius, 14.0);
    }

    #[test]
    fn transparency_is_inverted_into_alpha() {
        // Roblox is opaque at 0, the display list is opaque at 1.
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 10, 0, 10)
            a.BackgroundTransparency = 0.25
            a.Parent = root
        "#,
            100.0,
            100.0,
        );
        assert_eq!(f.nodes[0].alpha, 0.75);
    }

    #[test]
    fn a_colour_reaches_the_display_list_as_bytes() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 10, 0, 10)
            a.BackgroundColor3 = Color3.fromRGB(18, 23, 34)
            a.Parent = root
        "#,
            100.0,
            100.0,
        );
        assert_eq!(f.nodes[0].fill, Some(Rgb(18, 23, 34)));
    }

    #[test]
    fn text_reaches_the_display_list_only_from_a_text_class() {
        let f = render(
            r#"
            local t = Instance.new("TextLabel")
            t.Size = UDim2.new(0, 80, 0, 20)
            t.Text = "hello"
            t.TextSize = 18
            t.Parent = root
        "#,
            100.0,
            100.0,
        );
        assert_eq!(f.nodes[0].text.as_deref(), Some("hello"));
        assert_eq!(f.nodes[0].text_size, 18.0);
    }

    #[test]
    fn clips_descendants_carries_a_clip_to_the_children() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 50, 0, 50)
            a.ClipsDescendants = true
            a.Parent = root
            local b = Instance.new("Frame")
            b.Size = UDim2.new(0, 200, 0, 200)
            b.Parent = a
        "#,
            100.0,
            100.0,
        );
        let child = f.nodes.iter().find(|n| n.rect.w == 200.0).expect("child");
        let clip = child.clip.expect("clipped");
        assert_eq!((clip.w, clip.h), (50.0, 50.0));
        assert!(
            f.nodes[0].clip.is_none(),
            "the clipper itself is not clipped"
        );
    }

    #[test]
    fn ui_padding_insets_children() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 100, 0, 100)
            a.Parent = root

            local pad = Instance.new("UIPadding")
            pad.PaddingLeft = UDim.new(0, 10)
            pad.PaddingTop = UDim.new(0, 15)
            pad.PaddingRight = UDim.new(0, 20)
            pad.PaddingBottom = UDim.new(0, 25)
            pad.Parent = a

            local b = Instance.new("Frame")
            b.Size = UDim2.new(1, 0, 1, 0)
            b.Parent = a
        "#,
            200.0,
            200.0,
        );
        let parent = f.nodes.iter().find(|n| n.rect.w == 100.0).expect("parent");
        assert_eq!(parent.rect.x, 0.0);
        assert_eq!(parent.rect.y, 0.0);
        assert_eq!(parent.rect.w, 100.0);
        assert_eq!(parent.rect.h, 100.0);

        let child = f.nodes.iter().find(|n| n.rect.w == 70.0).expect("child");
        assert_eq!(child.rect.x, 10.0);
        assert_eq!(child.rect.y, 15.0);
        assert_eq!(child.rect.w, 70.0);
        assert_eq!(child.rect.h, 60.0);
    }

    // ── Images ───────────────────────────────────────────────────────────────

    #[test]
    fn an_image_reaches_the_display_list_as_decoded_pixels() {
        // THE SPRINT'S OWN DONE-TEST. `Image` and `ImageContent` were accepted by
        // the DataModel and there was no route from either to a pixel on any
        // path; this is that route, from Luau to a bitmap a painter can draw.
        let f = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 40, 0, 40)
            i.ImageContent = Content.fromUri("mod://dot.png")
            i.Parent = root
        "#,
            &[("dot.png", png(4, 2, [10, 20, 30, 255]))],
            100.0,
            100.0,
        );
        let image = f.nodes[0]
            .image
            .as_ref()
            .expect("the node carries an image");
        assert_eq!(image.uri, "mod://dot.png");
        let bitmap = image.bitmap.as_ref().expect("the asset resolved to pixels");
        assert_eq!((bitmap.width, bitmap.height), (4, 2));
        assert_eq!(&bitmap.rgba[..4], &[10, 20, 30, 255]);
    }

    #[test]
    fn the_legacy_and_the_modern_property_name_the_same_asset() {
        // TWO GENERATIONS OF ONE IDEA. `Image` is a `ContentId` string and
        // `ImageContent` is a `Content` URI; a host takes both, and a mod written
        // five years ago must not need editing to draw.
        let asset = &[("dot.png", png(2, 2, [7, 8, 9, 255]))];
        let legacy = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.Image = "mod://dot.png"
            i.Parent = root
        "#,
            asset,
            50.0,
            50.0,
        );
        let modern = render_with_assets(
            r#"
            local i = Instance.new("ImageButton")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.ImageContent = Content.fromUri("mod://dot.png")
            i.Parent = root
        "#,
            asset,
            50.0,
            50.0,
        );

        for (which, frame) in [("Image", &legacy), ("ImageContent", &modern)] {
            let image = frame.nodes[0]
                .image
                .as_ref()
                .unwrap_or_else(|| panic!("{which} produced no image node"));
            assert_eq!(image.uri, "mod://dot.png", "{which}");
            let bitmap = image.bitmap.as_ref().expect("resolved");
            assert_eq!(&bitmap.rgba[..4], &[7, 8, 9, 255], "{which}");
        }
    }

    #[test]
    fn image_content_wins_when_a_guest_sets_both() {
        // The engine's own migration writes through `Image` into `ImageContent`,
        // so the modern property is the more recent statement of intent.
        let f = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.Image = "mod://old.png"
            i.ImageContent = Content.fromUri("mod://new.png")
            i.Parent = root
        "#,
            &[
                ("old.png", png(1, 1, [1, 1, 1, 255])),
                ("new.png", png(1, 1, [2, 2, 2, 255])),
            ],
            50.0,
            50.0,
        );
        let image = f.nodes[0].image.as_ref().expect("image");
        assert_eq!(image.uri, "mod://new.png");
    }

    #[test]
    fn image_colour_and_transparency_reach_the_display_list() {
        let f = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.ImageContent = Content.fromUri("mod://dot.png")
            i.ImageColor3 = Color3.fromRGB(240, 186, 96)
            i.ImageTransparency = 0.25
            i.Parent = root
        "#,
            &[("dot.png", png(1, 1, [255, 255, 255, 255]))],
            50.0,
            50.0,
        );
        let image = f.nodes[0].image.as_ref().expect("image");
        assert_eq!(image.tint, Some(Rgb(240, 186, 96)));
        // Inverted at the source, like every other transparency in this file.
        assert_eq!(image.alpha, 0.75);
    }

    #[test]
    fn a_rect_offset_and_size_become_the_source_rectangle() {
        let f = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.ImageContent = Content.fromUri("mod://sheet.png")
            i.ImageRectOffset = Vector2.new(8, 4)
            i.ImageRectSize = Vector2.new(16, 16)
            i.Parent = root
        "#,
            &[("sheet.png", png(32, 32, [1, 2, 3, 255]))],
            50.0,
            50.0,
        );
        let source = f.nodes[0]
            .image
            .as_ref()
            .expect("image")
            .source
            .expect("a source rectangle");
        assert_eq!(
            (source.x, source.y, source.w, source.h),
            (8.0, 4.0, 16.0, 16.0)
        );
    }

    #[test]
    fn an_unset_rect_size_means_the_whole_image_rather_than_none_of_it() {
        // THE DEFAULT IS (0, 0) AND IT MEANS EVERYTHING. Reading it as "sample
        // nothing" would blank every image in every tree while every other
        // property still looked right.
        let f = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.ImageContent = Content.fromUri("mod://dot.png")
            i.Parent = root
        "#,
            &[("dot.png", png(4, 4, [1, 2, 3, 255]))],
            50.0,
            50.0,
        );
        assert!(f.nodes[0].image.as_ref().expect("image").source.is_none());
    }

    #[test]
    fn the_three_scale_types_that_are_drawn_arrive_as_themselves() {
        for (member, expected) in [
            ("Stretch", Scale::Stretch),
            ("Fit", Scale::Fit),
            ("Crop", Scale::Crop),
        ] {
            let f = render_with_assets(
                &format!(
                    r#"
                    local i = Instance.new("ImageLabel")
                    i.Size = UDim2.new(0, 10, 0, 10)
                    i.ImageContent = Content.fromUri("mod://dot.png")
                    i.ScaleType = Enum.ScaleType.{member}
                    i.Parent = root
                "#
                ),
                &[("dot.png", png(1, 1, [1, 2, 3, 255]))],
                50.0,
                50.0,
            );
            assert_eq!(
                f.nodes[0].image.as_ref().expect("image").scale,
                expected,
                "Enum.ScaleType.{member}"
            );
        }
    }

    #[test]
    fn a_scale_type_this_pass_cannot_draw_falls_back_to_stretch() {
        // WRITTEN DOWN RATHER THAN SILENT. `Slice` and `Tile` are not drawn yet,
        // this module's header says so, and asking for one prints a line naming
        // it. The assertion here is the fallback; the report is what stops the
        // fallback from being a lie by omission.
        for member in ["Slice", "Tile"] {
            let f = render_with_assets(
                &format!(
                    r#"
                    local i = Instance.new("ImageLabel")
                    i.Size = UDim2.new(0, 10, 0, 10)
                    i.ImageContent = Content.fromUri("mod://dot.png")
                    i.ScaleType = Enum.ScaleType.{member}
                    i.Parent = root
                "#
                ),
                &[("dot.png", png(1, 1, [1, 2, 3, 255]))],
                50.0,
                50.0,
            );
            assert_eq!(
                f.nodes[0].image.as_ref().expect("image").scale,
                Scale::Stretch,
                "Enum.ScaleType.{member}"
            );
        }
    }

    #[test]
    fn an_unresolvable_content_is_a_rendering_outcome_and_not_a_property_error() {
        // ADR-003, as a test. The assignment succeeds, the node reaches the
        // painter, it remembers what it asked for, and it has no pixels. Sprint 5
        // is what makes `rbxassetid://` resolve; until then this is what a Roblox
        // application moved to Dew looks like, and it is not a broken one.
        let f = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.ImageContent = Content.fromUri("rbxassetid://12345")
            i.Parent = root
        "#,
            &[],
            50.0,
            50.0,
        );
        let image = f.nodes[0].image.as_ref().expect("the node is still drawn");
        assert_eq!(image.uri, "rbxassetid://12345");
        assert!(image.bitmap.is_none(), "nothing should have resolved");
    }

    #[test]
    fn an_element_that_names_no_image_carries_none_at_all() {
        // NOT THE SAME AS AN UNRESOLVED ONE, and the painter draws them
        // differently: this is a plain rectangle, and a missing asset is a marked
        // box. An `ImageLabel` before its asset is assigned must not wear the
        // marker.
        let f = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.BackgroundColor3 = Color3.fromRGB(1, 2, 3)
            i.Parent = root
        "#,
            &[],
            50.0,
            50.0,
        );
        assert!(f.nodes[0].image.is_none());
        assert_eq!(f.nodes[0].fill, Some(Rgb(1, 2, 3)));
    }

    #[test]
    fn only_the_classes_that_draw_images_get_one() {
        // A `Frame` has no `Image` property at all, so this also exercises the
        // reflection database refusing it -- which is why the assignment is
        // wrapped and expected to fail.
        let f = render_with_assets(
            r#"
            local f = Instance.new("Frame")
            f.Size = UDim2.new(0, 10, 0, 10)
            f.Parent = root
            assert(pcall(function()
                f.Image = "mod://dot.png"
            end) == false, "a Frame has no Image property")
        "#,
            &[("dot.png", png(1, 1, [1, 2, 3, 255]))],
            50.0,
            50.0,
        );
        assert!(f.nodes[0].image.is_none());
    }

    #[test]
    fn a_mod_cannot_reach_outside_its_own_directory_with_an_image() {
        // The same boundary `Capabilities::require_roots` draws for `require`.
        // Images must not become the way around it.
        let f = render_with_assets(
            r#"
            local i = Instance.new("ImageLabel")
            i.Size = UDim2.new(0, 10, 0, 10)
            i.Image = "mod://../escape.png"
            i.Parent = root
        "#,
            &[("escape.png", png(1, 1, [1, 2, 3, 255]))],
            50.0,
            50.0,
        );
        assert!(f.nodes[0].image.as_ref().expect("image").bitmap.is_none());
    }

    #[test]
    fn absolute_position_and_size_become_readable_to_the_guest() {
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
        lua.load(
            r#"
            a = Instance.new("Frame")
            a.Size = UDim2.new(0.5, 0, 0, 40)
            a.Position = UDim2.new(0, 20, 0, 10)
            a.Parent = root
        "#,
        )
        .exec()
        .expect("guest");

        let _ = frame_of(&dom, root, 200.0, 100.0);

        let read: Vec<f32> = lua
            .load("return { a.AbsolutePosition.X, a.AbsolutePosition.Y, a.AbsoluteSize.X }")
            .eval()
            .expect("read back");
        assert_eq!(read, vec![20.0, 10.0, 100.0]);
    }

    #[test]
    fn unmeasurable_axis_keeps_authored_size_and_reports() {
        let f = render(
            r#"
            local a = Instance.new("Frame")
            a.Name = "Auto"
            a.Size = UDim2.new(0, 30, 0, 40)
            a.AutomaticSize = Enum.AutomaticSize.X
            a.Parent = root

            local s = Instance.new("Frame")
            s.Name = "Scaled"
            s.Size = UDim2.fromScale(1, 1)
            s.Parent = a
        "#,
            200.0,
            200.0,
        );
        let auto = f.nodes.iter().find(|n| n.name == "Auto").expect("auto");
        assert_eq!(auto.rect.w, 30.0, "keeps authored width");
        assert_eq!(auto.rect.h, 40.0, "keeps authored height");
    }

    #[test]
    fn input_action_label_reaches_the_display_list() {
        let f = render(
            r#"
            local a = Instance.new("InputActionLabel")
            a.Name = "Prompt"
            a.Size = UDim2.new(0, 60, 0, 24)
            a.InputAction = "Jump"
            a.TextSize = 16
            a.Parent = root
        "#,
            200.0,
            100.0,
        );
        let prompt = f.nodes.iter().find(|n| n.name == "Prompt").expect("prompt");
        assert_eq!(prompt.text.as_deref(), Some("Jump"));
        assert_eq!(prompt.text_size, 16.0);
    }

    #[test]
    fn scrolling_frame_clips_and_scrolls_descendants() {
        let f = render(
            r#"
            local scroll = Instance.new("ScrollingFrame")
            scroll.Name = "Scroll"
            scroll.Position = UDim2.new(0, 10, 0, 10)
            scroll.Size = UDim2.new(0, 100, 0, 50)
            scroll.CanvasSize = UDim2.new(1, 0, 0, 200)
            scroll.CanvasPosition = Vector2.new(0, 30)
            scroll.Parent = root

            local corner = Instance.new("UICorner")
            corner.CornerRadius = UDim.new(0, 8)
            corner.Parent = scroll

            local item = Instance.new("Frame")
            item.Name = "Item"
            item.Size = UDim2.new(1, 0, 0, 40)
            item.Position = UDim2.new(0, 0, 0, 10)
            item.Parent = scroll
        "#,
            200.0,
            200.0,
        );
        let scroll = f.nodes.iter().find(|n| n.name == "Scroll").expect("scroll");
        assert_eq!(scroll.rect.x, 10.0);
        assert_eq!(scroll.rect.y, 10.0);
        assert_eq!(scroll.rect.w, 100.0);
        assert_eq!(scroll.rect.h, 50.0);

        let item = f.nodes.iter().find(|n| n.name == "Item").expect("item");
        // Item is positioned at parent_box.y (10) + 10 = 20, then shifted by -CanvasPosition.y (30) -> 20 - 30 = -10.
        assert_eq!(item.rect.y, -10.0);
        // Descendants are clipped to the ScrollingFrame's rect with its corner radius
        assert_eq!(
            item.clip,
            Some(dew_runtime::Rect {
                x: 10.0,
                y: 10.0,
                w: 100.0,
                h: 50.0,
            })
        );
        assert_eq!(item.clip_radius, 8.0);
    }

    #[test]
    fn scrolling_frame_automatic_canvas_size_expands() {
        let f = render(
            r#"
            local scroll = Instance.new("ScrollingFrame")
            scroll.Name = "Scroll"
            scroll.Position = UDim2.new(0, 0, 0, 0)
            scroll.Size = UDim2.new(0, 100, 0, 50)
            scroll.CanvasSize = UDim2.new(1, 0, 0, 60)
            scroll.AutomaticCanvasSize = Enum.AutomaticSize.Y
            scroll.CanvasPosition = Vector2.new(0, 50)
            scroll.Parent = root

            local item = Instance.new("Frame")
            item.Name = "Item"
            item.Size = UDim2.new(1, 0, 0, 40)
            item.Position = UDim2.new(0, 0, 0, 80)
            item.Parent = scroll
        "#,
            200.0,
            200.0,
        );
        // Without AutomaticCanvasSize, CanvasSize height is 60 -> max scroll is 60 - 50 = 10,
        // so CanvasPosition.Y = 50 would clamp to 10.
        // With AutomaticCanvasSize.Y, canvas expands to item bottom (80 + 40 = 120),
        // so max scroll is 120 - 50 = 70. CanvasPosition.Y = 50 is unclamped.
        let item = f.nodes.iter().find(|n| n.name == "Item").expect("item");
        // item y is 80 - 50 = 30.
        assert_eq!(item.rect.y, 30.0);
    }
}
