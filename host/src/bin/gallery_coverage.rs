//! How much of the property surface the gallery actually shows.
//!
//! `datamodel-surface` answers "does the host accept this". This answers "does
//! anything show it doing something", and the gap between the two is the point.
//!
//!     cargo run --bin gallery-coverage                  # the number
//!     cargo run --bin gallery-coverage -- --render      # write the PNGs too
//!     cargo run --bin gallery-coverage -- --missing     # and what is not shown
//!     cargo run --bin gallery-coverage -- --scenes      # every scene, by pillar

use dew_host::gallery;
use mlua::Lua;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let list_missing = args.iter().any(|a| a == "--missing");
    let list_scenes = args.iter().any(|a| a == "--scenes");

    let dir = match gallery::find_scenes_dir(None) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("gallery: {e}");
            std::process::exit(1);
        }
    };

    // `--render` TAKES AN OPTIONAL PATH. With none, it writes beside the scenes
    // rather than into `target/`, which belongs to cargo and gets cleaned.
    let render_to: Option<PathBuf> = args.iter().position(|a| a == "--render").map(|i| {
        args.get(i + 1)
            .filter(|v| !v.starts_with("--"))
            .map(PathBuf::from)
            .unwrap_or_else(|| gallery::default_render_dir(&dir))
    });

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
        let pillar = gallery::pillar_of(&path, &dir);
        let lua = Lua::new();
        match gallery::decode_scene(&lua, &path, &pillar) {
            Ok(scene) => {
                if let Some(ref out_dir) = render_to {
                    let out = out_dir
                        .join(&scene.pillar)
                        .join(format!("{}.png", scene.file_stem));
                    if let Err(e) = gallery::render_scene(&lua, &scene, &out) {
                        eprintln!("FAIL {}/{}: {e}", scene.pillar, scene.file_stem);
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

    // THE DIFFERENTIAL PASS repaints every scene twice per variant, so it is
    // opt-out rather than always-on -- but it is the number that matters, so it
    // runs by default and `--no-diff` skips it for a quick coverage read.
    let skip_diff = args.iter().any(|a| a == "--no-diff");
    let (moved, inert) = if skip_diff {
        (Default::default(), Vec::new())
    } else {
        match gallery::differential(&scenes) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("gallery: differential pass failed: {e}");
                std::process::exit(1);
            }
        }
    };

    let cov = gallery::coverage_with_moved(&scenes, &moved);

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

    if !skip_diff {
        // THE STRONG NUMBER. "Set" is satisfied by a property the renderer
        // ignores; "differential" is not, because the image has to change.
        let checked = cov.differential.len() + cov.excused.len();
        println!(
            "DIFFERENTIAL: {} of {} moved pixels when changed, {} excused with a reason",
            cov.differential.len(),
            cov.total(),
            cov.excused.len()
        );
        println!(
            "  {} of {} neither shown to matter nor excused",
            cov.total() - checked,
            cov.total()
        );

        // A VARIANT THAT MOVED NOTHING IS THE MOST INTERESTING LINE HERE. The
        // property reached the host and did not reach the pixels, which is
        // exactly the gap this milestone exists to surface.
        if !inert.is_empty() {
            println!();
            println!("CHANGED NOTHING ({}):", inert.len());
            for line in &inert {
                println!("  {line}");
            }
        }
    }

    // PER PILLAR, INCLUDING THE EMPTY ONES. A pillar with no scenes is the most
    // useful line in this report: it is where the next scene should go. Printing
    // only the pillars that have scenes would hide exactly that.
    println!();
    println!("BY PILLAR:");
    for (pillar, (n, props)) in gallery::by_pillar(&dir, &scenes) {
        let note = if n == 0 { "   <- no scenes yet" } else { "" };
        println!("  {pillar:<12} {n:>2} scene(s), {props:>3} propert(ies){note}");
    }

    if list_scenes {
        println!();
        println!("SCENES:");
        let mut current = String::new();
        for s in &scenes {
            if s.pillar != current {
                println!("  {}/", s.pillar);
                current = s.pillar.clone();
            }
            println!("    {:<34} {}", s.file_stem, s.shows);
        }
    }

    // SAY WHAT IS MISSING, and say how much. A coverage tool that prints only a
    // percentage tells you to feel bad without telling you what to do.
    println!();
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
