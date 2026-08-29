//! 💧 Dew — a desktop applet platform.
//!
//! WHAT IS NOT IN THIS CRATE, and deliberately: layout, hit testing, pointer
//! arbitration, text measurement, rasterisation, the Luau VM, and the frame
//! loop. All of it comes from Aether, which is also what the Roblox host uses —
//! so a widget's visual half behaves the same in both places, and a rendering
//! fix reaches both at once.
//!
//! What IS Dew's: mod discovery, manifests, capabilities, and putting widgets on
//! a desktop. That is the whole remit.

mod capabilities;
mod manifest;
mod mods;

use aether_raster::{Backend, Font};
use aether_runtime::{Driver, Painter, RasterPainter, Rgb};
use aether_window::{Button, Event, Window};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Deep obsidian, behind every widget.
const BACKGROUND: Rgb = Rgb(13, 17, 23);

fn find_dir(name: &str) -> Option<PathBuf> {
    let mut cur = std::env::current_dir().ok()?;
    loop {
        let candidate = cur.join(name);
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn painter(width: u32, height: u32) -> Result<RasterPainter, String> {
    // VELLO, NOT TINY-SKIA: tiny-skia has no text, so every label in every widget
    // would silently vanish.
    let mut painter = RasterPainter::new(width, height, Backend::VelloCpu)
        .ok_or("could not create a drawing surface")?;
    if let Some(path) = aether_runtime::font::system_font() {
        if let Some(font) = Font::load(&path.to_string_lossy(), 0) {
            painter = painter.with_font(font);
        }
    }
    Ok(painter)
}

fn run() -> Result<(), String> {
    println!("💧 Dew starting");

    let mods_dir = find_dir("mods").ok_or("could not find a `mods` directory")?;
    let aether_root = find_aether()?;

    let state = Arc::new(Mutex::new(capabilities::HostState::default()));

    // ONE MOD FOR NOW, and the loop below drives one window. Multi-window
    // placement is the next piece of Dew's own remit; everything under it is
    // already per-mod, so adding windows does not reach back into any of it.
    let entries = std::fs::read_dir(&mods_dir).map_err(|e| e.to_string())?;
    let mut loaded = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        match mods::load(&dir, &aether_root, &state) {
            Ok(m) => loaded.push(m),
            // ONE BAD MOD MUST NOT TAKE THE HOST DOWN. It is reported and
            // skipped, which is the behaviour a platform running third-party
            // code has to have.
            Err(message) => eprintln!("[dew] skipping mod: {message}"),
        }
    }

    if loaded.is_empty() {
        return Err(format!("no mods loaded from {}", mods_dir.display()));
    }

    // WHICH MOD, CHOSEN RATHER THAN STUMBLED INTO. Without `--mod` this took
    // whatever `read_dir` happened to return first — which is not alphabetical,
    // not declared anywhere, and changes with the filesystem. A default that
    // cannot be predicted is worse than no default, so the available ids are
    // listed when the requested one is not among them.
    let wanted = flag("--mod");
    let active = match &wanted {
        Some(id) => loaded
            .into_iter()
            .find(|m| &m.manifest.id == id)
            .ok_or_else(|| format!("no mod with id {id:?}"))?,
        None => {
            let mut sorted = loaded;
            sorted.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
            let ids: Vec<&str> = sorted.iter().map(|m| m.manifest.id.as_str()).collect();
            println!("[dew] mods: {} (pick one with --mod <id>)", ids.join(", "));
            sorted.into_iter().next().expect("non-empty")
        }
    };

    // DESTRUCTURED, to move the session out by value. `Mod` implements no `Drop`,
    // so Rust permits this directly — and the alternative that suggests itself,
    // swapping in a placeholder, has no valid placeholder to swap: a `Session` is
    // Lua handles, and a zeroed one is undefined behaviour the moment it is
    // dropped rather than a temporarily invalid value.
    let mods::Mod { manifest, width, height, session, vm } = active;

    let mut driver = Driver::new(session, painter(width, height)?, Some(BACKGROUND));

    // `--snapshot <path>`: draw one frame, write it, exit.
    //
    // NEEDS NO WINDOW, which is what makes it useful beyond debugging — it is how
    // a widget gets diffed in CI, and how anyone without a desktop session can
    // see what a mod actually renders. It is also the only way to inspect a
    // widget's appearance from a terminal, which is where most of this gets
    // written.
    if let Some(path) = flag("--snapshot") {
        driver.frame(1.0 / 60.0).map_err(|e| e.to_string())?;
        driver
            .painter_mut()
            .write_png(&path)
            .map_err(|code| format!("could not write {path}: rasteriser status {code}"))?;
        println!("[dew] wrote {path} ({width}x{height}) for {}", manifest.id);
        return Ok(());
    }

    let mut window = Window::new(&format!("Dew — {}", manifest.id), width, height)?;

    // The VM outlives the driver that borrows its handles. Named rather than
    // `_vm`, because "this binding exists to keep something alive" is a fact
    // about the program, not an unused variable to silence.
    let _keep_alive = vm;

    let target = Duration::from_micros(16_667);
    let mut last = Instant::now();

    loop {
        let Some(events) = window.poll() else {
            break;
        };

        for event in events {
            match event {
                Event::PointerMove { x, y } => {
                    driver.pointer(aether_runtime::Pointer::Move, x, y).map_err(|e| e.to_string())?;
                }
                Event::PointerDown { x, y, button: Button::Left } => {
                    driver.pointer(aether_runtime::Pointer::Down, x, y).map_err(|e| e.to_string())?;
                }
                Event::PointerUp { x, y, button: Button::Left } => {
                    driver.pointer(aether_runtime::Pointer::Up, x, y).map_err(|e| e.to_string())?;
                }
                Event::PointerDown { .. } | Event::PointerUp { .. } => {}
                Event::Wheel { x, y, delta } => {
                    driver.wheel(x, y, delta).map_err(|e| e.to_string())?;
                }
                Event::Resized { .. } | Event::Exposed => driver.invalidate(),
                Event::CloseRequested => return Ok(()),
                Event::Char(_) | Event::Key { .. } => {}
            }
        }

        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();

        driver.frame(dt).map_err(|e| format!("while rendering: {e}"))?;
        if let Some(bgra) = driver.painter_mut().canvas_mut().bgra() {
            window.blit(bgra, width, height);
        }

        let elapsed = last.elapsed();
        if elapsed < target {
            std::thread::sleep(target - elapsed);
        }
    }

    Ok(())
}

/// `--name value`, or None.
fn flag(name: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next();
        }
    }
    None
}

/// Aether's source root, for the mod VMs' require resolver.
fn find_aether() -> Result<PathBuf, String> {
    if let Some(dir) = find_dir("aether") {
        let src = dir.join("src");
        if src.is_dir() {
            return Ok(dir);
        }
    }
    let sibling = Path::new("../aether");
    if sibling.join("src").is_dir() {
        return Ok(sibling.to_path_buf());
    }
    Err("could not find the aether checkout beside this repository".into())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("dew: {message}");
            ExitCode::FAILURE
        }
    }
}
