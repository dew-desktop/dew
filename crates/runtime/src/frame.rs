//! The display list, decoded.
//!
//! These structs mirror `src/host/Live.luau`'s `buildNode` field for field. That
//! file is the authority for the fields a Luau display list carries; this one
//! follows it. When a field is added there it is added here, and
//! `Frame::CONTRACT_FIELDS` below is what keeps the two in step rather than
//! memory.
//!
//! IT IS NOT THE ONLY AUTHORITY ANY MORE. Dew builds `Node`s from its own
//! DataModel arena, with no Luau in between (`dew_host::datamodel::render`), so
//! this is Dew's render IR as much as it is Aether's wire format — which is
//! ADR-004's point, and it is why `image` can exist here at all. A resolved
//! bitmap has no sensible Lua representation, so the fields split into the two
//! lists on `Frame`: `CONTRACT_FIELDS`, which Live.luau emits and `from_lua`
//! decodes, and `HOST_FIELDS`, which only a host that resolves assets can fill.
//!
//! `Option` IS LOAD-BEARING AND NOT A CONVENIENCE. Live.luau emits nil rather
//! than a default for `fill`, `stroke`, `gradient` and `text`, with the reason
//! stated there: "nothing set a colour" is a finding, and a default would erase
//! it. Decoding a missing fill as opaque black would invent geometry the engine
//! host does not draw — which is exactly the class of divergence this crate
//! exists to prevent.

use mlua::prelude::*;

/// 0-255 per channel, as Live.luau's `rgb()` emits them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    fn from_table(t: Option<LuaTable>) -> Option<Self> {
        let t = t?;
        Some(Rgb(t.get(1).ok()?, t.get(2).ok()?, t.get(3).ok()?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone)]
pub struct Stroke {
    pub colour: Option<Rgb>,
    pub thickness: f32,
    /// Already inverted from the engine `Transparency` by Live.luau: 1 is opaque.
    pub alpha: f32,
}

/// A colour ramp stop.
#[derive(Debug, Clone, Copy)]
pub struct Stop {
    pub at: f32,
    pub colour: Rgb,
}

/// An alpha ramp stop.
#[derive(Debug, Clone, Copy)]
pub struct AlphaStop {
    pub at: f32,
    pub alpha: f32,
}

/// The shape of a gradient ramp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradientKind {
    /// A straight ramp along a line, rotated by `rotation` degrees.
    #[default]
    Linear,
    /// A circular or elliptical ramp spreading outward from the center.
    Radial,
}

/// A UIGradient.
///
/// `stops` is a COLOUR ramp and `alpha_stops` an ALPHA ramp, and a gradient may
/// carry either or both. Live.luau records what happens when only the first is
/// honoured: one of ShopUI's two gradients went unexpressed and a window body
/// read flat, because its gradient is a `Transparency` NumberSequence with no
/// colour to interpolate. A painter that handles only `stops` reproduces that
/// bug, so both are decoded here whether or not a painter uses them yet.
#[derive(Debug, Clone, Default)]
pub struct Gradient {
    pub kind: GradientKind,
    pub stops: Vec<Stop>,
    pub alpha_stops: Vec<AlphaStop>,
    pub rotation: f32,
}

/// Decoded pixels, straight (NOT premultiplied) RGBA, row-major.
///
/// THE DISPLAY LIST CARRIES PIXELS RATHER THAN A BACKEND HANDLE, and that is the
/// same argument `painter.rs` makes for the trait: this list has survived four
/// rasterisers, and a `vello` image id or a `tiny_skia::Pixmap` in this struct
/// would end that the moment a fifth arrived. A painter uploads what it is given
/// into whatever form it needs, once, keyed by [`Bitmap::id`].
///
/// STRAIGHT RATHER THAN PREMULTIPLIED, because that is what a PNG decoder hands
/// back and what a second decoder would hand back too. Premultiplying here would
/// put a lossy step between "the host resolved an asset" and "a painter drew it",
/// and every painter would then have to know whether it had already happened.
#[derive(Debug)]
pub struct Bitmap {
    /// Unique for the life of the process, so a painter can memoise its upload
    /// without keying on an address that a freed allocation could reuse.
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Bitmap {
    /// `rgba` must be `width * height * 4` bytes; anything else is refused rather
    /// than truncated, because a short buffer drawn anyway is a torn image and a
    /// long one is a silent misread of the caller's stride.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Option<Bitmap> {
        if width == 0 || height == 0 {
            return None;
        }
        if rgba.len() != width as usize * height as usize * 4 {
            return None;
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Some(Bitmap {
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            width,
            height,
            rgba,
        })
    }
}

/// How the source pixels are fitted into the node's rectangle.
///
/// THREE OF ROBLOX'S FIVE `Enum.ScaleType` MEMBERS, and the other two are named
/// in [`Image`]'s documentation rather than quietly mapped onto `Stretch`. Each
/// of these is expressible as a source rectangle and a destination rectangle,
/// which is the whole of what a painter is asked for; `Slice` needs nine draws
/// and `Tile` needs a repeat, and neither is that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scale {
    /// Fill the node, ignoring the source's aspect ratio. The engine's default.
    #[default]
    Stretch,
    /// Fit inside the node, letterboxed, keeping the aspect ratio.
    Fit,
    /// Fill the node, keeping the aspect ratio, cropping the overflow. Expressed
    /// as a smaller SOURCE rectangle rather than as an oversized destination and
    /// a clip, so it needs nothing of the painter that `Stretch` does not.
    Crop,
}

/// The image a node draws.
///
/// WHAT IS HONOURED, AND WHAT IS NOT. `ImageColor3`, `ImageTransparency`,
/// `ImageRectOffset`/`ImageRectSize` and three of five `ScaleType` members reach
/// a painter through this struct. `Enum.ScaleType.Slice` (with `SliceCenter` and
/// `SliceScale`) and `Enum.ScaleType.Tile` (with `TileSize`) do NOT: they are
/// resolved to [`Scale::Stretch`] by whoever built this node, and that host is
/// expected to say so by name rather than let the difference pass as a rendering
/// quirk. See `dew_host::datamodel::render`, which does.
///
/// `bitmap` IS AN OPTION AND THAT IS THE INTERESTING CASE. ADR-003: "an
/// unresolvable `Content` is a rendering outcome, not a property error." A node
/// that names an asset the host could not produce pixels for still reaches the
/// painter, still knows what it asked for, and is DRAWN — see
/// [`crate::painter::Painter::draw_image`]. A node whose image property is empty
/// carries no `Image` at all and is a different thing entirely.
#[derive(Debug, Clone)]
pub struct Image {
    /// The resolved pixels, or `None` when the host could not produce them.
    pub bitmap: Option<std::sync::Arc<Bitmap>>,
    /// What the node asked for, kept whether or not it resolved, so a painter or
    /// a log can name it.
    pub uri: String,
    /// `ImageColor3`, multiplied through the source pixels. `None` is untinted,
    /// which is not the same as white on a backend that tints by alpha mask.
    pub tint: Option<Rgb>,
    /// Already inverted from `ImageTransparency`: 1 is opaque.
    pub alpha: f32,
    /// The sub-rectangle of the source to sample, in source pixels
    /// (`ImageRectOffset` and `ImageRectSize`). `None` is the whole image.
    pub source: Option<Rect>,
    pub scale: Scale,
}

impl Image {
    /// The source and destination rectangles a painter draws between.
    ///
    /// HERE RATHER THAN IN EACH PAINTER, for the reason `paint_node` gives about
    /// paint order: three backends independently deciding what `Fit` means is
    /// three chances to disagree, and the disagreement shows up on one backend
    /// only. A painter is handed two rectangles and maps one onto the other.
    ///
    /// Returns `None` when there is nothing to draw — no pixels, or a degenerate
    /// rectangle on either side.
    pub fn placement(&self, node: Rect) -> Option<(Rect, Rect)> {
        let bitmap = self.bitmap.as_ref()?;
        if node.w <= 0.0 || node.h <= 0.0 {
            return None;
        }
        let whole = Rect {
            x: 0.0,
            y: 0.0,
            w: bitmap.width as f32,
            h: bitmap.height as f32,
        };
        // THE SOURCE RECTANGLE IS CLAMPED TO THE IMAGE, not trusted. A guest sets
        // `ImageRectSize` freely and the engine samples nothing outside the asset; a
        // painter handed a rectangle that runs off the edge would either read out
        // of bounds or stretch an edge pixel across the overflow, and both are
        // wrong in a way that looks like a bad asset.
        let mut src = match self.source {
            None => whole,
            Some(r) => {
                let x = r.x.clamp(0.0, whole.w);
                let y = r.y.clamp(0.0, whole.h);
                Rect {
                    x,
                    y,
                    w: r.w.clamp(0.0, whole.w - x),
                    h: r.h.clamp(0.0, whole.h - y),
                }
            }
        };
        if src.w <= 0.0 || src.h <= 0.0 {
            return None;
        }

        let dst = match self.scale {
            Scale::Stretch => node,
            Scale::Fit => {
                let k = (node.w / src.w).min(node.h / src.h);
                let (w, h) = (src.w * k, src.h * k);
                Rect {
                    x: node.x + (node.w - w) / 2.0,
                    y: node.y + (node.h - h) / 2.0,
                    w,
                    h,
                }
            }
            Scale::Crop => {
                // Shrink the SOURCE to the destination's aspect ratio, centred.
                // The destination is untouched, so a cropped image needs no clip
                // and no painter has to know that this member exists.
                let k = (src.w / node.w).min(src.h / node.h);
                let (w, h) = (node.w * k, node.h * k);
                src = Rect {
                    x: src.x + (src.w - w) / 2.0,
                    y: src.y + (src.h - h) / 2.0,
                    w,
                    h,
                };
                node
            }
        };
        if dst.w <= 0.0 || dst.h <= 0.0 {
            return None;
        }
        Some((src, dst))
    }
}

/// Text alignment on one axis.
///
/// THE DEFAULT IS THE HALF THAT MATTERS. An unset `TextXAlignment` is CENTRE in
/// The engine, not left. Live.luau supplies that default once, at the source, because
/// three separate painters had each invented their own left inset and every icon
/// in the shop kit sat in the wrong place. Nothing downstream re-decides it, and
/// nothing here should either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
}

impl Align {
    fn parse(s: Option<String>) -> Option<Self> {
        Some(match s?.as_str() {
            "left" | "top" | "start" => Align::Start,
            "right" | "bottom" | "end" => Align::End,
            _ => Align::Center,
        })
    }
}

/// One node of the display list, in paint order.
#[derive(Debug, Clone)]
pub struct Node {
    /// Stable across frames, so a display can patch one node rather than rebuild
    /// the screen. This is what makes [`Delta`] worth having.
    pub id: u64,
    pub name: String,
    pub rect: Rect,
    pub fill: Option<Rgb>,
    pub alpha: f32,
    pub radius: f32,
    pub clip: Option<Rect>,
    /// The corner radius of that clip, 0.0 for a square one.
    ///
    /// BESIDE `clip` RATHER THAN INSIDE IT. An enum would be tidier in Rust and
    /// would change the shape `Live.luau` emits, which lives in the other
    /// repository; an added optional key is a change Aether does not have to
    /// make on the same day. The observable outcome is identical.
    pub clip_radius: f32,
    pub stroke: Option<Stroke>,
    pub gradient: Option<Gradient>,
    pub text: Option<String>,
    pub text_size: f32,
    pub text_align_x: Option<Align>,
    pub text_align_y: Option<Align>,
    pub text_colour: Option<Rgb>,
    /// How opaque the text is, 1.0 for solid.
    ///
    /// SEPARATE FROM `alpha`, which is the node's BACKGROUND. The engine has
    /// `BackgroundTransparency` and `TextTransparency` as independent
    /// properties, and a label with an invisible background and solid text is
    /// the ordinary case rather than an exotic one.
    ///
    /// The host accepted `TextTransparency` and had nowhere to put it, so the
    /// painter filled every run at 1.0 and the property reached no pixels. Found
    /// by the gallery's differential pass, not by a test.
    pub text_alpha: f32,
    /// What this node draws as an image, if anything.
    ///
    /// NOT DECODED FROM LUA, and it is the first field of which that is true —
    /// see [`Frame::HOST_FIELDS`]. Pixels do not travel through a display-list
    /// table, so a host that resolves an asset itself builds this directly.
    pub image: Option<Image>,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub width: f32,
    pub height: f32,
    pub nodes: Vec<Node>,
    /// Whether a text field holds focus. A display needs this to decide whether
    /// to swallow a keystroke's own default; it cannot wait to be told whether
    /// the guest consumed the key, because that answer is a round trip away and
    /// by then the surface has already acted on it.
    pub focused: bool,
}

impl Frame {
    /// Every key `buildNode` emits.
    ///
    /// Not decoration: `tests/frame_contract.rs` asserts a real snapshot carries
    /// no key outside this list, so a field added in Live.luau and forgotten here
    /// fails a test instead of being silently dropped on the way to the painter.
    /// The alternative is the failure mode this whole module is written against —
    /// a display that quietly paints less than the engine does.
    ///
    /// THE CONTRACT HAS TWO HALVES SINCE `image` ARRIVED, and pretending
    /// otherwise is what would have made this list decorative. `Node` is no
    /// longer only "what Live.luau emits, decoded": Dew builds `Node`s from its
    /// own DataModel, and a resolved bitmap has no representation in a Lua table
    /// worth having. So the fields split by whether they cross that boundary —
    /// this list, and [`Frame::HOST_FIELDS`] — and
    /// `every_node_field_is_named_in_one_contract_or_the_other` destructures a
    /// `Node` exhaustively so a field added to neither list does not compile.
    pub const CONTRACT_FIELDS: &'static [&'static str] = &[
        "id",
        "name",
        "x",
        "y",
        "w",
        "h",
        "fill",
        "alpha",
        "radius",
        "clip",
        "clipRadius",
        "stroke",
        "gradient",
        "text",
        "textSize",
        "textAlignX",
        "textAlignY",
        "textColour",
        "textAlpha",
    ];

    /// Node fields a host fills in directly, which no display-list table carries.
    ///
    /// `image` HOLDS DECODED PIXELS. Serialising those through a Lua table would
    /// mean a megabyte of numbers crossing the VM boundary every frame to say
    /// something the host already knew — so a host that resolves an asset builds
    /// the node itself, and a framework that emits a display list from Luau
    /// cannot produce one. That is a real asymmetry rather than an oversight, and
    /// naming it here is what stops the next reader from "fixing" the decoder to
    /// match a list it was never meant to satisfy.
    pub const HOST_FIELDS: &'static [&'static str] = &["image"];
}

/// What changed since the last delta.
///
/// `order` and `dirty` are absent when nothing moved, which is the property the
/// type exists for: an idle screen produces no traffic.
#[derive(Debug, Clone)]
pub struct Delta {
    pub width: f32,
    pub height: f32,
    pub focused: bool,
    pub changed: Vec<Node>,
    pub removed: Vec<u64>,
    pub order: Option<Vec<u64>>,
    pub dirty: Option<Rect>,
    /// The frame this delta was computed from, handed back because computing the
    /// delta already built it. Asking for both separately walks every node twice.
    pub frame: Frame,
}

fn rect_from_array(t: Option<LuaTable>) -> Option<Rect> {
    let t = t?;
    Some(Rect {
        x: t.get(1).ok()?,
        y: t.get(2).ok()?,
        w: t.get(3).ok()?,
        h: t.get(4).ok()?,
    })
}

fn stops_from(t: Option<LuaTable>) -> Vec<Stop> {
    let Some(t) = t else {
        return Vec::new();
    };
    t.sequence_values::<LuaTable>()
        .flatten()
        .filter_map(|s| {
            Some(Stop {
                at: s.get("at").or_else(|_| s.get(1)).ok()?,
                colour: Rgb::from_table(s.get("colour").or_else(|_| s.get(2)).ok())?,
            })
        })
        .collect()
}

fn alpha_stops_from(t: Option<LuaTable>) -> Vec<AlphaStop> {
    let Some(t) = t else {
        return Vec::new();
    };
    t.sequence_values::<LuaTable>()
        .flatten()
        .filter_map(|s| {
            Some(AlphaStop {
                at: s.get("at").or_else(|_| s.get(1)).ok()?,
                alpha: s.get("alpha").or_else(|_| s.get(2)).ok()?,
            })
        })
        .collect()
}

impl Node {
    pub fn from_lua(t: &LuaTable) -> LuaResult<Self> {
        let stroke = t.get::<Option<LuaTable>>("stroke")?.map(|s| Stroke {
            colour: Rgb::from_table(s.get("colour").ok()),
            thickness: s.get("thickness").unwrap_or(1.0),
            alpha: s.get("alpha").unwrap_or(1.0),
        });

        let gradient = t.get::<Option<LuaTable>>("gradient")?.map(|g| {
            let kind = match g.get::<Option<String>>("kind").ok().flatten() {
                Some(s) if s.eq_ignore_ascii_case("radial") => GradientKind::Radial,
                _ => match g.get::<Option<u32>>("kind").ok().flatten() {
                    Some(1) => GradientKind::Radial,
                    _ => GradientKind::Linear,
                },
            };
            Gradient {
                kind,
                stops: stops_from(g.get("stops").ok()),
                alpha_stops: alpha_stops_from(g.get("alphaStops").ok()),
                rotation: g.get("rotation").unwrap_or(0.0),
            }
        });

        Ok(Node {
            id: t.get("id")?,
            name: t.get::<Option<String>>("name")?.unwrap_or_default(),
            rect: Rect {
                x: t.get("x")?,
                y: t.get("y")?,
                w: t.get("w")?,
                h: t.get("h")?,
            },
            fill: Rgb::from_table(t.get("fill")?),
            alpha: t.get::<Option<f32>>("alpha")?.unwrap_or(1.0),
            radius: t.get::<Option<f32>>("radius")?.unwrap_or(0.0),
            clip: rect_from_array(t.get("clip")?),
            clip_radius: t.get::<Option<f32>>("clipRadius")?.unwrap_or(0.0),
            stroke,
            gradient,
            text: t.get("text")?,
            text_size: t.get::<Option<f32>>("textSize")?.unwrap_or(14.0),
            text_align_x: Align::parse(t.get("textAlignX")?),
            text_align_y: Align::parse(t.get("textAlignY")?),
            text_colour: Rgb::from_table(t.get("textColour")?),
            text_alpha: t.get::<Option<f32>>("textAlpha")?.unwrap_or(1.0),
            // NOT READ FROM THE TABLE, and `Frame::HOST_FIELDS` says why. A
            // display list built in Luau names an asset; it cannot carry one.
            image: None,
        })
    }
}

impl Frame {
    pub fn from_lua(t: &LuaTable) -> LuaResult<Self> {
        let nodes_tbl: LuaTable = t.get("Nodes")?;
        let mut nodes = Vec::with_capacity(nodes_tbl.raw_len());
        for n in nodes_tbl.sequence_values::<LuaTable>() {
            nodes.push(Node::from_lua(&n?)?);
        }
        Ok(Frame {
            width: t.get("Width")?,
            height: t.get("Height")?,
            nodes,
            focused: t.get::<Option<bool>>("Focused")?.unwrap_or(false),
        })
    }
}

impl Delta {
    pub fn from_lua(t: &LuaTable) -> LuaResult<Self> {
        let changed_tbl: LuaTable = t.get("Changed")?;
        let mut changed = Vec::with_capacity(changed_tbl.raw_len());
        for n in changed_tbl.sequence_values::<LuaTable>() {
            changed.push(Node::from_lua(&n?)?);
        }

        let removed = t
            .get::<LuaTable>("Removed")?
            .sequence_values::<u64>()
            .flatten()
            .collect();

        let order = t
            .get::<Option<LuaTable>>("Order")?
            .map(|o| o.sequence_values::<u64>().flatten().collect());

        Ok(Delta {
            width: t.get("Width")?,
            height: t.get("Height")?,
            focused: t.get::<Option<bool>>("Focused")?.unwrap_or(false),
            changed,
            removed,
            order,
            dirty: rect_from_array(t.get("Dirty")?),
            frame: Frame::from_lua(&t.get::<LuaTable>("Frame")?)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn bitmap(w: u32, h: u32) -> Arc<Bitmap> {
        Arc::new(Bitmap::new(w, h, vec![255u8; (w * h * 4) as usize]).expect("bitmap"))
    }

    fn image(scale: Scale, source: Option<Rect>, bitmap: Option<Arc<Bitmap>>) -> Image {
        Image {
            bitmap,
            uri: "mod://probe.png".into(),
            tint: None,
            alpha: 1.0,
            source,
            scale,
        }
    }

    /// THE ONE THAT FAILS TO COMPILE, WHICH IS THE POINT. `CONTRACT_FIELDS` was
    /// documented as being checked and was referenced by nothing at all — a list
    /// that keeps two things in step by being read, and nothing read it. Adding
    /// `image` to `Node` is exactly the change it existed to catch, so it is
    /// wired up in the commit that makes it necessary.
    ///
    /// The destructure is the mechanism: a field added to `Node` and named in
    /// neither list is a missing-pattern error here, before any assertion runs.
    #[test]
    fn every_node_field_is_named_in_one_contract_or_the_other() {
        let node = Node {
            id: 1,
            name: String::new(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            fill: None,
            alpha: 1.0,
            radius: 0.0,
            clip: None,
            clip_radius: 0.0,
            stroke: None,
            gradient: None,
            text: None,
            text_size: 14.0,
            text_align_x: None,
            text_align_y: None,
            text_colour: None,
            text_alpha: 1.0,
            image: None,
        };
        let Node {
            id: _,
            name: _,
            rect: _,
            fill: _,
            alpha: _,
            radius: _,
            clip: _,
            clip_radius: _,
            stroke: _,
            gradient: _,
            text: _,
            text_size: _,
            text_align_x: _,
            text_align_y: _,
            text_colour: _,
            text_alpha: _,
            image: _,
        } = node;

        // `rect` is four keys and the rest are one each; the names are Live.luau's
        // rather than Rust's, which is what the list is a contract with.
        let named: &[&str] = &[
            "id",
            "name",
            "x",
            "y",
            "w",
            "h",
            "fill",
            "alpha",
            "radius",
            "clip",
            "clipRadius",
            "stroke",
            "gradient",
            "text",
            "textSize",
            "textAlignX",
            "textAlignY",
            "textColour",
            "textAlpha",
            "image",
        ];
        for key in named {
            assert!(
                Frame::CONTRACT_FIELDS.contains(key) || Frame::HOST_FIELDS.contains(key),
                "`{key}` is a Node field named in neither contract list"
            );
        }
        assert_eq!(
            Frame::CONTRACT_FIELDS.len() + Frame::HOST_FIELDS.len(),
            named.len(),
            "a contract list holds a name that is not a Node field"
        );
    }

    #[test]
    fn stretch_fills_the_node_and_samples_the_whole_image() {
        let img = image(Scale::Stretch, None, Some(bitmap(20, 10)));
        let node = Rect {
            x: 5.0,
            y: 5.0,
            w: 100.0,
            h: 100.0,
        };
        let (src, dst) = img.placement(node).expect("placed");
        assert_eq!((src.x, src.y, src.w, src.h), (0.0, 0.0, 20.0, 10.0));
        assert_eq!((dst.x, dst.y, dst.w, dst.h), (5.0, 5.0, 100.0, 100.0));
    }

    #[test]
    fn fit_letterboxes_inside_the_node_and_keeps_the_aspect_ratio() {
        let img = image(Scale::Fit, None, Some(bitmap(20, 10)));
        let node = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let (_, dst) = img.placement(node).expect("placed");
        // 2:1 into a square: full width, half height, centred vertically.
        assert_eq!((dst.w, dst.h), (100.0, 50.0));
        assert_eq!((dst.x, dst.y), (0.0, 25.0));
    }

    #[test]
    fn crop_shrinks_the_source_rather_than_overflowing_the_destination() {
        // The destination is untouched, which is what lets Crop need no clip.
        let img = image(Scale::Crop, None, Some(bitmap(20, 10)));
        let node = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let (src, dst) = img.placement(node).expect("placed");
        assert_eq!((dst.w, dst.h), (100.0, 100.0));
        // A square window on a 20x10 source is 10x10, centred horizontally.
        assert_eq!((src.w, src.h), (10.0, 10.0));
        assert_eq!((src.x, src.y), (5.0, 0.0));
    }

    #[test]
    fn a_source_rectangle_past_the_edge_is_clamped_rather_than_sampled() {
        let img = image(
            Scale::Stretch,
            Some(Rect {
                x: 16.0,
                y: 0.0,
                w: 64.0,
                h: 64.0,
            }),
            Some(bitmap(20, 10)),
        );
        let node = Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        };
        let (src, _) = img.placement(node).expect("placed");
        assert_eq!((src.x, src.y, src.w, src.h), (16.0, 0.0, 4.0, 10.0));
    }

    #[test]
    fn an_unresolved_image_has_no_placement_but_is_still_a_node() {
        // The painter draws something for this; `placement` only answers where
        // pixels would go, and there are none.
        let img = image(Scale::Stretch, None, None);
        assert!(img
            .placement(Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            })
            .is_none());
        assert_eq!(img.uri, "mod://probe.png");
    }

    #[test]
    fn a_bitmap_refuses_a_buffer_that_is_not_its_own_size() {
        assert!(Bitmap::new(2, 2, vec![0; 16]).is_some());
        assert!(Bitmap::new(2, 2, vec![0; 15]).is_none());
        assert!(Bitmap::new(2, 2, vec![0; 17]).is_none());
        assert!(Bitmap::new(0, 4, vec![]).is_none());
    }

    #[test]
    fn bitmap_ids_are_distinct_so_a_painter_can_memoise_on_them() {
        let a = Bitmap::new(1, 1, vec![0; 4]).expect("a");
        let b = Bitmap::new(1, 1, vec![0; 4]).expect("b");
        assert_ne!(a.id, b.id);
    }
}
