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
use dew_host::datamodel;
mod manifest;
mod mods;
mod surface;
mod tray;

use aether_raster::{Backend, Font};
use aether_runtime::{Driver, RasterPainter, Rgb};
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

/// What the frame loop drives, whichever runtime the active mod declared.
///
/// TWO WAYS TO PRODUCE A `Frame`, ONE PAINTER. `Driver` diffs an Aether session
/// and repaints only when the framework says something changed; a DataModel tree
/// now answers the same question for itself, because every write a guest can make
/// goes through the path that fires `Changed`. Both arms end at
/// `Painter::paint_frame`, which is the seam that has survived four rasterisers.
///
/// NOT A TRAIT. The two do not share a lifecycle — one owns Lua handles the
/// framework maintains, the other owns ids into a `Dom` — and a trait over them
/// would exist to make this file shorter rather than to describe anything.
enum Renderer {
    Aether(Driver<RasterPainter>),
    DataModel {
        dom: datamodel::SharedDom,
        root: usize,
        painter: RasterPainter,
        background: Option<Rgb>,
        width: f32,
        height: f32,
    },
}

impl Renderer {
    /// Advance and paint. `true` means the canvas changed and is worth presenting.
    fn frame(&mut self, dt: f32) -> Result<bool, String> {
        match self {
            Renderer::Aether(driver) => driver.frame(dt).map_err(|e| e.to_string()),
            //--- WAS ALWAYS TRUE, AND IS NOT ANY MORE. This read "assume so", and
            //--- the comment was honest: a DataModel mod's `mount` ran once, and
            //--- anything it changed afterwards it changed by assigning to a
            //--- property with nothing watching. `--stats` on the live window
            //--- measured `painted` equal to `fps` -- 1424 repaints a second of a
            //--- card that never moves.
            //---
            //--- The arena answers now. Every guest-visible write marks it dirty
            //--- on the same path that fires `Changed`, and `take_dirty` both
            //--- reads and clears, so a frame that finds nothing is a frame that
            //--- is not drawn. `dt` is still unused: there is nothing to step, and
            //--- a mod that animates does it by assigning, which is what dirties
            //--- the tree.
            Renderer::DataModel {
                dom,
                root,
                painter,
                background,
                width,
                height,
            } => {
                let _ = dt;
                if !dom.lock().expect("dom").take_dirty() {
                    return Ok(false);
                }
                let frame = datamodel::render::frame_of(dom, *root, *width, *height);
                aether_runtime::Painter::paint_frame(painter, &frame, *background);
                Ok(true)
            }
        }
    }

    /// A pointer event, for a runtime that has somewhere to send it.
    ///
    /// DROPPED, LOUDLY IN THE COMMENT AND SILENTLY AT RUNTIME, for a DataModel
    /// mod. There is a signal model now -- sprint 8 built the type and the events
    /// an instance raises about ITSELF -- but no `Activated`, no `InputBegan`, and
    /// no hit testing to decide which instance a click at (x, y) belongs to.
    /// Sprint 9 is where this arm gets a body, and the milestone is not finished
    /// until it does.
    fn pointer(&mut self, kind: aether_runtime::Pointer, x: f32, y: f32) -> Result<(), String> {
        match self {
            Renderer::Aether(driver) => driver.pointer(kind, x, y).map_err(|e| e.to_string()),
            Renderer::DataModel { .. } => Ok(()),
        }
    }

    fn wheel(&mut self, x: f32, y: f32, delta: f32) -> Result<(), String> {
        match self {
            Renderer::Aether(driver) => driver.wheel(x, y, delta).map_err(|e| e.to_string()),
            Renderer::DataModel { .. } => Ok(()),
        }
    }

    /// Force the next frame to repaint.
    ///
    /// A NO-OP UNTIL THIS SPRINT, and it was wrong in a way nothing could show:
    /// every DataModel frame repainted anyway, so `Exposed` and `Resized` were
    /// served by accident. Now that a clean tree skips the paint, a window that
    /// was uncovered has to be able to say so -- nothing in the arena changed,
    /// and the pixels still need redrawing.
    fn invalidate(&mut self) {
        match self {
            Renderer::Aether(driver) => driver.invalidate(),
            Renderer::DataModel { dom, .. } => dom.lock().expect("dom").touch(),
        }
    }

    fn painter_mut(&mut self) -> &mut RasterPainter {
        match self {
            Renderer::Aether(driver) => driver.painter_mut(),
            Renderer::DataModel { painter, .. } => painter,
        }
    }
}

/// `--script <path> --snapshot <png>`: run a Luau file against Dew's OWN
/// DataModel and draw what it built. No Aether anywhere in the chain.
///
/// THIS IS THE STANDALONE STORY MADE CONCRETE. Everything else in this binary
/// mounts an Aether component and paints Aether's display list; this path hands a
/// guest `Instance`, the vocabulary and `Enum`, takes the tree it parents into a
/// root, and turns that into the same `Frame` the same painter consumes. It is
/// what the property surface has been measuring all along, finally reaching a
/// pixel.
///
/// `DewRoot` RATHER THAN `game`, and the reason is not cosmetic. `game` would be
/// the honest parity name and it is exactly what `Host.detect()` keys on:
/// `typeof(game) == "Instance"` is Aether's whole test for whether it is on an
/// engine. Installing one here without the services and the rest behind it would
/// make every Aether mod in this same binary take the Roblox branch and fail. It
/// arrives when there is enough behind it to be true.
fn run_script(path: &str, width: u32, height: u32) -> Result<(String, RasterPainter), String> {
    let script = PathBuf::from(path);
    let dir = script
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let caps = aether_runtime::Capabilities {
        require_roots: vec![dir],
        print: true,
        aliases: HashMap::new(),
    };
    let vm = aether_runtime::Vm::new(caps).map_err(|e| e.to_string())?;

    let dom = datamodel::SharedDom::default();
    datamodel::install(vm.lua(), &dom).map_err(|e| e.to_string())?;
    datamodel::install_vocabulary(vm.lua()).map_err(|e| e.to_string())?;

    // The surface the guest parents into. A `ScreenGui` because that is what a
    // Roblox application expects to find above its tree, so the same file has a
    // chance of running in both places later.
    let root = dom
        .lock()
        .expect("dom")
        .insert("ScreenGui".into(), "DewRoot".into());
    vm.lua()
        .globals()
        .set("DewRoot", datamodel::handle(&dom, root))
        .map_err(|e| e.to_string())?;

    let source =
        std::fs::read_to_string(&script).map_err(|e| format!("{}: {e}", script.display()))?;
    vm.lua()
        .load(&source)
        .set_name(script.display().to_string())
        .exec()
        .map_err(|e| format!("{}: {e}", script.display()))?;

    let frame = datamodel::render::frame_of(&dom, root, width as f32, height as f32);
    let drawn = frame.nodes.len();
    let mut surface = painter(width, height)?;
    aether_runtime::Painter::paint_frame(&mut surface, &frame, Some(BACKGROUND));
    Ok((format!("{drawn} node(s)"), surface))
}

fn run() -> Result<(), String> {
    println!("💧 Dew starting");

    // `--script` NEVER TOUCHES THE MOD DIRECTORY. A standalone run is a different
    // product from the applet host and shares only the painter, so it returns
    // before any of the discovery below.
    if let Some(script) = flag("--script") {
        let (width, height) = flag("--size")
            .and_then(|s| {
                let (w, h) = s.split_once('x')?;
                Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
            })
            .unwrap_or((400, 300));
        let (report, mut surface) = run_script(&script, width, height)?;
        let out = flag("--snapshot").unwrap_or_else(|| "dew.png".to_string());
        surface
            .write_png(&out)
            .map_err(|code| format!("could not write {out}: rasteriser status {code}"))?;
        println!("[dew] {script}: {report} -> {out} ({width}x{height})");
        return Ok(());
    }

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

    // DESTRUCTURED, to move the mounted half out by value. `Mod` implements no
    // `Drop`, so Rust permits this directly — and the alternative that suggests
    // itself, swapping in a placeholder, has no valid placeholder to swap: a
    // `Session` is Lua handles, and a zeroed one is undefined behaviour the moment
    // it is dropped rather than a temporarily invalid value.
    let mods::Mod {
        manifest,
        mut width,
        mut height,
        surface,
        mounted,
        vm,
    } = active;

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
    let background = if surface.is_transparent() {
        None
    } else {
        Some(BACKGROUND)
    };
    // THE BRANCH IS HERE AND NOWHERE ELSE IN THIS FUNCTION. Everything above --
    // discovery, the manifest, the screen, the surface, the background -- is Dew's
    // own remit and identical for both runtimes; everything below drives whatever
    // this produced.
    let mut renderer = match mounted {
        mods::Mounted::Aether(session) => {
            Renderer::Aether(Driver::new(session, painter(width, height)?, background))
        }
        //--- SIZED ONCE, at the size the window was opened at. A DataModel tree
        //--- lays out against the surface it is given, and `Event::Resized` does
        //--- not change `width` here for the Aether path either -- the window is
        //--- not resizable yet, and pretending otherwise would put a second
        //--- untested code path behind a feature that does not exist.
        mods::Mounted::DataModel { dom, root } => Renderer::DataModel {
            dom,
            root,
            painter: painter(width, height)?,
            background,
            width: width as f32,
            height: height as f32,
        },
    };

    // `--snapshot <path>`: draw one frame, write it, exit.
    //
    // NEEDS NO WINDOW, which is what makes it useful beyond debugging — it is how
    // a widget gets diffed in CI, and how anyone without a desktop session can
    // see what a mod actually renders. It is also the only way to inspect a
    // widget's appearance from a terminal, which is where most of this gets
    // written.
    if let Some(path) = flag("--snapshot") {
        renderer.frame(1.0 / 60.0)?;
        renderer
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
    // THE TOOLTIP NAMES THE MOD, not the host. `name` and `description` are the
    // two manifest fields that exist purely to be shown to a person, and this is
    // where they are shown; a mod that declares neither falls back to its id.
    let tooltip = if manifest.description.trim().is_empty() {
        format!("Dew — {}", manifest.display_name())
    } else {
        format!(
            "Dew — {}: {}",
            manifest.display_name(),
            manifest.description
        )
    };

    let _tray = match tray::Tray::new(icon_path().as_deref(), &tooltip) {
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

    // `poll` returning None is the window closing, which ends the loop -- the
    // condition IS the shutdown signal rather than a check inside the body.
    while let Some(events) = window.poll() {
        for event in events {
            match event {
                Event::PointerMove { x, y } => {
                    renderer.pointer(aether_runtime::Pointer::Move, x, y)?;
                }
                Event::PointerDown {
                    x,
                    y,
                    button: Button::Left,
                } => {
                    renderer.pointer(aether_runtime::Pointer::Down, x, y)?;
                }
                Event::PointerUp {
                    x,
                    y,
                    button: Button::Left,
                } => {
                    renderer.pointer(aether_runtime::Pointer::Up, x, y)?;
                }
                Event::PointerDown { .. } | Event::PointerUp { .. } => {}
                Event::Wheel { x, y, delta } => {
                    renderer.wheel(x, y, delta)?;
                }
                Event::Resized { .. } | Event::Exposed => renderer.invalidate(),
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
            renderer.invalidate();
        }

        let t0 = Instant::now();
        let painted = renderer
            .frame(dt)
            .map_err(|e| format!("while rendering: {e}"))?;
        let t_frame = t0.elapsed();

        // RASTERISE **AND** PRESENT. vello records during paint and rasterises on
        // demand inside `bgra()`, so this is not the blit — it is most of the
        // drawing. Timing it as "present" made the blit look like the bottleneck
        // when the blit is a memcpy.
        let t1 = Instant::now();
        if let Some(bgra) = renderer.painter_mut().canvas_mut().bgra() {
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
                let n = painted_frames.max(1);
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
