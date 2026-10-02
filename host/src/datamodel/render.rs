//! The DataModel becomes pixels.
//!
//! WHY THIS IS THE PIECE THAT MATTERED
//! Until this file, `SharedDom` was created, installed into every guest VM, and
//! never read again. A guest could call `Instance.new("Frame")`, set 136 of 138
//! properties on it and build a tree, and nothing drew any of it: the frame was
//! driven by Aether's own `Session`, on the parity path, which never touches the
//! DataModel at all. The property surface reading 136 of 138 measured a
//! language nothing spoke back.
//!
//! This gives `dom` its first reader, which is also the precondition the scheme
//! registry and the permission gate in Sprint 5 were waiting on: there was no
//! point gating a fetch when nothing consumed the result.
//!
//! WHAT IT BYPASSES, AND WHY THAT IS CORRECT
//! Not Aether's `Session`. That is a handle onto Luau objects that Aether's
//! `Live.luau` maintains. Dew's DataModel is a
//! Rust arena and owes nothing to that path. What both share is the far end --
//! `Frame`, `Node` and the `Painter` trait -- which is exactly the seam
//! `hosts/runtime` says has survived four rasterisers without the framework
//! changing a line. Building a `Frame` here and handing it to the same painter is
//! using that seam as intended rather than going around it.
//!
//! WHAT IT PLACES. It resolves offset and scale against the parent, applies
//! `AnchorPoint`, honours `Visible`, `ZIndex`, `ClipsDescendants`, `UIPadding`
//! (offsets only), `UICorner`, `UIStroke` and `UIGradient`, grows elements by
//! `AutomaticSize`, scrolls a `ScrollingFrame`, draws text, and draws an image.
//!
//! `UIListLayout` is placed in full: `FillDirection`, `SortOrder`, `Padding`
//! (scale and offset), alignment on both axes, `Wraps`, `HorizontalFlex` and
//! `VerticalFlex`, and `ItemLineAlignment`, and it reports
//! `AbsoluteContentSize`. A `UIFlexItem` on a list's child grows or shrinks
//! that child along the list, and a `UISizeConstraint` clamps any element's
//! size, flexed or not.
//!
//! `UIGridLayout` is placed in full: `CellSize`, `CellPadding`,
//! `FillDirection`, `FillDirectionMaxCells`, `StartCorner`, `SortOrder` and
//! alignment of the grid as a block, and it reports `AbsoluteContentSize`,
//! `AbsoluteCellSize` and `AbsoluteCellCount`. A `UIAspectRatioConstraint`
//! reshapes any element's size, and inside a grid cell it is centred there.
//!
//! Each rule is held to an engine-verified case in `conformance/cases`, and the
//! rules no case pins down say so where they are written.
//!
//! IT DOES NOT DO `UITableLayout`, `UIPageLayout` or `UITextSizeConstraint`.
//! They are skipped as modifiers, so a tree that uses one draws as though it
//! were absent.
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
//! So: enough to put a real tree on screen, and no claim beyond that. The
//! claim, property by property and class by class, is [`honours::honours`].

use dew_runtime::frame::{
    Align, AlphaStop, BlendMode, Frame, Gradient, GradientKind, Image, Node, Rect, Rgb, Scale,
    Stop, Stroke,
};
use dew_runtime::text::{self, lay_out, scaled_size, Block, Face, TextLayout};
use rbx_types::{Variant, Vector2};
use std::collections::HashMap;

use super::{Dom, SharedDom};

/// What this file reads, answered per class and property. Changing what the
/// solver reads and changing that answer belong in one diff.
pub mod honours;

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
/// The engine's shape and not ours to argue with. Drawing one as a rectangle would
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
    match dom.styled_property(id, key) {
        Some(Variant::UDim(v)) => (v.scale, v.offset as f32),
        _ => (0.0, 0.0),
    }
}

fn udim2(dom: &Dom, id: usize, key: &str) -> (f32, f32, f32, f32) {
    match dom.styled_property(id, key) {
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
    match dom.styled_property(id, key) {
        Some(Variant::Vector2(v)) => v,
        _ => Vector2::new(0.0, 0.0),
    }
}

fn number(dom: &Dom, id: usize, key: &str) -> Option<f32> {
    match dom.styled_property(id, key) {
        Some(Variant::Float32(v)) => Some(v),
        Some(Variant::Float64(v)) => Some(v as f32),
        Some(Variant::Int32(v)) => Some(v as f32),
        Some(Variant::Int64(v)) => Some(v as f32),
        _ => None,
    }
}

fn boolean(dom: &Dom, id: usize, key: &str) -> Option<bool> {
    match dom.styled_property(id, key) {
        Some(Variant::Bool(v)) => Some(v),
        _ => None,
    }
}

fn text(dom: &Dom, id: usize, key: &str) -> Option<String> {
    match dom.styled_property(id, key) {
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
    match dom.styled_property(id, key) {
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
/// The display list is opaque at 1 and the engine is opaque at 0. `Live.luau` inverts
/// at the source for exactly one reason, stated there: three painters each
/// inverted it themselves and one forgot.
fn alpha_from(dom: &Dom, id: usize, key: &str) -> f32 {
    1.0 - number(dom, id, key).unwrap_or(0.0).clamp(0.0, 1.0)
}

fn align(dom: &Dom, id: usize, key: &str, enum_name: &str) -> Option<Align> {
    let Some(Variant::Enum(raw)) = dom.styled_property(id, key) else {
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

/// The UIGridLayout child on a node, if any. Matched by its exact class, so a
/// `UITableLayout` or a `UIPageLayout` is never mistaken for one.
fn grid_layout_of(dom: &Dom, id: usize) -> Option<usize> {
    modifier_of(dom, id, "UIGridLayout")
}

/// The first child of `id` whose class is `class`, if any.
fn modifier_of(dom: &Dom, id: usize, class: &str) -> Option<usize> {
    dom.children(id)
        .into_iter()
        .find(|&child| dom.class_of(child).as_deref() == Some(class))
}

/// The bounds a `UISizeConstraint` child puts on a size, as
/// ((min width, min height), (max width, max height)). Without one the bounds
/// are 0 and infinity, which are also the constraint's own defaults
/// (LAYOUT.md section 11).
///
/// A missing, negative or non-finite minimum reads as 0 and a missing or NaN
/// maximum as infinity, so the default `MaxSize` of (inf, inf) arrives as
/// itself. Only the first constraint counts; how the engine combines two is
/// unverified.
fn size_bounds(dom: &Dom, id: usize) -> ((f32, f32), (f32, f32)) {
    let unbounded = ((0.0, 0.0), (f32::INFINITY, f32::INFINITY));
    let Some(constraint) = modifier_of(dom, id, "UISizeConstraint") else {
        return unbounded;
    };
    let read = |key: &str| match dom.styled_property(constraint, key) {
        Some(Variant::Vector2(v)) => Some((v.x, v.y)),
        _ => None,
    };
    let floor = |v: f32| if v.is_finite() && v > 0.0 { v } else { 0.0 };
    let ceiling = |v: f32| {
        if v.is_nan() {
            f32::INFINITY
        } else {
            v.max(0.0)
        }
    };
    let (min_w, min_h) = read("MinSize").unwrap_or((0.0, 0.0));
    let (max_w, max_h) = read("MaxSize").unwrap_or((f32::INFINITY, f32::INFINITY));
    (
        (floor(min_w), floor(min_h)),
        (ceiling(max_w), ceiling(max_h)),
    )
}

/// `v` held between `min` and `max`.
///
/// THE MINIMUM WINS when the two cross, because it is applied last. What the
/// engine does with a `MinSize` larger than its `MaxSize` is unverified.
fn bounded(v: f32, min: f32, max: f32) -> f32 {
    v.min(max).max(min)
}

/// A width and a height clamped by the element's `UISizeConstraint`.
fn clamp_size(dom: &Dom, id: usize, w: f32, h: f32) -> (f32, f32) {
    let ((min_w, min_h), (max_w, max_h)) = size_bounds(dom, id);
    (bounded(w, min_w, max_w), bounded(h, min_h, max_h))
}

/// A width and a height reshaped by the element's `UIAspectRatioConstraint`.
///
/// `AspectRatio` is width over height. With the constraint's defaults,
/// `AspectType` `FitWithinMaxSize` and `DominantAxis` `Width`, the result is the
/// largest size of that ratio inside the one given: a ratio of 2 in a 100 by
/// 100 cell is 100 by 50
/// (`uigridlayout_aspect_ratio_constraint_overrides_the_cell`).
///
/// UNVERIFIED, each the simplest rule that agrees with that case and the
/// documented defaults: `FitWithinMaxSize` ignores `DominantAxis`;
/// `ScaleWithParentSize` keeps the dominant axis at the size given and derives
/// the other from the ratio, so it can grow past what it was given; a ratio
/// that is not a positive finite number changes nothing; and only the first
/// constraint counts.
fn aspect_size(dom: &Dom, id: usize, w: f32, h: f32) -> (f32, f32) {
    let Some(constraint) = modifier_of(dom, id, "UIAspectRatioConstraint") else {
        return (w, h);
    };
    let ratio = number(dom, constraint, "AspectRatio").unwrap_or(1.0);
    if !(ratio.is_finite() && ratio > 0.0) {
        return (w, h);
    }
    let w = w.max(0.0);
    let h = h.max(0.0);
    if enum_name(dom, constraint, "AspectType", "AspectType") == Some("ScaleWithParentSize") {
        if enum_name(dom, constraint, "DominantAxis", "DominantAxis") == Some("Height") {
            (h * ratio, h)
        } else {
            (w, w / ratio)
        }
    } else {
        let fitted = w.min(h * ratio);
        (fitted, fitted / ratio)
    }
}

/// The size an element's constraints leave it: clamped by its
/// `UISizeConstraint`, then reshaped by its `UIAspectRatioConstraint`. The
/// order matters only for an element carrying both, and no case pins it.
fn constrain_size(dom: &Dom, id: usize, w: f32, h: f32) -> (f32, f32) {
    let (w, h) = clamp_size(dom, id, w, h);
    aspect_size(dom, id, w, h)
}

/// Resolve one element against the box its parent offers.
///
/// THE COORDINATE MODEL, and it is `conformance/LAYOUT.md` section 1 rather than
/// an invention: a `UDim` is `scale * available + offset` per axis, and
/// `AnchorPoint` then shifts the element by a fraction of ITS OWN resolved size.
/// The ordering is size first, then anchor, which is the thing
/// `anchor_point_after_automatic_size` was opened in Studio to confirm.
///
/// `laid_out` marks an element positioned by a layout container (`UIListLayout`
/// or `UIGridLayout`).
/// Its own `Position` is ignored per LAYOUT.md section 4, and so is its
/// `AnchorPoint` on both axes (`uilistlayout_ignores_a_child_anchor_point`).
///
/// A `UISizeConstraint` clamps the size before the anchor reads it, at the
/// element's own `Position` (`uisizeconstraint_min_size_grows_a_plain_child`,
/// `uisizeconstraint_max_size_shrinks_a_plain_child`). That the anchor then
/// multiplies the clamped size rather than the authored one follows the size
/// first rule and is unverified for a constrained element. A
/// `UIAspectRatioConstraint` reshapes the size at the same point, by the rules
/// on `aspect_size`.
fn solve_rect(dom: &Dom, id: usize, parent: Box2, laid_out: bool) -> Box2 {
    let (sxs, sxo, sys, syo) = udim2(dom, id, "Size");
    let (pxs, pxo, pys, pyo) = if laid_out {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        udim2(dom, id, "Position")
    };
    let anchor = if laid_out {
        Vector2::new(0.0, 0.0)
    } else {
        vector2(dom, id, "AnchorPoint")
    };

    let (w, h) = constrain_size(dom, id, sxs * parent.w + sxo, sys * parent.h + syo);
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
            if let Some(Variant::UDim(u)) = dom.styled_property(child, "CornerRadius") {
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
/// engine keeps. The engine's migration writes through `Image` into
/// `ImageContent`, so the modern property is the more recent statement of intent
/// whenever they disagree -- and an application that sets only `Image` never
/// reaches the tie at all.
fn image_uri(dom: &Dom, id: usize) -> Option<String> {
    if let Some(Variant::Content(content)) = dom.styled_property(id, "ImageContent") {
        if let Some(uri) = content.as_uri() {
            if !uri.is_empty() {
                return Some(uri.to_string());
            }
        }
    }
    match dom.styled_property(id, "Image") {
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
    let Some(Variant::Enum(raw)) = dom.styled_property(id, "ScaleType") else {
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
/// rectangle -- the same as it is in the engine -- and drawing a "missing" marker on
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

/// `GuiObject.BlendingMode`, an experimental Dew extension
/// (`datamodel::extensions`) with no Roblox equivalent -- see that module for
/// why. Reading it needs no flag check of its own: the property does not
/// exist on `dom` at all until `FFlagDewGuiObjectBlendingMode` is enabled for
/// this applet, so `dom.property` answers `None` and every node keeps the
/// ordinary `Alpha` compositing it always had.
fn blend_mode_of(dom: &Dom, id: usize) -> BlendMode {
    match dom.styled_property(id, "BlendingMode") {
        Some(Variant::Enum(raw)) => match raw.to_u32() {
            1 => BlendMode::Additive,
            2 => BlendMode::Multiply,
            _ => BlendMode::Alpha,
        },
        _ => BlendMode::Alpha,
    }
}

fn gradient_kind(dom: &Dom, id: usize) -> GradientKind {
    if let Some(prop) = dom.styled_property(id, "Type") {
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
    if let Some(Variant::String(s)) = dom.styled_property(id, "Shape") {
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
            if let Some(Variant::ColorSequence(cs)) = dom.styled_property(child, "Color") {
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
            if let Some(Variant::NumberSequence(ns)) = dom.styled_property(child, "Transparency") {
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
    /// that is the shape the engine masks against. Beside the box rather than inside
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
    let Some(Variant::Enum(raw)) = dom.styled_property(id, "AutomaticSize") else {
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
    let Some(Variant::Enum(raw)) = dom.styled_property(id, "AutomaticCanvasSize") else {
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

/// Everything one solve produces: the placed elements, in walk order, the
/// `AbsoluteContentSize` of every layout that placed children, and the
/// `AbsoluteCellSize` and `AbsoluteCellCount` of every `UIGridLayout`.
///
/// DEREFS TO THE ELEMENTS because the walk indexes, pushes and truncates them
/// everywhere; the content sizes ride along. A subtree placed twice (after
/// AutomaticSize grows its parent) appends a second content size for the same
/// layout, and the later one is the answer.
#[derive(Default)]
pub struct Solved {
    pub items: Vec<SolvedItem>,
    pub content_sizes: Vec<(usize, Vector2)>,
    /// A grid's id, its cell size and its cell count (columns, rows).
    pub cells: Vec<(usize, Vector2, Vector2)>,
}

impl std::ops::Deref for Solved {
    type Target = Vec<SolvedItem>;
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl std::ops::DerefMut for Solved {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
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
    // A grid child is as big as its cell whatever its own `Size` says, so it is
    // measured by where it was placed.
    let in_grid = grid_layout_of(dom, node).is_some();

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
                if sxs == 0.0 || (in_grid && d == child_depth) {
                    right = right.max(r.x + r.w + got_x);
                    inherit_x.insert(d + 1, got_x);
                } else if d == child_depth && sxs < 1.0 && (has_layout || !has_content) {
                    let resolved = if let Some(bx) = base_x {
                        bx * sxs + sxo
                    } else {
                        sxo
                    };
                    let ((min_w, _), (max_w, _)) = size_bounds(dom, item.id);
                    let resolved = bounded(resolved, min_w, max_w);
                    right = right.max(r.x + resolved + got_x);
                    inherit_x.insert(d + 1, got_x);
                } else {
                    inherit_x.insert(d + 1, got_x + own_r);
                }
            }

            if grow_y {
                if sys == 0.0 || (in_grid && d == child_depth) {
                    bottom = bottom.max(r.y + r.h + got_y);
                    inherit_y.insert(d + 1, got_y);
                } else if d == child_depth && sys < 1.0 && (has_layout || !has_content) {
                    let resolved = if let Some(by) = base_y {
                        by * sys + syo
                    } else {
                        syo
                    };
                    let ((_, min_h), (_, max_h)) = size_bounds(dom, item.id);
                    let resolved = bounded(resolved, min_h, max_h);
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
        let text_content = if boolean(dom, node, "RichText") == Some(true) {
            dew_runtime::text::strip_markup(&text_content)
        } else {
            text_content
        };
        let text_size = number(dom, node, "TextSize").unwrap_or(14.0);
        let line_height = number(dom, node, "LineHeight").unwrap_or(1.0);
        let wrapped = boolean(dom, node, "TextWrapped").unwrap_or(false);
        let wrap_width = if wrapped {
            if entry_rect.w > 0.0 {
                Some((entry_rect.w - pad_l - pad_r).max(0.0))
            } else if offered.w > 0.0 {
                Some((offered.w - pad_l - pad_r).max(0.0))
            } else {
                None
            }
        } else {
            None
        };
        let measured = face_of(dom, node).and_then(|face| {
            text::measure_spaced(&face, &text_content, text_size, wrap_width, line_height)
        });
        if let Some((tw, th)) = measured {
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

/// Place one element and everything under it.
///
/// `forced` is a width and a height a list layout has already decided for this
/// element (flex `Fill`, `ItemLineAlignment.Stretch`). A forced axis replaces
/// the authored size and AutomaticSize does not grow it again.
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
    forced: (Option<f32>, Option<f32>),
    out: &mut Solved,
) {
    if boolean(dom, id, "Visible") == Some(false) {
        return;
    }

    let mut rect = solve_rect(dom, id, parent_box, laid_out);
    if forced.0.is_some() || forced.1.is_some() {
        let (w, h) = constrain_size(
            dom,
            id,
            forced.0.unwrap_or(rect.w),
            forced.1.unwrap_or(rect.h),
        );
        rect.w = w;
        rect.h = h;
    }
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
    // A ROUNDED PARENT MASKS ITS DESCENDANTS TO THE ROUNDING. The engine does; Dew
    // clipped to a rectangle and leaked the corner pixels, which milestone 3
    // recorded as an observable divergence rather than fixing.
    //
    // THE ROUNDER OF THE TWO WINS on nesting, matching `ar_clip_push_rounded`: a
    // square clip inside a rounded one is still inside the rounded one.
    //
    // A SCROLLINGFRAME ALWAYS CLIPS ITS CONTENT TO ITS OWN BOX, matching the engine.
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
    let (auto_x, auto_y) = automatic_axes(dom, id);
    let grow_x = auto_x && forced.0.is_none();
    let grow_y = auto_y && forced.1.is_none();
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
            (false, false),
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
                        (false, false),
                        out,
                    );
                }
            }
        }
        (cw, ch)
    } else {
        place_children(
            dom,
            id,
            box_rect,
            child_clip,
            child_clip_radius,
            depth,
            (grow_x, grow_y),
            out,
        );
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
    // THE CONSTRAINTS HAVE THE LAST WORD over what AutomaticSize measured, so a
    // `MaxSize` caps a growing element. No case pins the order of measuring and
    // clamping; this is the simplest order that agrees with every case, and it
    // is unverified.
    let (clamped_w, clamped_h) = constrain_size(dom, id, new_rect.w, new_rect.h);
    out[entry_idx].rect = Box2 {
        w: clamped_w,
        h: clamped_h,
        ..new_rect
    };

    let grew_w = out[entry_idx].rect.w > before_w;
    let grew_h = out[entry_idx].rect.h > before_h;

    // AnchorPoint post-growth adjustment. A laid-out element has no anchor.
    if (grew_w || grew_h) && !laid_out {
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

    // Re-placement: if any descendant depends on an axis that grew via scale,
    // or a list was measured packed against the start of an axis AutomaticSize
    // owns and has to be placed again in the box it grew into. A grid is always
    // placed again: how many cells fit a line, the scale in `CellSize` and
    // `CellPadding` and its alignment all read the box.
    let mut dependent = (grow_x || grow_y)
        && !is_scrolling_frame
        && (grid_layout_of(dom, id).is_some()
            || list_layout_of(dom, id)
                .map(|layout| {
                    ListLayout::read(dom, layout, box_rect).measured_packed(grow_x, grow_y)
                })
                .unwrap_or(false));
    if (grew_w || grew_h) && !dependent {
        for item in &out[children_from..] {
            let (sxs, _, sys, _) = udim2(dom, item.id, "Size");
            if (grew_w && sxs != 0.0) || (grew_h && sys != 0.0) {
                dependent = true;
                break;
            }
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
            (false, false),
            out,
        );
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

/// Where a run gathers along one axis when it has room to spare.
///
/// `Left` and `Top` are the same answer on different axes, and the engine spells
/// them differently for the two enums, so both map onto one three-way.
#[derive(Clone, Copy, PartialEq)]
enum Gather {
    Start,
    Center,
    End,
}

impl Gather {
    /// How far from the start of an axis a run begins, given the room left over.
    /// `free` may be negative: an overflowing centred run starts before the
    /// edge rather than being clamped to it (`uilistlayout_center_overflow_is_not_clamped`).
    fn offset(self, free: f32) -> f32 {
        match self {
            Gather::Start => 0.0,
            Gather::Center => free / 2.0,
            Gather::End => free,
        }
    }
}

/// The name of an enum property's current item, if it has one this host knows.
fn enum_name(dom: &Dom, id: usize, property: &str, enum_type: &str) -> Option<&'static str> {
    let Some(Variant::Enum(raw)) = dom.styled_property(id, property) else {
        return None;
    };
    super::enums::item_by_value(enum_type, raw.to_u32()).map(|item| item.name)
}

fn gather_of(dom: &Dom, layout_id: usize, property: &str) -> Gather {
    match enum_name(dom, layout_id, property, property) {
        Some("Center") => Gather::Center,
        Some("Right") | Some("Bottom") => Gather::End,
        // `Left`, `Top`, and anything a newer build adds that this does not know.
        _ => Gather::Start,
    }
}

/// `Enum.UIFlexAlignment`: how a run shares the room left over on one axis.
#[derive(Clone, Copy, PartialEq)]
enum Flex {
    None,
    Fill,
    SpaceAround,
    SpaceBetween,
    SpaceEvenly,
}

fn flex_of(dom: &Dom, layout_id: usize, property: &str) -> Flex {
    match enum_name(dom, layout_id, property, "UIFlexAlignment") {
        Some("Fill") => Flex::Fill,
        Some("SpaceAround") => Flex::SpaceAround,
        Some("SpaceBetween") => Flex::SpaceBetween,
        Some("SpaceEvenly") => Flex::SpaceEvenly,
        _ => Flex::None,
    }
}

/// `Enum.ItemLineAlignment`: where a child sits across its own line.
#[derive(Clone, Copy, PartialEq)]
enum ItemLine {
    /// Follows the cross-axis alignment property.
    Automatic,
    At(Gather),
    Stretch,
}

/// Does a layout order its children by `Name`?
///
/// NAME IS THE DEFAULT on every layout class, and an unreadable value is
/// treated as the default. `Custom` names a sort function the host has no way
/// to receive, so it falls back to LayoutOrder, which is what this solver did
/// for every value before; that fallback is unverified.
fn sorts_by_name(dom: &Dom, layout_id: usize) -> bool {
    !matches!(
        enum_name(dom, layout_id, "SortOrder", "SortOrder"),
        Some("LayoutOrder") | Some("Custom")
    )
}

/// The children of `id` a layout places, in the layout's order. Modifiers are
/// left out; invisible children are left in, for the caller to skip.
///
/// Stable, so equal keys keep declaration order, which the engine does for
/// both keys (`uilistlayout_rows_with_one_name_keep_declaration`,
/// `uilistlayout_equal_layout_order_keeps_declaration`). Names compare as
/// bytes; how the engine orders case and digits is unverified.
fn layout_order(dom: &Dom, id: usize, by_name: bool) -> Vec<usize> {
    let mut kids: Vec<usize> = dom
        .children(id)
        .into_iter()
        .filter(|&child| {
            dom.class_of(child)
                .map(|c| !is_modifier(&c))
                .unwrap_or(false)
        })
        .collect();
    if by_name {
        let mut named: Vec<(String, usize)> = kids
            .iter()
            .map(|&child| (dom.name_of(child).unwrap_or_default(), child))
            .collect();
        named.sort_by(|a, b| a.0.cmp(&b.0));
        kids = named.into_iter().map(|(_, child)| child).collect();
    } else {
        kids.sort_by_key(|&child| number(dom, child, "LayoutOrder").unwrap_or(0.0) as i32);
    }
    kids
}

/// A `UIListLayout`, read once per placement and expressed along its own axes:
/// "main" is `FillDirection`, "cross" is the other one.
struct ListLayout {
    horizontal: bool,
    /// `Padding`, resolved. The scale is a fraction of the content box along
    /// the main axis (`uilistlayout_padding_scale_horizontal`, `_vertical`).
    gap: f32,
    main_gather: Gather,
    cross_gather: Gather,
    main_flex: Flex,
    cross_flex: Flex,
    item_line: ItemLine,
    wraps: bool,
    by_name: bool,
    /// Does any child carry a `UIFlexItem` that grows or shrinks it?
    flexed: bool,
}

impl ListLayout {
    fn read(dom: &Dom, layout_id: usize, box_rect: Box2) -> ListLayout {
        let horizontal =
            enum_name(dom, layout_id, "FillDirection", "FillDirection") == Some("Horizontal");
        let across = gather_of(dom, layout_id, "HorizontalAlignment");
        let down = gather_of(dom, layout_id, "VerticalAlignment");
        let across_flex = flex_of(dom, layout_id, "HorizontalFlex");
        let down_flex = flex_of(dom, layout_id, "VerticalFlex");
        let (main_gather, cross_gather, main_flex, cross_flex) = if horizontal {
            (across, down, across_flex, down_flex)
        } else {
            (down, across, down_flex, across_flex)
        };
        let (scale, offset) = udim(dom, layout_id, "Padding");
        let main_len = if horizontal { box_rect.w } else { box_rect.h };
        let item_line = item_line_of(dom, layout_id);
        let flexed = dom
            .parent_of(layout_id)
            .map(|parent| {
                dom.children(parent)
                    .into_iter()
                    .any(|child| flexes(dom, child))
            })
            .unwrap_or(false);
        ListLayout {
            flexed,
            horizontal,
            gap: scale * main_len + offset,
            main_gather,
            cross_gather,
            main_flex,
            cross_flex,
            item_line,
            wraps: boolean(dom, layout_id, "Wraps") == Some(true),
            by_name: sorts_by_name(dom, layout_id),
        }
    }

    /// Would placing the children in a longer box along an AutomaticSize axis
    /// move them? If so the first placement packs them against the start of
    /// that axis, so the measured size is the content's own, and they are
    /// placed again once the box has grown.
    fn measured_packed(&self, grow_x: bool, grow_y: bool) -> bool {
        let (grow_main, grow_cross) = if self.horizontal {
            (grow_x, grow_y)
        } else {
            (grow_y, grow_x)
        };
        let main = self.wraps
            || self.flexed
            || self.main_gather != Gather::Start
            || self.main_flex != Flex::None;
        let cross = self.cross_gather != Gather::Start || self.cross_flex != Flex::None;
        (grow_main && main) || (grow_cross && cross)
    }
}

/// Share an axis of length `avail` among `sizes`, in order: each one's start,
/// and its length after flex.
///
/// The same rule serves a line of children along the main axis and the lines
/// of a wrapped list across the cross axis.
///
/// What the verified cases pin down: `None` packs the run with `gap` between and
/// gathers it by `gather`; `Fill` keeps `gap`, grows each by an equal share of
/// the room left, and shrinks an overflow in proportion to each size; the three
/// `Space` values ignore `gap` and share the room left as their names say; a
/// single child under `SpaceAround` or `SpaceEvenly` is centred.
///
/// UNVERIFIED, each the simplest rule that agrees with every case:
/// a single child under `SpaceBetween` gathers as `None` does (the case has it
/// at the start, which is also where `Left` puts it); a `Space` value with no
/// room left over behaves as `None`; and the lines of a wrapped list take the
/// same rule across the cross axis, `gap` included.
fn distribute(sizes: &[f32], avail: f32, gap: f32, flex: Flex, gather: Gather) -> Vec<(f32, f32)> {
    let n = sizes.len();
    if n == 0 {
        return Vec::new();
    }
    let sum: f32 = sizes.iter().sum();
    let packed = |sizes: &[f32], start: f32| {
        let mut at = start;
        sizes
            .iter()
            .map(|&s| {
                let slot = (at, s);
                at += s + gap;
                slot
            })
            .collect::<Vec<_>>()
    };
    let spaced = |first: f32, between: f32| {
        let mut at = first;
        sizes
            .iter()
            .map(|&s| {
                let slot = (at, s);
                at += s + between;
                slot
            })
            .collect::<Vec<_>>()
    };
    let room = avail - sum;
    let count = n as f32;
    match flex {
        Flex::Fill => {
            let free = avail - sum - gap * (count - 1.0);
            let filled: Vec<f32> = if free > 0.0 {
                sizes.iter().map(|s| s + free / count).collect()
            } else if free < 0.0 && sum > 0.0 {
                sizes
                    .iter()
                    .map(|s| (s + free * s / sum).max(0.0))
                    .collect()
            } else {
                sizes.to_vec()
            };
            packed(&filled, 0.0)
        }
        Flex::SpaceBetween if room > 0.0 && n > 1 => spaced(0.0, room / (count - 1.0)),
        Flex::SpaceAround if room > 0.0 => spaced(room / count / 2.0, room / count),
        Flex::SpaceEvenly if room > 0.0 => spaced(room / (count + 1.0), room / (count + 1.0)),
        _ => {
            let run = sum + gap * (count - 1.0);
            packed(sizes, gather.offset(avail - run))
        }
    }
}

/// What a `UIFlexItem` asks of the list its element sits in.
#[derive(Clone, Copy)]
struct FlexItem {
    /// Weight in the line's free space. 0 takes none.
    grow: f32,
    /// Weight in the line's overflow, before it is multiplied by the basis.
    /// 0 gives none back.
    shrink: f32,
    /// `Automatic` defers to the list's own `ItemLineAlignment`.
    line: ItemLine,
}

/// The `UIFlexItem` on `id`, if it has one.
///
/// `Grow`, `Shrink` and `Fill` are fixed weights of 1 and 0, and `GrowRatio`
/// and `ShrinkRatio` count only under `Custom`
/// (`uiflexitem_grow_ratio_is_ignored_unless_custom`). `None`, the default,
/// flexes nothing; its `ItemLineAlignment` still applies.
fn flex_item_of(dom: &Dom, id: usize) -> Option<FlexItem> {
    let item = modifier_of(dom, id, "UIFlexItem")?;
    let ratio = |key: &str| number(dom, item, key).unwrap_or(0.0).max(0.0);
    let (grow, shrink) = match enum_name(dom, item, "FlexMode", "UIFlexMode") {
        Some("Grow") => (1.0, 0.0),
        Some("Shrink") => (0.0, 1.0),
        Some("Fill") => (1.0, 1.0),
        Some("Custom") => (ratio("GrowRatio"), ratio("ShrinkRatio")),
        _ => (0.0, 0.0),
    };
    Some(FlexItem {
        grow,
        shrink,
        line: item_line_of(dom, item),
    })
}

/// Does a `UIFlexItem` on `id` flex it at all?
fn flexes(dom: &Dom, id: usize) -> bool {
    flex_item_of(dom, id).is_some_and(|f| f.grow > 0.0 || f.shrink > 0.0)
}

/// `ItemLineAlignment` on a list or a flex item.
fn item_line_of(dom: &Dom, id: usize) -> ItemLine {
    match enum_name(dom, id, "ItemLineAlignment", "ItemLineAlignment") {
        Some("Start") => ItemLine::At(Gather::Start),
        Some("Center") => ItemLine::At(Gather::Center),
        Some("End") => ItemLine::At(Gather::End),
        Some("Stretch") => ItemLine::Stretch,
        _ => ItemLine::Automatic,
    }
}

/// One child's part in resolving a line's flex, along the main axis.
struct Flexing {
    basis: f32,
    grow: f32,
    shrink: f32,
    min: f32,
    max: f32,
}

/// The main-axis length of each member of a line once flex has shared out the
/// line's free space or its overflow.
///
/// The free space is `avail` less the bases and the gaps between them. When
/// it is positive, each member with a grow weight takes a share in proportion
/// to that weight alone, whatever its size: two `Grow` children of 40 and 120
/// in 200 become 60 and 140 (`uiflexitem_two_grow_children_split_equally`),
/// and `GrowRatio` 1 and 2 split 120 as 40 and 80
/// (`uiflexitem_custom_grow_ratio_weights_the_split`).
///
/// SHRINKING IS NOT AN EQUAL SHARE. When the free space is negative, each
/// member gives back in proportion to its shrink weight TIMES ITS BASIS. Two
/// cases pin that rule and a third agrees with it:
/// `uiflexitem_custom_shrink_ratio_weights_the_overflow` (100 and 140 with
/// `ShrinkRatio` 1 and 3 in 200 lose 40 split 100 to 420, leaving 1200/13 and
/// 1400/13), `uiflexitem_two_shrink_children_share_the_overflow` (100 and 200
/// in 150 become 50 and 100, where an equal share would give 25 and 125), and
/// `uilistlayout_horizontal_flex_fill_shrinks_an_overflow`, where the list's
/// own `Fill` shrinks 60, 90 and 150 in 240 to 48, 72 and 120.
///
/// A member that would cross its `UISizeConstraint` stops at the bound and the
/// rest is shared again among the others, which is how a flexing sibling takes
/// up what a bounded one left (`uisizeconstraint_max_size_hands_growth_to_a_sibling`,
/// `uisizeconstraint_min_size_hands_shrink_to_a_sibling`). When several cross
/// at once the loop is the CSS one: if the clamps add space overall the members
/// held at their minimum are fixed, if they remove it those held at their
/// maximum are, and the rest go round again. With one bound per case the cases
/// cannot tell this from fixing every clamped member at once.
///
/// UNVERIFIED: weights summing to less than 1 still share all the free space,
/// where CSS would hand out only that fraction of it.
fn flex_line(members: &[Flexing], avail: f32, gap: f32) -> Vec<f32> {
    let n = members.len();
    let gaps = gap * n.saturating_sub(1) as f32;
    let start: f32 = members.iter().map(|m| m.basis).sum::<f32>() + gaps;
    let growing = avail > start;
    let weight = |m: &Flexing| if growing { m.grow } else { m.shrink * m.basis };
    let mut target: Vec<f32> = members.iter().map(|m| m.basis).collect();
    let mut frozen: Vec<bool> = members
        .iter()
        .map(|m| avail == start || weight(m) <= 0.0)
        .collect();

    // Every pass fixes at least one member, so n passes always finish.
    for _ in 0..n {
        if frozen.iter().all(|&f| f) {
            break;
        }
        let used: f32 = members
            .iter()
            .zip(&frozen)
            .zip(&target)
            .map(|((m, &f), &t)| if f { t } else { m.basis })
            .sum::<f32>()
            + gaps;
        let free = avail - used;
        let total: f32 = members
            .iter()
            .zip(&frozen)
            .filter(|(_, &f)| !f)
            .map(|(m, _)| weight(m))
            .sum();
        if total <= 0.0 {
            break;
        }
        let mut violation = 0.0_f32;
        let mut moved = vec![0.0_f32; n];
        for (i, m) in members.iter().enumerate() {
            if frozen[i] {
                continue;
            }
            let wanted = m.basis + free * weight(m) / total;
            let held = bounded(wanted, m.min, m.max);
            target[i] = wanted;
            moved[i] = held - wanted;
            violation += moved[i];
        }
        for i in 0..n {
            if frozen[i] {
                continue;
            }
            let fix = if violation.abs() <= 0.001 {
                true
            } else if violation > 0.0 {
                moved[i] > 0.0
            } else {
                moved[i] < 0.0
            };
            if fix {
                target[i] += moved[i];
                frozen[i] = true;
            }
        }
    }
    target
}

/// Lay out the children of `id` inside `box_rect`.
///
/// `measuring` names the axes AutomaticSize is about to grow. On those axes a
/// list packs against the start, without flex or wrapping, so what the parent
/// measures is the content's own size; `visit` places the list again in the box
/// it grew into. See `ListLayout::measured_packed`. A grid's block sits at the
/// start of those axes, and `visit` always places a grid again.
#[allow(clippy::too_many_arguments)]
fn place_children(
    dom: &Dom,
    id: usize,
    box_rect: Box2,
    child_clip: Option<Box2>,
    child_clip_radius: f32,
    depth: usize,
    measuring: (bool, bool),
    out: &mut Solved,
) {
    if let Some(layout_id) = list_layout_of(dom, id) {
        place_list(
            dom,
            id,
            layout_id,
            box_rect,
            child_clip,
            child_clip_radius,
            depth,
            measuring,
            out,
        );
    } else if let Some(layout_id) = grid_layout_of(dom, id) {
        // A parent holding a list and a grid is placed by the list; which one
        // the engine honours is unverified.
        place_grid(
            dom,
            id,
            layout_id,
            box_rect,
            child_clip,
            child_clip_radius,
            depth,
            measuring,
            out,
        );
    } else {
        // A `UITableLayout` is not laid out yet: its rows and cells fall
        // through to here and stand at their own `Position` and `Size`, as
        // under no layout at all. So do a `UIPageLayout`'s pages.
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
                (None, None),
                out,
            );
        }
    }
}

/// One child of a list: which node, and its size before the list flexes it,
/// along the list's own axes.
struct Entry {
    id: usize,
    main: f32,
    cross: f32,
    flex: Option<FlexItem>,
}

/// The size a child asks for before a list moves or flexes it: (width, height).
///
/// A child sized only by `Size` resolves without visiting its subtree. One that
/// AutomaticSize grows has to be visited to be measured, and is visited again
/// when it is placed; that second walk is the price of knowing the run's
/// length before placing its first member.
fn basis_of(dom: &Dom, child: usize, box_rect: Box2, depth: usize) -> Option<(f32, f32)> {
    if boolean(dom, child, "Visible") == Some(false) {
        return None;
    }
    let (auto_x, auto_y) = automatic_axes(dom, child);
    if !auto_x && !auto_y {
        let r = solve_rect(dom, child, box_rect, true);
        return Some((r.w, r.h));
    }
    let mut scratch = Solved::default();
    visit(
        dom,
        child,
        box_rect,
        None,
        0.0,
        depth,
        true,
        box_rect,
        (None, None),
        &mut scratch,
    );
    scratch.items.first().map(|item| (item.rect.w, item.rect.h))
}

/// The cross size of a child a list's flex gave `main` along the main axis,
/// measured the way the list will place it: with that length forced.
fn cross_at_main(
    dom: &Dom,
    child: usize,
    box_rect: Box2,
    depth: usize,
    horizontal: bool,
    main: f32,
) -> Option<f32> {
    let forced = if horizontal {
        (Some(main), None)
    } else {
        (None, Some(main))
    };
    let mut scratch = Solved::default();
    visit(
        dom,
        child,
        box_rect,
        None,
        0.0,
        depth,
        true,
        box_rect,
        forced,
        &mut scratch,
    );
    scratch
        .items
        .first()
        .map(|item| if horizontal { item.rect.h } else { item.rect.w })
}

#[allow(clippy::too_many_arguments)]
fn place_list(
    dom: &Dom,
    id: usize,
    layout_id: usize,
    box_rect: Box2,
    child_clip: Option<Box2>,
    child_clip_radius: f32,
    depth: usize,
    measuring: (bool, bool),
    out: &mut Solved,
) {
    let list = ListLayout::read(dom, layout_id, box_rect);
    let horizontal = list.horizontal;
    let (main_len, cross_len) = if horizontal {
        (box_rect.w, box_rect.h)
    } else {
        (box_rect.h, box_rect.w)
    };
    let (measuring_main, measuring_cross) = if horizontal {
        measuring
    } else {
        (measuring.1, measuring.0)
    };
    let (main_flex, main_gather) = if measuring_main {
        (Flex::None, Gather::Start)
    } else {
        (list.main_flex, list.main_gather)
    };
    let (cross_flex, cross_gather) = if measuring_cross {
        (Flex::None, Gather::Start)
    } else {
        (list.cross_flex, list.cross_gather)
    };

    let kids = layout_order(dom, id, list.by_name);

    // An invisible child takes no slot and no Padding
    // (`uilistlayout_invisible_child_takes_no_slot`).
    let entries: Vec<Entry> = kids
        .into_iter()
        .filter_map(|child| {
            let (w, h) = basis_of(dom, child, box_rect, depth + 1)?;
            let (main, cross) = if horizontal { (w, h) } else { (h, w) };
            Some(Entry {
                id: child,
                main,
                cross,
                flex: flex_item_of(dom, child),
            })
        })
        .collect();

    // LINES. Without `Wraps` there is one. With it, a child starts a new line
    // when it would cross the end of the main axis, and a child longer than the
    // whole axis still gets a line of its own, unshrunk
    // (`uilistlayout_wraps_an_oversized_child_onto_its_own_row`).
    let mut lines: Vec<std::ops::Range<usize>> = Vec::new();
    if list.wraps && !measuring_main {
        let mut start = 0;
        let mut run = 0.0;
        for (i, entry) in entries.iter().enumerate() {
            if i > start && run + list.gap + entry.main > main_len + 0.001 {
                lines.push(start..i);
                start = i;
                run = entry.main;
            } else if i == start {
                run = entry.main;
            } else {
                run += list.gap + entry.main;
            }
        }
        if start < entries.len() {
            lines.push(start..entries.len());
        }
    } else if !entries.is_empty() {
        lines.push(0..entries.len());
    }

    // FLEX ON THE MAIN AXIS, per line (`uiflexitem_grow_is_per_wrapped_line`).
    // The list's `Fill` makes every child a `Fill` flex item, and a child's own
    // `UIFlexItem` that flexes replaces that for the child alone: under the
    // list's `Fill` a `Grow` child and its plain siblings all grow by one share
    // (`uiflexitem_grow_under_horizontal_flex_fill`). Whether a `Grow` child
    // there also shrinks with its siblings on an overflow is unverified; here it
    // does not. While AutomaticSize measures the main axis nothing flexes, as
    // the list's own flex does not.
    let fills = main_flex == Flex::Fill;
    let weights = |entry: &Entry| -> (f32, f32) {
        if measuring_main {
            return (0.0, 0.0);
        }
        match entry.flex {
            Some(item) if item.grow > 0.0 || item.shrink > 0.0 => (item.grow, item.shrink),
            _ if fills => (1.0, 1.0),
            _ => (0.0, 0.0),
        }
    };
    // Flex comes first and the list's `Space` value shares whatever it left,
    // which after a `Grow` child is nothing
    // (`uiflexitem_grow_beats_space_between`). After the list's own `Fill` the
    // line packs from the start, as it always has.
    let (spacing, spacing_gather) = if fills {
        (Flex::None, Gather::Start)
    } else {
        (main_flex, main_gather)
    };

    let alongs: Vec<Vec<(f32, f32)>> = lines
        .iter()
        .map(|line| {
            let flexing: Vec<Flexing> = entries[line.clone()]
                .iter()
                .map(|entry| {
                    let ((min_w, min_h), (max_w, max_h)) = size_bounds(dom, entry.id);
                    let (min, max) = if horizontal {
                        (min_w, max_w)
                    } else {
                        (min_h, max_h)
                    };
                    let (grow, shrink) = weights(entry);
                    Flexing {
                        basis: entry.main,
                        grow,
                        shrink,
                        min,
                        max,
                    }
                })
                .collect();
            let mains = flex_line(&flexing, main_len, list.gap);
            distribute(&mains, main_len, list.gap, spacing, spacing_gather)
        })
        .collect();

    // A CHILD FLEX RESIZED IS MEASURED AGAIN AT ITS NEW LENGTH when
    // AutomaticSize sizes its cross axis: its basis was measured at its own
    // length, and a column that wraps text or tiles is shorter once it has
    // grown. The engine sizes and aligns the line on the second answer. Only
    // such a child pays for the extra measure.
    let crosses: Vec<Vec<f32>> = lines
        .iter()
        .zip(&alongs)
        .map(|(line, along)| {
            entries[line.clone()]
                .iter()
                .zip(along)
                .map(|(entry, &(_, main_size))| {
                    let resized = (main_size - entry.main).abs() > 0.001;
                    let (auto_x, auto_y) = automatic_axes(dom, entry.id);
                    let auto_cross = if horizontal { auto_y } else { auto_x };
                    if resized && auto_cross {
                        cross_at_main(dom, entry.id, box_rect, depth + 1, horizontal, main_size)
                            .unwrap_or(entry.cross)
                    } else {
                        entry.cross
                    }
                })
                .collect()
        })
        .collect();

    // ACROSS: a line is as thick as its thickest child, not the box
    // (`uilistlayout_item_line_alignment_stretch`), and the lines share the
    // cross axis by the cross flex and alignment
    // (`uilistlayout_wraps_with_bottom_alignment`,
    // `uilistlayout_horizontal_flex_fill_on_a_vertical_list`).
    let thickness: Vec<f32> = crosses
        .iter()
        .map(|line| line.iter().fold(0.0_f32, |m, &c| m.max(c)))
        .collect();
    let across = distribute(&thickness, cross_len, list.gap, cross_flex, cross_gather);

    let mut content_main = 0.0_f32;
    let mut content_cross = 0.0_f32;
    for (line_index, line) in lines.iter().enumerate() {
        let (line_at, line_thickness) = across[line_index];
        let members = &entries[line.clone()];
        let along = &alongs[line_index];

        let mut line_main = 0.0_f32;
        let mut line_cross = 0.0_f32;
        for ((entry, &(main_at, main_size)), &cross) in
            members.iter().zip(along.iter()).zip(&crosses[line_index])
        {
            let forced_main = ((main_size - entry.main).abs() > 0.001).then_some(main_size);
            // A child's own `ItemLineAlignment` replaces the list's, and its
            // `Automatic` defers to the list's.
            let item_line = match entry.flex.map(|f| f.line) {
                Some(ItemLine::Automatic) | None => list.item_line,
                Some(own) => own,
            };
            let stretch = cross_flex == Flex::Fill || item_line == ItemLine::Stretch;
            let within = match item_line {
                ItemLine::At(gather) => gather,
                // Automatic, and Stretch where a child is not stretched.
                _ => list.cross_gather,
            };
            let (cross_at, forced_cross) = if stretch {
                (line_at, Some(line_thickness))
            } else {
                (line_at + within.offset(line_thickness - cross), None)
            };
            let (x, y, forced) = if horizontal {
                (main_at, cross_at, (forced_main, forced_cross))
            } else {
                (cross_at, main_at, (forced_cross, forced_main))
            };
            let slot = Box2 {
                x: box_rect.x + x,
                y: box_rect.y + y,
                w: box_rect.w,
                h: box_rect.h,
            };
            let before = out.len();
            visit(
                dom,
                entry.id,
                slot,
                child_clip,
                child_clip_radius,
                depth + 1,
                true,
                box_rect,
                forced,
                out,
            );
            if out.len() > before {
                let placed = out[before].rect;
                let (m, c) = if horizontal {
                    (placed.w, placed.h)
                } else {
                    (placed.h, placed.w)
                };
                line_main += m;
                line_cross = line_cross.max(c);
            }
        }
        // ABSOLUTECONTENTSIZE counts each child's final size and Padding, not
        // the room a `Space` flex spread between them: SpaceAround reads 120
        // where Fill reads 240 for the same children in 240.
        line_main += list.gap * (members.len().saturating_sub(1)) as f32;
        content_main = content_main.max(line_main);
        content_cross += line_cross;
    }
    content_cross += list.gap * (lines.len().saturating_sub(1)) as f32;
    let (content_w, content_h) = if horizontal {
        (content_main, content_cross)
    } else {
        (content_cross, content_main)
    };
    out.content_sizes
        .push((layout_id, Vector2::new(content_w, content_h)));
}

/// Place the children of `id` in the cells of the `UIGridLayout` `layout_id`.
///
/// Every child takes one cell, in the layout's sort order (`Name` by default,
/// `uigridlayout_default_sort_order_is_name`), and the cell replaces its own
/// `Size`, `Position` and `AnchorPoint` (`uigridlayout_defaults`). A
/// `UIFlexItem` on a child changes nothing
/// (`uiflexitem_under_a_grid_does_nothing`).
///
/// THE CELLS. `CellSize` and `CellPadding` both resolve scale against the
/// content box inside any `UIPadding`: a 0.5 cell in a 220 panel padded by 10
/// is 100 (`uigridlayout_cell_size_scale_uses_the_content_box`), and a 0.1 gap
/// there is 20, not 22 (`uigridlayout_cell_padding_scale_uses_the_parent`).
///
/// THE LINES. Cells fill along `FillDirection`, across first (the grid's own
/// default, `uigridlayout_defaults`) or down first
/// (`uigridlayout_fill_direction_vertical`). A line holds as many cells as fit
/// the box with the padding between them, at least one, and no more than
/// `FillDirectionMaxCells` when that is above 0
/// (`uigridlayout_fill_direction_max_cells`).
///
/// THE BLOCK. The filled cells make one block of columns by rows, and the
/// alignment moves the block, not each line
/// (`uigridlayout_alignment_moves_the_block`). `StartCorner` is a corner of
/// that block, not of the box, and the fill runs from it: from `TopRight` the
/// first cell is in the block's last column and the second to its left
/// (`uigridlayout_start_corner_top_left`, `_top_right`, `_bottom_left`,
/// `_bottom_right`).
///
/// A CONSTRAINED CHILD. A `UISizeConstraint` or `UIAspectRatioConstraint`
/// sizes the child within its cell and the cell centres it; the next cell does
/// not move (`uigridlayout_size_constraint_overrides_the_cell`,
/// `uigridlayout_aspect_ratio_constraint_overrides_the_cell`).
///
/// UNVERIFIED, each the simplest rule that agrees with every case: a line with
/// room for more cells than there are children is as long as the children, so
/// the block, its alignment and `AbsoluteCellCount` count children rather than
/// room; an invisible child takes no cell; a child its constraints make larger
/// than its cell is centred on it and overflows it evenly; a block larger than
/// the box overflows it by the same alignment rule as one that fits; and while
/// AutomaticSize measures an axis the block sits at the start of it, `visit`
/// placing it again in the grown box.
#[allow(clippy::too_many_arguments)]
fn place_grid(
    dom: &Dom,
    id: usize,
    layout_id: usize,
    box_rect: Box2,
    child_clip: Option<Box2>,
    child_clip_radius: f32,
    depth: usize,
    measuring: (bool, bool),
    out: &mut Solved,
) {
    let (csx, cox, csy, coy) = udim2(dom, layout_id, "CellSize");
    let cell_w = (csx * box_rect.w + cox).max(0.0);
    let cell_h = (csy * box_rect.h + coy).max(0.0);
    let (psx, pox, psy, poy) = udim2(dom, layout_id, "CellPadding");
    let pad_x = psx * box_rect.w + pox;
    let pad_y = psy * box_rect.h + poy;
    // Read through the layout's own class, whose default is `Horizontal`; a
    // list's `Vertical` default never applies here.
    let down_first =
        enum_name(dom, layout_id, "FillDirection", "FillDirection") == Some("Vertical");
    let corner = enum_name(dom, layout_id, "StartCorner", "StartCorner").unwrap_or("TopLeft");
    let from_right = corner.ends_with("Right");
    let from_bottom = corner.starts_with("Bottom");
    let max_cells = number(dom, layout_id, "FillDirectionMaxCells")
        .unwrap_or(0.0)
        .max(0.0) as usize;

    let kids: Vec<usize> = layout_order(dom, id, sorts_by_name(dom, layout_id))
        .into_iter()
        .filter(|&child| boolean(dom, child, "Visible") != Some(false))
        .collect();
    let n = kids.len();

    // How many cells one line holds.
    let (line_len, cell, gap) = if down_first {
        (box_rect.h, cell_h, pad_y)
    } else {
        (box_rect.w, cell_w, pad_x)
    };
    let mut per_line = if cell + gap > 0.0 {
        ((line_len + gap + 0.001) / (cell + gap)).floor().max(0.0) as usize
    } else {
        n
    };
    if max_cells > 0 {
        per_line = per_line.min(max_cells);
    }
    let per_line = per_line.min(n).max(1);
    let lines = n.div_ceil(per_line);
    let filled = if n == 0 { 0 } else { per_line };
    let (cols, rows) = if down_first {
        (lines, filled)
    } else {
        (filled, lines)
    };

    let span = |count: usize, cell: f32, gap: f32| {
        if count == 0 {
            0.0
        } else {
            count as f32 * cell + (count - 1) as f32 * gap
        }
    };
    let block_w = span(cols, cell_w, pad_x);
    let block_h = span(rows, cell_h, pad_y);
    let across = if measuring.0 {
        Gather::Start
    } else {
        gather_of(dom, layout_id, "HorizontalAlignment")
    };
    let down = if measuring.1 {
        Gather::Start
    } else {
        gather_of(dom, layout_id, "VerticalAlignment")
    };
    let left = box_rect.x + across.offset(box_rect.w - block_w);
    let top = box_rect.y + down.offset(box_rect.h - block_h);

    for (i, &child) in kids.iter().enumerate() {
        let (line, slot) = (i / per_line, i % per_line);
        let (mut col, mut row) = if down_first {
            (line, slot)
        } else {
            (slot, line)
        };
        if from_right {
            col = cols - 1 - col;
        }
        if from_bottom {
            row = rows - 1 - row;
        }
        let cell_x = left + col as f32 * (cell_w + pad_x);
        let cell_y = top + row as f32 * (cell_h + pad_y);
        let (w, h) = constrain_size(dom, child, cell_w, cell_h);
        let at = Box2 {
            x: cell_x + (cell_w - w) / 2.0,
            y: cell_y + (cell_h - h) / 2.0,
            w: box_rect.w,
            h: box_rect.h,
        };
        visit(
            dom,
            child,
            at,
            child_clip,
            child_clip_radius,
            depth + 1,
            true,
            box_rect,
            (Some(w), Some(h)),
            out,
        );
    }

    out.content_sizes
        .push((layout_id, Vector2::new(block_w, block_h)));
    out.cells.push((
        layout_id,
        Vector2::new(cell_w, cell_h),
        Vector2::new(cols as f32, rows as f32),
    ));
}

pub fn solve_layout(dom: &Dom, root: usize, surface: Box2) -> Vec<SolvedItem> {
    solve(dom, root, surface).items
}

/// The placement and the list content sizes it produced.
pub fn solve(dom: &Dom, root: usize, surface: Box2) -> Solved {
    let mut out = Solved::default();
    let root_box = content_box(dom, root, surface);
    place_children(dom, root, root_box, None, 0.0, 0, (false, false), &mut out);
    out
}

/// Where each node falls in paint order, back to front.
///
/// A CHILD ALWAYS DRAWS ABOVE ITS PARENT, and `ZIndex` orders SIBLINGS. That is
/// `ZIndexBehavior.Sibling`, which is what a `ScreenGui` defaults to.
///
/// A single global sort by `ZIndex` is the other behaviour, `Global`, and it is
/// wrong in a way that only shows up on a tree that uses both: a frame carrying
/// `ZIndex = 3` painted after its own children, which carry the default 1, so a
/// window's titlebar and text vanished under the window.
///
/// COMPUTED OVER THE TREE RATHER THAN BY REORDERING THE WALK, because the walk
/// also decides layout: a `UIListLayout` positions children in its own sort
/// order, and `ZIndex` must not move anything. This ranks nodes separately and
/// the solve is sorted by the rank afterwards.
fn paint_rank(dom: &Dom, root: usize) -> HashMap<usize, usize> {
    fn z_of(dom: &Dom, id: usize) -> i32 {
        number(dom, id, "ZIndex").unwrap_or(1.0) as i32
    }

    fn walk(dom: &Dom, id: usize, next: &mut usize, out: &mut HashMap<usize, usize>) {
        out.insert(id, *next);
        *next += 1;

        // STABLE, so siblings sharing a `ZIndex` keep declaration order, which is
        // the rest of the engine's rule.
        let mut kids = dom.children(id);
        kids.sort_by_key(|child| z_of(dom, *child));
        for child in kids {
            walk(dom, child, next, out);
        }
    }

    let mut out = HashMap::new();
    let mut next = 0usize;
    walk(dom, root, &mut next, &mut out);
    out
}

/// Resolve everything under `root` into paint order, back to front.
///
/// `root` itself is the surface and is not placed; its children are laid out
/// against the box the surface offers, which is how a `ScreenGui` behaves.
pub fn display_list(dom: &Dom, root: usize, width: f32, height: f32) -> Vec<Placed> {
    let opened = dom.open_style_memo();
    let surface = Box2 {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    };
    let solved = solve_layout(dom, root, surface);
    let placed = paint_order(dom, root, solved);
    dom.close_style_memo(opened);
    placed
}

/// A solve's elements in paint order, back to front, without the ones that
/// have no area to paint.
fn paint_order(dom: &Dom, root: usize, solved: Vec<SolvedItem>) -> Vec<Placed> {
    let rank = paint_rank(dom, root);
    let mut collected: Vec<(usize, Placed)> = solved
        .into_iter()
        .filter(|item| item.rect.w > 0.0 && item.rect.h > 0.0)
        .map(|item| {
            (
                rank.get(&item.id).copied().unwrap_or(usize::MAX),
                Placed {
                    id: item.id,
                    rect: item.rect,
                    clip: item.clip,
                    clip_radius: item.clip_radius,
                },
            )
        })
        .collect();
    collected.sort_by_key(|(rank, _)| *rank);
    collected.into_iter().map(|(_, placed)| placed).collect()
}

/// The TextSize a label draws at: its own, or with TextScaled the largest that
/// fits its content box, as the line model measures it.
fn resolved_text_size(dom: &Dom, id: usize, placed_rect: Box2) -> f32 {
    let authored_size = number(dom, id, "TextSize").unwrap_or(14.0);
    if boolean(dom, id, "TextScaled") != Some(true) {
        return authored_size;
    }

    let class = dom.class_of(id).unwrap_or_default();
    let Some(text_content) = text_for(dom, id, &class) else {
        return authored_size;
    };
    let content = content_box(dom, id, placed_rect);
    if content.w <= 0.0 || content.h <= 0.0 {
        return authored_size;
    }
    let text_content = if boolean(dom, id, "RichText") == Some(true) {
        dew_runtime::text::strip_markup(&text_content)
    } else {
        text_content
    };
    let wrapped = boolean(dom, id, "TextWrapped").unwrap_or(false);
    face_of(dom, id)
        .and_then(|face| scaled_size(&face, &text_content, content.w, content.h, wrapped))
        .unwrap_or(authored_size)
}

/// The face a text element is measured and drawn in: its `FontFace`, resolved.
///
/// ONE ANSWER FOR EVERY TEXT PATH. AutomaticSize, TextScaled, the layout that
/// `TextBounds` reports and the lines the painter draws all ask here, so a
/// label cannot be measured in one face and drawn in another. Assigning the
/// legacy `Font` sets `FontFace` (see the property path in `datamodel`), so
/// this reads only `FontFace`.
fn face_of(dom: &Dom, id: usize) -> Option<Face<dew_raster::Font>> {
    match dom.styled_property(id, "FontFace") {
        Some(Variant::Font(font)) => crate::services::face_for(&font),
        _ => match dom.styled_property(id, "Font") {
            Some(Variant::Enum(raw)) => {
                let item = super::enums::item_by_value("Font", raw.to_u32())?;
                let font = crate::fonts::from_enum(item.name)?;
                crate::services::face_for(&font)
            }
            Some(Variant::String(s)) => {
                let font = crate::fonts::from_enum(&s)?;
                crate::services::face_for(&font)
            }
            _ => crate::services::default_face(),
        },
    }
}

/// Lay out the text of one placed element: the line model, given this
/// element's properties and its box inset by its UIPadding.
///
/// ONE LAYOUT PER ELEMENT PER FRAME. `commit_geometry` makes it, reports
/// `TextBounds` and `TextFits` from it, and `frame_of` hands the same value to
/// the display list, so the painter draws what was reported.
fn text_layout_of(dom: &Dom, id: usize, class: &str, rect: Box2) -> Option<TextLayout> {
    if !draws_text(class) {
        return None;
    }
    let face = face_of(dom, id)?;
    let raw_text = text_for(dom, id, class).unwrap_or_default();
    let text = if boolean(dom, id, "RichText") == Some(true) {
        dew_runtime::text::strip_markup(&raw_text)
    } else {
        raw_text
    };
    let content = content_box(dom, id, rect);
    let truncate = enum_name(dom, id, "TextTruncate", "TextTruncate") == Some("AtEnd");
    let line_height = number(dom, id, "LineHeight").unwrap_or(1.0);
    lay_out(
        &face,
        &Block {
            text: &text,
            text_size: resolved_text_size(dom, id, rect),
            content: Rect {
                x: content.x,
                y: content.y,
                w: content.w,
                h: content.h,
            },
            wrap: boolean(dom, id, "TextWrapped").unwrap_or(false),
            align_x: align(dom, id, "TextXAlignment", "TextXAlignment").unwrap_or(Align::Center),
            align_y: align(dom, id, "TextYAlignment", "TextYAlignment").unwrap_or(Align::Center),
            truncate,
            line_height,
        },
    )
}

/// The classes with `TextBounds`, `TextFits` and `ContentText` members.
fn reports_text(class: &str) -> bool {
    matches!(class, "TextLabel" | "TextButton" | "TextBox")
}

/// Turn one placed element into the display list node the painter consumes.
///
/// `&mut Dom` FOR ONE REASON, AND IT IS WORTH THE SIGNATURE. Resolving an image
/// reads a file, decodes it, and remembers the answer; without the remembering,
/// a mod with an icon would decode that icon on every frame it is drawn. The
/// cache lives on the DOM because it is per-mod, so building a node writes to it.
/// `display_list` is still `&Dom` and `input::hit` still reads the same
/// placement, which is the property that mattered.
fn node(dom: &mut Dom, placed: &Placed, sequence: u64, laid: Option<TextLayout>) -> Node {
    let id = placed.id;
    let class = dom.class_of(id).unwrap_or_default();
    let image = if draws_image(&class) {
        image_of(dom, id)
    } else {
        None
    };
    let dom = &*dom;
    let laid = laid.or_else(|| text_layout_of(dom, id, &class, placed.rect));
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
        // FROM THE LAYOUT WHEN THERE IS ONE: TextScaled searches for its size,
        // and searching twice a frame doubled the cost of every scaled label.
        text_size: match &laid {
            Some(laid) => laid.text_size,
            None if draws_text(&class) => resolved_text_size(dom, id, placed.rect),
            None => number(dom, id, "TextSize").unwrap_or(14.0),
        },
        text_align_x: align(dom, id, "TextXAlignment", "TextXAlignment"),
        text_align_y: align(dom, id, "TextYAlignment", "TextYAlignment"),
        text_colour: colour(dom, id, "TextColor3"),
        // TRANSPARENCY IS THE INVERSE OF ALPHA, as it is everywhere in this
        // vocabulary: the engine counts how see-through a thing is and the painter
        // counts how solid it is.
        text_alpha: 1.0
            - number(dom, id, "TextTransparency")
                .unwrap_or(0.0)
                .clamp(0.0, 1.0),
        text_wrap: boolean(dom, id, "TextWrapped").unwrap_or(false),
        text_layout: laid,
        text_stroke_colour: colour(dom, id, "TextStrokeColor3"),
        text_stroke_alpha: 1.0
            - number(dom, id, "TextStrokeTransparency")
                .unwrap_or(1.0)
                .clamp(0.0, 1.0),
        image,
        blend_mode: blend_mode_of(dom, id),
    }
}

/// Build the display list for the subtree under `root`.
pub fn frame(dom: &mut Dom, root: usize, width: f32, height: f32) -> Frame {
    let opened = dom.open_style_memo();
    let placed = display_list(dom, root, width, height);
    let frame = frame_with(dom, placed, width, height, HashMap::new());
    dom.close_style_memo(opened);
    frame
}

/// [`frame`], from a display list already placed, reusing text layouts
/// `commit_geometry` already made, by instance id and the rect each was made
/// for.
fn frame_with(
    dom: &mut Dom,
    placed: Vec<Placed>,
    width: f32,
    height: f32,
    mut texts: HashMap<usize, (Box2, TextLayout)>,
) -> Frame {
    // THE SEQUENCE NUMBER IS ASSIGNED AFTER THE SORT, and it was assigned before
    // it when this walk built `Node`s inline. Nothing read it, so nothing broke;
    // it is now what it claims to be, a paint index.
    //
    // THE PLACEMENT PASS IS COLLECTED BEFORE THE NODES ARE BUILT, which it was
    // anyway, and now has to be: building a node may resolve an image and so
    // needs the DOM mutably, while placing reads it. One pass then the other
    // keeps both borrows to themselves without a second walk.
    let nodes = placed
        .iter()
        .enumerate()
        .map(|(i, placed)| {
            let laid = texts
                .remove(&placed.id)
                .filter(|(rect, _)| *rect == placed.rect)
                .map(|(_, laid)| laid);
            node(dom, placed, i as u64 + 1, laid)
        })
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
    commit(dom, root, width, height);
}

/// [`commit_geometry`], handing back the text layouts it made and the solve
/// it made them from, so a frame built straight after makes neither again.
/// Nothing written here is read by layout, so that frame would solve the
/// same tree to the same answer.
///
/// `TextBounds`, `TextFits` and `ContentText` are written here beside
/// `AbsoluteSize`, for the same reason and through the same door: read-only to
/// a guest, computed by the host, and wrong if left at the default. They
/// describe what is drawn, so they come from the layout the painter is given.
fn commit(
    dom: &mut Dom,
    root: usize,
    width: f32,
    height: f32,
) -> (HashMap<usize, (Box2, TextLayout)>, Vec<SolvedItem>) {
    let opened = dom.open_style_memo();
    let surface = Box2 {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    };
    let solved = solve(dom, root, surface);
    // A LIST'S `AbsoluteContentSize` is written the same way, and read-only to a
    // guest for the same reason. Before this it read (0, 0) forever, which a
    // scroll container sizing itself from it cannot tell from an empty list.
    for (layout, size) in solved.content_sizes {
        dom.set_internal(layout, "AbsoluteContentSize", Variant::Vector2(size));
    }
    for (grid, cell_size, cell_count) in solved.cells {
        dom.set_internal(grid, "AbsoluteCellSize", Variant::Vector2(cell_size));
        dom.set_internal(grid, "AbsoluteCellCount", Variant::Vector2(cell_count));
    }
    let mut texts = HashMap::new();
    // LAST PLACEMENT WINS, as it does for the writes below: a subtree placed
    // again after AutomaticSize grew its parent appears twice, and only the
    // later rect is drawn.
    for item in solved.items.iter().rev() {
        if texts.contains_key(&item.id) {
            continue;
        }
        let class = dom.class_of(item.id).unwrap_or_default();
        if let Some(laid) = text_layout_of(dom, item.id, &class, item.rect) {
            texts.insert(item.id, (item.rect, laid));
        }
    }
    for item in &solved.items {
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
    for (id, (_, laid)) in &texts {
        let class = dom.class_of(*id).unwrap_or_default();
        if !reports_text(&class) {
            continue;
        }
        let raw = text(dom, *id, "Text").unwrap_or_default();
        let content = if boolean(dom, *id, "RichText") == Some(true) {
            dew_runtime::text::strip_markup(&raw)
        } else {
            raw
        };
        dom.set_internal(
            *id,
            "TextBounds",
            Variant::Vector2(Vector2::new(laid.bounds.0, laid.bounds.1)),
        );
        dom.set_internal(*id, "TextFits", Variant::Bool(laid.fits));
        dom.set_internal(*id, "ContentText", Variant::String(content));
    }
    dom.close_style_memo(opened);
    (texts, solved.items)
}

/// Render whatever is under `root` in a shared DOM.
pub fn frame_of(dom: &SharedDom, root: usize, width: f32, height: f32) -> Frame {
    let mut guard = dom.lock().expect("dom");
    // ONE SOLVE AND ONE MEMO FOR BOTH HALVES: geometry is committed between
    // them, and nothing it writes changes what layout or the cascade reads.
    let opened = guard.open_style_memo();
    let (texts, solved) = commit(&mut guard, root, width, height);
    let placed = paint_order(&guard, root, solved);
    let f = frame_with(&mut guard, placed, width, height, texts);
    guard.close_style_memo(opened);
    f
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
    /// The engine masks descendants against the clipping parent's `UICorner`. The
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

    /// A list measures a flex grown AutomaticSize child across the main axis
    /// at the length it grew to, and sizes and aligns its line on that.
    ///
    /// `Text` holds six tiles in a wrapping list, so it is 60 thick at its
    /// minimum length of 120 and thinner once grown. The horizontal numbers
    /// are the engine's (`uilistlayout_wraps_measures_a_grown_child_at_its_grown_width`
    /// and `uilistlayout_wraps_centres_a_line_on_its_grown_child`); the
    /// vertical list is the same layout turned on its side.
    #[test]
    fn a_grown_automatic_child_is_measured_at_its_grown_length() {
        let build = |vertical: bool, wraps: bool, align: &str, length: f32| {
            let (fill, tile, icon, actions, size, auto, align_prop) = if vertical {
                (
                    "Vertical",
                    "fromOffset(20, 50)",
                    "fromOffset(40, 40)",
                    "fromOffset(28, 150)",
                    format!("UDim2.fromOffset(0, {length})"),
                    "X",
                    format!("HorizontalAlignment = Enum.HorizontalAlignment.{align}"),
                )
            } else {
                (
                    "Horizontal",
                    "fromOffset(50, 20)",
                    "fromOffset(40, 40)",
                    "fromOffset(150, 28)",
                    format!("UDim2.fromOffset({length}, 0)"),
                    "Y",
                    format!("VerticalAlignment = Enum.VerticalAlignment.{align}"),
                )
            };
            let min = if vertical { "0, 120" } else { "120, 0" };
            let f = render(
                &format!(
                    r#"
                local function list(p, wraps, gap)
                    local l = Instance.new("UIListLayout")
                    l.FillDirection = Enum.FillDirection.{fill}
                    l.SortOrder = Enum.SortOrder.LayoutOrder
                    l.Padding = UDim.new(0, gap)
                    l.Wraps = wraps
                    l.Parent = p
                    return l
                end
                local function frame(p, name, order, size)
                    local f = Instance.new("Frame")
                    f.Name = name
                    f.LayoutOrder = order
                    f.Size = size
                    f.Parent = p
                    return f
                end
                local panel = frame(root, "Panel", 0, {size})
                panel.AutomaticSize = Enum.AutomaticSize.{auto}
                list(panel, {wraps}, 10).{align_prop}
                frame(panel, "Icon", 1, UDim2.{icon})
                local text = frame(panel, "Text", 2, UDim2.fromOffset(0, 0))
                text.AutomaticSize = Enum.AutomaticSize.{auto}
                local item = Instance.new("UIFlexItem")
                item.FlexMode = Enum.UIFlexMode.Grow
                item.Parent = text
                local c = Instance.new("UISizeConstraint")
                c.MinSize = Vector2.new({min})
                c.Parent = text
                list(text, true, 0)
                for i = 1, 6 do
                    frame(text, "T" .. i, i, UDim2.{tile})
                end
                frame(panel, "Actions", 3, UDim2.{actions})
            "#
                ),
                1000.0,
                1000.0,
            );
            let rect = |name: &str| {
                let n = f
                    .nodes
                    .iter()
                    .find(|n| n.name == name)
                    .unwrap_or_else(|| panic!("{name} is drawn"));
                // Along the list's own axes: (main, cross, main size, cross size).
                if vertical {
                    (n.rect.y, n.rect.x, n.rect.h, n.rect.w)
                } else {
                    (n.rect.x, n.rect.y, n.rect.w, n.rect.h)
                }
            };
            (rect("Panel").3, rect("Text"), rect("Actions"))
        };

        for vertical in [false, true] {
            // Actions wraps; Text grows to 150 and is 40 thick, not 60.
            for align in ["Top", "Center"] {
                let align = if vertical && align == "Top" {
                    "Left"
                } else {
                    align
                };
                let (panel, text, actions) = build(vertical, true, align, 200.0);
                assert_eq!(panel, 78.0, "{vertical} {align}");
                assert_eq!(text, (50.0, 0.0, 150.0, 40.0), "{vertical} {align}");
                assert_eq!(actions, (0.0, 50.0, 150.0, 28.0), "{vertical} {align}");
            }
            // One line, centred on the Icon: Text is 20 thick once grown.
            for wraps in [true, false] {
                let (panel, text, actions) = build(vertical, wraps, "Center", 600.0);
                assert_eq!(panel, 40.0, "{vertical} {wraps}");
                assert_eq!(text, (50.0, 10.0, 390.0, 20.0), "{vertical} {wraps}");
                assert_eq!(actions, (450.0, 6.0, 150.0, 28.0), "{vertical} {wraps}");
            }
        }
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

    /// A table is a layout class of its own, not a grid: its children keep
    /// their own Position and Size, where a grid would put them in 100 cells.
    #[test]
    fn a_table_layout_is_not_placed_as_a_grid() {
        let f = render(
            r#"
            local panel = Instance.new("Frame")
            panel.Size = UDim2.new(0, 300, 0, 200)
            panel.Parent = root
            Instance.new("UITableLayout").Parent = panel
            local row = Instance.new("Frame")
            row.Name = "Row"
            row.Position = UDim2.new(0, 40, 0, 30)
            row.Size = UDim2.new(0, 60, 0, 20)
            row.Parent = panel
        "#,
            300.0,
            200.0,
        );
        let row = f
            .nodes
            .iter()
            .find(|n| n.name == "Row")
            .expect("the row is in the display list");
        assert_eq!(
            (row.rect.x, row.rect.y, row.rect.w, row.rect.h),
            (40.0, 30.0, 60.0, 20.0),
            "a table's child should stand where it was put"
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
        // The engine is opaque at 0, the display list is opaque at 1.
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

    // ── The StyleSheet cascade reaching paint ─────────────────────────────────

    #[test]
    fn a_style_rule_colours_an_instance_that_left_the_property_unset() {
        let f = render(
            r#"
            local sheet = Instance.new("StyleSheet")
            sheet.Parent = root

            local rule = Instance.new("StyleRule")
            rule.Selector = "Frame"
            rule:SetProperty("BackgroundColor3", Color3.fromRGB(18, 23, 34))
            rule.Parent = sheet

            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 10, 0, 10)
            a.Parent = root
        "#,
            100.0,
            100.0,
        );
        assert_eq!(f.nodes[0].fill, Some(Rgb(18, 23, 34)));
    }

    #[test]
    fn an_instance_s_own_explicit_colour_still_wins_over_a_matching_rule() {
        // THE DECISION THIS FILE'S OWN CASCADE MAKES, PROVEN AT PAINT: an
        // explicit assignment is not merely a stronger rule, it is not a
        // rule at all, and stays outside the contest `Priority` decides.
        let f = render(
            r#"
            local sheet = Instance.new("StyleSheet")
            sheet.Parent = root

            local rule = Instance.new("StyleRule")
            rule.Selector = "Frame"
            rule:SetProperty("BackgroundColor3", Color3.fromRGB(18, 23, 34))
            rule.Parent = sheet

            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 10, 0, 10)
            a.BackgroundColor3 = Color3.fromRGB(200, 200, 200)
            a.Parent = root
        "#,
            100.0,
            100.0,
        );
        assert_eq!(f.nodes[0].fill, Some(Rgb(200, 200, 200)));
    }

    #[test]
    fn mutating_a_style_rule_live_marks_the_tree_dirty_and_repaints() {
        // THE GAP THIS CLOSES: `SetProperty` and `SetProperties` wrote into
        // `Dom::style_properties` without marking anything dirty, so a mod
        // that restyled a rule at runtime would sit on a frame the render
        // loop never knew to redraw -- correct once painted, and never
        // painted again. No restart: one `Lua`, one `Dom`, one long-lived
        // `rule` handle, mutated between two reads of the same frame.
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
            local sheet = Instance.new("StyleSheet")
            sheet.Parent = root

            rule = Instance.new("StyleRule")
            rule.Selector = "Frame"
            rule:SetProperty("BackgroundColor3", Color3.fromRGB(10, 10, 10))
            rule.Parent = sheet

            local a = Instance.new("Frame")
            a.Size = UDim2.new(0, 10, 0, 10)
            a.Parent = root
        "#,
        )
        .exec()
        .expect("guest");

        let before = frame_of(&dom, root, 100.0, 100.0);
        assert_eq!(before.nodes[0].fill, Some(Rgb(10, 10, 10)));

        // Drain whatever setup above already marked dirty, so the assertion
        // below is about the mutation that follows and nothing earlier.
        dom.lock().expect("dom").take_dirty();

        lua.load(r#"rule:SetProperty("BackgroundColor3", Color3.fromRGB(200, 100, 50))"#)
            .exec()
            .expect("mutate live");

        assert!(
            dom.lock().expect("dom").take_dirty(),
            "a live StyleRule mutation should mark the tree dirty"
        );

        let after = frame_of(&dom, root, 100.0, 100.0);
        assert_eq!(after.nodes[0].fill, Some(Rgb(200, 100, 50)));
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
        // is what makes `rbxassetid://` resolve; until then this is what an engine
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

#[cfg(test)]
mod paint_order {
    use super::*;
    use crate::datamodel::{install, install_vocabulary, SharedDom};
    use mlua::prelude::*;

    /// The ids a tree paints, back to front.
    fn order(src: &str) -> Vec<String> {
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

        let guard = dom.lock().expect("dom");
        display_list(&guard, root, 200.0, 100.0)
            .into_iter()
            .filter_map(|placed| guard.name_of(placed.id))
            .collect()
    }

    /// A child draws above its parent, whatever the parent's `ZIndex`.
    ///
    /// THE BUG THIS PINS: paint order was one global sort by `ZIndex`, so a frame
    /// carrying `ZIndex = 3` painted after its own children, which carry the
    /// default 1. A window's titlebar and body text vanished underneath the
    /// window that contained them.
    ///
    /// The suite did not catch it because the case that covers `ZIndex` uses
    /// three siblings, and siblings order the same way under either rule.
    #[test]
    fn a_child_draws_above_its_parent() {
        let painted = order(
            r#"
            local card = Instance.new("Frame")
            card.Name = "Card"
            card.Size = UDim2.new(0, 100, 0, 100)
            card.ZIndex = 3
            card.Parent = root

            local label = Instance.new("Frame")
            label.Name = "Label"
            label.Size = UDim2.new(1, 0, 0, 20)
            label.Parent = card
            "#,
        );
        assert_eq!(
            painted,
            vec!["Card".to_string(), "Label".to_string()],
            "a child paints after the parent that contains it"
        );
    }

    /// `ZIndex` still orders siblings.
    #[test]
    fn zindex_orders_siblings() {
        let painted = order(
            r#"
            for _, spec in ipairs({ { "Front", 3 }, { "Behind", 1 }, { "Middle", 2 } }) do
                local f = Instance.new("Frame")
                f.Name = spec[1]
                f.Size = UDim2.new(0, 50, 0, 50)
                f.ZIndex = spec[2]
                f.Parent = root
            end
            "#,
        );
        assert_eq!(
            painted,
            vec![
                "Behind".to_string(),
                "Middle".to_string(),
                "Front".to_string()
            ]
        );
    }

    /// Siblings sharing a `ZIndex` keep declaration order.
    #[test]
    fn equal_siblings_keep_declaration_order() {
        let painted = order(
            r#"
            for _, name in ipairs({ "First", "Second", "Third" }) do
                local f = Instance.new("Frame")
                f.Name = name
                f.Size = UDim2.new(0, 50, 0, 50)
                f.Parent = root
            end
            "#,
        );
        assert_eq!(
            painted,
            vec![
                "First".to_string(),
                "Second".to_string(),
                "Third".to_string()
            ]
        );
    }

    /// A high-`ZIndex` sibling does not jump above another sibling's children.
    ///
    /// The distinction between the two behaviours, in one tree: under a global
    /// sort `Loud` outranks `Quiet`'s child and covers it. Under sibling order
    /// the child belongs to `Quiet` and paints with it.
    #[test]
    fn a_branch_paints_together() {
        let painted = order(
            r#"
            local quiet = Instance.new("Frame")
            quiet.Name = "Quiet"
            quiet.Size = UDim2.new(0, 100, 0, 100)
            quiet.ZIndex = 5
            quiet.Parent = root

            local inner = Instance.new("Frame")
            inner.Name = "Inner"
            inner.Size = UDim2.new(0, 50, 0, 50)
            inner.ZIndex = 1
            inner.Parent = quiet

            local loud = Instance.new("Frame")
            loud.Name = "Loud"
            loud.Size = UDim2.new(0, 100, 0, 100)
            loud.ZIndex = 2
            loud.Parent = root
            "#,
        );
        assert_eq!(
            painted,
            vec!["Loud".to_string(), "Quiet".to_string(), "Inner".to_string()],
            "Quiet outranks Loud as a sibling, and Inner belongs to Quiet"
        );
    }
}

#[cfg(test)]
mod text_paint {
    //! What the painter draws against what the line model measured, in real
    //! pixels and the real face. These skip on a machine with no font.
    use super::*;
    use crate::datamodel::{install, install_vocabulary, SharedDom};
    use dew_runtime::{Painter, RasterPainter};
    use mlua::prelude::*;

    const W: u32 = 240;
    const H: u32 = 120;

    /// Build `src` under a `root` global, lay it out and draw it. Returns the
    /// Luau state (to read members back), the frame, and the painted pixels as
    /// one brightness byte each.
    fn draw(src: &str) -> Option<(Lua, Frame, Vec<u8>)> {
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
        lua.load(src).exec().expect("guest");
        let frame = frame_of(&dom, root, W as f32, H as f32);

        let mut painter = RasterPainter::new(W, H, dew_raster::Backend::VelloCpu)
            .expect("surface")
            .with_face(face);
        painter.paint_frame(&frame, Some(Rgb(0, 0, 0)));
        let bgra = painter.canvas_mut().bgra().expect("pixels");
        let ink = bgra.chunks(4).map(|p| p[0].max(p[1]).max(p[2])).collect();
        Some((lua, frame, ink))
    }

    /// The smallest box holding every pixel brighter than half: (left, top,
    /// right, bottom), right and bottom exclusive.
    fn ink_box(ink: &[u8]) -> (u32, u32, u32, u32) {
        let (mut l, mut t, mut r, mut b) = (W, H, 0, 0);
        for (i, v) in ink.iter().enumerate() {
            if *v > 127 {
                let (x, y) = (i as u32 % W, i as u32 / W);
                l = l.min(x);
                t = t.min(y);
                r = r.max(x + 1);
                b = b.max(y + 1);
            }
        }
        (l, t, r, b)
    }

    fn label(props: &str) -> String {
        format!(
            r#"
            local t = Instance.new("TextLabel")
            t.Name = "Label"
            t.BackgroundTransparency = 1
            t.TextColor3 = Color3.new(1, 1, 1)
            {props}
            t.Parent = root
            label = t
            "#
        )
    }

    /// THE GUARD AGAINST PAINT AND MEASURE DRIFTING APART. A label sized by
    /// AutomaticSize is exactly as large as the model measured its text; every
    /// pixel the painter draws must land inside it, and the ink must start
    /// and end where the model's line does, give or take a glyph's side
    /// bearing.
    #[test]
    fn painted_ink_is_where_the_measurement_says() {
        let Some((_, frame, ink)) = draw(&label(
            r#"t.Text = "Hamburgefonts"
            t.TextSize = 20
            t.AutomaticSize = Enum.AutomaticSize.XY
            t.Position = UDim2.fromOffset(10, 10)"#,
        )) else {
            return;
        };
        let node = frame
            .nodes
            .iter()
            .find(|n| n.name == "Label")
            .expect("label");
        let laid = node.text_layout.as_ref().expect("a laid-out label");
        let line = &laid.lines[0];
        let (l, t, r, b) = ink_box(&ink);

        let rect = node.rect;
        assert!(
            l as f32 >= rect.x && r as f32 <= rect.x + rect.w + 0.5,
            "ink x {l}..{r} outside {rect:?}"
        );
        assert!(
            t as f32 >= rect.y && b as f32 <= rect.y + rect.h,
            "ink y {t}..{b} outside {rect:?}"
        );
        assert!(
            (l as f32 - line.x).abs() <= 3.0,
            "ink starts at {l}, the line at {}",
            line.x
        );
        assert!(
            (r as f32 - (line.x + line.width)).abs() <= 3.0,
            "ink ends at {r}, the line at {}",
            line.x + line.width
        );
    }

    /// One line, wrapped or not, is the same pixels, at every alignment.
    #[test]
    fn wrapping_one_line_paints_the_same_pixels() {
        for y in ["Top", "Center", "Bottom"] {
            let props = |wrap: bool| {
                format!(
                    r#"t.Text = "Hello"
                    t.TextSize = 20
                    t.Size = UDim2.fromOffset(200, 64)
                    t.TextYAlignment = Enum.TextYAlignment.{y}
                    t.TextWrapped = {wrap}"#
                )
            };
            let Some((_, _, plain)) = draw(&label(&props(false))) else {
                return;
            };
            let (_, _, wrapped) = draw(&label(&props(true))).expect("a face");
            assert!(plain == wrapped, "{y}: wrapping one line moved it");
        }
    }

    /// A UIPadding moves the painted text by exactly its offsets.
    #[test]
    fn padding_moves_the_ink_by_the_padding() {
        let props = r#"t.Text = "H"
            t.TextSize = 20
            t.Size = UDim2.fromOffset(200, 64)
            t.TextXAlignment = Enum.TextXAlignment.Left
            t.TextYAlignment = Enum.TextYAlignment.Top"#;
        let Some((_, _, bare)) = draw(&label(props)) else {
            return;
        };
        let padded_props = format!(
            r#"{props}
            local p = Instance.new("UIPadding")
            p.PaddingLeft = UDim.new(0, 16)
            p.PaddingTop = UDim.new(0, 16)
            p.Parent = t"#
        );
        let (_, _, padded) = draw(&label(&padded_props)).expect("a face");
        let (l0, t0, r0, b0) = ink_box(&bare);
        let (l1, t1, r1, b1) = ink_box(&padded);
        assert_eq!((l1 - l0, t1 - t0, r1 - r0, b1 - b0), (16, 16, 16, 16));
    }

    /// `TextBounds`, `TextFits` and `ContentText` read real values from Luau
    /// once a frame has laid the label out.
    #[test]
    fn text_members_are_readable_after_a_frame() {
        let Some((lua, frame, _)) = draw(&label(
            r#"t.Text = "one two three four five six"
            t.TextSize = 20
            t.TextWrapped = true
            t.Size = UDim2.fromOffset(120, 64)
            t.RichText = true"#,
        )) else {
            return;
        };
        let node = frame
            .nodes
            .iter()
            .find(|n| n.name == "Label")
            .expect("label");
        let laid = node.text_layout.as_ref().expect("laid out");
        let (x, y, fits, content): (f32, f32, bool, String) = lua
            .load(
                "return label.TextBounds.X, label.TextBounds.Y, label.TextFits, label.ContentText",
            )
            .eval()
            .expect("members");
        assert_eq!((x, y), laid.bounds);
        // Three lines of 30 in 64: two are drawn.
        assert_eq!(laid.lines.len(), 2);
        assert_eq!(y, 60.0);
        assert!(x <= 120.0);
        assert!(!fits);
        assert_eq!(content, "one two three four five six");
    }

    /// TextTruncate AtEnd in the default face cuts the line to a prefix and
    /// the single glyph U+2026, as the engine draws it: "A lon" and an
    /// ellipsis in a 100 by 64 box at TextSize 20, with `TextBounds` within a
    /// pixel of the engine's.
    #[test]
    fn truncation_ends_in_the_ellipsis_glyph() {
        let Some((lua, frame, _)) = draw(&label(
            r#"t.Text = "A long label that cannot fit"
            t.TextSize = 20
            t.Size = UDim2.fromOffset(100, 64)
            t.TextTruncate = Enum.TextTruncate.AtEnd"#,
        )) else {
            return;
        };
        let node = frame
            .nodes
            .iter()
            .find(|n| n.name == "Label")
            .expect("label");
        let laid = node.text_layout.as_ref().expect("laid out");
        assert_eq!(laid.lines.len(), 1);
        assert_eq!(laid.lines[0].text, "A lon\u{2026}");
        let (x, y, fits, content): (f32, f32, bool, String) = lua
            .load(
                "return label.TextBounds.X, label.TextBounds.Y, label.TextFits, label.ContentText",
            )
            .eval()
            .expect("members");
        // The engine reads 89 by 30.
        assert!((x - 89.0).abs() <= 1.0, "TextBounds.X {x}");
        assert_eq!(y, 30.0);
        assert!(!fits);
        assert_eq!(content, "A long label that cannot fit");
    }

    /// `ContentText` is the text without its markup when `RichText` is on, and
    /// the text as written when it is off.
    #[test]
    fn content_text_strips_markup_only_for_rich_text() {
        for (rich, want) in [(true, "bold plain"), (false, "<b>bold</b> plain")] {
            let Some((lua, _, _)) = draw(&label(&format!(
                r#"t.Text = "<b>bold</b> plain"
                t.Size = UDim2.fromOffset(200, 64)
                t.RichText = {rich}"#
            ))) else {
                return;
            };
            let content: String = lua.load("return label.ContentText").eval().expect("read");
            assert_eq!(content, want);
        }
    }

    /// A label nothing has given a face draws in the engine's default,
    /// LegacyArial, which is Arimo at one and a half times the TextSize.
    #[test]
    fn the_default_face_is_arimo_at_one_and_a_half() {
        let lua = Lua::new();
        install(&lua, &SharedDom::default()).expect("install");
        install_vocabulary(&lua).expect("vocabulary");
        let family: String = lua
            .load(r#"return Instance.new("TextLabel").FontFace.Family"#)
            .eval()
            .expect("default FontFace");
        assert_eq!(family, crate::services::default_font().family);

        let Some(resolved) = crate::fonts::resolve_font(&crate::services::default_font()) else {
            return;
        };
        assert_eq!(resolved.path.file_name().unwrap(), "Arimo-Regular.ttf");
        assert_eq!(resolved.em_scale, 1.5);

        let unset = draw(&label(
            r#"t.Text = "Hamburgefonts" t.Size = UDim2.fromOffset(200, 64)"#,
        ));
        let set = draw(&label(
            r#"t.Text = "Hamburgefonts" t.Size = UDim2.fromOffset(200, 64)
            t.FontFace = Font.new("rbxasset://fonts/families/LegacyArial.json")"#,
        ));
        let (Some((_, _, unset)), Some((_, _, set))) = (unset, set) else {
            return;
        };
        assert!(unset == set, "the default face is not LegacyArial");
    }

    /// Assigning the legacy `Font` sets `FontFace`, and the label draws in it.
    /// A modern face draws narrower than the default, so the face really
    /// changed.
    #[test]
    fn the_legacy_font_draws_in_the_face_it_names() {
        let props = |face: &str| {
            format!(
                r#"t.Text = "Hamburgefonts"
                t.TextSize = 20
                t.AutomaticSize = Enum.AutomaticSize.XY
                {face}"#
            )
        };
        let Some((lua, by_enum, by_enum_ink)) =
            draw(&label(&props("t.Font = Enum.Font.SourceSans")))
        else {
            return;
        };
        let family: String = lua
            .load("return label.FontFace.Family")
            .eval()
            .expect("FontFace");
        assert_eq!(family, "rbxasset://fonts/families/SourceSansPro.json");

        let (_, by_face, by_face_ink) = draw(&label(&props(
            r#"t.FontFace = Font.new("rbxasset://fonts/families/SourceSansPro.json")"#,
        )))
        .expect("a face");
        let (_, default, _) = draw(&label(&props(""))).expect("a face");
        let width = |frame: &Frame| {
            frame
                .nodes
                .iter()
                .find(|n| n.name == "Label")
                .expect("label")
                .rect
                .w
        };
        assert!(by_enum_ink == by_face_ink);
        assert_eq!(width(&by_enum), width(&by_face));
        assert!(width(&by_face) < width(&default) * 0.7);
    }
}

#[cfg(test)]
mod text_parity {
    //! Painted text against the engine's own pixels.
    //!
    //! Each row is a label the engine drew in Studio at display scale 1.000, white
    //! on black, 200 x 64, with the ink it drew measured relative to the label's
    //! box: left, right, top and bottom edges of every pixel brighter than 110 in
    //! any channel. Dew draws the same label through the real renderer and painter
    //! and is measured the same way. Every edge must be within a pixel.
    //!
    //! BUILDER SANS IS ONLY CHECKED WHERE IT CAN BE DRAWN. Dew may not ship it, so
    //! its rows run only when the face resolves from a local Studio install and are
    //! skipped, not failed, everywhere else.

    use super::frame_of;
    use crate::datamodel::{install, install_vocabulary, SharedDom};
    use crate::fonts::{self, Source};
    use dew_runtime::frame::Rgb;
    use dew_runtime::{Painter, RasterPainter};
    use mlua::prelude::*;

    /// The label's box, placed `MARGIN` in from the surface's corner so ink that
    /// strays outside it is still seen.
    const MARGIN: u32 = 8;
    const BOX_W: u32 = 200;
    const BOX_H: u32 = 64;

    /// A pixel counts as ink when a channel is brighter than this, as in the
    /// script that measured the engine's screenshots.
    const INK: u8 = 110;

    /// One engine measurement: family, TextSize, TextYAlignment, TextXAlignment,
    /// text, UIPadding left and top, and the engine's ink edges (left, right, top,
    /// bottom) relative to the label's box.
    type Row = (
        &'static str,
        f32,
        &'static str,
        &'static str,
        &'static str,
        u32,
        [f32; 4],
    );

    /// From the engine's screenshots of the text probe, pages 1 to 3. The engine
    /// drew every one of these identically wrapped and unwrapped.
    const ENGINE: &[Row] = &[
        (
            "LegacyArial",
            14.0,
            "Top",
            "Left",
            "H",
            0,
            [2.0, 13.0, 3.0, 17.0],
        ),
        (
            "LegacyArial",
            14.0,
            "Center",
            "Left",
            "H",
            0,
            [2.0, 13.0, 25.0, 39.0],
        ),
        (
            "LegacyArial",
            14.0,
            "Bottom",
            "Left",
            "H",
            0,
            [2.0, 13.0, 46.0, 60.0],
        ),
        (
            "LegacyArial",
            20.0,
            "Top",
            "Left",
            "H",
            0,
            [2.0, 18.0, 5.0, 24.0],
        ),
        (
            "LegacyArial",
            20.0,
            "Center",
            "Left",
            "H",
            0,
            [2.0, 18.0, 22.0, 41.0],
        ),
        (
            "LegacyArial",
            20.0,
            "Bottom",
            "Left",
            "H",
            0,
            [2.0, 18.0, 39.0, 58.0],
        ),
        (
            "LegacyArial",
            32.0,
            "Top",
            "Left",
            "H",
            0,
            [4.0, 28.0, 8.0, 39.0],
        ),
        (
            "LegacyArial",
            32.0,
            "Center",
            "Left",
            "H",
            0,
            [4.0, 28.0, 16.0, 47.0],
        ),
        (
            "LegacyArial",
            32.0,
            "Bottom",
            "Left",
            "H",
            0,
            [4.0, 28.0, 24.0, 55.0],
        ),
        (
            "LegacyArial",
            20.0,
            "Center",
            "Left",
            "Hello",
            0,
            [2.0, 61.0, 21.0, 41.0],
        ),
        (
            "LegacyArial",
            20.0,
            "Center",
            "Center",
            "Hello",
            0,
            [71.0, 130.0, 21.0, 41.0],
        ),
        (
            "LegacyArial",
            20.0,
            "Center",
            "Right",
            "Hello",
            0,
            [140.0, 199.0, 21.0, 41.0],
        ),
        (
            "LegacyArial",
            20.0,
            "Top",
            "Left",
            "H",
            16,
            [18.0, 34.0, 21.0, 40.0],
        ),
        (
            "BuilderSans",
            14.0,
            "Top",
            "Left",
            "H",
            0,
            [1.0, 7.0, 4.0, 12.0],
        ),
        (
            "BuilderSans",
            14.0,
            "Center",
            "Left",
            "H",
            0,
            [1.0, 7.0, 29.0, 37.0],
        ),
        (
            "BuilderSans",
            14.0,
            "Bottom",
            "Left",
            "H",
            0,
            [1.0, 7.0, 54.0, 62.0],
        ),
        (
            "BuilderSans",
            20.0,
            "Top",
            "Left",
            "H",
            0,
            [1.0, 10.0, 4.0, 16.0],
        ),
        (
            "BuilderSans",
            20.0,
            "Center",
            "Left",
            "H",
            0,
            [1.0, 10.0, 26.0, 38.0],
        ),
        (
            "BuilderSans",
            20.0,
            "Bottom",
            "Left",
            "H",
            0,
            [1.0, 10.0, 48.0, 60.0],
        ),
        (
            "BuilderSans",
            32.0,
            "Top",
            "Left",
            "H",
            0,
            [2.0, 16.0, 8.0, 26.0],
        ),
        (
            "BuilderSans",
            32.0,
            "Center",
            "Left",
            "H",
            0,
            [2.0, 16.0, 24.0, 42.0],
        ),
        (
            "BuilderSans",
            32.0,
            "Bottom",
            "Left",
            "H",
            0,
            [2.0, 16.0, 40.0, 58.0],
        ),
        (
            "SourceSansPro",
            14.0,
            "Top",
            "Left",
            "H",
            0,
            [1.0, 7.0, 4.0, 12.0],
        ),
        (
            "SourceSansPro",
            14.0,
            "Center",
            "Left",
            "H",
            0,
            [1.0, 7.0, 29.0, 37.0],
        ),
        (
            "SourceSansPro",
            14.0,
            "Bottom",
            "Left",
            "H",
            0,
            [1.0, 7.0, 54.0, 62.0],
        ),
        (
            "SourceSansPro",
            20.0,
            "Top",
            "Left",
            "H",
            0,
            [1.0, 10.0, 5.0, 16.0],
        ),
        (
            "SourceSansPro",
            20.0,
            "Center",
            "Left",
            "H",
            0,
            [1.0, 10.0, 27.0, 38.0],
        ),
        (
            "SourceSansPro",
            20.0,
            "Bottom",
            "Left",
            "H",
            0,
            [1.0, 10.0, 49.0, 60.0],
        ),
        (
            "SourceSansPro",
            32.0,
            "Top",
            "Left",
            "H",
            0,
            [2.0, 15.0, 9.0, 26.0],
        ),
        (
            "SourceSansPro",
            32.0,
            "Center",
            "Left",
            "H",
            0,
            [2.0, 15.0, 25.0, 42.0],
        ),
        (
            "SourceSansPro",
            32.0,
            "Bottom",
            "Left",
            "H",
            0,
            [2.0, 15.0, 41.0, 58.0],
        ),
    ];

    /// Draw one engine row's label, wrapped or not, and measure its ink relative
    /// to the label's box. `None` when nothing was drawn.
    fn ink_of(row: &Row, wrap: bool) -> Option<[f32; 4]> {
        let (family, size, y, x, text, pad, _) = *row;
        let (w, h) = (BOX_W + 2 * MARGIN, BOX_H + 2 * MARGIN);
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
        let padding = if pad > 0 {
            format!(
                "local p = Instance.new(\"UIPadding\")
                p.PaddingLeft = UDim.new(0, {pad})
                p.PaddingTop = UDim.new(0, {pad})
                p.Parent = t"
            )
        } else {
            String::new()
        };
        lua.load(format!(
            r#"
            local t = Instance.new("TextLabel")
            t.Position = UDim2.fromOffset({MARGIN}, {MARGIN})
            t.Size = UDim2.fromOffset({BOX_W}, {BOX_H})
            t.BackgroundTransparency = 1
            t.TextColor3 = Color3.new(1, 1, 1)
            t.FontFace = Font.new("rbxasset://fonts/families/{family}.json")
            t.Text = "{text}"
            t.TextSize = {size}
            t.TextXAlignment = Enum.TextXAlignment.{x}
            t.TextYAlignment = Enum.TextYAlignment.{y}
            t.TextWrapped = {wrap}
            {padding}
            t.Parent = root
            "#
        ))
        .exec()
        .expect("guest");
        let frame = frame_of(&dom, root, w as f32, h as f32);
        let mut painter = RasterPainter::new(w, h, dew_raster::Backend::VelloCpu).expect("surface");
        painter.paint_frame(&frame, Some(Rgb(0, 0, 0)));
        let bgra = painter.canvas_mut().bgra().expect("pixels");

        let (mut l, mut t, mut r, mut b) = (u32::MAX, u32::MAX, 0, 0);
        for (i, p) in bgra.chunks(4).enumerate() {
            if p[0].max(p[1]).max(p[2]) > INK {
                let (px, py) = (i as u32 % w, i as u32 / w);
                l = l.min(px);
                t = t.min(py);
                r = r.max(px + 1);
                b = b.max(py + 1);
            }
        }
        (l != u32::MAX).then(|| {
            let m = MARGIN as f32;
            [l as f32 - m, r as f32 - m, t as f32 - m, b as f32 - m]
        })
    }

    /// Whether a family can be drawn as itself here, and why not when it cannot.
    fn drawable(family: &str) -> Result<(), String> {
        let uri = format!("{}{family}.json", fonts::FAMILY_PREFIX);
        match fonts::resolve(
            &uri,
            rbx_types::FontWeight::Regular,
            rbx_types::FontStyle::Normal,
        ) {
            Some(face) if family != "BuilderSans" || face.source == Source::LocalStudio => Ok(()),
            Some(face) => Err(format!("{family} resolves {:?}", face.source)),
            None => Err(format!("{family} does not resolve")),
        }
    }

    /// Every row of one family, wrapped and unwrapped, against the engine. Prints
    /// each row, and fails listing every edge more than a pixel out.
    fn check(family: &str) {
        if let Err(why) = drawable(family) {
            eprintln!("skipped: {why}");
            return;
        }
        let mut misses = Vec::new();
        for row in ENGINE.iter().filter(|row| row.0 == family) {
            let engine = row.6;
            for wrap in [false, true] {
                let name = format!(
                    "{family} {} {} {} {} pad {} {}",
                    row.1,
                    row.2,
                    row.3,
                    row.4,
                    row.5,
                    if wrap { "wrap" } else { "nowrap" }
                );
                let Some(dew) = ink_of(row, wrap) else {
                    misses.push(format!("{name}: no ink"));
                    continue;
                };
                eprintln!("{name:<48} dew {dew:?} engine {engine:?}");
                for (edge, (d, e)) in ["L", "R", "T", "B"].iter().zip(dew.iter().zip(engine)) {
                    if (d - e).abs() > 1.0 {
                        misses.push(format!("{name}: {edge} {d} against the engine's {e}"));
                    }
                }
            }
        }
        assert!(misses.is_empty(), "{}", misses.join("\n"));
    }

    #[test]
    fn legacy_arial_ink_is_within_a_pixel_of_the_engine() {
        check("LegacyArial");
    }

    #[test]
    fn source_sans_pro_ink_is_within_a_pixel_of_the_engine() {
        check("SourceSansPro");
    }

    /// Runs only where a Studio install provides the face.
    #[test]
    fn builder_sans_ink_is_within_a_pixel_of_the_engine() {
        check("BuilderSans");
    }
}

/// A render pass resolves each instance's cascade once and keeps the answer
/// until the pass ends. These change one input of the cascade between two
/// frames, through the same calls a guest makes, and check the second frame
/// paints the new answer rather than the first frame's.
#[cfg(test)]
mod cascade_between_frames {
    use super::*;
    use crate::datamodel::{install, install_vocabulary, SharedDom};
    use mlua::prelude::*;

    /// A sheet with one rule styling `#Card`, a `Card` frame it reaches, and
    /// a second frame holding no sheet for a reparent to move `Card` under.
    fn scene() -> (Lua, SharedDom, usize) {
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
            r##"
            styled = Instance.new("Frame")
            styled.Name = "Styled"
            styled.Size = UDim2.fromOffset(100, 50)
            styled.Parent = root

            sheet = Instance.new("StyleSheet")
            sheet:SetAttribute("CardColor", Color3.fromRGB(10, 20, 30))
            sheet.Parent = styled

            rule = Instance.new("StyleRule")
            rule.Selector = "#Card"
            rule:SetProperty("BackgroundColor3", "$CardColor")
            rule.Parent = sheet

            plain = Instance.new("Frame")
            plain.Name = "Plain"
            plain.Position = UDim2.fromOffset(0, 50)
            plain.Size = UDim2.fromOffset(100, 50)
            plain.BackgroundColor3 = Color3.fromRGB(1, 1, 1)
            plain.Parent = root

            card = Instance.new("Frame")
            card.Name = "Card"
            card.Size = UDim2.fromOffset(20, 20)
            card.Parent = styled
            "##,
        )
        .exec()
        .expect("guest");
        (lua, dom, root)
    }

    /// The fill the frame painted for the node named `name`.
    fn fill(dom: &SharedDom, root: usize, name: &str) -> Option<Rgb> {
        frame_of(dom, root, 100.0, 100.0)
            .nodes
            .into_iter()
            .find(|node| node.name == name)
            .expect("painted")
            .fill
    }

    fn run(lua: &Lua, src: &str) {
        lua.load(src).exec().expect("guest");
    }

    /// The colour a fresh `Frame` paints with no rule reaching it.
    fn unstyled() -> Option<Rgb> {
        let (lua, dom, root) = scene();
        run(&lua, "rule.Selector = '#Nothing'");
        fill(&dom, root, "Card")
    }

    #[test]
    fn the_scene_starts_styled_by_its_rule() {
        let (_lua, dom, root) = scene();
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        assert_ne!(unstyled(), Some(Rgb(10, 20, 30)));
    }

    #[test]
    fn renaming_the_instance_stops_a_name_selector_matching() {
        let (lua, dom, root) = scene();
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        run(&lua, "card.Name = 'Other'");
        assert_eq!(fill(&dom, root, "Other"), unstyled());
    }

    #[test]
    fn a_rules_new_property_value_is_painted() {
        let (lua, dom, root) = scene();
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        run(
            &lua,
            "rule:SetProperty('BackgroundColor3', Color3.fromRGB(200, 100, 50))",
        );
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(200, 100, 50)));
    }

    #[test]
    fn a_rules_new_selector_is_matched() {
        let (lua, dom, root) = scene();
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        run(&lua, "rule.Selector = '.Accent'");
        assert_eq!(fill(&dom, root, "Card"), unstyled());
    }

    #[test]
    fn a_new_rule_is_applied() {
        let (lua, dom, root) = scene();
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        run(
            &lua,
            r##"
            local stronger = Instance.new("StyleRule")
            stronger.Selector = "Frame"
            stronger.Priority = 5
            stronger:SetProperty("BackgroundColor3", Color3.fromRGB(0, 128, 0))
            stronger.Parent = sheet
            "##,
        );
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(0, 128, 0)));
    }

    #[test]
    fn adding_and_removing_a_tag_is_matched() {
        let (lua, dom, root) = scene();
        run(&lua, "rule.Selector = '.Accent'");
        assert_eq!(fill(&dom, root, "Card"), unstyled());
        run(
            &lua,
            "services:GetService('CollectionService'):AddTag(card, 'Accent')",
        );
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        run(
            &lua,
            "services:GetService('CollectionService'):RemoveTag(card, 'Accent')",
        );
        assert_eq!(fill(&dom, root, "Card"), unstyled());
    }

    #[test]
    fn a_new_parent_outside_the_sheet_is_not_styled() {
        let (lua, dom, root) = scene();
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        run(&lua, "card.Parent = plain");
        assert_eq!(fill(&dom, root, "Card"), unstyled());
        run(&lua, "card.Parent = styled");
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
    }

    #[test]
    fn a_new_token_value_is_painted() {
        let (lua, dom, root) = scene();
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        run(
            &lua,
            "sheet:SetAttribute('CardColor', Color3.fromRGB(40, 50, 60))",
        );
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(40, 50, 60)));
    }

    #[test]
    fn a_new_gui_state_is_matched() {
        let (lua, dom, root) = scene();
        run(&lua, "rule.Selector = ':Hover'");
        assert_eq!(fill(&dom, root, "Card"), unstyled());
        run(&lua, "card:SetGuiState(Enum.GuiState.Hover)");
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
    }

    #[test]
    fn an_explicit_value_set_after_a_frame_wins() {
        let (lua, dom, root) = scene();
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(10, 20, 30)));
        run(&lua, "card.BackgroundColor3 = Color3.fromRGB(7, 7, 7)");
        assert_eq!(fill(&dom, root, "Card"), Some(Rgb(7, 7, 7)));
    }

    /// A hit test is its own pass, nested in nothing, and must read the tree
    /// as it is now: a rule that hides an element takes it out of the list.
    #[test]
    fn a_display_list_after_a_change_reads_the_change() {
        let (lua, dom, root) = scene();
        let card_id = |list: &[Placed], dom: &SharedDom| {
            let guard = dom.lock().expect("dom");
            list.iter()
                .any(|p| guard.name_of(p.id).as_deref() == Some("Card"))
        };
        let before = display_list(&dom.lock().expect("dom"), root, 100.0, 100.0);
        assert!(card_id(&before, &dom));
        run(&lua, "rule:SetProperty('Visible', false)");
        let after = display_list(&dom.lock().expect("dom"), root, 100.0, 100.0);
        assert!(!card_id(&after, &dom));
    }
}

/// The cost of one render pass over a styled list of rows, shaped like a
/// settings or library window: a sheet of rules most elements match none
/// of, rows that size themselves, buttons that size themselves inside them,
/// and an auto-sized scroller holding it all.
#[cfg(test)]
mod styled_list_cost {
    use super::*;
    use crate::datamodel::{install, install_vocabulary, SharedDom};
    use mlua::prelude::*;

    const ROWS: usize = 24;

    fn scene() -> (Lua, SharedDom, usize) {
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
        lua.globals().set("ROWS", ROWS).expect("rows");
        lua.load(
            r##"
            local CollectionService = services:GetService("CollectionService")
            local window = Instance.new("Frame")
            window.Size = UDim2.fromOffset(720, 560)
            window.Parent = root

            local sheet = Instance.new("StyleSheet")
            sheet:SetAttribute("Surface", Color3.fromRGB(30, 32, 40))
            sheet.Parent = window
            local function rule(selector, props)
                local r = Instance.new("StyleRule")
                r.Selector = selector
                r:SetProperties(props)
                r.Parent = sheet
            end
            rule(".Row", { BackgroundColor3 = "$Surface", BorderSizePixel = 0 })
            rule(".Button", { BackgroundColor3 = Color3.fromRGB(60, 90, 200), TextSize = 13 })
            rule("TextLabel", { TextColor3 = Color3.fromRGB(230, 230, 230), BackgroundTransparency = 1 })
            rule("TextButton", { TextColor3 = Color3.new(1, 1, 1), AutoButtonColor = false })
            rule("#Name", { TextSize = 15, TextTransparency = 0 })
            rule("#Detail", { TextSize = 13, TextTransparency = 0.2 })
            rule("#Description", { TextWrapped = true, TextSize = 13 })
            for i = 1, 30 do
                rule("#Unused" .. i, { BackgroundTransparency = 0.5 })
            end

            local scroller = Instance.new("ScrollingFrame")
            scroller.Size = UDim2.fromScale(1, 1)
            scroller.CanvasSize = UDim2.new()
            scroller.AutomaticCanvasSize = Enum.AutomaticSize.Y
            scroller.Parent = window
            local list = Instance.new("UIListLayout")
            list.Padding = UDim.new(0, 6)
            list.Parent = scroller

            for i = 1, ROWS do
                local row = Instance.new("Frame")
                row.Name = "Row" .. i
                row.Size = UDim2.fromScale(1, 0)
                row.AutomaticSize = Enum.AutomaticSize.Y
                CollectionService:AddTag(row, "Row")
                row.Parent = scroller
                local rows = Instance.new("UIListLayout")
                rows.Parent = row

                local head = Instance.new("Frame")
                head.Size = UDim2.new(1, 0, 0, 0)
                head.AutomaticSize = Enum.AutomaticSize.Y
                head.BackgroundTransparency = 1
                head.Parent = row
                local line = Instance.new("UIListLayout")
                line.FillDirection = Enum.FillDirection.Horizontal
                line.Wraps = true
                line.Parent = head

                local text = Instance.new("Frame")
                text.Size = UDim2.fromOffset(300, 0)
                text.AutomaticSize = Enum.AutomaticSize.Y
                text.BackgroundTransparency = 1
                text.Parent = head
                Instance.new("UIListLayout").Parent = text
                for _, name in { "Name", "Detail" } do
                    local label = Instance.new("TextLabel")
                    label.Name = name
                    label.Text = name .. " of row " .. i
                    label.Size = UDim2.new(1, 0, 0, 20)
                    label.Parent = text
                end

                local actions = Instance.new("Frame")
                actions.AutomaticSize = Enum.AutomaticSize.XY
                actions.BackgroundTransparency = 1
                actions.Parent = head
                local buttons = Instance.new("UIListLayout")
                buttons.FillDirection = Enum.FillDirection.Horizontal
                buttons.Padding = UDim.new(0, 6)
                buttons.Parent = actions
                for _, label in { "Disable", "Uninstall" } do
                    local button = Instance.new("TextButton")
                    button.Text = label
                    button.AutomaticSize = Enum.AutomaticSize.XY
                    CollectionService:AddTag(button, "Button")
                    button.Parent = actions
                    local pad = Instance.new("UIPadding")
                    pad.PaddingLeft = UDim.new(0, 10)
                    pad.PaddingRight = UDim.new(0, 10)
                    pad.Parent = button
                end

                local description = Instance.new("TextLabel")
                description.Name = "Description"
                description.Text = "A description long enough that a narrow window wraps it onto a second line"
                description.Size = UDim2.fromScale(1, 0)
                description.AutomaticSize = Enum.AutomaticSize.Y
                description.Parent = row
            end
            "##,
        )
        .exec()
        .expect("guest");
        (lua, dom, root)
    }

    fn instances(dom: &SharedDom, root: usize) -> usize {
        let guard = dom.lock().expect("dom");
        let mut stack = vec![root];
        let mut count = 0;
        while let Some(id) = stack.pop() {
            count += 1;
            stack.extend(guard.children(id));
        }
        count
    }

    /// Each instance's cascade is resolved at most once in a frame, however
    /// many times layout reads it, measures it or solves it again.
    #[test]
    fn a_frame_resolves_each_instance_at_most_once() {
        let (_lua, dom, root) = scene();
        let total = instances(&dom, root);
        crate::datamodel::cascade::RESOLVES.with(|count| count.set(0));
        let frame = frame_of(&dom, root, 720.0, 560.0);
        let resolves = crate::datamodel::cascade::RESOLVES.with(|count| count.get());
        assert!(frame.nodes.len() > ROWS * 6, "{} nodes", frame.nodes.len());
        assert!(
            resolves <= total,
            "{resolves} cascade resolves for {total} instances in one frame"
        );
    }

    /// `cargo test --release -p dew-host styled_list_cost -- --ignored --nocapture`
    #[test]
    #[ignore = "a timing, not an assertion: run it to measure a frame"]
    fn time_a_frame() {
        let (_lua, dom, root) = scene();
        let mut times: Vec<std::time::Duration> = (0..21)
            .map(|_| {
                dom.lock().expect("dom").touch();
                let start = std::time::Instant::now();
                let _ = frame_of(&dom, root, 720.0, 560.0);
                start.elapsed()
            })
            .collect();
        times.sort();
        println!(
            "{} instances, {} rows: median frame {:?} (min {:?}, max {:?})",
            instances(&dom, root),
            ROWS,
            times[times.len() / 2],
            times[0],
            times[times.len() - 1]
        );
    }
}
