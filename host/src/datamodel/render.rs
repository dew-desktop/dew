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

use dew_runtime::frame::{Align, Frame, Image, Node, Rect, Rgb, Scale, Stroke};
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
    matches!(class, "TextLabel" | "TextButton" | "TextBox")
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
    matches!(class, "ImageLabel" | "ImageButton")
}

fn udim2(dom: &Dom, id: usize, key: &str) -> (f32, f32, f32, f32) {
    match dom.property(id, key) {
        Some(Variant::UDim2(v)) => (v.x.scale, v.x.offset as f32, v.y.scale, v.y.offset as f32),
        _ => (0.0, 0.0, 0.0, 0.0),
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

/// Resolve one element against the box its parent offers.
///
/// THE COORDINATE MODEL, and it is `conformance/LAYOUT.md` section 1 rather than
/// an invention: a `UDim` is `scale * available + offset` per axis, and
/// `AnchorPoint` then shifts the element by a fraction of ITS OWN resolved size.
/// The ordering is size first, then anchor, which is the thing
/// `anchor_point_after_automatic_size` was opened in Studio to confirm.
fn resolve(dom: &Dom, id: usize, parent: Box2) -> Box2 {
    let (sxs, sxo, sys, syo) = udim2(dom, id, "Size");
    let (pxs, pxo, pys, pyo) = udim2(dom, id, "Position");
    let anchor = vector2(dom, id, "AnchorPoint");

    let w = sxs * parent.w + sxo;
    let h = sys * parent.h + syo;
    Box2 {
        x: parent.x + pxs * parent.w + pxo - anchor.x * w,
        y: parent.y + pys * parent.h + pyo - anchor.y * h,
        w,
        h,
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
    /// The clipping box inherited from the nearest `ClipsDescendants` ancestor.
    ///
    /// APPLIES TO HIT TESTING AS WELL AS DRAWING. A child outside a clipping
    /// parent is not visible and therefore not clickable, and honouring it in
    /// the painter alone is the easy half to do and the easy half to forget.
    pub clip: Option<Box2>,
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

    let mut collected: Vec<(i32, Placed)> = Vec::new();

    fn walk(
        dom: &Dom,
        id: usize,
        parent: Box2,
        clip: Option<Box2>,
        collected: &mut Vec<(i32, Placed)>,
    ) {
        for child in dom.children(id) {
            let Some(class) = dom.class_of(child) else {
                continue;
            };
            if is_modifier(&class) {
                continue;
            }
            // INVISIBLE HIDES THE SUBTREE, not just the element. A child of an
            // invisible parent is not drawn in the engine either -- and it is not
            // clickable there either, which now follows from the same line.
            if boolean(dom, child, "Visible") == Some(false) {
                continue;
            }

            let rect = resolve(dom, child, parent);

            // ZERO AREA IS ABSENT FROM THE DISPLAY LIST, which the conformance
            // case `zero_area_nodes_are_not_drawn` verifies against the engine.
            // Its children still lay out against it. A zero-area box could not
            // contain a point either, so the hit test needs no rule of its own.
            if rect.w > 0.0 && rect.h > 0.0 {
                let z = number(dom, child, "ZIndex").unwrap_or(1.0) as i32;
                collected.push((
                    z,
                    Placed {
                        id: child,
                        rect,
                        clip,
                    },
                ));
            }

            let inner = if boolean(dom, child, "ClipsDescendants") == Some(true) {
                Some(rect)
            } else {
                clip
            };
            walk(dom, child, rect, inner, collected);
        }
    }

    walk(dom, root, surface, None, &mut collected);
    collected.sort_by_key(|(z, _)| *z);
    collected.into_iter().map(|(_, placed)| placed).collect()
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
        clip: placed.clip.map(|c| Rect {
            x: c.x,
            y: c.y,
            w: c.w,
            h: c.h,
        }),
        stroke: stroke_of(dom, id),
        gradient: None,
        text: if draws_text(&class) {
            text(dom, id, "Text").filter(|t| !t.is_empty())
        } else {
            None
        },
        text_size: number(dom, id, "TextSize").unwrap_or(14.0),
        text_align_x: align(dom, id, "TextXAlignment", "TextXAlignment"),
        text_align_y: align(dom, id, "TextYAlignment", "TextYAlignment"),
        text_colour: colour(dom, id, "TextColor3"),
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
    fn walk(dom: &mut Dom, id: usize, parent: Box2) {
        for child in dom.children(id) {
            let rect = resolve(dom, child, parent);
            dom.set_internal(
                child,
                "AbsolutePosition",
                Variant::Vector2(Vector2::new(rect.x, rect.y)),
            );
            dom.set_internal(
                child,
                "AbsoluteSize",
                Variant::Vector2(Vector2::new(rect.w, rect.h)),
            );
            walk(dom, child, rect);
        }
    }
    walk(
        dom,
        root,
        Box2 {
            x: 0.0,
            y: 0.0,
            w: width,
            h: height,
        },
    );
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
}
