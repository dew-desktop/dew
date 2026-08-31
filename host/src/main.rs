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
mod surface;
mod tray;

use aether_raster::{Backend, Font};
use aether_runtime::{Driver, Painter, RasterPainter, Rgb};
use aether_window::{Button, Event, Window};
use std::collections::HashMap;
use std::path::PathBuf;
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
    let (aether_root, aliases) = aether_aliases()?;

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
        match mods::load(&dir, &aether_root, &aliases, &state) {
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
    let mods::Mod { manifest, mut width, mut height, surface, session, vm } = active;

    let screen = aether_window::screen_size();
    if surface.fills_screen() {
        width = screen.0.max(1) as u32;
        height = screen.1.max(1) as u32;
    }

    // A WIDGET IS CLEARED TO NOTHING, a window to the platform's own background.
    //
    // `None` here means the painter clears transparent, so a pixel the tree did
    // not paint is a pixel the window does not occupy — which is what turns a
    // rounded card into a rounded WINDOW rather than a rounded shape on a dark
    // rectangle.
    let background = if surface.is_transparent() { None } else { Some(BACKGROUND) };
    let mut driver = Driver::new(session, painter(width, height)?, background);

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

    let resolved = surface.resolve(screen, (width, height));
    let mut window = Window::new(&resolved, width, height)?;

    // The VM outlives the driver that borrows its handles. Named rather than
    // `_vm`, because "this binding exists to keep something alive" is a fact
    // about the program, not an unused variable to silence.
    let _keep_alive = vm;

    // THE TRAY OUTLIVES THE LOOP. Dropping it removes the icon, and Windows
    // leaves a dead one on screen until something repaints — so it is bound here
    // rather than created inline and dropped immediately.
    let _tray = match tray::Tray::new(icon_path().as_deref()) {
        Ok(tray) => Some(tray),
        // A missing tray is not a reason to refuse to run.
        Err(message) => {
            eprintln!("[dew] no tray icon: {message}");
            None
        }
    };

    let mut last = Instant::now();

    let stats = std::env::args().any(|a| a == "--stats");
    let bench = std::env::args().any(|a| a == "--bench");
    let (mut frames, mut painted_frames) = (0u32, 0u32);
    let (mut sum_frame, mut sum_present) = (Duration::ZERO, Duration::ZERO);
    let mut last_report = Instant::now();

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

        // `--bench`: repaint every frame whether or not anything changed, which
        // is the load a drag produces. Without it `--stats` measures an idle
        // screen, where the interesting number is always zero.
        if bench {
            driver.invalidate();
        }

        let t0 = Instant::now();
        let painted = driver.frame(dt).map_err(|e| format!("while rendering: {e}"))?;
        let t_frame = t0.elapsed();

        // RASTERISE **AND** PRESENT. vello records during paint and rasterises on
        // demand inside `bgra()`, so this is not the blit — it is most of the
        // drawing. Timing it as "present" made the blit look like the bottleneck
        // when the blit is a memcpy.
        let t1 = Instant::now();
        if let Some(bgra) = driver.painter_mut().canvas_mut().bgra() {
            window.present(bgra, width, height);
        }
        let t_raster = t1.elapsed();

        // `--stats`: where the frame time actually goes. Reported as a rolling
        // average rather than per frame, because a per-frame print costs more
        // than the frame it is measuring.
        if stats {
            frames += 1;
            if painted {
                sum_frame += t_frame;
                sum_present += t_raster;
                painted_frames += 1;
            }
            if last_report.elapsed() >= Duration::from_secs(1) {
                let n = painted_frames.max(1) as u32;
                println!(
                    "[dew] {frames} fps | painted {painted_frames} | solve {:?} | raster+blit {:?}",
                    sum_frame / n,
                    sum_present / n
                );
                frames = 0;
                painted_frames = 0;
                sum_frame = Duration::ZERO;
                sum_present = Duration::ZERO;
                last_report = Instant::now();
            }
        }

        if tray::exit_requested() {
            return Ok(());
        }

        // READ EVERY FRAME, because the tray menu can change it between any two.
        // `None` is uncapped and does not sleep at all.
        if let Some(target) = tray::frame_budget() {
            let elapsed = last.elapsed();
            if elapsed < target {
                std::thread::sleep(target - elapsed);
            }
        }
    }

    Ok(())
}

/// Dew's tray icon, beside the executable or in the source tree.
fn icon_path() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("host/assets/dew.ico"),
        PathBuf::from("assets/dew.ico"),
        PathBuf::from("../host/assets/dew.ico"),
    ];
    candidates.into_iter().find(|p| p.is_file())
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

/// The installed vide's `src`, whatever version pesde resolved.
///
/// GLOBBED rather than spelled out: pesde writes the version into the path, so
/// naming one here would go stale on the next resolve and report itself as "no
/// vide" rather than as "a different vide".
fn find_vide() -> Option<PathBuf> {
    //--- WALKS UP, like `mods` does. `cargo run` from `host/` finds the mods
    //--- directory and then failed here, because this looked only in the current
    //--- one -- so the host reported "no mods loaded" from a directory it had
    //--- just found, which points at everything except the actual cause.
    let base = find_dir("roblox_packages")?.join(".pesde");
    for entry in std::fs::read_dir(base).ok()?.flatten() {
        if entry.file_name().to_string_lossy().contains("vide") {
            let src = entry.path().join("vide").join("src");
            if src.is_dir() {
                return Some(src);
            }
        }
    }
    None
}

/// What every mod VM is given: Aether's source, and the aliases that name it.
///
/// NOTHING IS WRITTEN TO DISK. An earlier version generated a `.luaurc`, and it
/// could not work: aliases resolve by walking up from the requiring FILE, and
/// Aether's source lives in Cargo's package cache — a config file in this
/// repository is never on that path. `Capabilities.aliases` reaches the resolver
/// directly instead, so it applies wherever the requiring module happens to be.
///
/// BOTH PATHS COME FROM THE PIN. `luau_source_root()` reports the checkout Cargo
/// made for the revision in host/Cargo.toml, so the Luau a mod requires is the
/// same commit as the Rust driving it.
fn aether_aliases() -> Result<(PathBuf, HashMap<String, PathBuf>), String> {
    let root = aether_runtime::luau_source_root().ok_or(
        "aether_runtime could not locate Aether's Luau source beside itself —          a vendoring layout this does not understand",
    )?;

    let mut aliases = HashMap::new();
    aliases.insert("aether".to_string(), root.join("src"));

    // AETHER'S OWN DEPENDENCY, SUPPLIED BY US. A pinned checkout carries the
    // framework's source and NOT its `roblox_packages`, which is generated — so
    // vide is installed here by pesde and handed over through the `@vide` seam
    // VideCore exposes. Without it the framework loads and then reports "no
    // installed vide reachable" from a checkout that is otherwise perfect.
    match find_vide() {
        Some(vide) => {
            aliases.insert("vide".to_string(), vide);
        }
        None => eprintln!("[dew] no vide installed — run `pesde install`; widgets will not mount"),
    }

    Ok((root, aliases))
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
