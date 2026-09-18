//! Painting a real application to real pixels.
//!
//! `parity.rs` proves the display list is correct. This proves something is
//! actually DRAWN from it — which is the part a display-list assertion cannot
//! reach, and the exact gap the poison gates in `dew_raster` exist for: a
//! backend that draws nothing satisfies every "the wrong thing is absent" check
//! ever written.
//!
//! So these assert on PIXELS, at coordinates the fixture's own geometry decides.
//!
//! Run with a PNG to look at:
//!
//! ```sh
//! cargo test --features raster -- --nocapture
//! ```

use dew_raster::{Backend, Canvas, Font};
use dew_runtime::{Application, Capabilities, Painter, RasterPainter, Rgb};
use std::path::PathBuf;

mod common;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 80;

/// Aether's checkout, and it is no longer a directory above this crate.
///
/// This read `CARGO_MANIFEST_DIR/../..` while the crate lived in Aether's
/// repository, where that WAS the framework. ADR-004 moved the crate to Dew and
/// `../..` is now Dew's own root, so the framework is found where every other
/// guest package is found: the pesde install, pinned by commit in `pesde.toml`.
fn aether_root() -> PathBuf {
    {
        //  AN EXAMPLE'S INSTALL. This repository declares no framework of its own.
        let mut found = None;
        for example in ["examples/aether/pressable", "examples/aether/calculator"] {
            for prefix in ["", "../"] {
                let dir = PathBuf::from(format!("{prefix}{example}"));
                if let Some(p) = dew_runtime::installed_package_in(&dir, "aether") {
                    found = Some(p);
                }
            }
        }
        found.expect(
            "no example has aether installed -- run `pesde install` in examples/aether/pressable",
        )
    }
}

/// What the fixtures below are loaded under.
///
/// THE ALIASES ARE NOT OPTIONAL HERE, and they used to be. A `.luaurc` resolves
/// by walking up from the REQUIRING FILE: while the fixtures sat inside Aether,
/// the repository's own `.luaurc` was on that path and `@aether` resolved
/// itself. It is not on the path from `crates/runtime/tests/fixtures`, so the
/// host supplies what it supplies — the same two aliases, through the same seam,
/// that `main.rs` hands every mod.
fn caps() -> Capabilities {
    let root = aether_root();
    let mut caps = Capabilities::cli(root.clone());
    caps.aliases.insert("aether".to_string(), root.join("src"));
    match dew_runtime::installed_package_in(&root, "vide") {
        Some(vide) => {
            caps.aliases.insert("vide".to_string(), vide.join("src"));
        }
        None => panic!("no installed vide — run `pesde install` at the repository root"),
    }
    caps
}

fn load() -> Application {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/app.luau");
    Application::load_with(caps(), &fixture, common::install_host).expect("fixture loads")
}

/// A face to draw with, or `None` where there is not one.
///
/// NOT A HARD REQUIREMENT of the tests below. Text needs a font file that exists
/// on the machine running this, and a suite that fails on a bare CI container has
/// told you about the container rather than about the renderer.
fn system_font() -> Option<Font> {
    // The candidate list moved into `dew_runtime::font`, where both native
    // shells reach it. A test carrying its own copy is a test that can pass while
    // the shells fail on a machine it never tried.
    let path = dew_runtime::font::system_font()?;
    Font::load(&path.to_string_lossy(), 0)
}

fn pixel(canvas: &mut Canvas, x: u32, y: u32) -> (u8, u8, u8) {
    // Round-trip through the PNG writer's own buffer rather than a second path,
    // so what is asserted is what would be written out.
    let out = std::env::temp_dir().join(format!("aether_probe_{x}_{y}.png"));
    let path = out.to_string_lossy().to_string();
    canvas.write_png(&path).expect("png");
    let img = image::open(&path).expect("read back").to_rgb8();
    let p = img.get_pixel(x, y);
    let _ = std::fs::remove_file(&path);
    (p[0], p[1], p[2])
}

fn painted() -> RasterPainter {
    let app = load();
    let session = app.session().expect("session");
    session.step(1.0 / 120.0).expect("step");
    let frame = session.snapshot().expect("snapshot");

    // VELLO, NOT TINY-SKIA, and the difference is not performance. tiny-skia is a
    // SHAPE backend here — `ar_fill_text` returns 0 on one without drawing — so a
    // frame with text renders its rectangles perfectly and silently omits every
    // label. Which is exactly the failure that looks like a layout bug.
    let mut painter = RasterPainter::new(WIDTH, HEIGHT, Backend::VelloCpu).expect("surface");
    if let Some(font) = system_font() {
        painter = painter.with_font(font);
    }
    painter.paint_frame(&frame, Some(Rgb(0, 0, 0)));
    painter
}

/// The root frame's fill must reach the middle of the surface.
///
/// The fixture paints `Color3.new(0.07, 0.09, 0.15)`, which is (18, 23, 38) once
/// Live.luau has rounded it to bytes. Asserting the VALUE and not merely
/// "something non-black" is the point: a backend that cleared to a default would
/// pass the weaker check.
#[test]
fn the_root_fill_reaches_the_surface() {
    let mut painter = painted();
    let (r, g, b) = pixel(painter.canvas_mut(), 6, 6);

    assert_eq!(
        (r, g, b),
        (18, 23, 38),
        "expected the root frame's own fill at a corner inside it"
    );
}

/// The label sits at (20, 28) and is 120x24, so its own fill occupies a band the
/// root does not. If this reads as the root's colour, child nodes are not being
/// painted at all — which every display-list assertion in `parity.rs` would still
/// pass.
#[test]
fn a_child_node_paints_over_its_parent() {
    let mut painter = painted();
    let (r, g, b) = pixel(painter.canvas_mut(), 25, 40);

    assert_eq!(
        (r, g, b),
        (56, 189, 247),
        "expected the label's fill, got the parent's — children are not painting"
    );
}

/// Write a PNG next to the target directory and say where it is.
///
/// Not an assertion about appearance — there is no reference image yet, and one
/// invented here would pin whatever today's rounding happens to produce. It
/// exists so the pipeline can be LOOKED AT, which is how the first wrong-looking
/// frame will be found.
#[test]
fn writes_a_png_to_look_at() {
    let mut painter = painted();
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/aether_frame.png");
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let path = out.to_string_lossy().to_string();

    painter.write_png(&path).expect("write the frame out");

    let size = std::fs::metadata(&path).expect("the file exists").len();
    assert!(size > 0, "a zero-byte PNG is a backend that drew nothing");

    println!("wrote {path} ({size} bytes)");
}

/// Text must reach the surface, not just the display list.
///
/// The label is 120x24 at (20, 28) filled cyan, with "aether" centred in white.
/// Counting near-white pixels inside it is deliberately cruder than comparing a
/// reference image: it survives hinting and antialiasing differences between
/// machines while still failing outright if no glyph is drawn — which is the
/// regression that matters, and the one a shape-only backend produces silently.
#[test]
fn text_is_actually_drawn() {
    if system_font().is_none() {
        eprintln!("no system font on this machine; skipping the glyph check");
        return;
    }

    let mut painter = painted();
    let out = std::env::temp_dir().join("aether_text_probe.png");
    let path = out.to_string_lossy().to_string();
    painter.write_png(&path).expect("png");

    let img = image::open(&path).expect("read back").to_rgb8();
    let _ = std::fs::remove_file(&path);

    // PIXELS THAT ARE NOT THE LABEL'S OWN FILL, rather than near-white ones.
    //
    // This counted near-white and that was an accident of a broken vide. The
    // fixture never sets `TextColor3`; vide's `defaults.luau` does, to pure
    // black, on the engine and here alike. But `defaults` is one of the modules
    // behind vide's `game` gate, so when the gate could not open it failed to
    // require, no default was applied, and the painter's own
    // `unwrap_or(Rgb(255, 255, 255))` filled in. The white text this asserted
    // was the SYMPTOM of vide not loading, not evidence that glyphs were drawn.
    //
    // What the test is for is unchanged and is what the comment above always
    // said: a shape-only backend draws the rectangle and silently omits every
    // label. Counting pixels that differ from the fill catches that whatever
    // colour the text ends up.
    let fill = img.get_pixel(22, 30);
    let mut glyph = 0u32;
    for y in 28..52u32 {
        for x in 20..140u32 {
            let p = img.get_pixel(x, y);
            let d = (p[0] as i32 - fill[0] as i32).abs()
                + (p[1] as i32 - fill[1] as i32).abs()
                + (p[2] as i32 - fill[2] as i32).abs();
            if d > 60 {
                glyph += 1;
            }
        }
    }

    assert!(
        glyph > 20,
        "found {glyph} pixels differing from the label fill {fill:?} -- no glyphs were drawn"
    );
}

/// Does clipping the frame to a small rectangle actually cost less?
///
/// Reported rather than asserted: the ratio depends on the machine and a
/// threshold would be flaky. The number that matters is that it is a ratio at
/// all — before `paint_delta` used the dirty rectangle, both paths did identical
/// work and the ratio was 1.
#[test]
fn a_clipped_repaint_costs_less_than_a_full_one() {
    use std::time::Instant;

    // Desktop-sized, which is where this matters. At widget size the whole
    // surface is small enough that clipping saves little and hides the effect.
    const W: u32 = 2560;
    const H: u32 = 1440;

    let mut painter = RasterPainter::new(W, H, Backend::VelloCpu).expect("surface");

    let rounds = 5;
    let mut full = std::time::Duration::ZERO;
    for _ in 0..rounds {
        let t = Instant::now();
        painter.canvas_mut().begin_alpha(0, 0, 0, 0);
        painter
            .canvas_mut()
            .fill_rect(40.0, 40.0, 300.0, 150.0, 8.0, (56, 189, 248, 255), 0);
        let _ = painter.canvas_mut().bgra();
        full += t.elapsed();
    }

    let mut clipped = std::time::Duration::ZERO;
    for _ in 0..rounds {
        let t = Instant::now();
        painter
            .canvas_mut()
            .begin_rect((0, 0, 0, 0), 40, 40, 310, 160);
        painter
            .canvas_mut()
            .fill_rect(40.0, 40.0, 300.0, 150.0, 8.0, (56, 189, 248, 255), 0);
        let _ = painter.canvas_mut().bgra();
        clipped += t.elapsed();
    }

    let (full, clipped) = (full / rounds, clipped / rounds);
    println!(
        "{W}x{H}: full {full:?} | clipped to 310x160 {clipped:?} | {:.1}x",
        full.as_secs_f64() / clipped.as_secs_f64().max(f64::EPSILON)
    );
}
