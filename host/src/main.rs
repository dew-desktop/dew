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
use dew_host::manifest;
mod mods;
mod surface;
#[cfg(windows)]
mod tray;
use dew_host::datamodel::input;
use dew_host::services::{self, SharedClock};
use dew_raster::Backend;
use dew_runtime::{Driver, RasterPainter, Rgb};
#[cfg(windows)]
use dew_window::{Button, Event, Window};
use mlua::Lua;
use std::collections::HashMap;
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
/// TWO ENUMS RATHER THAN ONE SHARED ONE. `dew_window::Button` is what the
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
                dew_runtime::Painter::paint_frame(painter, &frame, *background);

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
                .pointer(dew_runtime::Pointer::Move, x, y)
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
                    .pointer(dew_runtime::Pointer::Down, x, y)
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
                    .pointer(dew_runtime::Pointer::Up, x, y)
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

    #[cfg(windows)]
    fn key(&mut self, name: &str) -> Result<(), String> {
        match self {
            Renderer::Aether { .. } => Ok(()),
            Renderer::DataModel {
                dom,
                root,
                width,
                height,
                lua,
                pointer,
                ..
            } => pointer
                .key(
                    &input::Surface {
                        lua,
                        dom,
                        root: *root,
                        size: (*width, *height),
                    },
                    name,
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

    let caps = dew_runtime::Capabilities {
        require_roots: vec![dir.clone()],
        print: true,
        aliases: HashMap::new(),
    };
    let vm = dew_runtime::Vm::new(caps).map_err(|e| e.to_string())?;

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
    dew_runtime::Painter::paint_frame(&mut surface, &frame, Some(BACKGROUND));
    Ok((format!("{drawn} node(s)"), surface))
}

fn create_renderer(
    mounted: mods::Mounted,
    vm: &dew_runtime::Vm,
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

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Run {
        mod_id: Option<String>,
        stats: bool,
        bench: bool,
    },
    Snapshot {
        target: SnapshotTarget,
        output: String,
    },
    Check {
        target: Option<String>,
    },
    Init {
        name: String,
        runtime: manifest::Runtime,
        surface: String,
        size: (u32, u32),
    },
    Test {
        filter: Option<String>,
        dir: Option<PathBuf>,
    },
    Conformance {
        filter: Option<String>,
        dir: Option<PathBuf>,
        pixel: bool,
        generate_goldens: bool,
        run_unsupported: bool,
    },
    Help {
        subcommand: Option<String>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum SnapshotTarget {
    Mod(Option<String>),
    Script {
        path: String,
        width: u32,
        height: u32,
    },
}

pub fn parse_args<I: Iterator<Item = String>>(
    args: I,
    is_windows: bool,
) -> Result<Command, String> {
    let args_vec: Vec<String> = args.collect();
    if args_vec.is_empty() {
        if is_windows {
            return Ok(Command::Run {
                mod_id: None,
                stats: false,
                bench: false,
            });
        } else {
            return Err("no headless action specified (use --script or --snapshot)".to_string());
        }
    }

    let first = &args_vec[0];
    match first.as_str() {
        "run" => parse_run(&args_vec[1..], is_windows),
        "snapshot" => parse_snapshot(&args_vec[1..]),
        "check" => parse_check(&args_vec[1..]),
        "init" | "scaffold" => parse_init(&args_vec[1..]),
        "test" => parse_test(&args_vec[1..]),
        "conformance" => parse_conformance(&args_vec[1..]),
        "help" | "--help" | "-h" => Ok(Command::Help {
            subcommand: args_vec.get(1).cloned(),
        }),
        _ if first.starts_with("--") => parse_legacy_flags(&args_vec, is_windows),
        _ => Err(format!("unrecognised argument '{first}'")),
    }
}

fn parse_run(args: &[String], is_windows: bool) -> Result<Command, String> {
    let mut mod_id = None;
    let mut stats = false;
    let mut bench = false;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--mod" | "-m" => {
                let val = iter.next().ok_or("missing value for --mod")?;
                mod_id = Some(val.clone());
            }
            "--stats" => {
                if !is_windows {
                    return Err(
                        "'--stats' is Windows-only: it drives the window loop; use --snapshot to render headlessly".to_string()
                    );
                }
                stats = true;
            }
            "--bench" => {
                if !is_windows {
                    return Err(
                        "'--bench' is Windows-only: it drives the window loop; use --snapshot to render headlessly".to_string()
                    );
                }
                bench = true;
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }

    if !is_windows {
        return Err(
            "'run' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                .to_string(),
        );
    }

    Ok(Command::Run {
        mod_id,
        stats,
        bench,
    })
}

fn parse_snapshot(args: &[String]) -> Result<Command, String> {
    let mut mod_id = None;
    let mut script = None;
    let mut size = (400, 300);
    let mut output = None;
    let mut positionals = Vec::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--mod" | "-m" => {
                let val = iter.next().ok_or("missing value for --mod")?;
                mod_id = Some(val.clone());
            }
            "--script" | "-s" => {
                let val = iter.next().ok_or("missing value for --script")?;
                script = Some(val.clone());
            }
            "--size" => {
                let val = iter.next().ok_or("missing value for --size")?;
                let (w, h) = val
                    .split_once('x')
                    .ok_or_else(|| format!("invalid size '{val}', expected WxH (e.g. 400x300)"))?;
                let width: u32 = w
                    .trim()
                    .parse()
                    .map_err(|_| format!("invalid width in '{val}'"))?;
                let height: u32 = h
                    .trim()
                    .parse()
                    .map_err(|_| format!("invalid height in '{val}'"))?;
                size = (width, height);
            }
            "--output" | "--out" | "-o" => {
                let val = iter.next().ok_or("missing value for --output")?;
                output = Some(val.clone());
            }
            s if !s.starts_with('-') => {
                positionals.push(s.to_string());
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }

    if let Some(out) = output {
        if let Some(pos) = positionals.first() {
            if mod_id.is_none() && script.is_none() {
                mod_id = Some(pos.clone());
            } else {
                return Err(format!("unexpected argument '{pos}'"));
            }
        }
        output = Some(out);
    } else {
        match positionals.len() {
            0 => {}
            1 => {
                let pos = positionals.remove(0);
                if mod_id.is_some()
                    || script.is_some()
                    || pos.ends_with(".png")
                    || pos.contains('/')
                    || pos.contains('\\')
                {
                    output = Some(pos);
                } else {
                    mod_id = Some(pos);
                }
            }
            2 => {
                if mod_id.is_some() || script.is_some() {
                    return Err(format!("unexpected argument '{}'", positionals[1]));
                }
                mod_id = Some(positionals.remove(0));
                output = Some(positionals.remove(0));
            }
            _ => return Err(format!("unexpected argument '{}'", positionals[2])),
        }
    }

    let out = output.unwrap_or_else(|| "dew.png".to_string());
    let target = if let Some(path) = script {
        SnapshotTarget::Script {
            path,
            width: size.0,
            height: size.1,
        }
    } else {
        SnapshotTarget::Mod(mod_id)
    };

    Ok(Command::Snapshot {
        target,
        output: out,
    })
}

fn parse_check(args: &[String]) -> Result<Command, String> {
    let mut target = None;
    for arg in args {
        if arg.starts_with('-') {
            return Err(format!("unrecognised argument '{arg}'"));
        }
        if target.is_some() {
            return Err(format!("unexpected argument '{arg}'"));
        }
        target = Some(arg.clone());
    }
    Ok(Command::Check { target })
}

fn parse_init(args: &[String]) -> Result<Command, String> {
    let mut name = None;
    let mut runtime = manifest::Runtime::DataModel;
    let mut surface = "window".to_string();
    let mut size = (340, 180);

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--runtime" | "-r" => {
                let val = iter.next().ok_or("missing value for --runtime")?;
                runtime = match val.to_lowercase().as_str() {
                    "datamodel" => manifest::Runtime::DataModel,
                    "aether" => manifest::Runtime::Aether,
                    _ => {
                        return Err(format!(
                            "unknown runtime '{val}', expected 'datamodel' or 'aether'"
                        ))
                    }
                };
            }
            "--surface" => {
                let val = iter.next().ok_or("missing value for --surface")?;
                surface = val.clone();
            }
            "--size" => {
                let val = iter.next().ok_or("missing value for --size")?;
                let (w, h) = val
                    .split_once('x')
                    .ok_or_else(|| format!("invalid size '{val}', expected WxH (e.g. 340x180)"))?;
                let width: u32 = w
                    .trim()
                    .parse()
                    .map_err(|_| format!("invalid width in '{val}'"))?;
                let height: u32 = h
                    .trim()
                    .parse()
                    .map_err(|_| format!("invalid height in '{val}'"))?;
                size = (width, height);
            }
            s if !s.starts_with('-') => {
                if name.is_none() {
                    name = Some(s.to_string());
                } else {
                    return Err(format!("unexpected argument '{s}'"));
                }
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }

    let name = name.ok_or("missing mod name for init (usage: dew init <name>)")?;
    Ok(Command::Init {
        name,
        runtime,
        surface,
        size,
    })
}

fn parse_test(args: &[String]) -> Result<Command, String> {
    let mut filter: Option<String> = None;
    let mut dir: Option<PathBuf> = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dir" | "-d" => {
                let val = iter.next().ok_or("missing value for --dir")?;
                dir = Some(PathBuf::from(val));
            }
            s if !s.starts_with('-') => {
                if filter.is_none() {
                    filter = Some(s.to_string());
                } else {
                    return Err(format!("unexpected argument '{s}'"));
                }
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }

    Ok(Command::Test { filter, dir })
}

fn parse_conformance(args: &[String]) -> Result<Command, String> {
    let mut filter: Option<String> = None;
    let mut dir: Option<PathBuf> = None;
    let mut pixel = false;
    let mut generate_goldens = false;
    let mut run_unsupported = false;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dir" | "-d" => {
                let val = iter.next().ok_or("missing value for --dir")?;
                dir = Some(PathBuf::from(val));
            }
            "--pixel" | "-p" => {
                pixel = true;
            }
            "--generate-goldens" => {
                pixel = true;
                generate_goldens = true;
            }
            "--run-unsupported" => {
                run_unsupported = true;
            }
            s if !s.starts_with('-') => {
                if filter.is_none() {
                    filter = Some(s.to_string());
                } else {
                    return Err(format!("unexpected argument '{s}'"));
                }
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }

    Ok(Command::Conformance {
        filter,
        dir,
        pixel,
        generate_goldens,
        run_unsupported,
    })
}

fn parse_legacy_flags(args: &[String], is_windows: bool) -> Result<Command, String> {
    let mut iter = args.iter();
    let mut script = None;
    let mut size = (400, 300);
    let mut snapshot_out = None;
    let mut mod_id = None;
    let mut stats = false;
    let mut bench = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--script" => {
                let val = iter.next().ok_or("missing value for --script")?;
                script = Some(val.clone());
            }
            "--size" => {
                let val = iter.next().ok_or("missing value for --size")?;
                if let Some((w, h)) = val.split_once('x') {
                    if let (Ok(w), Ok(h)) = (w.trim().parse(), h.trim().parse()) {
                        size = (w, h);
                    }
                }
            }
            "--snapshot" => {
                let val = iter.next().ok_or("missing value for --snapshot")?;
                snapshot_out = Some(val.clone());
            }
            "--mod" => {
                let val = iter.next().ok_or("missing value for --mod")?;
                mod_id = Some(val.clone());
            }
            "--stats" => {
                if !is_windows {
                    return Err(format!(
                        "'{arg}' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                    ));
                }
                stats = true;
            }
            "--bench" => {
                if !is_windows {
                    return Err(format!(
                        "'{arg}' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                    ));
                }
                bench = true;
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }

    if let Some(out) = snapshot_out {
        let target = if let Some(s) = script {
            SnapshotTarget::Script {
                path: s,
                width: size.0,
                height: size.1,
            }
        } else {
            SnapshotTarget::Mod(mod_id)
        };
        Ok(Command::Snapshot {
            target,
            output: out,
        })
    } else if let Some(s) = script {
        Ok(Command::Snapshot {
            target: SnapshotTarget::Script {
                path: s,
                width: size.0,
                height: size.1,
            },
            output: "dew.png".to_string(),
        })
    } else {
        if !is_windows {
            return Err("no headless action specified (use --script or --snapshot)".to_string());
        }
        Ok(Command::Run {
            mod_id,
            stats,
            bench,
        })
    }
}

fn load_active_mod(wanted: Option<&str>) -> Result<mods::Mod, String> {
    let mods_dir = find_dir("mods").ok_or("could not find a `mods` directory")?;
    let (aether_root, aliases) = aether_aliases()?;

    let state = Arc::new(Mutex::new(capabilities::HostState::default()));

    let entries = std::fs::read_dir(&mods_dir).map_err(|e| e.to_string())?;
    let mut loaded = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        match mods::load(&dir, &aether_root, &aliases, &state) {
            Ok(m) => loaded.push(m),
            Err(message) => eprintln!("[dew] skipping mod: {message}"),
        }
    }

    if loaded.is_empty() {
        return Err(format!("no mods loaded from {}", mods_dir.display()));
    }

    match wanted {
        Some(id) => {
            // `--mod` TAKES AN ID, AND A PATH IS WHAT PEOPLE TRY FIRST.
            //
            // Shell completion offers `mods/nameplate/` and the flag reads like it
            // wants one, so `--mod ./mods/nameplate/` is the natural first guess.
            // It failed with `no mod with id ".\mods\nameplate\\"`, which
            // names the mistake without saying what to type instead -- and for a
            // directory that IS a mod, the id is sitting right there in its
            // manifest.
            let looks_like_a_path = id.contains('/') || id.contains('\\') || id.starts_with('.');
            let ids: Vec<&str> = loaded.iter().map(|m| m.manifest.id.as_str()).collect();

            if let Some(found) = loaded.iter().position(|m| m.manifest.id == id) {
                return Ok(loaded.into_iter().nth(found).expect("found"));
            }

            if looks_like_a_path {
                let leaf = id
                    .trim_end_matches(['/', '\\'])
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&id);
                if let Some(m) = loaded.iter().find(|m| m.manifest.id == leaf) {
                    return Err(format!(
                        "--mod takes an id, not a path. That directory's mod is {:?}, so: --mod {}",
                        m.manifest.id, m.manifest.id
                    ));
                }
                return Err(format!(
                    "--mod takes an id, not a path, and nothing loaded from {id:?}. Loaded: {}",
                    ids.join(", ")
                ));
            }

            Err(format!("no mod with id {id:?}. Loaded: {}", ids.join(", ")))
        }
        None => {
            let mut sorted = loaded;
            sorted.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
            let ids: Vec<&str> = sorted.iter().map(|m| m.manifest.id.as_str()).collect();
            println!("[dew] mods: {} (pick one with --mod <id>)", ids.join(", "));
            Ok(sorted.into_iter().next().expect("non-empty"))
        }
    }
}

fn execute_snapshot(target: SnapshotTarget, output: String) -> Result<(), String> {
    match target {
        SnapshotTarget::Script {
            path,
            width,
            height,
        } => {
            let (report, mut surface) = run_script(&path, width, height)?;
            surface
                .write_png(&output)
                .map_err(|code| format!("could not write {output}: rasteriser status {code}"))?;
            println!("[dew] {path}: {report} -> {output} ({width}x{height})");
            Ok(())
        }
        SnapshotTarget::Mod(wanted) => {
            let active = load_active_mod(wanted.as_deref())?;
            let mods::Mod {
                manifest,
                width,
                height,
                surface,
                mounted,
                clock,
                vm,
            } = active;

            let mut renderer = create_renderer(mounted, &vm, &clock, &surface, width, height)?;
            renderer.frame(1.0 / 60.0)?;
            renderer
                .painter_mut()
                .write_png(&output)
                .map_err(|code| format!("could not write {output}: rasteriser status {code}"))?;
            println!(
                "[dew] wrote {output} ({width}x{height}) for {}",
                manifest.id
            );
            Ok(())
        }
    }
}

fn execute_run(wanted: Option<&str>, stats: bool, bench: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (wanted, stats, bench);
        Err(
            "'run' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                .to_string(),
        )
    }

    #[cfg(windows)]
    {
        println!("💧 Dew starting");
        let active = load_active_mod(wanted)?;
        let mods::Mod {
            manifest,
            mut width,
            mut height,
            surface,
            mounted,
            clock,
            vm,
        } = active;

        let screen = dew_window::screen_size();
        if surface.fills_screen() {
            width = screen.0.max(1) as u32;
            height = screen.1.max(1) as u32;
        }

        if let mods::Mounted::DataModel { ref dom, .. } = mounted {
            dom.lock().expect("dom").assets.set_blocking(false);
        }

        let mut renderer = create_renderer(mounted, &vm, &clock, &surface, width, height)?;

        let resolved = surface.resolve(screen, (width, height));
        let mut window = Window::new(&resolved, width, height)?;

        let _keep_alive = vm;

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
            Err(message) => {
                eprintln!("[dew] no tray icon: {message}");
                None
            }
        };

        let mut last = Instant::now();
        let (mut frames, mut painted_frames) = (0u32, 0u32);
        let (mut sum_frame, mut sum_present) = (Duration::ZERO, Duration::ZERO);
        let mut last_report = Instant::now();

        while let Some(events) = window.poll() {
            for event in events {
                match event {
                    Event::PointerMove { x, y } => renderer.moved(x, y)?,
                    Event::PointerDown { x, y, button } => renderer.down(button, x, y)?,
                    Event::PointerUp { x, y, button } => renderer.up(button, x, y)?,
                    Event::Wheel { x, y, delta } => {
                        renderer.wheel(x, y, delta)?;
                    }
                    Event::Resized { .. } | Event::Exposed => renderer.invalidate(),
                    Event::CloseRequested => return Ok(()),
                    Event::Key { name, .. } => renderer.key(&name)?,
                    Event::Char(_) => {}
                }
            }

            let dt = last.elapsed().as_secs_f32();
            last = Instant::now();

            if bench {
                renderer.invalidate();
            }

            let t0 = Instant::now();
            let painted = renderer
                .frame(dt)
                .map_err(|e| format!("while rendering: {e}"))?;
            let t_frame = t0.elapsed();

            let t1 = Instant::now();
            if let Some(bgra) = renderer.painter_mut().canvas_mut().bgra() {
                window.present(bgra, width, height);
            }
            let t_raster = t1.elapsed();

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

            if let Some(target) = tray::frame_budget() {
                let elapsed = last.elapsed();
                if elapsed < target {
                    std::thread::sleep(target - elapsed);
                }
            }
        }

        Ok(())
    }
}

fn check_mod_dir(dir: &Path) -> Result<(manifest::Manifest, PathBuf, Vec<String>), String> {
    let manifest = manifest::Manifest::load(dir)?;
    let entry = manifest
        .entry(dir)
        .ok_or_else(|| format!("{}: no {}.luau or main.luau", dir.display(), manifest.id))?;

    let lua = Lua::new();
    let src = std::fs::read_to_string(&entry)
        .map_err(|e| format!("{}: could not read entry file: {e}", entry.display()))?;
    lua.load(&src)
        .set_name(entry.file_name().unwrap_or_default().to_string_lossy())
        .into_function()
        .map_err(|e| format!("{}: syntax error: {e}", entry.display()))?;

    let mut warnings = manifest.unhonoured();
    for key in &manifest.unknown {
        warnings.push(format!("{key:?} is not a field Dew reads, and was ignored"));
    }

    Ok((manifest, entry, warnings))
}

fn execute_check(target: Option<String>) -> Result<(), String> {
    let dirs: Vec<PathBuf> = match target {
        Some(t) => {
            let p = PathBuf::from(&t);
            if p.is_dir() {
                vec![p]
            } else if let Some(mods_dir) = find_dir("mods") {
                let candidate = mods_dir.join(&t);
                if candidate.is_dir() {
                    vec![candidate]
                } else {
                    return Err(format!(
                        "no directory or mod found at '{t}' (checked '{}')",
                        candidate.display()
                    ));
                }
            } else {
                return Err(format!("no directory or mod found at '{t}'"));
            }
        }
        None => {
            if Path::new("mod.json").is_file() {
                vec![PathBuf::from(".")]
            } else if let Some(mods_dir) = find_dir("mods") {
                let mut found = Vec::new();
                for entry in std::fs::read_dir(&mods_dir)
                    .map_err(|e| e.to_string())?
                    .flatten()
                {
                    if entry.path().is_dir() {
                        found.push(entry.path());
                    }
                }
                found.sort();
                found
            } else {
                return Err("could not find a `mods` directory or a local `mod.json`".to_string());
            }
        }
    };

    if dirs.is_empty() {
        return Err("no mods found to check".to_string());
    }

    let mut failed = 0;
    for dir in &dirs {
        match check_mod_dir(dir) {
            Ok((manifest, entry, warnings)) => {
                println!(
                    "[dew] check {}: ok ({}, entry: {})",
                    manifest.id,
                    manifest.runtime.name(),
                    entry.file_name().unwrap_or_default().to_string_lossy()
                );
                for warn in warnings {
                    println!("[dew] {}: warning: {warn}", manifest.id);
                }
            }
            Err(err) => {
                failed += 1;
                eprintln!("[dew] check failed: {err}");
            }
        }
    }

    if failed > 0 {
        Err(format!("{failed} mod(s) failed validation"))
    } else {
        Ok(())
    }
}

fn execute_init(
    name: String,
    runtime: manifest::Runtime,
    _surface: String,
    size: (u32, u32),
) -> Result<(), String> {
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "invalid mod name '{name}': use alphanumeric characters, dashes, or underscores"
        ));
    }

    let target_dir = if name.contains('/') || name.contains('\\') {
        PathBuf::from(&name)
    } else if let Some(mods_dir) = find_dir("mods") {
        mods_dir.join(&name)
    } else {
        PathBuf::from(&name)
    };

    if target_dir.exists() {
        return Err(format!(
            "directory '{}' already exists",
            target_dir.display()
        ));
    }

    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("could not create directory {}: {e}", target_dir.display()))?;

    let display_name = {
        let mut chars = name.chars();
        match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    };

    let mod_json = serde_json::json!({
        "id": name,
        "name": display_name,
        "description": format!("A Dew mod ({})", runtime.name()),
        "runtime": runtime.name(),
        "permissions": []
    });

    let mod_json_str = serde_json::to_string_pretty(&mod_json).map_err(|e| e.to_string())?;
    let mod_json_path = target_dir.join("mod.json");
    std::fs::write(&mod_json_path, mod_json_str + "\n")
        .map_err(|e| format!("could not write {}: {e}", mod_json_path.display()))?;

    let entry_code = match runtime {
        manifest::Runtime::DataModel => format!(
            r#"--!strict
--[[
	{display_name} -- a Dew mod written against the native DataModel.
]]

local function mount(_dew: any, root: Instance)
	local frame = Instance.new("Frame")
	frame.Name = "Root"
	frame.Size = UDim2.fromScale(1, 1)
	frame.BackgroundColor3 = Color3.fromRGB(30, 30, 35)
	frame.BorderSizePixel = 0
	frame.Parent = root

	local label = Instance.new("TextLabel")
	label.Name = "Title"
	label.Size = UDim2.new(1, 0, 0, 40)
	label.Position = UDim2.new(0, 0, 0.5, -20)
	label.BackgroundTransparency = 1
	label.Text = "Hello from {display_name}!"
	label.TextColor3 = Color3.fromRGB(240, 240, 245)
	label.TextSize = 20
	label.Parent = frame
end

return {{
	size = {{ width = {}, height = {} }},
	mount = mount,
}}
"#,
            size.0, size.1
        ),
        manifest::Runtime::Aether => format!(
            r#"--!strict
--[[
	{display_name} -- a Dew mod written against Aether.
]]

local aether = require("@aether/api")

local function mount(_dew: any)
	return aether.create("Frame", {{
		Name = "Root",
		Size = aether.UDim2.fromScale(1, 1),
		BackgroundColor3 = aether.Color3.fromRGB(30, 30, 35),
		BorderSizePixel = 0,
	}}, {{
		aether.create("TextLabel", {{
			Name = "Title",
			Size = aether.UDim2.new(1, 0, 0, 40),
			Position = aether.UDim2.new(0, 0, 0.5, -20),
			BackgroundTransparency = 1,
			Text = "Hello from {display_name}!",
			TextColor3 = aether.Color3.fromRGB(240, 240, 245),
			TextSize = 20,
		}}),
	}})
end

return {{
	size = {{ width = {}, height = {} }},
	mount = mount,
}}
"#,
            size.0, size.1
        ),
    };

    let entry_path = target_dir.join(format!("{name}.luau"));
    std::fs::write(&entry_path, entry_code)
        .map_err(|e| format!("could not write {}: {e}", entry_path.display()))?;

    println!("[dew] initialized mod '{name}' in {}", target_dir.display());
    println!("[dew] check with: dew check {name}");
    println!("[dew] snapshot with: dew snapshot --mod {name}");
    Ok(())
}

fn execute_test(filter: Option<String>, dir: Option<PathBuf>) -> Result<(), String> {
    let target_dir = match dir {
        Some(d) => {
            if !d.is_dir() {
                return Err(format!("directory '{}' does not exist", d.display()));
            }
            d.canonicalize().unwrap_or(d)
        }
        None => {
            let cur = std::env::current_dir().map_err(|e| e.to_string())?;
            if cur.join("tests").is_dir() || cur.join("src").join("api.luau").is_file() {
                cur
            } else if let Some(aether) = find_dir("aether") {
                aether
            } else if PathBuf::from("../aether").is_dir() {
                PathBuf::from("../aether")
                    .canonicalize()
                    .unwrap_or_else(|_| PathBuf::from("../aether"))
            } else {
                cur
            }
        }
    };

    let temp_dir = std::env::temp_dir().join("dew_test_lune");
    let lune_dir = temp_dir.join("lune");
    std::fs::create_dir_all(&lune_dir)
        .map_err(|e| format!("could not create lune shim directory: {e}"))?;
    let process_luau = r#"
local process = {}
function process.exit(code: number?)
    local c = code or 0
    if c ~= 0 then
        error(string.format("[dew test] process.exit(%d)", c), 0)
    end
end
process.args = {}
process.env = {}
return process
"#;
    std::fs::write(lune_dir.join("process.luau"), process_luau)
        .map_err(|e| format!("could not write lune shim process.luau: {e}"))?;

    fn discover(dir: &Path, found: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if [
                "roblox_packages",
                "luau_packages",
                ".pesde",
                "target",
                "node_modules",
                ".git",
            ]
            .contains(&name.as_str())
            {
                continue;
            }
            if path.is_dir() {
                discover(&path, found);
            } else if name.ends_with(".test.luau") {
                found.push(path);
            }
        }
    }

    let mut suites = Vec::new();
    discover(&target_dir, &mut suites);
    suites.sort();

    let filter_lower = filter.as_ref().map(|s| s.to_lowercase());
    let matching_suites: Vec<PathBuf> = suites
        .into_iter()
        .filter(|p| {
            if let Some(ref f) = filter_lower {
                p.to_string_lossy().to_lowercase().contains(f)
            } else {
                true
            }
        })
        .collect();

    if matching_suites.is_empty() {
        if let Some(ref f) = filter {
            return Err(format!(
                "no test suites matched filter '{f}' in {}",
                target_dir.display()
            ));
        } else {
            return Err(format!(
                "no test suites (*.test.luau) found in {}",
                target_dir.display()
            ));
        }
    }

    let aether_root = if target_dir.join("src").join("api.luau").is_file() {
        target_dir.clone()
    } else if let Some(d) = find_dir("aether") {
        d
    } else if let Some(p) = dew_runtime::installed_package("aether") {
        p
    } else {
        target_dir.clone()
    };

    let mut vide_src = None;
    let roblox_packages = aether_root.join("roblox_packages");
    let pesde_dir = roblox_packages.join(".pesde");
    if pesde_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&pesde_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("centau_vide@") {
                    let candidate = entry.path().join("vide").join("src");
                    if candidate.is_dir() {
                        vide_src = Some(candidate);
                        break;
                    }
                }
            }
        }
    }
    if vide_src.is_none() {
        if let Some(vide_pkg) = dew_runtime::installed_package("vide") {
            vide_src = Some(vide_pkg.join("src"));
        }
    }

    let total = matching_suites.len();
    let mut passed = 0;
    let mut failed = 0;

    for suite_path in &matching_suites {
        let rel_path = suite_path.strip_prefix(&target_dir).unwrap_or(suite_path);
        let suite_name = rel_path.to_string_lossy().replace('\\', "/");

        let suite_dir = suite_path.parent().unwrap_or(&target_dir);
        let mut roots = vec![suite_dir.to_path_buf(), aether_root.clone()];
        if roblox_packages.is_dir() {
            roots.push(roblox_packages.clone());
        }

        let mut aliases = HashMap::new();
        aliases.insert("aether".to_string(), aether_root.join("src"));
        aliases.insert(
            "testkit".to_string(),
            aether_root.join("tests").join("testkit"),
        );
        aliases.insert("lune".to_string(), lune_dir.clone());
        if let Some(ref v) = vide_src {
            aliases.insert("vide".to_string(), v.clone());
        }

        let caps = dew_runtime::Capabilities {
            require_roots: roots,
            print: true,
            aliases,
        };

        let vm = match dew_runtime::Vm::new(caps.clone()) {
            Ok(v) => v,
            Err(e) => {
                failed += 1;
                eprintln!("::error::{suite_name}");
                eprintln!("  VM creation failed: {e}");
                continue;
            }
        };

        let dom = dew_host::datamodel::SharedDom::default();
        if let Err(e) = dew_host::datamodel::install(vm.lua(), &dom) {
            failed += 1;
            eprintln!("::error::{suite_name}");
            eprintln!("  DataModel installation failed: {e}");
            continue;
        }
        if let Err(e) = dew_host::datamodel::install_vocabulary(vm.lua()) {
            failed += 1;
            eprintln!("::error::{suite_name}");
            eprintln!("  Vocabulary installation failed: {e}");
            continue;
        }
        let clock: crate::services::SharedClock =
            Arc::new(Mutex::new(crate::services::Clock::default()));
        if let Err(e) = crate::services::install(vm.lua(), &clock) {
            failed += 1;
            eprintln!("::error::{suite_name}");
            eprintln!("  Services installation failed: {e}");
            continue;
        }

        // Test runner provides a steppable clock via DewHost.Clock.Step(dt)
        // so transition tests can step simulated time. This is strictly isolated
        // to `dew test` and absent in guest mods run via `dew run` / `dew snapshot`.
        let step_clock = Arc::clone(&clock);
        if let Ok(dew_host) = vm.lua().globals().get::<mlua::Table>("DewHost") {
            if let Ok(dew_clock) = dew_host.get::<mlua::Table>("Clock") {
                let _ = dew_clock.set(
                    "Step",
                    match vm.lua().create_function(move |_, dt: Option<f32>| {
                        crate::services::tick(&step_clock, dt.unwrap_or(1.0 / 60.0));
                        Ok(())
                    }) {
                        Ok(f) => f,
                        Err(e) => {
                            failed += 1;
                            eprintln!("::error::{suite_name}");
                            eprintln!("  Clock.Step installation failed: {e}");
                            continue;
                        }
                    },
                );
            }
        }

        // Provide os.clock for test suites that measure relative time (e.g. Scrollbar)
        let os_clock = Arc::clone(&clock);
        let os_table = match vm.lua().create_table() {
            Ok(t) => t,
            Err(e) => {
                failed += 1;
                eprintln!("::error::{suite_name}");
                eprintln!("  os table creation failed: {e}");
                continue;
            }
        };
        if let Err(e) = os_table.set(
            "clock",
            match vm
                .lua()
                .create_function(move |_, ()| Ok(os_clock.lock().unwrap().now()))
            {
                Ok(f) => f,
                Err(e) => {
                    failed += 1;
                    eprintln!("::error::{suite_name}");
                    eprintln!("  os.clock function creation failed: {e}");
                    continue;
                }
            },
        ) {
            failed += 1;
            eprintln!("::error::{suite_name}");
            eprintln!("  os.clock installation failed: {e}");
            continue;
        }
        if let Err(e) = vm.lua().globals().set("os", os_table) {
            failed += 1;
            eprintln!("::error::{suite_name}");
            eprintln!("  os global installation failed: {e}");
            continue;
        }
        if let Err(e) = dew_runtime::modules::install(&vm, &caps) {
            failed += 1;
            eprintln!("::error::{suite_name}");
            eprintln!("  Require installation failed: {e}");
            continue;
        }

        let source = match std::fs::read_to_string(suite_path) {
            Ok(s) => s,
            Err(e) => {
                failed += 1;
                eprintln!("::error::{suite_name}");
                eprintln!("  Read error: {e}");
                continue;
            }
        };

        let mut stem = dew_runtime::strip_extended_prefix(
            suite_path
                .canonicalize()
                .unwrap_or_else(|_| suite_path.to_path_buf()),
        );
        stem.set_extension("");
        let chunk_name = format!("@{}", stem.display());

        let result: mlua::prelude::LuaResult<mlua::prelude::LuaValue> = (|| {
            let func = vm
                .lua()
                .load(&source)
                .set_name(&chunk_name)
                .into_function()?;
            func.call(())
        })();

        match result {
            Ok(_) => {
                passed += 1;
            }
            Err(e) => {
                failed += 1;
                eprintln!("::error::{suite_name}");
                eprintln!("  {e}");
            }
        }
    }

    println!();
    if failed > 0 {
        println!("SUITES: FAILED ({passed} passed, {failed} failed of {total})");
        Err(format!("{failed} of {total} suites failed"))
    } else {
        println!("SUITES: PASS ({passed} of {total})");
        Ok(())
    }
}

fn execute_help(subcommand: Option<String>) {
    match subcommand.as_deref() {
        Some("snapshot") => {
            println!("Usage: dew snapshot [OPTIONS] [OUTPUT]");
            println!();
            println!("Render a mod or standalone script headlessly to a PNG image.");
            println!();
            println!("Options:");
            println!("  --mod, -m <ID>        Mod to snapshot (defaults to first available mod)");
            println!("  --script, -s <PATH>   Standalone script to execute and snapshot");
            println!("  --size <WxH>          Dimensions for standalone script (default: 400x300)");
            println!("  --output, -o <PATH>   Output PNG path (default: dew.png)");
        }
        Some("run") => {
            println!("Usage: dew run [OPTIONS]");
            println!();
            println!("Run a mod interactively in a desktop window (Windows only).");
            println!();
            println!("Options:");
            println!("  --mod, -m <ID>        Mod to run (defaults to first available mod)");
            println!("  --stats               Print FPS and render timings");
            println!("  --bench               Run in benchmark mode");
        }
        Some("check") => {
            println!("Usage: dew check [TARGET]");
            println!();
            println!("Validate a mod's manifest (mod.json), entrypoint, and Luau syntax.");
            println!();
            println!("Arguments:");
            println!(
                "  [TARGET]              Mod ID or path to directory (checks all mods if omitted)"
            );
        }
        Some("init") | Some("scaffold") => {
            println!("Usage: dew init <NAME> [OPTIONS]");
            println!();
            println!("Scaffold a new Dew mod with a valid manifest and working entrypoint.");
            println!();
            println!("Arguments:");
            println!("  <NAME>                Mod identifier and directory name");
            println!();
            println!("Options:");
            println!("  --runtime, -r <RT>    Runtime: 'datamodel' (default) or 'aether'");
            println!("  --surface <SURFACE>   Surface: 'window' (default), 'overlay', or 'widget'");
            println!("  --size <WxH>          Default size (default: 340x180)");
        }
        Some("test") => {
            println!("Usage: dew test [FILTER] [OPTIONS]");
            println!();
            println!(
                "Run Luau test suites (*.test.luau) in Dew's VM against Dew's native DataModel."
            );
            println!();
            println!("Arguments:");
            println!("  [FILTER]              Optional substring to filter suite paths");
            println!();
            println!("Options:");
            println!(
                "  --dir, -d <PATH>      Directory to search for suites (defaults to current dir)"
            );
        }
        Some("conformance") => {
            println!("Usage: dew conformance [FILTER] [OPTIONS]");
            println!();
            println!("Run the DataModel Standard layout conformance suite against Dew's native DataModel.");
            println!();
            println!("Arguments:");
            println!("  [FILTER]              Optional substring to filter case names");
            println!();
            println!("Options:");
            println!(
                "  --dir, -d <PATH>      Directory containing cases (defaults to auto-discovery)"
            );
            println!(
                "  --pixel, -p           Enable pixel-level probe and golden image verification"
            );
            println!("  --generate-goldens    Generate reference golden PNGs from rendered output");
            println!("  --run-unsupported     Execute cases marked with unsupported requirements");
        }
        _ => {
            println!("Dew -- desktop applet platform over a native DataModel");
            println!();
            println!("Usage: dew <COMMAND> [OPTIONS]");
            println!();
            println!("Commands:");
            println!("  run          Run a mod interactively in a desktop window (Windows only)");
            println!("  snapshot     Render a mod or script headlessly to a PNG image");
            println!("  check        Validate mod manifest, entrypoint, and Luau syntax");
            println!("  init         Scaffold a new mod with manifest and entrypoint");
            println!("  test         Run Luau test suites against Dew's DataModel");
            println!("  conformance  Run layout conformance suite against Dew's DataModel");
            println!("  help         Show help for a command");
            println!();
            println!("Legacy Flags:");
            println!("  --snapshot <PATH>     Render default or --mod to PNG");
            println!("  --mod <ID>            Select mod for run or snapshot");
            println!("  --script <PATH>       Run standalone script");
            println!("  --size <WxH>          Dimensions for standalone script");
            println!("  --stats, --bench      Performance monitoring (Windows only)");
        }
    }
}

fn execute_conformance(
    filter: Option<String>,
    dir: Option<PathBuf>,
    pixel: bool,
    generate_goldens: bool,
    run_unsupported: bool,
) -> Result<(), String> {
    let cases_dir = dew_host::conformance::find_cases_dir(dir.as_deref())?;
    println!(
        "[dew] running conformance suite from {}{}",
        cases_dir.display(),
        if pixel {
            " [pixel verification enabled]"
        } else {
            ""
        }
    );
    let options = dew_host::conformance::PixelOptions {
        enabled: pixel,
        generate_goldens,
        run_unsupported,
        tolerance: 8,
    };
    let (results, summary) =
        dew_host::conformance::run_suite_with_options(&cases_dir, filter.as_deref(), &options);
    dew_host::conformance::print_report(&results, &summary);
    if summary.undecodable > 0 {
        return Err(format!("{} case(s) failed to decode", summary.undecodable));
    }
    if summary.failed > 0 {
        return Err(format!("{} case(s) failed", summary.failed));
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let cmd = parse_args(std::env::args().skip(1), cfg!(windows))?;
    match cmd {
        Command::Run {
            mod_id,
            stats,
            bench,
        } => execute_run(mod_id.as_deref(), stats, bench),
        Command::Snapshot { target, output } => execute_snapshot(target, output),
        Command::Check { target } => execute_check(target),
        Command::Init {
            name,
            runtime,
            surface,
            size,
        } => execute_init(name, runtime, surface, size),
        Command::Test { filter, dir } => execute_test(filter, dir),
        Command::Conformance {
            filter,
            dir,
            pixel,
            generate_goldens,
            run_unsupported,
        } => execute_conformance(filter, dir, pixel, generate_goldens, run_unsupported),
        Command::Help { subcommand } => {
            execute_help(subcommand);
            Ok(())
        }
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
    // NOTHING IS INJECTED ANY MORE. A mod declares Aether and vide in its own
    // `pesde.toml` and requires them through the redirect pesde writes beside
    // it, exactly as a Roblox place does. The host used to hand every mod an
    // `@aether` and a `@vide` pointing into its OWN installed packages, which
    // meant a mod could not say what it depended on and could not be built
    // without a Dew checkout.
    //
    // The path is still returned because callers thread it through; it names
    // nothing a guest can reach.
    Ok((PathBuf::from("."), HashMap::new()))
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

    fn validate_args_iter<I: Iterator<Item = String>>(args: I) -> Result<(), String> {
        validate_args_for_platform(args, cfg!(windows))
    }

    fn validate_args_for_platform<I: Iterator<Item = String>>(
        args: I,
        is_windows: bool,
    ) -> Result<(), String> {
        parse_args(args, is_windows).map(|_| ())
    }

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

    #[test]
    fn subcommand_snapshot_mod() {
        let args = ["snapshot", "--mod", "nameplate", "out.png"]
            .iter()
            .map(|s| s.to_string());
        let cmd = parse_args(args, true).unwrap();
        match cmd {
            Command::Snapshot { target, output } => {
                assert_eq!(output, "out.png");
                match target {
                    SnapshotTarget::Mod(Some(id)) => assert_eq!(id, "nameplate"),
                    _ => panic!("expected Mod target with nameplate"),
                }
            }
            _ => panic!("expected Snapshot command"),
        }
    }

    #[test]
    fn subcommand_snapshot_positional_mod() {
        let args = ["snapshot", "nameplate", "out.png"]
            .iter()
            .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Snapshot { target, output } => {
                assert_eq!(output, "out.png");
                match target {
                    SnapshotTarget::Mod(Some(id)) => assert_eq!(id, "nameplate"),
                    _ => panic!("expected Mod target with nameplate"),
                }
            }
            _ => panic!("expected Snapshot command"),
        }
    }

    #[test]
    fn subcommand_snapshot_default() {
        let args = ["snapshot", "out.png"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Snapshot { target, output } => {
                assert_eq!(output, "out.png");
                match target {
                    SnapshotTarget::Mod(None) => {}
                    _ => panic!("expected default Mod target"),
                }
            }
            _ => panic!("expected Snapshot command"),
        }
    }

    #[test]
    fn subcommand_snapshot_script() {
        let args = [
            "snapshot",
            "--script",
            "app.luau",
            "--size",
            "360x240",
            "/tmp/app.png",
        ]
        .iter()
        .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Snapshot { target, output } => {
                assert_eq!(output, "/tmp/app.png");
                match target {
                    SnapshotTarget::Script {
                        path,
                        width,
                        height,
                    } => {
                        assert_eq!(path, "app.luau");
                        assert_eq!(width, 360);
                        assert_eq!(height, 240);
                    }
                    _ => panic!("expected Script target"),
                }
            }
            _ => panic!("expected Snapshot command"),
        }
    }

    #[test]
    fn subcommand_run_rejected_off_windows() {
        let args = ["run"].iter().map(|s| s.to_string());
        let err = parse_args(args, false).unwrap_err();
        assert_eq!(
            err,
            "'run' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
        );
    }

    #[test]
    fn subcommand_run_accepted_on_windows() {
        let args = ["run", "--mod", "nameplate"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, true).unwrap();
        match cmd {
            Command::Run {
                mod_id,
                stats,
                bench,
            } => {
                assert_eq!(mod_id.as_deref(), Some("nameplate"));
                assert!(!stats);
                assert!(!bench);
            }
            _ => panic!("expected Run command"),
        }
    }

    #[test]
    fn subcommand_check() {
        let args = ["check", "nameplate"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Check { target } => assert_eq!(target.as_deref(), Some("nameplate")),
            _ => panic!("expected Check command"),
        }
    }

    #[test]
    fn subcommand_init() {
        let args = [
            "init",
            "my_mod",
            "--runtime",
            "datamodel",
            "--size",
            "400x200",
        ]
        .iter()
        .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Init {
                name,
                runtime,
                surface: _,
                size,
            } => {
                assert_eq!(name, "my_mod");
                assert_eq!(runtime, manifest::Runtime::DataModel);
                assert_eq!(size, (400, 200));
            }
            _ => panic!("expected Init command"),
        }
    }

    #[test]
    fn subcommand_scaffold_alias() {
        let args = ["scaffold", "my_mod"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Init { name, .. } => assert_eq!(name, "my_mod"),
            _ => panic!("expected Init command"),
        }
    }

    #[test]
    fn subcommand_help() {
        let args = ["help", "snapshot"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Help { subcommand } => assert_eq!(subcommand.as_deref(), Some("snapshot")),
            _ => panic!("expected Help command"),
        }
    }

    #[test]
    fn check_mod_dir_nameplate() {
        if let Some(mods_dir) = find_dir("mods") {
            let nameplate = mods_dir.join("nameplate");
            if nameplate.is_dir() {
                let (manifest, entry, warnings) = check_mod_dir(&nameplate).unwrap();
                assert_eq!(manifest.id, "nameplate");
                assert!(entry.is_file());
                assert!(warnings.is_empty());
            }
        }
    }

    #[test]
    fn subcommand_test_default() {
        let args = ["test"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Test { filter, dir } => {
                assert_eq!(filter, None);
                assert_eq!(dir, None);
            }
            _ => panic!("expected Test command"),
        }
    }

    #[test]
    fn subcommand_test_filter() {
        let args = ["test", "FloatingMath"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Test { filter, dir } => {
                assert_eq!(filter.as_deref(), Some("FloatingMath"));
                assert_eq!(dir, None);
            }
            _ => panic!("expected Test command"),
        }
    }

    #[test]
    fn subcommand_test_dir() {
        let args = ["test", "--dir", "my_tests"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Test { filter, dir } => {
                assert_eq!(filter, None);
                assert_eq!(dir, Some(PathBuf::from("my_tests")));
            }
            _ => panic!("expected Test command"),
        }
    }

    #[test]
    fn subcommand_test_filter_and_dir() {
        let args = ["test", "FloatingMath", "--dir", "my_tests"]
            .iter()
            .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Test { filter, dir } => {
                assert_eq!(filter.as_deref(), Some("FloatingMath"));
                assert_eq!(dir, Some(PathBuf::from("my_tests")));
            }
            _ => panic!("expected Test command"),
        }
    }

    #[test]
    fn subcommand_test_help() {
        let args = ["help", "test"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Help { subcommand } => assert_eq!(subcommand.as_deref(), Some("test")),
            _ => panic!("expected Help command"),
        }
    }

    #[test]
    fn execute_test_runs_passing_suite() {
        let aether_root = match find_dir("aether") {
            Some(d) => d,
            None => {
                let candidate = PathBuf::from("../aether");
                if candidate.is_dir() {
                    candidate.canonicalize().unwrap()
                } else {
                    return;
                }
            }
        };

        // FloatingMath is pure math and passes under Dew's VM
        let res = execute_test(Some("FloatingMath".into()), Some(aether_root));
        assert!(res.is_ok(), "FloatingMath test suite must pass");
    }

    #[test]
    fn execute_test_runs_standalone_suite() {
        let temp_dir = std::env::temp_dir().join(format!("dew_test_unit_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let suite_path = temp_dir.join("sample.test.luau");
        let _ = std::fs::write(&suite_path, "local x = 42\nassert(x == 42)\n");

        let res = execute_test(None, Some(temp_dir.clone()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        assert!(
            res.is_ok(),
            "Standalone sample suite must pass under execute_test"
        );
    }

    #[test]
    fn subcommand_conformance_default() {
        let args = ["conformance"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Conformance {
                filter,
                dir,
                pixel,
                generate_goldens,
                run_unsupported,
            } => {
                assert_eq!(filter, None);
                assert_eq!(dir, None);
                assert!(!pixel);
                assert!(!generate_goldens);
                assert!(!run_unsupported);
            }
            _ => panic!("expected Conformance command"),
        }
    }

    #[test]
    fn subcommand_conformance_filter() {
        let args = ["conformance", "01_"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Conformance { filter, dir, .. } => {
                assert_eq!(filter.as_deref(), Some("01_"));
                assert_eq!(dir, None);
            }
            _ => panic!("expected Conformance command"),
        }
    }

    #[test]
    fn subcommand_conformance_dir() {
        let args = ["conformance", "--dir", "cases"]
            .iter()
            .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Conformance { filter, dir, .. } => {
                assert_eq!(filter, None);
                assert_eq!(dir, Some(PathBuf::from("cases")));
            }
            _ => panic!("expected Conformance command"),
        }
    }

    #[test]
    fn subcommand_conformance_filter_and_dir() {
        let args = ["conformance", "01_", "--dir", "cases"]
            .iter()
            .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Conformance { filter, dir, .. } => {
                assert_eq!(filter.as_deref(), Some("01_"));
                assert_eq!(dir, Some(PathBuf::from("cases")));
            }
            _ => panic!("expected Conformance command"),
        }
    }

    #[test]
    fn subcommand_conformance_pixel_flags() {
        let args = [
            "conformance",
            "--pixel",
            "--generate-goldens",
            "--run-unsupported",
        ]
        .iter()
        .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Conformance {
                pixel,
                generate_goldens,
                run_unsupported,
                ..
            } => {
                assert!(pixel);
                assert!(generate_goldens);
                assert!(run_unsupported);
            }
            _ => panic!("expected Conformance command"),
        }
    }

    #[test]
    fn subcommand_conformance_help() {
        let args = ["help", "conformance"].iter().map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Help { subcommand } => assert_eq!(subcommand.as_deref(), Some("conformance")),
            _ => panic!("expected Help command"),
        }
    }
}
