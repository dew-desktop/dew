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
//! LAYOUT, HIT TESTING AND POINTER ARBITRATION USED TO BE ON THAT LIST. Dew's
//! own DataModel has no framework under it, so the host answers for itself —
//! `datamodel::render` resolves the geometry and `datamodel::input` says which
//! instance is at (x, y). ADR-001 is why: a host that owns its DataModel owns
//! the questions asked of it.
//!
//! What IS Dew's: mod discovery, manifests, capabilities, and putting widgets on
//! a desktop. That is the whole remit.

mod capabilities;
use dew_host::datamodel;
use dew_host::manifest;
mod applets;
#[cfg(windows)]
mod coordinator;
#[cfg(windows)]
mod installed;
#[cfg(windows)]
mod manage;
#[cfg(windows)]
mod positions;
mod surface;
#[cfg(windows)]
mod tray;
use dew_host::datamodel::input;
use dew_host::services::{self, SharedClock};
use dew_raster::Backend;
use dew_runtime::{RasterPainter, Rgb};
#[cfg(windows)]
use dew_window::{Button, Event, Pump, Surface, Window};
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

/// What the frame loop drives, for the one runtime a mod can declare.
///
/// A DataModel tree answers its own "did anything change" question, because
/// every write a guest can make goes through the path that fires `Changed`,
/// and ends at `Painter::paint_frame`, the seam that has survived four
/// rasterisers.
///
/// THE CLOCK SITS OUTSIDE THE RENDERED SHAPE for the reason `Mod::clock` does:
/// frames are the host's, not the framework's, so it is carried alongside
/// rather than folded into what gets painted.
enum Renderer {
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
    fn moved(
        &mut self,
        x: f32,
        y: f32,
        service_pointer: &services::SharedPointer,
    ) -> Result<(), String> {
        // THE HOST ANSWERS WHERE THE POINTER IS (ADR-010), so it has to know. A
        // guest deciding hover by geometry polls this on frames with no input at
        // all, which is why it is recorded here rather than only delivered.
        //
        // ON THIS APPLET'S OWN CELL, not the process-global one -- a process
        // running more than one applet has more than one cursor position to
        // report, and `services::pointer()` can only ever hold the last one
        // written. See `applets::Applet::pointer`'s doc comment.
        crate::services::pointer_moved_on(service_pointer, x, y);
        match self {
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
    /// THE BUTTON TRAVELS, because a DataModel mod has `MouseButton2Click` and
    /// `SecondaryActivated` and not just a left click.
    #[cfg(windows)]
    fn down(
        &mut self,
        button: Button,
        x: f32,
        y: f32,
        service_pointer: &services::SharedPointer,
    ) -> Result<(), String> {
        crate::services::pointer_moved_on(service_pointer, x, y);
        crate::services::pointer_button_on(service_pointer, button as usize, true);
        match self {
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
    fn up(
        &mut self,
        button: Button,
        x: f32,
        y: f32,
        service_pointer: &services::SharedPointer,
    ) -> Result<(), String> {
        crate::services::pointer_moved_on(service_pointer, x, y);
        crate::services::pointer_button_on(service_pointer, button as usize, false);
        match self {
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
    fn wheel(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        service_pointer: &services::SharedPointer,
    ) -> Result<(), String> {
        crate::services::pointer_moved_on(service_pointer, x, y);
        crate::services::pointer_wheel_on(service_pointer, x, y, delta);
        match self {
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
            Renderer::DataModel { dom, .. } => dom.lock().expect("dom").touch(),
        }
    }

    fn painter_mut(&mut self) -> &mut RasterPainter {
        match self {
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
/// behind it, and those are `dew.Text` and `dew.Clock`, installed above: the two
/// things a guest framework genuinely could not compute for itself, and required
/// of a conforming host by `docs/host_services.md` since 2026-09-04.
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
    // `dew.Text`/`dew.Clock` HERE TOO, so a standalone script measures text the same way a mod
    // does. NOTHING DRIVES THE CLOCK ON THIS PATH and that is honest rather than
    // missing: `--script` draws one frame and exits, so there are no frames to be
    // called on. A script may still subscribe -- it simply never gets a tick,
    // which is the truthful answer for a renderer that runs once.
    let clock: services::SharedClock = Arc::new(Mutex::new(Default::default()));
    services::install(vm.lua(), &clock).map_err(|e| e.to_string())?;

    // The surface the guest parents into. A `ScreenGui` because that is what a
    // The engine application expects to find above its tree, so the same file has a
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
    mounted: applets::Mounted,
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
        applets::Mounted::DataModel { dom, root } => Ok(Renderer::DataModel {
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
        /// The applets to mount, each a directory containing a `dew.toml`.
        applets: Vec<PathBuf>,
        stats: bool,
        bench: bool,
    },
    Snapshot {
        target: SnapshotTarget,
        output: String,
    },
    Check {
        /// Directories to check. Empty means the working directory, if it is one.
        targets: Vec<PathBuf>,
    },
    Install {
        dir: PathBuf,
        force: bool,
    },
    Uninstall {
        id: String,
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
    Applet(PathBuf),
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
            // NOTHING TO RUN WITHOUT A PATH. This used to mount everything in a
            // blessed directory; there is no blessed directory any more, so an
            // empty invocation is a usage question rather than a default.
            return Err(
                "nothing to run: pass an applet directory, e.g. `dew examples/aether/timetracker`"
                    .to_string(),
            );
        } else {
            return Err("no headless action specified (use --script or --snapshot)".to_string());
        }
    }

    let first = &args_vec[0];
    match first.as_str() {
        "run" => parse_run(&args_vec[1..], is_windows),
        "snapshot" => parse_snapshot(&args_vec[1..]),
        "check" => parse_check(&args_vec[1..]),
        "install" => parse_install(&args_vec[1..]),
        "uninstall" => parse_uninstall(&args_vec[1..]),
        "init" | "scaffold" => parse_init(&args_vec[1..]),
        "test" => parse_test(&args_vec[1..]),
        "conformance" => parse_conformance(&args_vec[1..]),
        "help" | "--help" | "-h" => Ok(Command::Help {
            subcommand: args_vec.get(1).cloned(),
        }),
        _ if first.starts_with("--") => parse_legacy_flags(&args_vec, is_windows),
        // A BARE PATH RUNS IT. `dew examples/aether/timetracker` is the shortest true
        // thing to type, and it is what the README documents; requiring `run`
        // in front of it would make the subcommand the only way to do the most
        // common thing.
        //
        // A mistyped subcommand lands here too and reports that there is no such
        // directory, which names the mistake as well as `unrecognised argument`
        // did and better than it did for a path.
        _ => parse_run(&args_vec, is_windows),
    }
}

fn parse_run(args: &[String], is_windows: bool) -> Result<Command, String> {
    let mut applets: Vec<PathBuf> = Vec::new();
    let mut stats = false;
    let mut bench = false;

    for arg in args.iter() {
        match arg.as_str() {
            "--applet" | "-a" => {
                return Err(
                    "`--applet` is gone: pass the applet's directory, e.g. `dew examples/aether/timetracker`"
                        .to_string(),
                );
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
            // A PATH, NOT A NAME. Anything that is not a flag is an applet
            // directory, so the shape of the argument says what it is.
            _ if !arg.starts_with('-') => applets.push(PathBuf::from(arg)),
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
        applets,
        stats,
        bench,
    })
}

fn parse_snapshot(args: &[String]) -> Result<Command, String> {
    let mut applet: Option<PathBuf> = None;
    let mut script = None;
    let mut size = (400, 300);
    let mut output = None;
    let mut positionals = Vec::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            // THE FLAG IS GONE, and saying so beats `unrecognised argument`. It
            // took a name that had to be looked up, which is the id system this
            // removed.
            "--applet" | "-a" => {
                return Err(
                    "`--applet` is gone: pass the applet's directory, e.g. `dew snapshot examples/aether/timetracker -o out.png`"
                        .to_string(),
                );
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

    // ONE POSITIONAL, AND ITS KIND IS READ FROM THE FILESYSTEM.
    //
    // This used to guess: a positional containing a slash or ending in `.png` was
    // treated as the output, anything else as an applet name. Once an applet is
    // named by its PATH that heuristic is wrong on every invocation, because every
    // path contains a slash. The output now comes from `-o` and nothing else.
    //
    // A directory is an applet; a file is a script. The filesystem already knows
    // which, so neither needs a flag.
    match positionals.len() {
        0 => {}
        1 => {
            let pos = PathBuf::from(positionals.remove(0));
            if script.is_some() {
                return Err(format!("unexpected argument '{}'", pos.display()));
            }
            if pos.is_dir() {
                applet = Some(pos);
            } else if pos.is_file() {
                script = Some(pos.display().to_string());
            } else {
                // A TYPO IS NOT A SCRIPT. Falling through to the script branch
                // made a mistyped directory fail later, inside the Luau loader,
                // with a message about a module rather than about the path.
                return Err(format!("{}: no such file or directory", pos.display()));
            }
        }
        _ => return Err(format!("unexpected argument '{}'", positionals[1])),
    }

    let out = output.unwrap_or_else(|| "dew.png".to_string());
    let target = if let Some(path) = script {
        SnapshotTarget::Script {
            path,
            width: size.0,
            height: size.1,
        }
    } else {
        let dir = applet.ok_or(
            "nothing to snapshot: pass an applet directory or a script, e.g. `dew snapshot examples/aether/timetracker -o out.png`",
        )?;
        SnapshotTarget::Applet(dir)
    };

    Ok(Command::Snapshot {
        target,
        output: out,
    })
}

fn parse_check(args: &[String]) -> Result<Command, String> {
    let mut targets: Vec<PathBuf> = Vec::new();
    for arg in args {
        if arg.starts_with('-') {
            return Err(format!("unrecognised argument '{arg}'"));
        }
        targets.push(PathBuf::from(arg));
    }
    Ok(Command::Check { targets })
}

fn parse_install(args: &[String]) -> Result<Command, String> {
    let mut dir: Option<PathBuf> = None;
    let mut force = false;
    for arg in args {
        match arg.as_str() {
            "--force" => force = true,
            s if !s.starts_with('-') => {
                if dir.is_some() {
                    return Err(format!("unexpected argument '{s}'"));
                }
                dir = Some(PathBuf::from(s));
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }
    let dir = dir.ok_or(
        "nothing to install: pass an applet directory, e.g. `dew install examples/host/basic-widget`",
    )?;
    Ok(Command::Install { dir, force })
}

fn parse_uninstall(args: &[String]) -> Result<Command, String> {
    let mut id: Option<String> = None;
    for arg in args {
        if arg.starts_with('-') {
            return Err(format!("unrecognised argument '{arg}'"));
        }
        if id.is_some() {
            return Err(format!("unexpected argument '{arg}'"));
        }
        id = Some(arg.clone());
    }
    let id =
        id.ok_or("nothing to uninstall: pass an applet id, e.g. `dew uninstall basic-widget`")?;
    Ok(Command::Uninstall { id })
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
                    _ => return Err(format!("unknown runtime '{val}', expected 'datamodel'")),
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

    let name = name.ok_or("missing applet name for init (usage: dew init <name>)")?;
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
            //  A DIRECTORY IS A DIRECTORY, and anything else is a filter. Every
            //  other command takes a path positionally, and `dew test
            //  examples/aether/contextmenu` reading as a name to match meant the
            //  suite beside an applet could only be reached through `--dir`.
            s if !s.starts_with('-') && PathBuf::from(s).is_dir() => {
                if dir.is_some() {
                    return Err(format!("unexpected argument '{s}'"));
                }
                dir = Some(PathBuf::from(s));
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
    let applets: Vec<PathBuf> = Vec::new();
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
            "--applet" => {
                return Err(
                    "`--applet` is gone: pass the applet's directory, e.g. `dew examples/aether/timetracker`"
                        .to_string(),
                );
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
            let dir = applets.first().cloned().ok_or(
                "nothing to snapshot: pass an applet directory, e.g. `dew snapshot examples/aether/timetracker -o out.png`",
            )?;
            SnapshotTarget::Applet(dir)
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
            applets,
            stats,
            bench,
        })
    }
}

/// Load the applet in `dir`.
///
/// NO SCANNING, AND NO IDS. An applet is a directory containing a `dew.toml`.
/// That is the whole definition, and it is the one a browser uses for a page: the
/// address is the identity.
///
/// This used to read every manifest under a blessed `applets/` directory to build
/// an id list, so a directory with a perfectly good manifest somewhere else was
/// not an applet, and the same applet moved was a different applet. It also meant
/// the CLI took a name that had to be looked up, which is why `--applet
/// ./examples/widgets/nameplate/` failed with a message about ids.
fn load_applet(dir: &Path) -> Result<applets::Applet, String> {
    if !dir.is_dir() {
        return Err(format!("{}: not a directory", dir.display()));
    }
    if !dir.join("dew.toml").is_file() {
        return Err(format!(
            "{}: no dew.toml, so this is not an applet",
            dir.display()
        ));
    }

    let (aether_root, aliases) = aether_aliases()?;
    let state = Arc::new(Mutex::new(capabilities::HostState::default()));
    applets::load(dir, &aether_root, &aliases, &state)
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
        SnapshotTarget::Applet(dir) => {
            let active = load_applet(&dir)?;
            // `pointer` IS NOT NEEDED HERE. A snapshot renders one frame and
            // exits without ever calling `Renderer::moved`/`down`/`up`/`wheel`,
            // so there is no cursor for `dew.Pointer`/`dew.Input` to report.
            let applets::Applet {
                manifest,
                width,
                height,
                surface,
                mounted,
                clock,
                vm,
                ..
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

/// A drag in progress on a `draggable` widget.
///
/// ARMED BY EVERY PRESS ON A DRAGGABLE WIDGET, not only the ones that turn into
/// a drag. `dragging` is false until the pointer has moved past the threshold,
/// which is what lets a press-and-hold button still work: the press dispatches
/// to the hit-test pipeline exactly as it always has, and only a move that
/// crosses the threshold diverts from it.
///
/// SCREEN-SPACE, NOT A RUNNING CLIENT-RELATIVE DELTA. Once a drag is under way
/// the window itself moves under a stationary cursor, so a later `PointerMove`'s
/// client-relative coordinates are reported against a DIFFERENT window origin
/// than the press was. Converting every reading to screen space with the
/// window's position at the time keeps "distance moved since the press"
/// correct regardless of how many times the window has already been
/// repositioned.
#[cfg(windows)]
struct DragState {
    press_screen: (f32, f32),
    window_origin: (i32, i32),
    dragging: bool,
}

/// Apply `snapToEdges` and then `keepOnScreen` to a candidate window position.
///
/// SNAP FIRST, CLAMP SECOND — clamping after snapping is what keeps a widget
/// near the bottom-right corner from being snapped to an edge and then shoved
/// back off it by the clamp; running them the other way could undo the snap.
#[cfg(windows)]
fn place_widget(
    candidate: (i32, i32),
    size: (u32, u32),
    snap_to_edges: bool,
    keep_on_screen: bool,
) -> (i32, i32) {
    let screen = dew_window::screen_size();
    let (mut x, mut y) = candidate;
    let (w, h) = (size.0 as i32, size.1 as i32);

    if snap_to_edges {
        const THRESHOLD: i32 = 12;
        if x.abs() <= THRESHOLD {
            x = 0;
        }
        if (screen.0 - (x + w)).abs() <= THRESHOLD {
            x = screen.0 - w;
        }
        if y.abs() <= THRESHOLD {
            y = 0;
        }
        if (screen.1 - (y + h)).abs() <= THRESHOLD {
            y = screen.1 - h;
        }
    }

    if keep_on_screen {
        // A HARD CLAMP, AFTER SNAPPING. `min`/`max` swap places when the widget
        // is wider (or taller) than the screen, so the widget still ends up
        // fully on screen rather than the range becoming empty.
        let (min_x, max_x) = ((screen.0 - w).min(0), (screen.0 - w).max(0));
        let (min_y, max_y) = ((screen.1 - h).min(0), (screen.1 - h).max(0));
        x = x.clamp(min_x, max_x);
        y = y.clamp(min_y, max_y);
    }

    (x, y)
}

/// Set or clear the `Dragging` attribute on a widget's root, marking the tree
/// dirty so a guest polling it sees the change the same frame it happened.
///
/// AN ATTRIBUTE, NOT A PROPERTY. `root` is a real `ScreenGui`, validated
/// against Roblox's own schema like every instance this host creates, and
/// `ScreenGui` has no `Dragging` property there -- inventing one would mean a
/// second, host-only property system alongside the reflection-backed one
/// everything else goes through. `SetAttribute`/`GetAttribute` is Roblox's
/// own mechanism for exactly this: host- or author-defined data a class's
/// schema was never going to have, read the same way on both.
#[cfg(windows)]
fn set_dragging_attribute(dom: &datamodel::SharedDom, root: usize, dragging: bool) {
    dom.lock().expect("dom").set_attribute(
        root,
        "Dragging",
        Some(rbx_types::Variant::Bool(dragging)),
    );
}

/// `dew run <dir>`'s entry point. A second invocation while a Dew service is
/// already running hands its applet to that service instead of starting a
/// second process — see `coordinator.rs` for the single-instance check, the
/// named pipe that carries the request, and the tray, thread-per-applet loop
/// the first invocation becomes.
fn execute_run(dir: &Path, stats: bool, bench: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (dir, stats, bench);
        Err(
            "'run' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                .to_string(),
        )
    }

    #[cfg(windows)]
    {
        println!("💧 Dew starting");
        match coordinator::acquire()? {
            coordinator::Role::Primary(guard) => {
                coordinator::run(guard, dir.to_path_buf(), stats, bench)
            }
            coordinator::Role::Secondary => coordinator::send_to_running(dir),
        }
    }
}

/// Run one applet's window, VM and frame loop on whichever thread calls this,
/// until its window closes, `close` is set, or the whole service is asked to
/// exit. `coordinator::run` calls this once per loaded applet, each on a
/// thread of its own.
///
/// THIS IS TODAY'S `execute_run` BODY, WITH TWO CHANGES. No tray is created
/// here — the coordinator owns the one tray for the whole process, not each
/// applet its own — and the loop now has a second way to end besides the
/// window closing: `close`, which the coordinator's tray menu sets to unload
/// this one applet without taking the process down.
#[cfg(windows)]
fn run_applet(
    dir: &Path,
    stats: bool,
    bench: bool,
    close: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    let active = load_applet(dir)?;
    let applets::Applet {
        manifest,
        mut width,
        mut height,
        surface,
        mounted,
        clock,
        pointer: service_pointer,
        vm,
    } = active;

    let screen = dew_window::screen_size();
    if surface.fills_screen() {
        width = screen.0.max(1) as u32;
        height = screen.1.max(1) as u32;
    }

    let applets::Mounted::DataModel { ref dom, root } = mounted;
    dom.lock().expect("dom").assets.set_blocking(false);
    // OWNED, SO IT OUTLIVES THE MOVE BELOW. `create_renderer` takes `mounted`
    // by value; the drag state machine further down still needs a handle on
    // the same arena to set `Dragging` on `root`, and a `SharedDom` clone is
    // an `Arc` clone -- the same dom, not a second one.
    let dom = dom.clone();

    let mut renderer = create_renderer(mounted, &vm, &clock, &surface, width, height)?;

    // WHICH DRAGGABLE BEHAVIOURS THIS SURFACE ASKED FOR, if it is a widget
    // at all. `None` for everything else, which turns the whole drag state
    // machine below into dead branches that never arm — a window or an
    // overlay is never draggable, so this is not a case those surfaces need
    // to think about.
    let drag_options = match &surface {
        surface::Declared::Widget {
            draggable,
            keep_on_screen,
            snap_to_edges,
            save_position,
            ..
        } => Some((*draggable, *keep_on_screen, *snap_to_edges, *save_position)),
        _ => None,
    };

    let mut resolved = surface.resolve(screen, (width, height));

    // A SAVED POSITION OVERRIDES THE DECLARED ANCHOR, exactly like a real
    // Rainmeter skin: the anchor is what a widget that has never been
    // dragged falls back to, and a drag that happened once wins from then
    // on until the next drag replaces it.
    if let (Surface::Widget { x, y, .. }, Some((_, _, _, true))) = (&mut resolved, drag_options) {
        if let Some(saved) = positions::load(&manifest.id) {
            (*x, *y) = saved;
        }
    }

    let mut window = Window::new(&resolved, width, height)?;

    // THE WINDOW'S CURRENT ON-SCREEN POSITION, TRACKED HERE because nothing
    // else does: `Window` itself only knows its size (`resized` exists for
    // exactly that reason), and a resolved `Surface` is consumed once at
    // creation. Dragging needs to know where the window is NOW, both to
    // compute a candidate position and to convert a future pointer event's
    // client-relative coordinates back to screen space.
    let mut position: (i32, i32) = match &resolved {
        Surface::Widget { x, y, .. } => (*x, *y),
        _ => (0, 0),
    };
    let mut drag: Option<DragState> = None;

    let _keep_alive = vm;

    let mut last = Instant::now();
    let (mut frames, mut painted_frames) = (0u32, 0u32);
    let (mut sum_frame, mut sum_present) = (Duration::ZERO, Duration::ZERO);
    let mut last_report = Instant::now();

    // THE PUMP IS THE THREAD'S, NOT THE WINDOW'S, and every event says which
    // surface produced it. One surface is open here, so routing is a
    // comparison that always succeeds; it is written out rather than
    // assumed because a popover is what would add a second, and an event
    // silently applied to the wrong tree is not a failure that announces
    // itself.
    let mut pump = Pump::new();
    while let Some(events) = pump.poll() {
        for (from, event) in events {
            if from != window.id() {
                continue;
            }
            match event {
                Event::PointerMove { x, y } => {
                    // ARMED BUT NOT YET DRAGGING: keep forwarding to the
                    // hit-test pipeline (hover still works for a press that
                    // turns out not to be a drag) and watch for the
                    // threshold. `screen_now` is what keeps the distance
                    // moved correct across a window that has already been
                    // repositioned once — see `DragState`'s doc comment.
                    let mut forward = true;
                    if let Some(ds) = &mut drag {
                        let screen_now = (position.0 as f32 + x, position.1 as f32 + y);
                        if !ds.dragging {
                            let (dx, dy) = (
                                screen_now.0 - ds.press_screen.0,
                                screen_now.1 - ds.press_screen.1,
                            );
                            if (dx * dx + dy * dy).sqrt() >= 4.0 {
                                ds.dragging = true;
                                set_dragging_attribute(&dom, root, true);
                            }
                        }
                        if ds.dragging {
                            // PAST THE THRESHOLD: the hover pipeline stops
                            // seeing moves entirely. The window is about to
                            // slide under a stationary cursor, and
                            // forwarding that as a hover delta would be
                            // nonsense.
                            forward = false;
                            let dx = (screen_now.0 - ds.press_screen.0).round() as i32;
                            let dy = (screen_now.1 - ds.press_screen.1).round() as i32;
                            let candidate = (ds.window_origin.0 + dx, ds.window_origin.1 + dy);
                            let (_, keep_on_screen, snap_to_edges, _) =
                                drag_options.expect("drag only arms for a widget");
                            let placed = place_widget(
                                candidate,
                                (width, height),
                                snap_to_edges,
                                keep_on_screen,
                            );
                            window.set_position(placed.0, placed.1);
                            position = placed;
                        }
                    }
                    if forward {
                        renderer.moved(x, y, &service_pointer)?;
                    }
                }
                Event::PointerDown { x, y, button } => {
                    // DISPATCHED UNCHANGED, EVERY TIME. A press-and-hold
                    // button must still work even on a draggable widget, so
                    // arming a drag never replaces this.
                    renderer.down(button, x, y, &service_pointer)?;
                    if button == Button::Left {
                        if let Some((draggable, _, _, _)) = drag_options {
                            if draggable {
                                drag = Some(DragState {
                                    press_screen: (position.0 as f32 + x, position.1 as f32 + y),
                                    window_origin: position,
                                    dragging: false,
                                });
                            }
                        }
                    }
                }
                Event::PointerUp { x, y, button } => {
                    let mut suppress = false;
                    if button == Button::Left {
                        if let Some(ds) = drag.take() {
                            if ds.dragging {
                                // A REAL DRAG HAPPENED. `OnReleased` will not
                                // fire for whatever was pressed underneath —
                                // an accepted, cosmetic simplification — and
                                // this event is not forwarded at all.
                                suppress = true;
                                set_dragging_attribute(&dom, root, false);
                                if let Some((_, _, _, true)) = drag_options {
                                    positions::save(&manifest.id, position);
                                }
                            }
                        }
                    }
                    if !suppress {
                        renderer.up(button, x, y, &service_pointer)?;
                    }
                }
                Event::Wheel { x, y, delta } => {
                    renderer.wheel(x, y, delta, &service_pointer)?;
                }
                Event::Resized {
                    width: w,
                    height: h,
                } => {
                    // THE SHELL APPLIES THE SIZE NOW. The window used to
                    // catch its own resize while draining its own queue.
                    window.resized(w, h);
                    renderer.invalidate();
                }
                Event::Exposed => renderer.invalidate(),
                // UNLOADS THIS APPLET, AND NOTHING ELSE. The process used to
                // exit the moment its one window closed; the coordinator
                // outlives every applet now, so this thread simply ends and
                // leaves the registry entry for the coordinator to notice
                // and drop.
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

        // TWO WAYS TO BE TOLD TO STOP: `close`, set by the coordinator when
        // this one applet is unloaded from the tray menu, and
        // `tray::exit_requested`, set when the whole service is. Either ends
        // this thread's own loop; whether the PROCESS exits is the
        // coordinator's call, made once every applet thread has.
        if close.load(std::sync::atomic::Ordering::Relaxed) || tray::exit_requested() {
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

fn execute_check(targets: Vec<PathBuf>) -> Result<(), String> {
    // PATHS, AND NO SEARCHING. `check examples/aether/timetracker` checks that directory.
    // Bare `check` checks the working directory if it is an applet, which makes
    // `cd` into one and `dew check` the obvious thing.
    //
    // This used to resolve a name against a blessed `applets/` directory and, with
    // no argument, check everything it found there. A directory with a manifest
    // somewhere else was not checkable, which is the id system in another costume.
    let dirs: Vec<PathBuf> = if targets.is_empty() {
        if Path::new("dew.toml").is_file() {
            vec![PathBuf::from(".")]
        } else {
            return Err(
                "nothing to check: pass an applet directory, or run this from inside one"
                    .to_string(),
            );
        }
    } else {
        targets
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

/// Copy an applet directory into the per-user store, keyed by its own
/// manifest id, and mark it enabled.
///
/// THIS ONLY AFFECTS THE NEXT COORDINATOR STARTUP. An already-running Dew
/// service keeps whatever it loaded when it started; toggling a live one is
/// not something this sprint answers, and the printed line below says so
/// rather than leaving that a silent surprise.
fn execute_install(dir: PathBuf, force: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (dir, force);
        Err("'install' is Windows-only: an installed applet loads into the coordinator's window, which only runs on Windows".to_string())
    }

    #[cfg(windows)]
    {
        let id = installed::install(&dir, force)?;
        println!("[dew] installed '{id}' from {}", dir.display());
        println!(
            "[dew] this loads the next time the Dew service starts; a service already running keeps what it started with"
        );
        Ok(())
    }
}

/// Remove an installed applet from the store. Refuses if that id is
/// currently running in an active coordinator, so `dew uninstall` never
/// leaves a running applet with no installed copy behind it.
fn execute_uninstall(id: String) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = id;
        Err("'uninstall' is Windows-only: an installed applet loads into the coordinator's window, which only runs on Windows".to_string())
    }

    #[cfg(windows)]
    {
        if coordinator::query_running(&id)? {
            return Err(format!(
                "'{id}' is currently running; exit it (or exit Dew) before uninstalling"
            ));
        }
        installed::uninstall(&id)?;
        println!("[dew] uninstalled '{id}'");
        println!(
            "[dew] a Dew service already running keeps what it started with until it is restarted"
        );
        Ok(())
    }
}

/// The `dew.toml` that `dew init` writes.
///
/// TOML, BECAUSE THE FILE IS CALLED `dew.toml`. This emitted JSON for as long as
/// the file was called `mod.json`, and renaming the file did not rename the
/// format, so `dew init` produced a manifest `dew check` refused to parse. Split
/// out from `execute_init` so a test can assert the text parses without
/// scaffolding a directory to read it back from.
///
/// A TEMPLATE RATHER THAN A SERIALISER, so the scaffold carries the comments that
/// tell an author what the keys are for. A round-tripped struct cannot.
fn scaffold_manifest(
    name: &str,
    display_name: &str,
    runtime: manifest::Runtime,
    surface: manifest::Permission,
) -> String {
    format!(
        r#"# {display_name}
#
# Generated by `dew init`. Everything here is yours to edit.

id = "{name}"
name = "{display_name}"
description = "A Dew applet ({runtime_name})"

# Which runtime `mount` is written against. Never guessed, always declared.
runtime = "{runtime_name}"

# WHAT THIS APPLET MAY REACH. A surface is one of these, and an applet that asks
# for no surface has nowhere to draw and is refused at mount. Add `storage`,
# `clipboard` and the rest as you need them, and no sooner.
permissions = ["{surface_name}"]
"#,
        display_name = display_name,
        name = name,
        runtime_name = runtime.name(),
        surface_name = surface.name(),
    )
}

fn execute_init(
    name: String,
    runtime: manifest::Runtime,
    surface: String,
    size: (u32, u32),
) -> Result<(), String> {
    // REFUSED HERE, NOT AT MOUNT. `--surface` was accepted and then dropped, so
    // `--surface windwo` scaffolded a window and said nothing about it. The word
    // has to name a surface now, because it is written into the manifest as the
    // applet's one grant.
    let surface_grant = manifest::Permission::surface_from_name(&surface).ok_or_else(|| {
        format!("unknown surface '{surface}': use 'window', 'widget', 'overlay' or 'popover'")
    })?;
    // A PATH OR A NAME, AND THE VALIDATION USED TO FORBID THE PATH. Every
    // character was checked against an alphanumeric set and the next statement
    // then branched on whether the name contained a separator, which it could
    // never reach. `dew init examples/host/thing` was refused by a rule about
    // names while the code below was written to accept it.
    let target_dir = PathBuf::from(&name);
    let leaf = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();

    if leaf.is_empty()
        || !leaf
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "invalid applet name '{leaf}': use alphanumeric characters, dashes, or underscores"
        ));
    }

    // NO BLESSED DIRECTORY. This resolved a bare name against `applets/`, which
    // was the last place the host decided where an applet lives. An applet is a
    // directory you name, here as everywhere else.
    let name = leaf;

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

    let manifest_text = scaffold_manifest(&name, &display_name, runtime, surface_grant);

    let manifest_path = target_dir.join("dew.toml");
    std::fs::write(&manifest_path, manifest_text)
        .map_err(|e| format!("could not write {}: {e}", manifest_path.display()))?;

    let entry_code = match runtime {
        manifest::Runtime::DataModel => format!(
            r#"--!strict
--[[
	{display_name} -- a Dew applet written against the native DataModel.
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
	-- THE SURFACE THE MANIFEST GRANTS. These two have to agree: the manifest says
	-- what this applet may draw into, and this says what it draws into. A mismatch
	-- is refused at mount rather than quietly resolved in either direction.
	surface = {{ kind = "{}" }},
	mount = mount,
}}
"#,
            size.0,
            size.1,
            surface_grant.name()
        ),
    };

    let entry_path = target_dir.join(format!("{name}.luau"));
    std::fs::write(&entry_path, entry_code)
        .map_err(|e| format!("could not write {}: {e}", entry_path.display()))?;

    // THE HINTS PRINT PATHS, because that is what the commands take. They named a
    // bare id and an `--applet` flag that no longer exists, which handed a new
    // author two commands that fail on the first thing they ran.
    let shown = target_dir.display();
    println!("[dew] initialized applet '{name}' in {shown}");
    println!("[dew] check with:    dew check {shown}");
    println!("[dew] snapshot with: dew snapshot {shown} -o {name}.png");
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
        //  A SUITE BESIDE AN APPLET GETS A SURFACE TO LOAD IT INTO. Anywhere
        //  else this installs nothing, so Aether's own suites see exactly what
        //  they always saw.
        if suite_path
            .parent()
            .map(|d| d.join("dew.toml").is_file())
            .unwrap_or(false)
        {
            let state: crate::capabilities::Shared =
                Arc::new(Mutex::new(crate::capabilities::HostState::default()));
            if let Err(e) = install_test_surface(vm.lua(), &dom, &state) {
                failed += 1;
                eprintln!("::error::{suite_name}");
                eprintln!("  test surface installation failed: {e}");
                continue;
            }
        }

        let clock: crate::services::SharedClock =
            Arc::new(Mutex::new(crate::services::Clock::default()));
        if let Err(e) = crate::services::install(vm.lua(), &clock) {
            failed += 1;
            eprintln!("::error::{suite_name}");
            eprintln!("  Services installation failed: {e}");
            continue;
        }

        // Test runner provides a steppable clock via dew.Clock.Step(dt)
        // so transition tests can step simulated time. This is strictly isolated
        // to `dew test` and absent in guest mods run via `dew run` / `dew snapshot`.
        let step_clock = Arc::clone(&clock);
        if let Ok(dew) = vm.lua().globals().get::<mlua::Table>("dew") {
            if let Ok(dew_clock) = dew.get::<mlua::Table>("Clock") {
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
            println!(
                "  --applet, -m <ID>        Mod to snapshot (defaults to first available mod)"
            );
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
            println!("  --applet, -m <ID>        Mod to run (defaults to first available mod)");
            println!("  --stats               Print FPS and render timings");
            println!("  --bench               Run in benchmark mode");
        }
        Some("check") => {
            println!("Usage: dew check [TARGET]");
            println!();
            println!("Validate a mod's manifest (dew.toml), entrypoint, and Luau syntax.");
            println!();
            println!("Arguments:");
            println!(
                "  [TARGET]              Mod ID or path to directory (checks all mods if omitted)"
            );
        }
        Some("init") | Some("scaffold") => {
            println!("Usage: dew init <NAME> [OPTIONS]");
            println!();
            println!("Scaffold a new Dew applet with a valid manifest and working entrypoint.");
            println!();
            println!("Arguments:");
            println!("  <NAME>                Mod identifier and directory name");
            println!();
            println!("Options:");
            println!("  --runtime, -r <RT>    Runtime: 'datamodel' (default, and the only one)");
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
        Some("install") => {
            println!("Usage: dew install <DIR> [OPTIONS]");
            println!();
            println!("Copy an applet into the per-user store, so a Dew service loads it on its next start.");
            println!();
            println!("Arguments:");
            println!("  <DIR>                 Applet directory (its dew.toml names its id)");
            println!();
            println!("Options:");
            println!("  --force               Replace an already-installed applet with this id");
        }
        Some("uninstall") => {
            println!("Usage: dew uninstall <ID>");
            println!();
            println!(
                "Remove an applet from the per-user store. Refuses if it is currently running."
            );
            println!();
            println!("Arguments:");
            println!(
                "  <ID>                  The applet's manifest id, as `dew install` reported it"
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
            println!("Dew, a desktop applet platform over a native DataModel");
            println!();
            println!("Usage: dew <PATH>            run the applet in that directory");
            println!("       dew <COMMAND> [ARGS]");
            println!();
            println!("An applet is any directory holding a dew.toml. There is no id.");
            println!();
            println!("Commands:");
            println!("  run <PATH>       Run an applet in a desktop window (Windows only)");
            println!("  snapshot <PATH>  Render an applet or a script to a PNG, no window needed");
            println!("  check [PATH...]  Validate manifests and entry points, or this directory");
            println!("  install <DIR>    Copy an applet into the per-user store (Windows only)");
            println!("  uninstall <ID>   Remove an applet from the per-user store (Windows only)");
            println!("  init <NAME>      Scaffold a new applet");
            println!("  test             Run Luau test suites against Dew's DataModel");
            println!("  conformance      Run the layout conformance suite");
            println!("  help [COMMAND]   Show help for a command");
            println!();
            println!("Options:");
            println!("  -o <PATH>        Where snapshot writes its image");
            println!("  --script <PATH>  Render a standalone script rather than an applet");
            println!("  --size <WxH>     Dimensions for a standalone script");
            println!("  --stats          Report where frame time goes (Windows only)");
            println!("  --bench          Repaint every frame (Windows only)");
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
        // ONE WINDOW, ONE APPLET, FOR NOW. Several paths parse, and mounting more
        // than one is a window-management question this host has not answered, so
        // it says so rather than silently running the first.
        Command::Run {
            applets,
            stats,
            bench,
        } => match applets.as_slice() {
            [] => Err(
                "nothing to run: pass an applet directory, e.g. `dew examples/aether/timetracker`"
                    .to_string(),
            ),
            [one] => execute_run(one, stats, bench),
            _ => Err("running more than one applet at once is not supported yet".to_string()),
        },
        Command::Snapshot { target, output } => execute_snapshot(target, output),
        Command::Check { targets } => execute_check(targets),
        Command::Install { dir, force } => execute_install(dir, force),
        Command::Uninstall { id } => execute_uninstall(id),
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
    // it, exactly as an engine place does. The host used to hand every mod an
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

/// What a `.test.luau` beside an applet is given, on top of the DataModel.
///
/// AN APPLET RETURNS NOTHING, so a test cannot read one. It requires the applet
/// like any module, the applet asks for its surface and builds under it, and the
/// test then drives input and reads the tree the host holds. That is the same
/// path a person takes, which is the point: the old arrangement mounted the tree
/// a second time under a recording copy of the framework and measured that.
///
/// INSTALLED ONLY BY `dew test`. `DewTest` is not part of what an applet may
/// reach, and an applet that found one would be reading a test harness.
fn install_test_surface(
    lua: &mlua::Lua,
    dom: &dew_host::datamodel::SharedDom,
    state: &crate::capabilities::Shared,
) -> mlua::Result<()> {
    let root = dom
        .lock()
        .expect("dom")
        .insert("ScreenGui".into(), "DewRoot".into());
    let handle = dew_host::datamodel::handle(lua, dom, root)?;

    //  EVERY SURFACE IS GRANTED HERE, unlike in the host, where the table
    //  carries only what a manifest asked for. A test is not the place to
    //  rehearse a refusal, and one that wanted to would assert on `applets::load`
    //  instead.
    let granted = [
        crate::manifest::Permission::Widget,
        crate::manifest::Permission::Window,
        crate::manifest::Permission::Overlay,
        crate::manifest::Permission::Popover,
        crate::manifest::Permission::Storage,
        crate::manifest::Permission::Clipboard,
    ];
    let grant = crate::capabilities::SurfaceGrant {
        requested: Arc::new(Mutex::new(None)),
        root: Some(mlua::IntoLua::into_lua(handle.clone(), lua)?),
        title: "test".to_string(),
    };
    let dew = crate::capabilities::build(lua, &granted, state, &grant)?;
    lua.globals().set("dew", dew)?;

    let harness = lua.create_table()?;
    harness.set("Root", handle)?;

    //  RECORDED AND DELIVERED, in the order the renderer uses. A framework polls
    //  `services` for where the pointer is and connects to the input service for
    //  what happened; driving one without the other leaves half of it reading a
    //  pointer that never moved.
    let pointer = lua.create_table()?;
    let surface_dom = dom.clone();
    pointer.set(
        "Move",
        lua.create_function(move |lua, (x, y): (f32, f32)| {
            crate::services::pointer_moved(x, y);
            let surface = dew_host::datamodel::input::Surface {
                lua,
                dom: &surface_dom,
                root,
                size: (0.0, 0.0),
            };
            let mut p = dew_host::datamodel::input::Pointer::default();
            let _ = p.moved(&surface, x, y);
            Ok(())
        })?,
    )?;

    for (name, down) in [("Down", true), ("Up", false)] {
        let surface_dom = dom.clone();
        pointer.set(
            name,
            lua.create_function(move |lua, (x, y, button): (f32, f32, Option<usize>)| {
                let button = button.unwrap_or(0);
                crate::services::pointer_moved(x, y);
                crate::services::pointer_button(button, down);
                let surface = dew_host::datamodel::input::Surface {
                    lua,
                    dom: &surface_dom,
                    root,
                    size: (0.0, 0.0),
                };
                let kind = match button {
                    1 => dew_host::datamodel::input::Button::Right,
                    2 => dew_host::datamodel::input::Button::Middle,
                    _ => dew_host::datamodel::input::Button::Left,
                };
                let mut p = dew_host::datamodel::input::Pointer::default();
                let _ = if down {
                    p.down(&surface, kind, x, y)
                } else {
                    p.up(&surface, kind, x, y)
                };
                Ok(())
            })?,
        )?;
    }
    harness.set("Pointer", pointer)?;

    //  LAYOUT HAS TO HAVE RUN before a test can ask where anything is.
    //  `AbsolutePosition` reads the reflection database's default of zero until
    //  something computes it, and zero is a plausible coordinate rather than an
    //  error: a test that clicks there hits whatever is at the origin and passes
    //  against the wrong element.
    //
    //  The window loop does this every frame. A test says when.
    let settle_dom = dom.clone();
    harness.set(
        "Settle",
        lua.create_function(move |_, size: Option<mlua::Table>| {
            let (w, h) = match size {
                Some(t) => (
                    t.get("width").unwrap_or(0.0),
                    t.get("height").unwrap_or(0.0),
                ),
                None => (0.0, 0.0),
            };
            let mut guard = settle_dom.lock().expect("dom");
            dew_host::datamodel::render::commit_geometry(&mut guard, root, w, h);
            Ok(())
        })?,
    )?;

    lua.globals().set("DewTest", harness)?;
    Ok(())
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

    /// `dew init` has to write something `dew check` can read.
    ///
    /// THE BUG THIS PINS: the scaffolder emitted `serde_json::to_string_pretty`
    /// into a file named `dew.toml`. Both halves were individually reasonable and
    /// the pair was not, and nothing failed until CI scaffolded an applet and
    /// checked it. A round trip through the real parser is the only assertion
    /// that would have caught it, because the file was well-formed JSON.
    #[test]
    fn init_writes_a_manifest_that_parses() {
        let runtime = manifest::Runtime::DataModel;
        let raw = scaffold_manifest(
            "my_applet",
            "My_applet",
            runtime,
            manifest::Permission::Window,
        );
        let parsed = manifest::Manifest::parse(&raw, "dew.toml")
            .unwrap_or_else(|e| panic!("init wrote a manifest that will not parse: {e}"));

        assert_eq!(parsed.id, "my_applet");
        assert_eq!(parsed.runtime, runtime);
    }

    /// The grant `--surface` asked for is the grant that lands in the manifest.
    ///
    /// Not a restatement of the template: an applet is refused at mount unless the
    /// manifest grants the surface its entry declares, so a scaffold that writes
    /// the wrong word here produces an applet that cannot run on the first try.
    #[test]
    fn init_grants_the_surface_it_was_asked_for() {
        for surface in [
            manifest::Permission::Window,
            manifest::Permission::Widget,
            manifest::Permission::Overlay,
            manifest::Permission::Popover,
        ] {
            let raw = scaffold_manifest("a", "A", manifest::Runtime::DataModel, surface);
            let parsed = manifest::Manifest::parse(&raw, "dew.toml").expect("parses");
            assert_eq!(parsed.permissions, vec![surface]);
        }
    }

    /// A surface the host does not have is refused before anything is written.
    #[test]
    fn an_unknown_surface_is_not_a_surface() {
        assert!(manifest::Permission::surface_from_name("windwo").is_none());
        assert!(manifest::Permission::surface_from_name("window").is_some());

        // NOT EVERY PERMISSION IS A SURFACE. `--surface storage` names a real
        // capability and still has no answer to where the applet draws.
        assert!(manifest::Permission::surface_from_name("storage").is_none());
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

    /// A DIRECTORY IS AN APPLET, decided by the filesystem rather than a flag.
    #[test]
    fn subcommand_snapshot_takes_a_path() {
        // A REAL DIRECTORY, because the kind is read from the filesystem. The test
        // makes one rather than assuming what exists beside the crate.
        let dir = std::env::temp_dir().join("dew-cli-test-applet");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let args = [
            "snapshot".to_string(),
            dir.display().to_string(),
            "-o".to_string(),
            "out.png".to_string(),
        ]
        .into_iter();
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Snapshot { target, output } => {
                assert_eq!(output, "out.png");
                match target {
                    SnapshotTarget::Applet(got) => assert_eq!(got, dir),
                    _ => panic!("a directory should read as an applet"),
                }
            }
            _ => panic!("expected Snapshot command"),
        }
    }

    /// The flag is gone, and saying so beats `unrecognised argument`.
    #[test]
    fn snapshot_refuses_the_old_flag_and_says_what_to_type() {
        let args = ["snapshot", "--applet", "nameplate"]
            .iter()
            .map(|s| s.to_string());
        let err = parse_args(args, false).unwrap_err();
        assert!(
            err.contains("directory"),
            "the error should point at a path, got: {err}"
        );
    }

    /// WITHOUT A TARGET THERE IS NOTHING TO RENDER. This used to fall back to
    /// whichever applet a blessed directory listed first.
    #[test]
    fn subcommand_snapshot_needs_a_target() {
        let args = ["snapshot", "-o", "out.png"].iter().map(|s| s.to_string());
        let err = parse_args(args, false).unwrap_err();
        assert!(err.contains("nothing to snapshot"), "got: {err}");
    }

    #[test]
    fn subcommand_snapshot_script() {
        let args = [
            "snapshot",
            "--script",
            "app.luau",
            "--size",
            "360x240",
            "-o",
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
        let args = ["run", "examples/widgets/nameplate"]
            .iter()
            .map(|s| s.to_string());
        let cmd = parse_args(args, true).unwrap();
        match cmd {
            Command::Run {
                applets,
                stats,
                bench,
            } => {
                assert_eq!(applets, vec![PathBuf::from("examples/widgets/nameplate")]);
                assert!(!stats);
                assert!(!bench);
            }
            _ => panic!("expected Run command"),
        }
    }

    /// THE SHORTEST FORM, AND THE ONE THE README DOCUMENTS.
    ///
    /// `dew examples/aether/timetracker` with no subcommand. This was documented and not
    /// tested, so it shipped reporting `unrecognised argument` for the exact
    /// command the README gave. Tested now.
    #[test]
    fn a_bare_path_runs_it() {
        let args = ["examples/aether/timetracker"]
            .iter()
            .map(|s| s.to_string());
        match parse_args(args, true).unwrap() {
            Command::Run { applets, .. } => {
                assert_eq!(applets, vec![PathBuf::from("examples/aether/timetracker")]);
            }
            other => panic!("expected Run, got {other:?}"),
        }
    }

    /// A MISTYPED SUBCOMMAND IS A PATH, and that is the better error of the two.
    #[test]
    fn a_mistyped_subcommand_reads_as_a_path() {
        let args = ["chek", "examples/aether/timetracker"]
            .iter()
            .map(|s| s.to_string());
        match parse_args(args, true).unwrap() {
            Command::Run { applets, .. } => assert_eq!(applets.len(), 2),
            other => panic!("expected Run, got {other:?}"),
        }
    }

    #[test]
    fn subcommand_check() {
        let args = ["check", "examples/widgets/nameplate"]
            .iter()
            .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Check { targets } => {
                assert_eq!(targets, vec![PathBuf::from("examples/widgets/nameplate")])
            }
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

    /// NOT SKIPPED WHEN THE DIRECTORY IS ABSENT. Both lookups here were `if let`
    /// and `if`, so this passed by doing nothing the moment `applets/` moved, and
    /// it would have gone on passing for as long as the path stayed wrong.
    #[test]
    fn check_mod_dir_nameplate() {
        let examples = find_dir("examples").expect("the examples tree should be findable");
        let nameplate = examples.join("widgets").join("nameplate");
        assert!(
            nameplate.is_dir(),
            "expected an applet at {}",
            nameplate.display()
        );

        let (manifest, entry, warnings) = check_mod_dir(&nameplate).unwrap();
        assert_eq!(manifest.id, "nameplate");
        assert!(entry.is_file());
        assert!(warnings.is_empty());
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
