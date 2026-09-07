//! How much of the property surface the gallery actually shows.
//!
//! `datamodel-surface` answers "does the host accept this". This answers "does
//! anything show it doing something", and the gap between the two is the point.
//!
//! Run with `cargo run --bin gallery-coverage`, or
//! `cargo run --bin gallery-coverage -- --render <dir>` to write the PNGs.

use dew_host::gallery;
use mlua::Lua;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let render_to: Option<PathBuf> = args
        .iter()
        .position(|a| a == "--render")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    let list_missing = args.iter().any(|a| a == "--missing");

    let dir = match gallery::find_scenes_dir(None) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("gallery: {e}");
            std::process::exit(1);
        }
    };

    // ONE VM PER SCENE. `install` puts a DataModel into a Lua state, and a
    // second scene sharing that state would inherit the first one's globals and
    // its arena. Scenes are meant to be independent, and a scene that only
    // renders because a previous one set something up is a scene that lies.
    let mut scenes = Vec::new();
    for path in match gallery::scene_paths(&dir) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("gallery: {e}");
            std::process::exit(1);
        }
    } {
        let lua = Lua::new();
        match gallery::decode_scene(&lua, &path) {
            Ok(scene) => {
                if let Some(ref out_dir) = render_to {
                    let out = out_dir.join(format!("{}.png", scene.file_stem));
                    if let Err(e) = gallery::render_scene(&lua, &scene, &out) {
                        eprintln!("FAIL {}: {e}", scene.file_stem);
                        std::process::exit(1);
                    }
                    println!("  rendered {}", out.display());
                }
                scenes.push(scene);
            }
            Err(e) => {
                eprintln!("FAIL {e}");
                std::process::exit(1);
            }
        }
    }

    let cov = gallery::coverage(&scenes);

    println!();
    println!(
        "GALLERY: {} scene(s), {} class(es)",
        scenes.len(),
        gallery::classes_used(&scenes).len()
    );
    println!(
        "DEMONSTRATED: {} of {} in-scope properties ({:.0}%)",
        cov.count(),
        cov.total(),
        100.0 * cov.count() as f64 / cov.total() as f64
    );

    // THE FINER FIGURE, kept beside the headline because counting by name is
    // exactly how one class's answer once masked another's and the surface read
    // 100%. `Color` demonstrated on `UIGradient` says nothing about
    // `UIStroke.Color`, and the by-name number cannot tell them apart.
    println!(
        "  by class and property: {} of {} pairs",
        cov.pairs_demonstrated, cov.pairs_in_scope
    );

    // SAY WHAT IS MISSING, and say how much. A coverage tool that prints only a
    // percentage tells you to feel bad without telling you what to do.
    println!(
        "  {} property name(s) not demonstrated by any scene",
        cov.missing.len()
    );
    if list_missing {
        for chunk in cov.missing.chunks(6) {
            println!("    {}", chunk.join(", "));
        }
    } else if !cov.missing.is_empty() {
        println!("    (--missing lists them)");
    }
}
