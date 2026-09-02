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
//! `UICorner` and `UIStroke`, and draws text. It does NOT do `AutomaticSize`,
//! `UIListLayout`, gradients, or any of the constraints. Those are
//! `conformance/LAYOUT.md`'s subject and they arrive with the Rust conformance
//! runner that can hold them to the engine's own answers -- `AutomaticSize` alone
//! cost four wrong rules in Aether, each fitting every case that existed when it
//! was written, and reimplementing it here from memory would be the fifth.
//!
//! So: enough to put a real tree on screen, and no claim beyond that.

use aether_runtime::frame::{Align, Frame, Node, Rect, Rgb, Stroke};
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

/// Build the display list for the subtree under `root`.
///
/// `root` itself is the surface and is not drawn; its children are laid out
/// against the box the surface offers, which is how a `ScreenGui` behaves.
pub fn frame(dom: &Dom, root: usize, width: f32, height: f32) -> Frame {
    let surface = Box2 {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    };

    // (z, sequence, node). PAINT ORDER IS ZINDEX, THEN DEPTH, THEN DECLARATION,
    // per LAYOUT.md section 6. A stable sort on ZIndex alone gives the other two
    // for free, because the walk below is already depth-first in declaration
    // order.
    let mut collected: Vec<(i32, Node)> = Vec::new();
    let mut sequence: u64 = 0;

    fn walk(
        dom: &Dom,
        id: usize,
        parent: Box2,
        clip: Option<Rect>,
        collected: &mut Vec<(i32, Node)>,
        sequence: &mut u64,
    ) {
        for child in dom.children(id) {
            let Some(class) = dom.class_of(child) else {
                continue;
            };
            if is_modifier(&class) {
                continue;
            }
            // INVISIBLE HIDES THE SUBTREE, not just the element. A child of an
            // invisible parent is not drawn in the engine either.
            if boolean(dom, child, "Visible") == Some(false) {
                continue;
            }

            let rect = resolve(dom, child, parent);

            // ZERO AREA IS ABSENT FROM THE DISPLAY LIST, which the conformance
            // case `zero_area_nodes_are_not_drawn` verifies against the engine.
            // Its children still lay out against it.
            let drawable = rect.w > 0.0 && rect.h > 0.0;

            if drawable {
                *sequence += 1;
                let node = Node {
                    id: *sequence,
                    name: dom.name_of(child).unwrap_or_default(),
                    rect: Rect {
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: rect.h,
                    },
                    fill: colour(dom, child, "BackgroundColor3"),
                    alpha: alpha_from(dom, child, "BackgroundTransparency"),
                    radius: corner_radius(dom, child),
                    clip,
                    stroke: stroke_of(dom, child),
                    gradient: None,
                    text: if draws_text(&class) {
                        text(dom, child, "Text").filter(|t| !t.is_empty())
                    } else {
                        None
                    },
                    text_size: number(dom, child, "TextSize").unwrap_or(14.0),
                    text_align_x: align(dom, child, "TextXAlignment", "TextXAlignment"),
                    text_align_y: align(dom, child, "TextYAlignment", "TextYAlignment"),
                    text_colour: colour(dom, child, "TextColor3"),
                };
                let z = number(dom, child, "ZIndex").unwrap_or(1.0) as i32;
                collected.push((z, node));
            }

            let inner = if boolean(dom, child, "ClipsDescendants") == Some(true) {
                Some(Rect {
                    x: rect.x,
                    y: rect.y,
                    w: rect.w,
                    h: rect.h,
                })
            } else {
                clip
            };
            walk(dom, child, rect, inner, collected, sequence);
        }
    }

    walk(dom, root, surface, None, &mut collected, &mut sequence);
    collected.sort_by_key(|(z, _)| *z);

    Frame {
        width,
        height,
        nodes: collected.into_iter().map(|(_, n)| n).collect(),
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
    frame(&guard, root, width, height)
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
            .set("root", crate::datamodel::handle(&dom, root))
            .expect("root");
        lua.load(src).exec().expect("guest");
        frame_of(&dom, root, width, height)
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
            .set("root", crate::datamodel::handle(&dom, root))
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
