//! The shared example, driven and drawn.
//!
//! IT LIVES IN `host/` BECAUSE IT NEEDS A DATAMODEL. It sat in
//! `crates/runtime/tests` while a runtime alone could host Aether, and stopped
//! being able to in milestone 2's sprint 8, when `Host.detect()` became a
//! capability probe: it builds an `Instance`, round-trips a `UDim2` and reads a
//! class back. `dew_runtime` installs none of that on purpose -- the guest
//! reaches the outside world only through capability tables THE HOST installs by
//! name -- so the test was asserting an arrangement the project had removed, and
//! passing only because the pesde pin predated the change.
//!
//! Moving it here is not a boundary being bent. It is a test being put where its
//! dependencies live: `dew_host` is what grants the DataModel, exactly as
//! `mods.rs` grants it to every mod.
//!
//! `examples/counter/src/Counter.luau` is loaded here through its DESKTOP entry
//! point. The same component file is mounted by `entry/roblox.client.luau` inside
//! a place. Nothing in the component differs between the two, and this suite is
//! what keeps that true — if the shared file grows a dependency on either host,
//! it stops loading here.
//!
//! It also exercises `Driver`, which is the loop both native shells run, rather
//! than calling `Session` directly the way `parity.rs` does. Two things are
//! therefore under test at once, deliberately: the example, and the code path a
//! real shell will take to run it.

// NO `#![cfg(feature = "raster")]`. That gate belonged to `dew_runtime`, where
// the rasteriser is optional so a consumer wanting only the display list does not
// compile one. `dew_host` depends on `dew_runtime` WITH that feature and defines
// no feature of its own, so the attribute would be false here and this file would
// compile to nothing -- reporting `0 passed`, which is the sentence a passing
// suite prints.

use dew_raster::{Backend, Font};
use dew_runtime::{Application, Capabilities, Driver, Rgb};
use mlua::Function;
use std::path::PathBuf;

mod common;

/// Aether's checkout, and it is no longer a directory above this crate.
///
/// This read `CARGO_MANIFEST_DIR/../..` while the crate lived in Aether's
/// repository, where that WAS the framework. ADR-004 moved the crate to Dew and
/// `../..` is now Dew's own root, so the framework is found where every other
/// guest package is found: the pesde install, pinned by commit in `pesde.toml`.
// THE FIXTURES STAYED IN `crates/runtime/tests`, with `render.rs`, which has not
// moved. See this suite's header: `render.rs` asserts on PIXELS and installing a
// DataModel changes which Aether host `detect()` selects, so moving it is not a
// relocation but a behaviour change. It is filed rather than forced.
fn aether_root() -> PathBuf {
    dew_runtime::installed_package("aether")
        .expect("no installed aether — run `pesde install` at the repository root")
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
    match dew_runtime::installed_package("vide") {
        Some(vide) => {
            caps.aliases.insert("vide".to_string(), vide.join("src"));
        }
        None => panic!("no installed vide — run `pesde install` at the repository root"),
    }
    caps
}

fn entry() -> PathBuf {
    aether_root().join("examples/counter/entry/desktop.luau")
}

fn app() -> Application {
    Application::load_with(caps(), &entry(), common::install_host)
        .expect("the shared example should load through its desktop entry")
}

fn driver() -> Driver<dew_runtime::RasterPainter> {
    let app = app();
    let session = app.session().expect("session");

    let mut painter = dew_runtime::RasterPainter::new(240, 96, Backend::VelloCpu).expect("surface");
    if let Some(path) = dew_runtime::font::system_font() {
        if let Some(font) = Font::load(&path.to_string_lossy(), 0) {
            painter = painter.with_font(font);
        }
    }

    // The application is kept alive by the Driver holding its Session, which
    // holds the Lua handles. Leaking it here is deliberate and local to the test:
    // dropping the Application would drop the VM out from under the session.
    std::mem::forget(app);

    Driver::new(session, painter, Some(Rgb(0, 0, 0)))
}

#[test]
fn the_shared_component_loads_off_engine() {
    let _ = driver();
}

/// The first frame must be FULL — the surface holds nothing a delta could patch.
#[test]
fn the_first_frame_paints_everything() {
    let mut driver = driver();
    let painted = driver.frame(1.0 / 120.0).expect("frame");
    assert!(
        painted,
        "the first frame must paint; there is nothing to patch"
    );
}

/// And the second must not, because nothing moved.
///
/// This is the property that lets a desktop host idle instead of burning a core
/// repainting an unchanged screen, and it is the reason `Driver::frame` answers
/// with a bool rather than nothing.
#[test]
fn an_unchanged_second_frame_paints_nothing() {
    let mut driver = driver();
    driver.frame(1.0 / 120.0).expect("first");
    let painted = driver.frame(1.0 / 120.0).expect("second");
    assert!(
        !painted,
        "nothing changed, so nothing should have been painted"
    );
}

/// Writes the example out so it can be compared against the same component
/// running in Studio.
#[test]
fn writes_the_example_png() {
    let mut driver = driver();
    driver.frame(1.0 / 120.0).expect("frame");

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/counter.png");
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let path = out.to_string_lossy().to_string();
    driver.painter_mut().write_png(&path).expect("png");

    let size = std::fs::metadata(&path).expect("exists").len();
    assert!(size > 0);
    println!("wrote {path} ({size} bytes)");
}

/// THE REACTIVE GRAPH DRIVES THE DISPLAY LIST, off-engine, with no engine to
/// write properties back.
///
/// The component gives `Text` a FUNCTION of a `source`. On Roblox vide reacts by
/// assigning the property and the engine repaints. Here nothing assigns anything
/// — the same graph simply produces a different `Live.Frame` on the next solve.
/// A static tree would pass every other test in this file and fail this one, so
/// this is the assertion that the two hosts share behaviour rather than merely
/// sharing a file.
#[test]
fn incrementing_changes_what_is_rendered() {
    let app = app();
    let session = app.session().expect("session");
    let increment: Function = app.get("Increment").expect("the entry exposes Increment");

    session.step(1.0 / 120.0).expect("step");
    let before = session.snapshot().expect("snapshot");
    // BY CONTENT, not by position. The first text node is the title; picking
    // `nodes[0]` would assert against "aether counter" and pass or fail for
    // reasons unrelated to the counter.
    let text_before = before
        .nodes
        .iter()
        .filter_map(|n| n.text.clone())
        .find(|t| t.contains("count:"))
        .expect("the value label should carry the count");
    assert!(
        text_before.contains("count: 0"),
        "expected the initial count, got {text_before:?}"
    );

    increment.call::<()>(()).expect("increment");
    session.step(1.0 / 120.0).expect("step");

    let after = session.snapshot().expect("snapshot");
    let texts: Vec<String> = after.nodes.iter().filter_map(|n| n.text.clone()).collect();
    assert!(
        texts.iter().any(|t| t.contains("count: 1")),
        "the reactive update never reached the display list; texts were {texts:?}"
    );
}

/// A small change must produce a small dirty rectangle, and a cheaper paint.
///
/// This is the property the damage-clipped repaint rests on. Without it
/// `paint_delta` clips to a rectangle covering everything and saves nothing,
/// which would look exactly like working code on a small surface and cost 30ms a
/// frame on a desktop-sized one.
#[test]
fn a_small_change_dirties_a_small_rectangle() {
    let app = app();
    let session = app.session().expect("session");
    let increment: mlua::Function = app.get("Increment").expect("Increment");

    session.step(1.0 / 60.0).unwrap();
    let full = session.delta(true).expect("full delta");
    let (fw, fh) = (full.frame.width, full.frame.height);

    // A forced full delta covers the surface, which is what makes it full.
    let covers_all = full.dirty.map(|d| d.w >= fw && d.h >= fh).unwrap_or(true);
    assert!(covers_all, "a forced full delta should dirty everything");

    increment.call::<()>(()).expect("increment");
    session.step(1.0 / 60.0).unwrap();

    let delta = session.delta(false).expect("delta");
    let dirty = delta
        .dirty
        .expect("changing the count should dirty something");

    let changed_area = dirty.w * dirty.h;
    let whole = fw * fh;
    assert!(
        changed_area < whole / 2.0,
        "one label changed but the dirty rect covers {:.0}% of the frame \
         ({}x{} of {fw}x{fh}) — the damage-clipped repaint saves nothing",
        100.0 * changed_area / whole,
        dirty.w,
        dirty.h
    );

    println!(
        "dirty {:.0}x{:.0} of {fw:.0}x{fh:.0} — {:.1}% of the surface",
        dirty.w,
        dirty.h,
        100.0 * changed_area / whole
    );
}
