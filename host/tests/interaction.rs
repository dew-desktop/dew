//! Driving a real press through a real session.
//!
//! The router's own suites call `PointerRouter.Input` against hand-registered
//! records, which is a genuine unit test of arbitration and skips everything
//! between a TREE and a hit: mounting, layout, bounds being written back, and
//! the router finding a pressable it was never told about directly.
//!
//! A pressable that renders and does not respond passes every one of those
//! suites, which is how this went unnoticed.

use dew_runtime::{Application, Capabilities, Pointer};
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

fn app() -> Application {
    let entry = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pressable.luau");
    Application::load_with(caps(), &entry, common::install_host).expect("fixture loads")
}

/// The button occupies (20,20)-(120,60); this is its middle.
const INSIDE: (f32, f32) = (70.0, 40.0);

#[test]
fn a_press_inside_a_pressable_fires_it() {
    let app = app();
    let session = app.session().unwrap();
    let presses: Function = app.get("Presses").unwrap();

    // Step first: layout has to have run and written bounds back, or the router
    // resolves against a tree whose rectangles are all zero.
    session.step(1.0 / 60.0).unwrap();

    session.pointer(Pointer::Down, INSIDE.0, INSIDE.1).unwrap();
    session.pointer(Pointer::Up, INSIDE.0, INSIDE.1).unwrap();

    let count: i32 = presses.call(()).unwrap();
    assert_eq!(count, 1, "the press never reached the pressable");
}

#[test]
fn a_press_outside_it_does_not() {
    let app = app();
    let session = app.session().unwrap();
    let presses: Function = app.get("Presses").unwrap();

    session.step(1.0 / 60.0).unwrap();
    session.pointer(Pointer::Down, 180.0, 90.0).unwrap();
    session.pointer(Pointer::Up, 180.0, 90.0).unwrap();

    let count: i32 = presses.call(()).unwrap();
    assert_eq!(count, 0, "a press outside the pressable fired it anyway");
}

#[test]
fn moving_over_it_reports_hover() {
    let app = app();
    let session = app.session().unwrap();
    let hovers: Function = app.get("Hovers").unwrap();

    session.step(1.0 / 60.0).unwrap();
    session.pointer(Pointer::Move, INSIDE.0, INSIDE.1).unwrap();
    session.step(1.0 / 60.0).unwrap();

    let count: i32 = hovers.call(()).unwrap();
    assert!(count >= 1, "hover never reached the pressable");
}

/// `OnActivated` is what a button should use, and it is not `OnPressed`.
///
/// A press followed by a release INSIDE the element activates it; a press that
/// wanders off and releases elsewhere must not. Worth pinning separately because
/// a widget wired to `OnPressed` fires on mouse-down and cannot be cancelled,
/// which is a subtly wrong button rather than a broken one.
#[test]
fn activation_needs_a_press_and_a_release_inside() {
    let app = app();
    let session = app.session().unwrap();
    let activations: Function = app.get("Activations").unwrap();

    session.step(1.0 / 60.0).unwrap();

    session.pointer(Pointer::Down, INSIDE.0, INSIDE.1).unwrap();
    session.pointer(Pointer::Up, INSIDE.0, INSIDE.1).unwrap();
    assert_eq!(
        activations.call::<i32>(()).unwrap(),
        1,
        "press and release inside should activate"
    );

    // Press inside, release outside: not an activation.
    session.pointer(Pointer::Down, INSIDE.0, INSIDE.1).unwrap();
    session.pointer(Pointer::Up, 180.0, 90.0).unwrap();
    assert_eq!(
        activations.call::<i32>(()).unwrap(),
        1,
        "releasing outside the element should not activate it"
    );
}
