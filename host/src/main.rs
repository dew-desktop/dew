//! 💧 Dew — a desktop applet platform.
//!
//! WHAT IS NOT IN THIS CRATE, and deliberately: text measurement,
//! rasterisation, the Luau VM, and the frame loop. Those are in `crates/raster`,
//! `crates/runtime` and `crates/window` — Dew's, one workspace along, and a
//! rendering change belongs there rather than upstream. They lived in Aether's
//! repository until ADR-004 measured what they were: no Rust there reached a
//! Luau consumer, and the only two dependents were this host and a CLI that
//! retires. The crates still carry Aether's names; the rename is its own commit.
//!
//! LAYOUT, HIT TESTING AND POINTER ARBITRATION USED TO BE ON THAT LIST, and for
//! the Aether arm they still are: `Driver::pointer` hands an event to the
//! framework and the framework decides. Dew's own DataModel has no framework
//! under it, so the host answers for itself — `datamodel::render` resolves the
//! geometry and `datamodel::input` says which instance is at (x, y). ADR-001 is
//! why: a host that owns its DataModel owns the questions asked of it.
//!
//! What IS Dew's: mod discovery, manifests, capabilities, and putting widgets on
//! a desktop. That is the whole remit.

mod capabilities;
use dew_host::datamodel;
mod manifest;
mod mods;
mod services;
mod surface;
#[cfg(windows)]
mod tray;

use crate::services::SharedClock;
use aether_raster::Backend;
use aether_runtime::{Driver, RasterPainter, Rgb};
#[cfg(windows)]
use aether_window::{Button, Event, Window};
use dew_host::datamodel::input;
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
    // THE FACE COMES FROM `services::face`, WHICH IS ALSO WHAT MEASURES. This
    // used to call `Font::load` here, and once a guest can ask "how wide is this
    // string" that is no longer merely wasteful -- `Font::load` hands back a fresh
    // id per call, so the painter and the measurement would have been two
    // registrations of the same file, and a measurement that does not describe the
    // pixels is worse than no measurement. One memo, one id, one face.
    if let Some(font) = services::face() {
        painter = painter.with_font(font);
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
///
/// BOTH ARMS CARRY A CLOCK, and it sits outside the enum's two shapes for the
/// reason `Mod::clock` does: frames are the host's, not the framework's. An
/// Aether mod subscribing through `Host.Clock` and a DataModel mod subscribing
/// through `DewHost.Clock` are the same subscription, and a clock that only one
/// arm drove would make animation a property of the runtime a mod declared.
enum Renderer {
    Aether {
        driver: Driver<RasterPainter>,
        clock: SharedClock,
    },
    DataModel {
        dom: datamodel::SharedDom,
        root: usize,
        painter: RasterPainter,
        background: Option<Rgb>,
        width: f32,
        height: f32,
        /// The VM the tree's handlers live in.
        ///
        /// HELD BECAUSE FIRING NEEDS IT. A handler's arguments are Lua values —
        /// two numbers for `MouseMoved`, an `InputObject` for `InputBegan` — and
        /// building one needs a `&Lua`. `Vm` itself stays in `run`'s
        /// `_keep_alive`; mlua's `Lua` is a reference-counted handle onto the same
        /// state, so this is a second reference to that VM rather than a second
        /// VM.
        lua: mlua::Lua,
        /// Hover and press state, carried across frames. See
        /// `datamodel::input::Pointer` — `MouseEnter` is the difference between
        /// two events, so somebody has to remember the previous answer.
        pointer: input::Pointer,
        clock: SharedClock,
    },
}

impl Renderer {
    /// The frame subscriptions this renderer drives, whichever arm it is.
    fn clock(&self) -> &SharedClock {
        match self {
            Renderer::Aether { clock, .. } => clock,
            Renderer::DataModel { clock, .. } => clock,
        }
    }
}

/// Which mouse button, in the DataModel's spelling.
///
/// TWO ENUMS RATHER THAN ONE SHARED ONE. `aether_window::Button` is what the
/// platform reports and `input::Button` is what the DataModel fires as; making
/// Dew's host depend on the window crate's enum inside the datamodel would put an
/// Aether type in `dew_host`, which is exactly the boundary ADR-004's checker
/// counts. It is three lines to translate and the allowlist stays where it is.
#[cfg(windows)]
fn button(button: Button) -> input::Button {
    match button {
        Button::Left => input::Button::Left,
        Button::Right => input::Button::Right,
        Button::Middle => input::Button::Middle,
    }
}

impl Renderer {
    /// Advance and paint. `true` means the canvas changed and is worth presenting.
    ///
    /// FRAME LISTENERS RUN FIRST, AND THEY DO NOT DECIDE WHETHER TO PAINT. Those
    /// are two separate sentences and the second is the one milestone 1's sprint 9
    /// is riding on.
    ///
    /// First, because a listener that assigns a property must have that assignment
    /// land in THIS frame rather than the next -- an animation running one frame
    /// behind its own clock is the kind of wrong that looks like jitter and reads
    /// like a rasteriser problem.
    ///
    /// Not deciding, because a tick is not a change. Nothing in `services::tick`
    /// touches the arena, so a mod that subscribes to frames and animates nothing
    /// leaves the tree clean and this function still returns `false`. A listener
    /// that DOES assign dirties the tree on the same path every other write takes,
    /// and the paint follows the change rather than the tick. Getting this
    /// backwards -- ticking straight into `invalidate`, or dirtying on subscribe --
    /// would hand back the whole of sprint 9's gain, and `--stats` is where it
    /// would show: `painted` climbing to meet `fps` the moment anything subscribed.
    fn frame(&mut self, dt: f32) -> Result<bool, String> {
        // IDLE IS CHECKED BEFORE ANYTHING IS BUILT. A mod that never subscribed
        // pays one uncontended lock per frame, which is what keeps a clock nobody
        // asked for off the hot path entirely.
        if !self.clock().lock().expect("clock").idle() {
            services::tick(&self.clock().clone(), dt);
        }
        match self {
            Renderer::Aether { driver, .. } => driver.frame(dt).map_err(|e| e.to_string()),
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
            //--- is not drawn. `dt` IS UNUSED IN THIS ARM AND STILL MEANS
            //--- SOMETHING: the frame listeners above already had it, and a mod
            //--- that animates does so by assigning inside one, which is what
            //--- dirties the tree. There is nothing left to step down here.
            Renderer::DataModel {
                dom,
                root,
                painter,
                background,
                width,
                height,
                lua,
                pointer,
                ..
            } => {
                let _ = dt;
                if !dom.lock().expect("dom").take_dirty() {
                    return Ok(false);
                }
                let frame = datamodel::render::frame_of(dom, *root, *width, *height);
                aether_runtime::Painter::paint_frame(painter, &frame, *background);

                //--- HOVER IS RECONCILED AFTER A PAINT, and this is the case a
                //--- naive implementation gets wrong silently. `MouseEnter` and
                //--- `MouseLeave` are the difference between two pointer events,
                //--- so an implementation that updates them only when the pointer
                //--- moves is correct until the TREE moves instead — and then a
                //--- mod that destroys the button under a stationary cursor keeps
                //--- a stale hover forever, and one that puts a new element there
                //--- never fires `MouseEnter` until the person jiggles the mouse.
                //---
                //--- ONLY ON A PAINTED FRAME, which is what keeps this off the
                //--- idle path: a frame that found nothing dirty returned above
                //--- and never reaches here, so an untouched tree costs one
                //--- `take_dirty` and nothing else. `refresh` does not dirty the
                //--- tree either, so a settled hover does not schedule the next
                //--- frame — which is the whole of sprint 8's gain, preserved.
                pointer
                    .refresh(&input::Surface {
                        lua,
                        dom,
                        root: *root,
                        size: (*width, *height),
                    })
                    .map_err(|e| e.to_string())?;
                Ok(true)
            }
        }
    }

    /// The cursor moved.
    ///
    /// THIS ARM WAS `Ok(())` WITH A COMMENT ADMITTING IT, for three sprints. A
    /// DataModel mod could be drawn, navigated, torn down and told about its own
    /// properties, and could not be clicked — sprint 7 measured that on the live
    /// window and named the cause: fourteen methods and no events. The signal type
    /// arrived in sprint 8; what was still missing was somewhere to send a
    /// pointer, which is a hit test.
    #[cfg(windows)]
    fn moved(&mut self, x: f32, y: f32) -> Result<(), String> {
        match self {
            Renderer::Aether { driver, .. } => driver
                .pointer(aether_runtime::Pointer::Move, x, y)
                .map_err(|e| e.to_string()),
            Renderer::DataModel {
                dom,
                root,
                width,
                height,
                lua,
                pointer,
                ..
            } => pointer
                .moved(
                    &input::Surface {
                        lua,
                        dom,
                        root: *root,
                        size: (*width, *height),
                    },
                    x,
                    y,
                )
                .map_err(|e| e.to_string()),
        }
    }

    /// A button went down.
    ///
    /// THE AETHER ARM HAS ONE BUTTON AND THIS ONE HAS THREE. `Driver::pointer`
    /// takes a `Pointer` with no button in it, so the loop below used to match
    /// `button: Button::Left` and drop the other two with a bare arm. A DataModel
    /// mod has `MouseButton2Click` and `SecondaryActivated`, so the button now
    /// travels — and the Aether arm keeps ignoring anything but the left, which is
    /// the framework's own limitation and not one to paper over here.
    #[cfg(windows)]
    fn down(&mut self, button: Button, x: f32, y: f32) -> Result<(), String> {
        match self {
            Renderer::Aether { driver, .. } => {
                if button != Button::Left {
                    return Ok(());
                }
                driver
                    .pointer(aether_runtime::Pointer::Down, x, y)
                    .map_err(|e| e.to_string())
            }
            Renderer::DataModel {
                dom,
                root,
                width,
                height,
                lua,
                pointer,
                ..
            } => pointer
                .down(
                    &input::Surface {
                        lua,
                        dom,
                        root: *root,
                        size: (*width, *height),
                    },
                    self::button(button),
                    x,
                    y,
                )
                .map_err(|e| e.to_string()),
        }
    }

    /// A button came up.
    #[cfg(windows)]
    fn up(&mut self, button: Button, x: f32, y: f32) -> Result<(), String> {
        match self {
            Renderer::Aether { driver, .. } => {
                if button != Button::Left {
                    return Ok(());
                }
                driver
                    .pointer(aether_runtime::Pointer::Up, x, y)
                    .map_err(|e| e.to_string())
            }
            Renderer::DataModel {
                dom,
                root,
                width,
                height,
                lua,
                pointer,
                ..
            } => pointer
                .up(
                    &input::Surface {
                        lua,
                        dom,
                        root: *root,
                        size: (*width, *height),
                    },
                    self::button(button),
                    x,
                    y,
                )
                .map_err(|e| e.to_string()),
        }
    }

    #[cfg(windows)]
    fn wheel(&mut self, x: f32, y: f32, delta: f32) -> Result<(), String> {
        match self {
            Renderer::Aether { driver, .. } => driver.wheel(x, y, delta).map_err(|e| e.to_string()),
            Renderer::DataModel {
                dom,
                root,
                width,
                height,
                lua,
                pointer,
                ..
            } => pointer
                .wheel(
                    &input::Surface {
                        lua,
                        dom,
                        root: *root,
                        size: (*width, *height),
                    },
                    x,
                    y,
                    delta,
                )
                .map_err(|e| e.to_string()),
        }
    }

    /// Force the next frame to repaint.
    ///
    /// A NO-OP UNTIL THIS SPRINT, and it was wrong in a way nothing could show:
    /// every DataModel frame repainted anyway, so `Exposed` and `Resized` were
    /// served by accident. Now that a clean tree skips the paint, a window that
    /// was uncovered has to be able to say so -- nothing in the arena changed,
    /// and the pixels still need redrawing.
    #[cfg(windows)]
    fn invalidate(&mut self) {
        match self {
            Renderer::Aether { driver, .. } => driver.invalidate(),
            Renderer::DataModel { dom, .. } => dom.lock().expect("dom").touch(),
        }
    }

    fn painter_mut(&mut self) -> &mut RasterPainter {
        match self {
            Renderer::Aether { driver, .. } => driver.painter_mut(),
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
/// `DewRoot` RATHER THAN `game`, AND THAT IS ABOUT THIS PATH RATHER THAN ABOUT
/// the plan. A standalone script parents into a root; `DewRoot` names the root it
/// was handed. What a guest reaches through `game` on the engine is the SERVICES
/// behind it, and those are `DewHost`, installed above: the two things a guest
/// framework genuinely could not compute for itself, and required of a conforming
/// host by `docs/host_services.md` since 2026-09-04.
///
/// THIS COMMENT HAS BEEN WRONG IN BOTH DIRECTIONS AND IS NOW SCOPED SO IT CANNOT
/// BE AGAIN. It first said `game` "arrives when there is enough behind it to be
/// true"; it then said `game` was dropped from the plan, was never a capability,
/// and was only a sentinel two consumers used as a proxy. That was right about
/// Aether and had never been measured of vide, which reaches `typeof`, `Instance`,
/// `Enum` and `Color3` THROUGH `game` as a truthiness gate -- so `game` came back
/// into scope on 2026-09-04, for that one consumer, as sprint 6's item 0a. See
/// the roadmap's "Why `game` was dropped from step F, and why it came back".
///
/// None of which changes this line: a `game` installed for vide's gate is an
/// Aether-mod concern, and no version of it would be what a standalone script
/// parents into.
fn run_script(path: &str, width: u32, height: u32) -> Result<(String, RasterPainter), String> {
    let script = PathBuf::from(path);
    let dir = script
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let caps = aether_runtime::Capabilities {
        require_roots: vec![dir.clone()],
        print: true,
        aliases: HashMap::new(),
    };
    let vm = aether_runtime::Vm::new(caps).map_err(|e| e.to_string())?;

    let dom = datamodel::SharedDom::default();
    // `mod://` RESOLVES BESIDE THE SCRIPT, which is the same directory the
    // requirer was just given. A standalone script has no mod directory and no
    // manifest, but it does have a file, and "beside the thing that named the
    // asset" is the rule in both cases rather than two rules that agree by
    // accident.
    dom.lock().expect("dom").assets.set_root(dir.clone());
    datamodel::install(vm.lua(), &dom).map_err(|e| e.to_string())?;
    datamodel::install_vocabulary(vm.lua()).map_err(|e| e.to_string())?;
    // `DewHost` HERE TOO, so a standalone script measures text the same way a mod
    // does. NOTHING DRIVES THE CLOCK ON THIS PATH and that is honest rather than
    // missing: `--script` draws one frame and exits, so there are no frames to be
    // called on. A script may still subscribe -- it simply never gets a tick,
    // which is the truthful answer for a renderer that runs once.
    let clock: services::SharedClock = Arc::new(Mutex::new(Default::default()));
    services::install(vm.lua(), &clock).map_err(|e| e.to_string())?;

    // The surface the guest parents into. A `ScreenGui` because that is what a
    // Roblox application expects to find above its tree, so the same file has a
    // chance of running in both places later.
    let root = dom
        .lock()
        .expect("dom")
        .insert("ScreenGui".into(), "DewRoot".into());
    let root_handle = datamodel::handle(vm.lua(), &dom, root).map_err(|e| e.to_string())?;
    vm.lua()
        .globals()
        .set("DewRoot", root_handle)
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

fn create_renderer(
    mounted: mods::Mounted,
    vm: &aether_runtime::Vm,
    clock: &SharedClock,
    surface: &surface::Declared,
    width: u32,
    height: u32,
) -> Result<Renderer, String> {
    let background = if surface.is_transparent() {
        None
    } else {
        Some(BACKGROUND)
    };
    match mounted {
        mods::Mounted::Aether(session) => Ok(Renderer::Aether {
            driver: Driver::new(session, painter(width, height)?, background),
            clock: clock.clone(),
        }),
        mods::Mounted::DataModel { dom, root } => Ok(Renderer::DataModel {
            dom,
            root,
            painter: painter(width, height)?,
            background,
            width: width as f32,
            height: height as f32,
            lua: vm.lua().clone(),
            pointer: input::Pointer::default(),
            clock: clock.clone(),
        }),
    }
}

fn validate_args() -> Result<(), String> {
    validate_args_iter(std::env::args().skip(1))
}

fn validate_args_iter<I: Iterator<Item = String>>(args: I) -> Result<(), String> {
    validate_args_for_platform(args, cfg!(windows))
}

fn validate_args_for_platform<I: Iterator<Item = String>>(
    mut args: I,
    is_windows: bool,
) -> Result<(), String> {
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--script" | "--size" | "--snapshot" | "--mod" => {
                let _ = args.next();
            }
            "--stats" | "--bench" => {
                if !is_windows {
                    return Err(format!(
                        "'{arg}' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                    ));
                }
            }
            _ => {
                return Err(format!("unrecognised argument '{arg}'"));
            }
        }
    }
    Ok(())
}

fn run() -> Result<(), String> {
    validate_args()?;

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
        clock,
        vm,
    } = active;

    // `--snapshot <path>`: draw one frame, write it, exit.
    //
    // NEEDS NO WINDOW, which is what makes it useful beyond debugging — it is how
    // a widget gets diffed in CI, and how anyone without a desktop session can
    // see what a mod actually renders. It is also the only way to inspect a
    // widget's appearance from a terminal, which is where most of this gets
    // written.
    //
    // NO SCREEN SIZE HERE. A snapshot renders at the declared size and does not
    // place anything, so querying the display here would invent a requirement
    // headless runs do not have.
    if let Some(path) = flag("--snapshot") {
        let mut renderer = create_renderer(mounted, &vm, &clock, &surface, width, height)?;
        renderer.frame(1.0 / 60.0)?;
        renderer
            .painter_mut()
            .write_png(&path)
            .map_err(|code| format!("could not write {path}: rasteriser status {code}"))?;
        println!("[dew] wrote {path} ({width}x{height}) for {}", manifest.id);
        return Ok(());
    }

    #[cfg(windows)]
    {
        let screen = aether_window::screen_size();
        if surface.fills_screen() {
            width = screen.0.max(1) as u32;
            height = screen.1.max(1) as u32;
        }

        let mut renderer = create_renderer(mounted, &vm, &clock, &surface, width, height)?;

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
                    Event::PointerMove { x, y } => renderer.moved(x, y)?,
                    //--- THE BUTTON TRAVELS NOW. These two arms matched
                    //--- `button: Button::Left` and a third dropped the rest, because
                    //--- `Driver::pointer` has nowhere to put a button. A DataModel mod
                    //--- has `MouseButton2Click` and `SecondaryActivated`, so which
                    //--- button it was is no longer the host's to discard — the Aether
                    //--- arm ignores everything but the left inside `Renderer`, where
                    //--- that limitation belongs.
                    Event::PointerDown { x, y, button } => renderer.down(button, x, y)?,
                    Event::PointerUp { x, y, button } => renderer.up(button, x, y)?,
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

    #[cfg(not(windows))]
    {
        Err("no headless action specified (use --script or --snapshot)".to_string())
    }
}

/// Dew's tray icon, beside the executable or in the source tree.
#[cfg(windows)]
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

/// What every mod VM is given: Aether's source, and the aliases that name it.
///
/// NOTHING IS WRITTEN TO DISK. An earlier version generated a `.luaurc`, and it
/// could not work: aliases resolve by walking up from the requiring FILE, and a
/// package's source lives under `roblox_packages/.pesde/` — a config file at the
/// root of this repository is never on that path. `Capabilities.aliases` reaches
/// the resolver directly instead, so it applies wherever the requiring module
/// happens to be.
///
/// BOTH PATHS COME FROM THE SAME INSTALL, AND AETHER IS NOW ONE OF THEM. It used
/// to come from Cargo: `luau_source_root()` reported the checkout made for the
/// revision in `host/Cargo.toml`, so the Luau a mod required was the same commit
/// as the Rust driving it. ADR-004 moved that Rust here, so there is no Aether
/// checkout to read it out of — and Aether was never Dew's Rust dependency in the
/// first place, it is a GUEST FRAMEWORK, exactly like vide. It is pinned by
/// commit in `pesde.toml` and installed beside vide, and both are found the same
/// way.
fn aether_aliases() -> Result<(PathBuf, HashMap<String, PathBuf>), String> {
    let root = aether_runtime::installed_package("aether").ok_or(
        "no installed aether — run `pesde install`; `pesde.toml` pins the revision          Dew's mods are written against",
    )?;

    let mut aliases = HashMap::new();
    aliases.insert("aether".to_string(), root.join("src"));

    // AETHER'S OWN DEPENDENCY, SUPPLIED BY US. pesde installs each package's own
    // tree without its generated `roblox_packages`, so the framework arrives
    // without the vide it declares — which is handed over through the `@vide`
    // seam VideCore exposes. Without it the framework loads and then reports "no
    // installed vide reachable" from a checkout that is otherwise perfect.
    match aether_runtime::installed_package("vide") {
        Some(vide) => {
            aliases.insert("vide".to_string(), vide.join("src"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_arguments_are_accepted() {
        let args = [
            "--script",
            "test.luau",
            "--size",
            "400x300",
            "--snapshot",
            "out.png",
            "--mod",
            "nameplate",
        ]
        .iter()
        .map(|s| s.to_string());
        assert!(validate_args_iter(args).is_ok());
    }

    #[test]
    fn unknown_arguments_are_refused() {
        let args = ["--unknown-flag"].iter().map(|s| s.to_string());
        let err = validate_args_iter(args).unwrap_err();
        assert_eq!(err, "unrecognised argument '--unknown-flag'");
    }

    #[test]
    fn stats_and_bench_refused_off_windows() {
        for flag in ["--stats", "--bench"] {
            let args = [flag.to_string()].into_iter();
            let err = validate_args_for_platform(args, false).unwrap_err();
            assert_eq!(
                err,
                format!(
                    "'{flag}' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                )
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn stats_and_bench_are_accepted_on_windows() {
        let args = ["--stats", "--bench"].into_iter().map(String::from);
        assert!(validate_args_iter(args).is_ok());
    }

    #[cfg(not(windows))]
    #[test]
    fn stats_and_bench_are_refused_on_non_windows_target() {
        for flag in ["--stats", "--bench"] {
            let args = [flag.to_string()].into_iter();
            let err = validate_args_iter(args).unwrap_err();
            assert_eq!(
                err,
                format!(
                    "'{flag}' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                )
            );
        }
    }
}
