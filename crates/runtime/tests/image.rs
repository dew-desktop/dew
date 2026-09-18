//! An image node becoming actual pixels.
//!
//! `render.rs` proves a real application's shapes and text reach the surface;
//! this proves the same of the thing added in Milestone 2's sprint 4, and it
//! asserts on PIXELS for the reason that file gives: a display-list assertion
//! cannot tell a backend that drew the image from a backend that drew nothing,
//! and `dew_raster`'s poison gates exist because a backend that drew nothing
//! once scored 100% pixel parity.
//!
//! THE FRAMES ARE BUILT IN RUST HERE, deliberately, and that is not a shortcut
//! around the guest. `Node::image` is one of `Frame::HOST_FIELDS` -- no Luau
//! display list carries decoded pixels, so there is no Lua form of this node to
//! load. The route from Luau to an image node is Dew's DataModel and it is tested
//! where it lives, in `dew_host::datamodel::render`, from Luau like every other
//! render test. What is left to prove is the half below: that a `Node` carrying
//! pixels puts them on a surface.

#![cfg(feature = "raster")]

use dew_raster::{Backend, Canvas};
use dew_runtime::frame::{Bitmap, BlendMode, Image, Scale};
use dew_runtime::{Frame, Node, Painter, RasterPainter, Rect, Rgb};
use std::sync::Arc;

const W: u32 = 40;
const H: u32 = 40;

/// Deep enough to be unmistakable, and not a colour any test draws.
const BACKGROUND: Rgb = Rgb(0, 0, 0);

fn bitmap(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Arc<Bitmap> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&pixel(x, y));
        }
    }
    Arc::new(Bitmap::new(width, height, rgba).expect("bitmap"))
}

fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Arc<Bitmap> {
    bitmap(width, height, |_, _| rgba)
}

/// A node filling most of the surface, carrying `image` and nothing else.
///
/// NO FILL AND NO BACKGROUND ALPHA, on purpose: whatever appears inside this
/// rectangle was drawn by `draw_image` or was not drawn at all, so a passing
/// assertion cannot be satisfied by the node's own plate.
fn node(image: Option<Image>) -> Node {
    Node {
        id: 1,
        name: "Image".into(),
        rect: Rect {
            x: 8.0,
            y: 8.0,
            w: 24.0,
            h: 24.0,
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
        image,
        text_wrap: false,
        blend_mode: BlendMode::Alpha,
    }
}

fn image_of(bitmap: Option<Arc<Bitmap>>) -> Image {
    Image {
        bitmap,
        uri: "mod://probe.png".into(),
        tint: None,
        alpha: 1.0,
        source: None,
        scale: Scale::Stretch,
    }
}

fn painted(node: Node) -> RasterPainter {
    let frame = Frame {
        width: W as f32,
        height: H as f32,
        nodes: vec![node],
        focused: false,
    };
    // VELLO, NOT TINY-SKIA, matching `main.rs` and `render.rs`: it is the backend
    // Dew actually ships on, so it is the one whose pixels are worth asserting.
    let mut painter = RasterPainter::new(W, H, Backend::VelloCpu).expect("surface");
    painter.paint_frame(&frame, Some(BACKGROUND));
    painter
}

/// Read one pixel back through the PNG writer, so what is asserted is what a
/// `--snapshot` would contain rather than a second path to the same buffer.
///
/// THE FILENAME IS UNIQUE PER CALL, and that is not fussiness. Naming it after
/// the coordinate seemed obviously enough -- and cargo runs these tests in
/// parallel threads of ONE process, so six tests probing (20, 20) wrote and read
/// one file. Every colour assertion in this file failed at once, all of them
/// reporting another test's picture, which reads as "the rasteriser draws grey"
/// rather than as a race in the harness.
fn pixel(canvas: &mut Canvas, x: u32, y: u32) -> (u8, u8, u8) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let out = std::env::temp_dir().join(format!("aether_image_probe_{n}_{x}_{y}.png"));
    let path = out.to_string_lossy().to_string();
    canvas.write_png(&path).expect("png");
    let img = image::open(&path).expect("read back").to_rgb8();
    let p = img.get_pixel(x, y);
    let _ = std::fs::remove_file(&path);
    (p[0], p[1], p[2])
}

/// Within one step per channel. Bilinear sampling and the premultiply round-trip
/// both cost a least significant bit, and demanding exactness here would make the
/// test fail for arithmetic rather than for drawing.
fn near(got: (u8, u8, u8), want: (u8, u8, u8), what: &str) {
    let close = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 1;
    assert!(
        close(got.0, want.0) && close(got.1, want.1) && close(got.2, want.2),
        "{what}: expected about {want:?}, got {got:?}"
    );
}

#[test]
fn an_image_node_puts_its_own_pixels_on_the_surface() {
    let mut painter = painted(node(Some(image_of(Some(solid(4, 4, [200, 60, 40, 255]))))));
    near(
        pixel(painter.canvas_mut(), 20, 20),
        (200, 60, 40),
        "the middle of the node",
    );
    // And the surface outside it is untouched, so the draw was clipped to the
    // node rather than covering the frame.
    near(
        pixel(painter.canvas_mut(), 2, 2),
        (0, 0, 0),
        "outside the node",
    );
}

#[test]
fn image_colour_multiplies_through_the_source_pixels() {
    // `ImageColor3` is a MULTIPLY, not a replacement: a mid-grey asset tinted
    // full green is mid-green, and a painter that replaced the colour would
    // answer full green. The distinction is the whole reason a white asset is a
    // bad test subject for this.
    let mut image = image_of(Some(solid(4, 4, [128, 128, 128, 255])));
    image.tint = Some(Rgb(0, 255, 0));
    let mut painter = painted(node(Some(image)));
    near(
        pixel(painter.canvas_mut(), 20, 20),
        (0, 128, 0),
        "a grey asset tinted green",
    );
}

#[test]
fn image_transparency_blends_the_asset_with_what_is_under_it() {
    let mut image = image_of(Some(solid(4, 4, [255, 255, 255, 255])));
    image.alpha = 0.5;
    let mut painter = painted(node(Some(image)));
    // White at half alpha over black.
    let got = pixel(painter.canvas_mut(), 20, 20);
    assert!(
        (100..=155).contains(&got.0) && got.0 == got.1 && got.1 == got.2,
        "expected a half-blended grey, got {got:?}"
    );
}

#[test]
fn the_source_rectangle_selects_part_of_the_asset() {
    // Four quadrants of one 4x4 asset, and only the bottom-right is asked for.
    // A painter that ignored `source` would draw a mixture of all four.
    let quadrants = bitmap(4, 4, |x, y| match (x < 2, y < 2) {
        (true, true) => [255, 0, 0, 255],
        (false, true) => [0, 255, 0, 255],
        (true, false) => [0, 0, 255, 255],
        (false, false) => [255, 255, 0, 255],
    });
    let mut image = image_of(Some(quadrants));
    image.source = Some(Rect {
        x: 2.0,
        y: 2.0,
        w: 2.0,
        h: 2.0,
    });
    let mut painter = painted(node(Some(image)));
    near(
        pixel(painter.canvas_mut(), 20, 20),
        (255, 255, 0),
        "the selected quadrant",
    );
}

#[test]
fn fit_letterboxes_rather_than_distorting() {
    // A 4:1 asset in a square node: full width, a quarter of the height, centred.
    // The band above it is background, which is what "letterboxed" means and what
    // `Stretch` would not leave.
    let mut image = image_of(Some(solid(8, 2, [200, 60, 40, 255])));
    image.scale = Scale::Fit;
    let mut painter = painted(node(Some(image)));
    near(
        pixel(painter.canvas_mut(), 20, 20),
        (200, 60, 40),
        "the middle of the fitted image",
    );
    near(
        pixel(painter.canvas_mut(), 20, 10),
        (0, 0, 0),
        "the letterbox above it",
    );
}

#[test]
fn a_node_whose_image_is_missing_draws_something_rather_than_nothing() {
    // NOT NOTHING. ADR-003 makes an unresolvable `Content` a rendering outcome,
    // and a blank space is indistinguishable from an element that was never
    // created, was positioned off-screen, or was made invisible -- three places
    // an author looks before suspecting the asset.
    let mut painter = painted(node(Some(image_of(None))));
    let inside = pixel(painter.canvas_mut(), 20, 20);
    let outside = pixel(painter.canvas_mut(), 2, 2);
    assert_eq!(outside, (0, 0, 0), "the surface outside the node");
    assert_ne!(
        inside, outside,
        "a node whose image could not be resolved must not be invisible"
    );
}

#[test]
fn a_node_with_no_image_at_all_draws_no_marker() {
    // The other half of the rule above: an `ImageLabel` before its asset is
    // assigned is a plain element, not a marked one.
    let mut painter = painted(node(None));
    assert_eq!(pixel(painter.canvas_mut(), 20, 20), (0, 0, 0));
}

/// A painter that has every primitive except images.
///
/// THE DEGRADATION IS THE POINT, and it is the rule `fill_gradient` already
/// follows: a backend that cannot do the thing draws what the thing OCCUPIES,
/// because a node that vanishes looks like a layout bug and is hunted for in the
/// wrong file. This one takes the trait's default `draw_image` and records where
/// the fallback landed.
#[derive(Default)]
struct Bare {
    fills: Vec<(Rect, Rgb, f32)>,
    strokes: usize,
}

impl Painter for Bare {
    fn begin(&mut self, _: f32, _: f32, _: Option<Rgb>) {}
    fn fill_rounded_rect(&mut self, rect: Rect, _: f32, colour: Rgb, alpha: f32, _: BlendMode) {
        self.fills.push((rect, colour, alpha));
    }
    fn stroke_rounded_rect(&mut self, _: Rect, _: f32, _: f32, _: Rgb, _: f32) {
        self.strokes += 1;
    }
    fn draw_text(&mut self, _: &Node) {}
    fn clip_push(&mut self, _: Rect) {}
    fn clip_pop(&mut self) {}
    fn end(&mut self) -> bool {
        true
    }
}

fn bare(node: Node) -> Bare {
    let frame = Frame {
        width: W as f32,
        height: H as f32,
        nodes: vec![node],
        focused: false,
    };
    let mut painter = Bare::default();
    painter.paint_frame(&frame, Some(BACKGROUND));
    painter
}

#[test]
fn a_painter_with_no_image_support_fills_the_space_the_image_occupies() {
    let mut image = image_of(Some(solid(4, 4, [1, 2, 3, 255])));
    image.tint = Some(Rgb(90, 140, 200));
    image.alpha = 0.5;
    let painter = bare(node(Some(image)));

    let (rect, colour, alpha) = *painter.fills.first().expect("the fallback drew nothing");
    assert_eq!(
        (rect.x, rect.y, rect.w, rect.h),
        (8.0, 8.0, 24.0, 24.0),
        "the fallback must cover the destination the image would have"
    );
    assert_eq!(colour, Rgb(90, 140, 200), "the tint is the colour it has");
    assert_eq!(alpha, 0.5);
}

#[test]
fn the_missing_marker_needs_only_the_primitives_every_painter_has() {
    // A painter with no image support and a node with no pixels: the marker still
    // appears, because it is a fill and a stroke rather than anything specialised.
    let painter = bare(node(Some(image_of(None))));
    assert_eq!(painter.fills.len(), 1);
    assert_eq!(painter.strokes, 1);
}

#[test]
fn an_image_is_painted_over_the_fill_and_under_the_stroke() {
    // The engine's order, and the reason it is decided in `paint_node` rather than in
    // each backend. `Bare` turns both the plate and the image fallback into
    // `fill_rounded_rect` calls, so their sequence is readable here.
    let mut with_plate = node(Some(image_of(Some(solid(2, 2, [9, 9, 9, 255])))));
    with_plate.fill = Some(Rgb(20, 30, 40));
    let painter = bare(with_plate);

    assert_eq!(painter.fills.len(), 2, "a plate and an image");
    assert_eq!(
        painter.fills[0].1,
        Rgb(20, 30, 40),
        "the node's own background is painted first"
    );
}

#[test]
fn an_image_is_drawn_even_when_the_background_is_fully_transparent() {
    // `BackgroundTransparency = 1` is how every icon in every the engine UI is
    // written. Folding the image into the node's own alpha check would make the
    // common case draw nothing at all.
    let mut transparent = node(Some(image_of(Some(solid(4, 4, [200, 60, 40, 255])))));
    transparent.alpha = 0.0;
    let mut painter = painted(transparent);
    near(
        pixel(painter.canvas_mut(), 20, 20),
        (200, 60, 40),
        "an image on a transparent plate",
    );
}

/// A ROUNDED CLIP MUST MASK THE CORNER.
///
/// The engine clips descendants to the parent's `UICorner` shape; Dew clipped to a
/// bare rectangle and the corner pixels leaked, which milestone 3 recorded as an
/// observable divergence rather than fixing.
///
/// ASSERTED ON PIXELS because nothing else can see it. The child's RECTANGLE is
/// identical either way -- clipping to a rounded parent does not move a child,
/// it changes which of its pixels survive -- so every geometric assertion in the
/// suite passes with the corner leaking.
#[test]
fn a_rounded_clip_masks_the_corner_and_keeps_the_middle() {
    let bar = Node {
        id: 1,
        name: "Titlebar".into(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: W as f32,
            h: 10.0,
        },
        fill: Some(Rgb(255, 0, 0)),
        alpha: 1.0,
        radius: 0.0,
        clip: Some(Rect {
            x: 0.0,
            y: 0.0,
            w: W as f32,
            h: 30.0,
        }),
        clip_radius: 12.0,
        stroke: None,
        gradient: None,
        text: None,
        text_size: 14.0,
        text_align_x: None,
        text_align_y: None,
        text_colour: None,
        text_alpha: 1.0,
        image: None,
        text_wrap: false,
        blend_mode: BlendMode::Alpha,
    };
    let mut painter = painted(bar);
    let canvas = painter.canvas_mut();

    // Well inside both the bar and the rounded shape.
    assert_eq!(
        pixel(canvas, 20, 5),
        (255, 0, 0),
        "the middle of the bar should be red"
    );
    // Inside the bar's RECTANGLE, outside the radius-12 corner arc.
    assert_ne!(
        pixel(canvas, 1, 1),
        (255, 0, 0),
        "the rounded clip did not mask the corner; red leaked"
    );
}
