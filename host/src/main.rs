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
mod bundled;
#[cfg(windows)]
mod coordinator;
#[cfg(windows)]
mod dashboard;
mod examples_compat;
#[cfg(windows)]
mod installed;
#[cfg(windows)]
mod library;
#[cfg(windows)]
mod package;
#[cfg(windows)]
mod platform;
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
use dew_window::{Button, DisplayTopology, Event, Pump, Rect, Surface, Window};
use mlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
#[cfg(windows)]
use windows::Win32::Graphics::Dwm::DwmFlush;

/// Deep obsidian, behind every widget.
const BACKGROUND: Rgb = Rgb(13, 17, 23);

/// `{CARGO_PKG_VERSION}.{build number}` -- an incrementing build stamp, not
/// a compatibility promise. Milestone 12 sprints 1 and 2 already gave that
/// job to the flag registry (`Tier::Experimental { flag, revision }`), so
/// this number carries no gating meaning of its own: bumping
/// `host/Cargo.toml`'s `version`, or the build count moving, says a new
/// build exists, nothing about whether an applet written against the last
/// one still works.
///
/// A DOTTED NUMBER, NOT A COMMIT SHA. This originally read
/// `{CARGO_PKG_VERSION}+g{short sha}`, reasoned from the milestone plan's
/// own "SemVer build metadata on a short commit hash" wording -- and that
/// turned out not to match what the plan was actually modeling: Roblox's
/// real version string, `0.739.0.7390687`, is entirely numeric, with an
/// opaque incrementing build number as its fourth segment, no VCS hash
/// anywhere in it. `{CARGO_PKG_VERSION}.{build number}` (e.g. `0.1.0.482`)
/// is the corrected, literal analogue: three segments from Cargo's own
/// version, a fourth that increments the same way Roblox's does. A commit
/// sha is a reasonable crash-traceability idea on its own merits, but it is
/// not this milestone's reference shape.
///
/// `DEW_BUILD_NUMBER` COMES FROM `build.rs`, BAKED IN AT COMPILE TIME. A
/// shipped `dew.exe` has no `.git` directory to ask, so the lookup happens
/// once, here, where a checkout is expected to exist -- see that file for
/// the shallow-clone and no-`.git` fallbacks, and for why a plain commit
/// count is NOT shallow-clone-safe the way a short sha was (a shallow clone
/// answers `1`, not the repository's real count -- see the sprint-3
/// amendment record for how CI's own checkout was fixed for this).
pub(crate) const BUILD_IDENTIFIER: &str =
    concat!(env!("CARGO_PKG_VERSION"), ".", env!("DEW_BUILD_NUMBER"));

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
    // THE FACE COMES FROM `services::default_face`, WHICH IS ALSO WHAT MEASURES. This
    // used to call `Font::load` here, and once a guest can ask "how wide is this
    // string" that is no longer merely wasteful -- `Font::load` hands back a fresh
    // id per call, so the painter and the measurement would have been two
    // registrations of the same file, and a measurement that does not describe the
    // pixels is worse than no measurement. One memo, one id, one face.
    if let Some(face) = services::default_face() {
        painter = painter.with_face(face);
    }
    Ok(painter)
}

/// What the frame loop drives.
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
                // A TRANSITION IN FLIGHT (milestone 29 part C4) KEEPS
                // PAINTING even on a frame `take_dirty` alone would call
                // settled -- an in-progress animation is not a tree
                // mutation `Changed` fires for, so nothing else would mark
                // it dirty on its own.
                // AND A CARET THAT BLINKED. A focused `TextBox` owes one
                // repaint each time its caret shows or hides, twice a second,
                // and none between.
                let should_paint = {
                    let mut guard = dom.lock().expect("dom");
                    let caret = guard.caret_due(std::time::Instant::now());
                    guard.take_dirty() || guard.transitions_active() || caret
                };
                if !should_paint {
                    return Ok(false);
                }
                // VIEWPORT FIRST -- a `@ViewportDisplaySize*` query (part
                // C3) reads this, and `apply_modifiers` just below may
                // itself depend on a query gate that reads it too.
                // `Dom::now` is set by `advance_transitions` below, the one
                // call that owns it (see that method's own doc comment).
                {
                    let mut guard = dom.lock().expect("dom");
                    guard.set_viewport(*width as u32, *height as u32);
                    guard.refresh_style_queries(*root);
                }
                // `::MODIFIER` AUTO-SPAWN, BEFORE THIS FRAME'S OWN PAINT --
                // milestone 29 part C2. Idempotent, so running it every
                // painted frame costs nothing once a modifier's own child
                // already exists; a child it creates just now still needs
                // to reach THIS frame's `frame_of`, not next one.
                dom.lock().expect("dom").apply_modifiers(*root);
                // ADVANCE ANY TRANSITION (part C4) BEFORE THIS FRAME'S OWN
                // PAINT, same reasoning as `apply_modifiers` just above --
                // a newly-started or newly-finished animation still needs
                // to reach THIS frame's `frame_of`. `dt` is the render
                // loop's own real per-frame delta, not `services::Clock`
                // (see `advance_transitions`'s own doc comment for why).
                dom.lock().expect("dom").advance_transitions(*root, dt);
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
            } => {
                // SHIFT IS READ AT THE PRESS, from the window, because the
                // press event does not carry it: a Shift+click extends a
                // `TextBox` selection.
                pointer.mods.shift = dew_window::shift_held();
                pointer
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
                    .map_err(|e| e.to_string())
            }
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
    fn key(
        &mut self,
        name: &str,
        mods: input::Mods,
        service_pointer: &services::SharedPointer,
    ) -> Result<(), String> {
        // `desktop.Input.InputBegan` HEARS EVERY NAMED KEY, focus or no focus
        // -- the same as the engine's `UserInputService.InputBegan` does.
        // What follows is the DataModel's own, narrower reaction: `TextBox`
        // editing, gated on which instance currently holds focus.
        crate::services::key_down_on(service_pointer, name);
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
                    mods,
                )
                .map_err(|e| e.to_string()),
        }
    }

    #[cfg(windows)]
    fn char(&mut self, c: char) -> Result<(), String> {
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
                .char(
                    &input::Surface {
                        lua,
                        dom,
                        root: *root,
                        size: (*width, *height),
                    },
                    c,
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

    /// Take a size the window was just told it now has.
    ///
    /// WITHOUT THIS, A RESIZE STRETCHES RATHER THAN REDRAWS. `width`/`height`
    /// here are what `frame_of` lays the DataModel out against every frame;
    /// left at whatever they were when this renderer was created, the next
    /// frame keeps rendering the OLD layout into a canvas also still the old
    /// size, and `Window::present`'s `StretchDIBits` then stretches that
    /// stale-sized buffer to fill the window's new, already-resized client
    /// rect -- which is indistinguishable, on screen, from the content
    /// itself being stretched, because it is being stretched, just not by
    /// anything that knows what it is stretching.
    #[cfg(windows)]
    fn resize(&mut self, width: u32, height: u32) {
        match self {
            Renderer::DataModel {
                dom,
                painter,
                width: w,
                height: h,
                ..
            } => {
                *w = width as f32;
                *h = height as f32;
                painter.resize(width, height);
                dom.lock().expect("dom").touch();
            }
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
/// behind it, and those are `desktop.Text` and `desktop.Clock`, installed above: the two
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
    let (_vm, dom, root) = load_script(path, &mut |_| {})?;
    let frame = datamodel::render::frame_of(&dom, root, width as f32, height as f32);
    let drawn = frame.nodes.len();
    let mut surface = painter(width, height)?;
    dew_runtime::Painter::paint_frame(&mut surface, &frame, Some(BACKGROUND));
    Ok((format!("{drawn} node(s)"), surface))
}

/// Run a standalone script and hand back the tree it built under `DewRoot`.
///
/// `before_exec` sees the VM once the host has installed everything and before
/// the script's own Luau runs.
fn load_script(
    path: &str,
    before_exec: &mut dyn FnMut(&Lua),
) -> Result<(dew_runtime::Vm, datamodel::SharedDom, usize), String> {
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

    // NO MANIFEST HERE, SO NO EXPERIMENTAL SURFACE. Cleared rather than left
    // alone, because this thread may have run a mod with flags enabled a
    // moment earlier -- the thread-local `extensions` state is per-thread,
    // not per-call, and a standalone script never gets to opt into anything.
    datamodel::extensions::set_enabled_flags(&[]);

    let dom = datamodel::SharedDom::default();
    // `mod://` RESOLVES BESIDE THE SCRIPT, which is the same directory the
    // requirer was just given. A standalone script has no mod directory and no
    // manifest, but it does have a file, and "beside the thing that named the
    // asset" is the rule in both cases rather than two rules that agree by
    // accident.
    dom.lock().expect("dom").assets.set_root(dir.clone());
    datamodel::install(vm.lua(), &dom).map_err(|e| e.to_string())?;
    datamodel::install_vocabulary(vm.lua()).map_err(|e| e.to_string())?;
    // `desktop.Text`/`desktop.Clock` HERE TOO, so a standalone script measures text the same way a mod
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

    before_exec(vm.lua());

    let source =
        std::fs::read_to_string(&script).map_err(|e| format!("{}: {e}", script.display()))?;
    vm.lua()
        .load(&source)
        .set_name(script.display().to_string())
        .exec()
        .map_err(|e| format!("{}: {e}", script.display()))?;

    Ok((vm, dom, root))
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
            pointer: {
                #[allow(unused_mut)]
                let mut pointer = input::Pointer::default();
                // THE WINDOWS CLIPBOARD, for a `TextBox`'s Ctrl+C, X and V.
                #[cfg(windows)]
                {
                    pointer.clipboard = input::Clipboard::System {
                        read: dew_window::clipboard_text,
                        write: dew_window::set_clipboard_text,
                    };
                }
                pointer
            },
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
    /// Starts the service with no applet named: whatever is installed and
    /// enabled loads on its own. `dew run <path>` stays the way to see one
    /// specific applet, installed or not; this is the way to see
    /// everything that already is.
    Start {
        stats: bool,
        bench: bool,
    },
    Snapshot {
        target: SnapshotTarget,
        output: String,
        /// `--after <seconds>`: run the applet's frames for this long, in real
        /// time, before drawing the one that is written.
        after: Option<std::time::Duration>,
    },
    Check {
        /// Directories to check. Empty means the working directory, if it is one.
        targets: Vec<PathBuf>,
    },
    Install {
        /// An applet directory, or a `.dewpkg` file produced by `dew package`.
        path: PathBuf,
        force: bool,
    },
    /// `dew install @<owner_user_id>/<applet_id>`. The leading `@` is what
    /// tells this apart from a filesystem path -- npm's scoped-package
    /// syntax, chosen for the same reason: no local path legally starts
    /// with `@`, so there is nothing to disambiguate at parse time.
    InstallFromMarketplace {
        owner_user_id: String,
        applet_id: String,
        force: bool,
    },
    Package {
        dir: PathBuf,
        output: Option<PathBuf>,
    },
    Uninstall {
        id: String,
    },
    Login,
    Signup,
    Logout,
    Whoami,
    Discover,
    Sync,
    Publish {
        path: PathBuf,
        public: bool,
    },
    Init {
        name: String,
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
    /// Regenerate the examples compatibility table, or with `check` fail when
    /// the committed one differs from a regeneration.
    Compat {
        check: bool,
    },
    Help {
        subcommand: Option<String>,
    },
    Version,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SnapshotTarget {
    Applet(PathBuf),
    /// An applet named by its manifest id: the frame it is showing if it is
    /// running, otherwise its installed copy rendered fresh. `fresh` skips
    /// the running one.
    Installed {
        id: String,
        fresh: bool,
    },
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
        "start" => parse_start(&args_vec[1..], is_windows),
        "snapshot" => parse_snapshot(&args_vec[1..]),
        "check" => parse_check(&args_vec[1..]),
        "install" => parse_install(&args_vec[1..]),
        "package" => parse_package(&args_vec[1..]),
        "uninstall" => parse_uninstall(&args_vec[1..]),
        "login" => Ok(Command::Login),
        "signup" => Ok(Command::Signup),
        "logout" => Ok(Command::Logout),
        "whoami" => Ok(Command::Whoami),
        "discover" => Ok(Command::Discover),
        "sync" => Ok(Command::Sync),
        "publish" => parse_publish(&args_vec[1..]),
        "init" | "scaffold" => parse_init(&args_vec[1..]),
        "test" => parse_test(&args_vec[1..]),
        "conformance" => parse_conformance(&args_vec[1..]),
        "compat" => parse_compat(&args_vec[1..]),
        "help" | "--help" | "-h" => Ok(Command::Help {
            subcommand: args_vec.get(1).cloned(),
        }),
        "version" | "--version" | "-V" => Ok(Command::Version),
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

fn parse_start(args: &[String], is_windows: bool) -> Result<Command, String> {
    let mut stats = false;
    let mut bench = false;

    for arg in args {
        match arg.as_str() {
            "--stats" => stats = true,
            "--bench" => bench = true,
            // `dew start` names no applet; a path here is asking for
            // `dew run <path>` instead, so that mistake gets its own
            // message rather than "unrecognised argument".
            s if !s.starts_with('-') => {
                return Err(format!(
                    "'start' takes no applet; did you mean `dew run {s}`?"
                ));
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }

    if !is_windows {
        return Err(
            "'start' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                .to_string(),
        );
    }

    Ok(Command::Start { stats, bench })
}

fn parse_snapshot(args: &[String]) -> Result<Command, String> {
    let mut applet: Option<PathBuf> = None;
    let mut script = None;
    let mut size = (400, 300);
    let mut output = None;
    let mut after = None;
    let mut fresh = false;
    let mut installed: Option<String> = None;
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
            "--fresh" => fresh = true,
            // AN APPLET THAT WATCHES SOMETHING HAS NOTHING TO SHOW AT ITS
            // FIRST FRAME. A system monitor's graph is a minute of samples and a
            // clock's seconds hand moves; this lets a snapshot show either
            // after it has run, rather than only as it mounts.
            "--after" => {
                let val = iter.next().ok_or("missing value for --after")?;
                let seconds: f64 = val
                    .trim()
                    .parse()
                    .ok()
                    .filter(|s: &f64| s.is_finite() && *s >= 0.0)
                    .ok_or_else(|| format!("invalid --after '{val}', expected seconds (e.g. 5)"))?;
                after = Some(std::time::Duration::from_secs_f64(seconds));
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
            } else if is_applet_id(&pos) {
                // A BARE WORD IS AN APPLET'S ID. `--applet` was removed for
                // taking a name that had to be looked up when a path would
                // do; an applet already running, or installed where a person
                // never sees its directory, has no path to hand over.
                installed = Some(pos.display().to_string());
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
    if fresh && installed.is_none() {
        return Err(
            "`--fresh` renders an applet named by its id instead of capturing it running; a directory or script is always rendered fresh"
                .to_string(),
        );
    }
    let target = if let Some(id) = installed {
        SnapshotTarget::Installed { id, fresh }
    } else if let Some(path) = script {
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

    if after.is_some() && matches!(target, SnapshotTarget::Script { .. }) {
        return Err(
            "`--after` runs an applet's frames; a script draws one frame and has none".to_string(),
        );
    }

    Ok(Command::Snapshot {
        target,
        output: out,
        after,
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

fn parse_publish(args: &[String]) -> Result<Command, String> {
    let mut path: Option<PathBuf> = None;
    let mut public = false;
    for arg in args {
        match arg.as_str() {
            "--public" => public = true,
            s if !s.starts_with('-') => {
                if path.is_some() {
                    return Err(format!("unexpected argument '{s}'"));
                }
                path = Some(PathBuf::from(s));
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }
    let path = path.ok_or(
        "nothing to publish: pass an applet directory, e.g. `dew publish examples/host/basic-widget`",
    )?;
    Ok(Command::Publish { path, public })
}

fn parse_install(args: &[String]) -> Result<Command, String> {
    let mut positional: Option<String> = None;
    let mut force = false;
    for arg in args {
        match arg.as_str() {
            "--force" => force = true,
            s if !s.starts_with('-') => {
                if positional.is_some() {
                    return Err(format!("unexpected argument '{s}'"));
                }
                positional = Some(s.to_string());
            }
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }
    let positional = positional.ok_or(
        "nothing to install: pass an applet directory, a .dewpkg file, or @<owner>/<applet_id>, e.g. `dew install examples/host/basic-widget`",
    )?;

    if let Some(reference) = positional.strip_prefix('@') {
        let (owner_user_id, applet_id) = reference.split_once('/').ok_or_else(|| {
            format!("'{positional}': expected @<owner_user_id>/<applet_id>, e.g. `dew install @00000000-0000-0000-0000-000000000000/calculator`")
        })?;
        return Ok(Command::InstallFromMarketplace {
            owner_user_id: owner_user_id.to_string(),
            applet_id: applet_id.to_string(),
            force,
        });
    }

    Ok(Command::Install {
        path: PathBuf::from(positional),
        force,
    })
}

fn parse_package(args: &[String]) -> Result<Command, String> {
    let mut dir: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--output" | "-o" => {
                let val = iter.next().ok_or("missing value for --output")?;
                output = Some(PathBuf::from(val));
            }
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
        "nothing to package: pass an applet directory, e.g. `dew package examples/host/basic-widget`",
    )?;
    Ok(Command::Package { dir, output })
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
    let mut surface = "window".to_string();
    let mut size = (340, 180);

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
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

fn parse_compat(args: &[String]) -> Result<Command, String> {
    let mut check = false;
    for arg in args {
        match arg.as_str() {
            "--check" => check = true,
            _ => return Err(format!("unrecognised argument '{arg}'")),
        }
    }
    Ok(Command::Compat { check })
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
            after: None,
        })
    } else if let Some(s) = script {
        Ok(Command::Snapshot {
            target: SnapshotTarget::Script {
                path: s,
                width: size.0,
                height: size.1,
            },
            output: "dew.png".to_string(),
            after: None,
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

    let state = Arc::new(Mutex::new(capabilities::HostState::default()));
    applets::load(dir, &HashMap::new(), &state)
}

/// Does `pos` read as an applet id rather than a path? One word, as an
/// installed id is (`installed::valid_id`): no separator and not `.` or `..`.
/// A word ending like a script or an image is still a path, so a mistyped
/// `examples/clok` or `app.luau` is reported as a missing file.
fn is_applet_id(pos: &Path) -> bool {
    let word = pos.as_os_str().to_string_lossy();
    let lower = word.to_ascii_lowercase();
    !word.is_empty()
        && word != "."
        && word != ".."
        && !word.contains(['/', '\\', ':'])
        && ![".luau", ".lua", ".png"]
            .iter()
            .any(|ext| lower.ends_with(ext))
}

/// `dew snapshot <id>`: the running applet's frame, or its installed copy.
#[cfg(windows)]
fn snapshot_by_id(
    id: &str,
    fresh: bool,
    output: String,
    after: Option<std::time::Duration>,
) -> Result<(), String> {
    if !fresh {
        // ABSOLUTE, because the running Dew writes it from its own working
        // directory, not this terminal's.
        let path =
            std::path::absolute(&output).map_err(|e| format!("could not resolve {output}: {e}"))?;
        if let coordinator::LiveSnapshot::Written(width, height) =
            coordinator::snapshot_running(id, &path)?
        {
            if after.is_some() {
                println!("[dew] --after ignored: '{id}' is running, so its frame is already live");
            }
            println!("[dew] wrote {output} ({width}x{height}) from the running {id}");
            return Ok(());
        }
    }
    let dir = installed::list()
        .into_iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.dir)
        .ok_or_else(|| {
            format!(
                "no applet '{id}' is running or installed: install it with `dew install <dir>`, or pass its directory"
            )
        })?;
    execute_snapshot(SnapshotTarget::Applet(dir), output, after)
}

#[cfg(not(windows))]
fn snapshot_by_id(
    id: &str,
    _fresh: bool,
    _output: String,
    _after: Option<std::time::Duration>,
) -> Result<(), String> {
    Err(format!(
        "'{id}' names a running or installed applet, which only Windows has: pass the applet's directory instead"
    ))
}

fn execute_snapshot(
    target: SnapshotTarget,
    output: String,
    after: Option<std::time::Duration>,
) -> Result<(), String> {
    match target {
        SnapshotTarget::Installed { id, fresh } => snapshot_by_id(&id, fresh, output, after),
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
            // so there is no cursor for `desktop.Pointer`/`desktop.Input` to report.
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
            const STEP: f32 = 1.0 / 60.0;
            let started = std::time::Instant::now();
            let run_for = after.unwrap_or_default();
            while started.elapsed() < run_for {
                renderer.frame(STEP)?;
                std::thread::sleep(std::time::Duration::from_secs_f32(STEP));
            }
            renderer.frame(STEP)?;
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

/// Apply `snapToEdges` and then `keepOnScreen` to a candidate window position,
/// evaluated against multi-monitor desktop topology.
///
/// SNAP FIRST, CLAMP SECOND — clamping after snapping is what keeps a widget
/// near the bottom-right corner from being snapped to an edge and then shoved
/// back off it by the clamp; running them the other way could undo the snap.
#[cfg(windows)]
fn valid_y_for_x(topology: &DisplayTopology, x: i32, w: i32, h: i32) -> Option<(i32, i32)> {
    let mut intervals: Vec<(i32, i32)> = Vec::new();

    // 1. Single monitors covering [x, x + w]
    for m in &topology.monitors {
        if m.work_area.left <= x && m.work_area.right >= x + w {
            intervals.push((m.work_area.top, m.work_area.bottom));
        }
    }

    // 2. Pairs of horizontally adjacent monitors whose seam is within [x, x + w]
    for m1 in &topology.monitors {
        for m2 in &topology.monitors {
            if m1.work_area.right == m2.work_area.left
                && m1.work_area.left <= x
                && m2.work_area.right >= x + w
            {
                let top = m1.work_area.top.max(m2.work_area.top);
                let bottom = m1.work_area.bottom.min(m2.work_area.bottom);
                if top < bottom {
                    intervals.push((top, bottom));
                }
            }
        }
    }

    let mut best: Option<(i32, i32)> = None;
    for (start, end) in intervals {
        if start <= end - h {
            let valid = (start, end - h);
            best = match best {
                None => Some(valid),
                Some(b) => Some((b.0.min(valid.0), b.1.max(valid.1))),
            };
        }
    }
    best
}

#[cfg(windows)]
fn valid_x_for_y(topology: &DisplayTopology, y: i32, w: i32, h: i32) -> Option<(i32, i32)> {
    let mut intervals: Vec<(i32, i32)> = Vec::new();

    // 1. Single monitors covering [y, y + h]
    for m in &topology.monitors {
        if m.work_area.top <= y && m.work_area.bottom >= y + h {
            intervals.push((m.work_area.left, m.work_area.right));
        }
    }

    // 2. Pairs of vertically adjacent monitors whose seam is within [y, y + h]
    for m1 in &topology.monitors {
        for m2 in &topology.monitors {
            if m1.work_area.bottom == m2.work_area.top
                && m1.work_area.top <= y
                && m2.work_area.bottom >= y + h
            {
                let left = m1.work_area.left.max(m2.work_area.left);
                let right = m1.work_area.right.min(m2.work_area.right);
                if left < right {
                    intervals.push((left, right));
                }
            }
        }
    }

    let mut best: Option<(i32, i32)> = None;
    for (start, end) in intervals {
        if start <= end - w {
            let valid = (start, end - w);
            best = match best {
                None => Some(valid),
                Some(b) => Some((b.0.min(valid.0), b.1.max(valid.1))),
            };
        }
    }
    best
}

/// Project a candidate position onto valid monitor topology spans when no history is available.
#[cfg(windows)]
fn static_place_widget(
    (x, y): (i32, i32),
    (w, h): (i32, i32),
    work: &Rect,
    topology: &DisplayTopology,
) -> (i32, i32) {
    let proj_x = valid_y_for_x(topology, x, w, h).map(|(min_y, max_y)| (x, y.clamp(min_y, max_y)));
    let proj_y = valid_x_for_y(topology, y, w, h).map(|(min_x, max_x)| (x.clamp(min_x, max_x), y));

    match (proj_x, proj_y) {
        (Some(px), Some(py)) => {
            let dist_sq_x = (px.0 as i64 - x as i64).pow(2) + (px.1 as i64 - y as i64).pow(2);
            let dist_sq_y = (py.0 as i64 - x as i64).pow(2) + (py.1 as i64 - y as i64).pow(2);
            if dist_sq_x <= dist_sq_y {
                px
            } else {
                py
            }
        }
        (Some(px), None) => px,
        (None, Some(py)) => py,
        (None, None) => {
            let desk = topology.bounding_work_area();
            let cx = x.clamp(desk.left.min(desk.right - w), desk.left.max(desk.right - w));
            let cy = y.clamp(desk.top.min(desk.bottom - h), desk.top.max(desk.bottom - h));
            let min_x = work.left.min(work.right - w);
            let max_x = work.left.max(work.right - w);
            let min_y = work.top.min(work.bottom - h);
            let max_y = work.top.max(work.bottom - h);
            (cx.clamp(min_x, max_x), cy.clamp(min_y, max_y))
        }
    }
}

/// Apply `snapToEdges` and then `keepOnScreen` to a candidate window position,
/// evaluated against multi-monitor desktop topology.
#[cfg(windows)]
fn place_widget(
    candidate: (i32, i32),
    size: (u32, u32),
    snap_to_edges: bool,
    keep_on_screen: bool,
    topology: &DisplayTopology,
) -> (i32, i32) {
    place_widget_with_history(
        candidate,
        size,
        snap_to_edges,
        keep_on_screen,
        topology,
        None,
    )
}

/// Apply `snapToEdges` and `keepOnScreen` with hysteresis history of the current window position
/// to prevent erratic snapping when sliding along clamped edges and around multi-monitor corners.
#[cfg(windows)]
fn place_widget_with_history(
    candidate: (i32, i32),
    size: (u32, u32),
    snap_to_edges: bool,
    keep_on_screen: bool,
    topology: &DisplayTopology,
    current: Option<(i32, i32)>,
) -> (i32, i32) {
    let (mut x, mut y) = candidate;
    let (w, h) = (size.0 as i32, size.1 as i32);

    let monitor = topology.find_by_centroid(x, y, size.0, size.1);
    let work = monitor.work_area;

    if snap_to_edges {
        const THRESHOLD: i32 = 12;

        // Snap to Left edge of current monitor work area
        if (x - work.left).abs() <= THRESHOLD {
            x = work.left;
        }

        // Snap to Right edge of current monitor work area
        if (work.right - (x + w)).abs() <= THRESHOLD {
            x = work.right - w;
        }

        // Snap to Top edge of current monitor work area
        if (y - work.top).abs() <= THRESHOLD {
            y = work.top;
        }

        // Snap to Bottom edge of current monitor work area
        if (work.bottom - (y + h)).abs() <= THRESHOLD {
            y = work.bottom - h;
        }
    }

    if keep_on_screen {
        if snap_to_edges {
            let min_x = work.left.min(work.right - w);
            let max_x = work.left.max(work.right - w);
            let min_y = work.top.min(work.bottom - h);
            let max_y = work.top.max(work.bottom - h);

            x = x.clamp(min_x, max_x);
            y = y.clamp(min_y, max_y);
        } else {
            let rect = dew_window::Rect::new(x, y, x + w, y + h);
            if !topology.is_window_fully_on_screen(&rect) {
                let chosen = if let Some((cur_x, cur_y)) = current {
                    let cand_a = valid_x_for_y(topology, cur_y, w, h).and_then(|(min_x, max_x)| {
                        let ax = x.clamp(min_x, max_x);
                        valid_y_for_x(topology, ax, w, h)
                            .map(|(min_y, max_y)| (ax, y.clamp(min_y, max_y)))
                    });

                    let cand_b = valid_y_for_x(topology, cur_x, w, h).and_then(|(min_y, max_y)| {
                        let by = y.clamp(min_y, max_y);
                        valid_x_for_y(topology, by, w, h)
                            .map(|(min_x, max_x)| (x.clamp(min_x, max_x), by))
                    });

                    match (cand_a, cand_b) {
                        (Some(a), Some(b)) => {
                            let dist_a =
                                (a.0 as i64 - x as i64).pow(2) + (a.1 as i64 - y as i64).pow(2);
                            let dist_b =
                                (b.0 as i64 - x as i64).pow(2) + (b.1 as i64 - y as i64).pow(2);
                            if dist_a < dist_b {
                                a
                            } else if dist_b < dist_a {
                                b
                            } else {
                                let cur_dist_a = (a.0 as i64 - cur_x as i64).pow(2)
                                    + (a.1 as i64 - cur_y as i64).pow(2);
                                let cur_dist_b = (b.0 as i64 - cur_x as i64).pow(2)
                                    + (b.1 as i64 - cur_y as i64).pow(2);
                                if cur_dist_a <= cur_dist_b {
                                    a
                                } else {
                                    b
                                }
                            }
                        }
                        (Some(a), None) => a,
                        (None, Some(b)) => b,
                        (None, None) => static_place_widget((x, y), (w, h), &work, topology),
                    }
                } else {
                    static_place_widget((x, y), (w, h), &work, topology)
                };
                x = chosen.0;
                y = chosen.1;
            }
        }
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
                coordinator::run(guard, Some(dir.to_path_buf()), stats, bench)
            }
            coordinator::Role::Secondary => coordinator::send_to_running(dir),
        }
    }
}

/// Starts the service with no applet named, loading whatever is already
/// installed and enabled. If the service is already running, there is
/// nothing to send it -- unlike `dew run <path>`, this names no applet
/// the running instance might not already have.
///
/// STARTS WITH NOTHING ENABLED, TOO. The tray and the bundled dashboard
/// (`dashboard::open_or_focus`, reachable from the tray regardless of
/// `dew.Library`'s own state) do not depend on any applet being installed,
/// so refusing to start here would block the one surface a person would use
/// to install or enable something in the first place.
fn execute_start(stats: bool, bench: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (stats, bench);
        Err(
            "'start' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
                .to_string(),
        )
    }

    #[cfg(windows)]
    {
        match coordinator::acquire()? {
            coordinator::Role::Primary(guard) => {
                let first_dir = installed::enabled().pop().map(|(_, dir)| dir);
                println!("💧 Dew starting");
                coordinator::run(guard, first_dir, stats, bench)
            }
            coordinator::Role::Secondary => {
                println!("[dew] service already running with whatever it started with");
                Ok(())
            }
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

    // SHARED, NOT OWNED OUTRIGHT, from here on -- the live-resize hook
    // registered below needs its own handle on both this and `window`,
    // callable from inside `WM_NCCALCSIZE` while this function's own loop is
    // doing nothing at all (blocked inside `pump.poll()`, itself blocked
    // inside Windows' own modal drag loop). A `RefCell` is enough rather
    // than a `Mutex`: the hook and this loop's own code never run at once,
    // only ever nested one inside the other, on this one thread.
    let renderer = Rc::new(RefCell::new(create_renderer(
        mounted, &vm, &clock, &surface, width, height,
    )?));

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

    let mut topology = DisplayTopology::current();
    let primary_work = topology.primary().work_area;
    let mut resolved = surface.resolve_in_work_area(
        (
            primary_work.left,
            primary_work.top,
            primary_work.width(),
            primary_work.height(),
        ),
        (width, height),
    );

    // A SAVED POSITION OVERRIDES THE DECLARED ANCHOR, exactly like a real
    // Rainmeter skin: the anchor is what a widget that has never been
    // dragged falls back to, and a drag that happened once wins from then
    // on until the next drag replaces it.
    if let (Surface::Widget { x, y, .. }, Some((_, keep_on_screen, snap_to_edges, true))) =
        (&mut resolved, drag_options)
    {
        if let Some(saved) = positions::load(&manifest.id) {
            let candidate_rect = Rect::new(
                saved.0,
                saved.1,
                saved.0 + width as i32,
                saved.1 + height as i32,
            );
            if topology.intersects_any_work_area(&candidate_rect) {
                // If it intersects an active display, clamp it to ensure it obeys keepOnScreen
                let placed = place_widget(
                    saved,
                    (width, height),
                    snap_to_edges,
                    keep_on_screen,
                    &topology,
                );
                (*x, *y) = placed;
            } else {
                // ORPHANED SURFACE RECOVERY: the monitor it was saved on is no longer
                // connected or arranged here. Re-home to primary display's work area.
                let primary_work = topology.primary().work_area;
                let rehomed = place_widget(
                    (primary_work.left + 24, primary_work.top + 24),
                    (width, height),
                    snap_to_edges,
                    keep_on_screen,
                    &topology,
                );
                (*x, *y) = rehomed;
            }
        }
    }

    let window = Rc::new(RefCell::new(Window::new(&resolved, width, height)?));

    // REPAINT LIVE, DURING A BORDER-DRAG RESIZE, NOT ONLY ONCE IT ENDS.
    // `WM_NCCALCSIZE` calls this synchronously from inside Windows' own
    // modal drag loop -- see `dew_window::set_live_resize_hook`'s own doc
    // comment for why nothing else reaches this window at all while that
    // loop is running, and `crate::win32`'s `WM_NCCALCSIZE` match arm for
    // why this fires there rather than from `WM_SIZE`: `WM_NCCALCSIZE` is
    // sent BEFORE the border visually moves to its new position, so
    // rendering here keeps content in step with the border instead of one
    // message behind it. `resized` before `present`, in that order: the
    // presenter's destination is the window's OWN tracked size, and a
    // present with a source that does not match it is exactly the stretch
    // this whole hook exists to stop happening again.
    {
        let renderer = Rc::clone(&renderer);
        let window = Rc::clone(&window);
        // PACED TO THE COMPOSITOR, THE SAME WAY AN UNCAPPED WIDGET ALREADY
        // IS -- see `tray::CAPS`'s own comment on why `DwmFlush` is the
        // right wait, not a fixed interval guessed at here a second time.
        // `renderer.resize` rebuilds the native drawing surface from
        // scratch (`Canvas` has no in-place resize of its own), and a fast
        // border drag fires `WM_NCCALCSIZE` far more often than any display
        // can show a new frame -- blocking here until the next vertical blank
        // is what stops that from reallocating and repainting faster than
        // anything could ever be shown, at whatever the real refresh rate
        // of whichever monitor this window is actually on happens to be,
        // rather than a number picked in this file.
        dew_window::set_live_resize_hook(move |w, h| {
            let t0 = Instant::now();
            window.borrow_mut().resized(w, h);
            let t_resized = t0.elapsed();

            let t1 = Instant::now();
            let mut renderer = renderer.borrow_mut();
            renderer.resize(w, h);
            let t_resize = t1.elapsed();

            let t2 = Instant::now();
            let painted = renderer.frame(0.0).unwrap_or(false);
            let t_frame = t2.elapsed();

            let t3 = Instant::now();
            if painted {
                if let Some(bgra) = renderer.painter_mut().canvas_mut().bgra() {
                    window.borrow_mut().present(bgra, w, h);
                }
            }
            let t_present = t3.elapsed();
            drop(renderer);

            let _ = unsafe { DwmFlush() };

            // GATED ON `--stats`, NOT PRINTED UNCONDITIONALLY: this fires on
            // every message during a live drag, which is far too often for
            // a print nobody asked to see. `present` HERE IS NOT JUST GPU
            // PRESENTATION -- `canvas.bgra()` calls into `dew_raster`'s
            // `ar_bgra`, which renders the scene lazily on first read after
            // a resize. `renderer.resize` REBUILDS the canvas from scratch,
            // so Vello's own "already rendered" cache never survives a
            // resize step, and this number is dominated by that full
            // CPU rasterization, not by anything in `crates/window`. A
            // steady-state `--bench` reading never resizes, so it never
            // pays this cost and cannot show it.
            if stats {
                println!(
                    "[dew] resize {w}x{h} | resized {t_resized:?} | resize {t_resize:?} | frame {t_frame:?} | present {t_present:?}"
                );
            }
        });
    }

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

    // `dew snapshot <id>` FROM ANOTHER TERMINAL lands here, by manifest id,
    // and is answered between frames on this thread, the only one that may
    // touch the canvas. Unregistered when this function returns.
    let snapshots = coordinator::SnapshotInbox::open(&manifest.id);

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
    let mut iter_end = Instant::now();
    while let Some(events) = pump.poll() {
        let t_poll = iter_end.elapsed();
        let t_dispatch_start = Instant::now();
        for (from, event) in events {
            if from != window.borrow().id() {
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
                        // Use actual screen cursor coordinates via GetCursorPos when on Windows.
                        // When a window position is clamped on an axis, calculating screen coordinates
                        // from `position + (x, y)` drifts because `position` was clamped and
                        // client-relative mouse events during capture reflect the clamped window offset.
                        #[cfg(windows)]
                        let screen_now = {
                            let mut pt = windows::Win32::Foundation::POINT::default();
                            unsafe {
                                let _ =
                                    windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
                            }
                            (pt.x as f32, pt.y as f32)
                        };
                        #[cfg(not(windows))]
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
                            let placed = place_widget_with_history(
                                candidate,
                                (width, height),
                                snap_to_edges,
                                keep_on_screen,
                                &topology,
                                Some(position),
                            );
                            window.borrow().set_position(placed.0, placed.1);
                            position = placed;
                        }
                    }
                    if forward {
                        renderer.borrow_mut().moved(x, y, &service_pointer)?;
                    }
                }
                Event::PointerDown { x, y, button } => {
                    // DISPATCHED UNCHANGED, EVERY TIME. A press-and-hold
                    // button must still work even on a draggable widget, so
                    // arming a drag never replaces this.
                    renderer.borrow_mut().down(button, x, y, &service_pointer)?;
                    if button == Button::Left {
                        if let Some((draggable, _, _, _)) = drag_options {
                            if draggable {
                                #[cfg(windows)]
                                let press_screen = {
                                    let mut pt = windows::Win32::Foundation::POINT::default();
                                    unsafe {
                                        let _ =
                                            windows::Win32::UI::WindowsAndMessaging::GetCursorPos(
                                                &mut pt,
                                            );
                                    }
                                    (pt.x as f32, pt.y as f32)
                                };
                                #[cfg(not(windows))]
                                let press_screen = (position.0 as f32 + x, position.1 as f32 + y);

                                drag = Some(DragState {
                                    press_screen,
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
                        renderer.borrow_mut().up(button, x, y, &service_pointer)?;
                    }
                }
                Event::Wheel { x, y, delta } => {
                    renderer.borrow_mut().wheel(x, y, delta, &service_pointer)?;
                }
                Event::Resized {
                    width: w,
                    height: h,
                } => {
                    // THIS IS THE DEFERRED CATCH-UP, NOT THE LIVE REPAINT --
                    // that already happened, synchronously, in the
                    // `set_live_resize_hook` closure above, for every size
                    // Windows reported during the drag. This handler exists
                    // for a resize that was never a live drag at all
                    // (maximizing, snapping to a screen edge, `Win`+arrow)
                    // and, for a real drag, catches up this function's own
                    // `window`/`renderer` handles to whatever the hook's
                    // OWN clones already settled on -- redundant with the
                    // hook's last call in that case, not wrong, since both
                    // `resized` and `resize` are idempotent at a size they
                    // are already at.
                    window.borrow_mut().resized(w, h);
                    renderer.borrow_mut().resize(w, h);
                    width = w;
                    height = h;
                }
                Event::Exposed => renderer.borrow_mut().invalidate(),
                Event::DisplayChanged => {
                    // Update our topology snapshot from Win32
                    topology = DisplayTopology::current();

                    if let Some((_, keep_on_screen, snap_to_edges, save_position)) = drag_options {
                        let current_rect = Rect::new(
                            position.0,
                            position.1,
                            position.0 + width as i32,
                            position.1 + height as i32,
                        );

                        let new_pos = if topology.intersects_any_work_area(&current_rect) {
                            place_widget(
                                position,
                                (width, height),
                                snap_to_edges,
                                keep_on_screen,
                                &topology,
                            )
                        } else {
                            // Surface was orphaned by a disconnected / rearranged monitor.
                            // Re-home to the primary monitor work area.
                            let primary_work = topology.primary().work_area;
                            place_widget(
                                (primary_work.left + 24, primary_work.top + 24),
                                (width, height),
                                snap_to_edges,
                                keep_on_screen,
                                &topology,
                            )
                        };

                        if new_pos != position {
                            window.borrow().set_position(new_pos.0, new_pos.1);
                            position = new_pos;
                            if save_position {
                                positions::save(&manifest.id, position);
                            }
                        }
                    }
                    renderer.borrow_mut().invalidate();
                }
                // UNLOADS THIS APPLET, AND NOTHING ELSE. The process used to
                // exit the moment its one window closed; the coordinator
                // outlives every applet now, so this thread simply ends and
                // leaves the registry entry for the coordinator to notice
                // and drop.
                Event::CloseRequested => return Ok(()),
                Event::Key { name, shift, ctrl } => renderer.borrow_mut().key(
                    &name,
                    input::Mods { shift, ctrl },
                    &service_pointer,
                )?,
                // WAS A NO-OP UNTIL FOUND LIVE, building the dashboard's own
                // sign-in form: a `TextBox` could be focused, but typing did
                // nothing at all. See `input::Renderer::char`'s own doc
                // comment.
                Event::Char(c) => renderer.borrow_mut().char(c)?,
            }
        }
        let t_dispatch = t_dispatch_start.elapsed();

        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();

        if bench {
            renderer.borrow_mut().invalidate();
        }

        let t0 = Instant::now();
        let painted = renderer
            .borrow_mut()
            .frame(dt)
            .map_err(|e| format!("while rendering: {e}"))?;
        let t_frame = t0.elapsed();

        let t1 = Instant::now();
        if let Some(bgra) = renderer.borrow_mut().painter_mut().canvas_mut().bgra() {
            window.borrow_mut().present(bgra, width, height);
        }
        let t_raster = t1.elapsed();

        // THE FRAME JUST PRESENTED, so the file is exactly what is on screen:
        // the same pixels at the same size, with the desktop behind it left
        // out because the canvas never had it.
        while let Some(request) = snapshots.take() {
            let path = request.path.display().to_string();
            let written = renderer
                .borrow_mut()
                .painter_mut()
                .write_png(&path)
                .map(|()| (width, height))
                .map_err(|code| format!("could not write {path}: rasteriser status {code}"));
            let _ = request.reply.send(written);
        }

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
                    "[dew] {frames} fps | painted {painted_frames} | solve {:?} | raster+present {:?}",
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

        let t_pace_start = Instant::now();
        match tray::frame_budget() {
            Some(target) => {
                let elapsed = last.elapsed();
                if elapsed < target {
                    std::thread::sleep(target - elapsed);
                }
            }
            // UNCAPPED MEANS "AS FAST AS THE DISPLAY CAN ACTUALLY SHOW A NEW
            // FRAME," not "as fast as the CPU can spin." `DwmFlush` blocks this
            // thread until the compositor's next vertical blank -- the same
            // wait a real swap-chain present would impose -- so a widget doing
            // nothing between frames sleeps through the desktop's own timing
            // instead of busy-polling `PeekMessageW` hundreds of thousands of
            // times a second for no visible gain. It is a few-hundred-
            // microsecond OS call, not a measured sleep, so it costs nothing
            // extra once an applet IS producing a new frame every refresh.
            None => {
                let _ = unsafe { DwmFlush() };
            }
        }
        let t_pace = t_pace_start.elapsed();

        // A CANARY, KEPT RATHER THAN THROWN AWAY AFTER DIAGNOSIS. This is
        // what actually found the message-pump stall `solve`/`raster+present`
        // alone could not explain (both stayed under 30ms even on the frame
        // that was visibly slow) -- `crates/window`'s composition swap
        // chains going quiet for a while and stalling `pump.poll()` itself,
        // fixed by presenting the backdrop every frame instead of once. It
        // costs a handful of comparisons per iteration and stays silent
        // unless something takes this long again, so it stays on rather
        // than being removed the moment this particular cause was found.
        if stats {
            let total = t_poll + t_dispatch + t_frame + t_raster + t_pace;
            if total > Duration::from_millis(50) {
                println!(
                    "[dew] SLOW ITERATION {total:?} | poll {t_poll:?} | dispatch {t_dispatch:?} | frame {t_frame:?} | present {t_raster:?} | pace {t_pace:?}"
                );
            }
        }
        iter_end = Instant::now();
    }

    Ok(())
}

fn check_mod_dir(dir: &Path) -> Result<(manifest::Manifest, PathBuf, Vec<String>), String> {
    let manifest = manifest::Manifest::load(dir)?;

    // SAME RESOLUTION `applets::load` DOES, so a bad `experimentalDatamodel`
    // entry is caught by `dew check` rather than only at the moment an author
    // tries to run the mod. Printed unconditionally, like every other run of
    // this registry -- `dew check` is a run too.
    let declared_flags = manifest.experimental_flags()?;
    let effective_flags = datamodel::extensions::effective_flags(&declared_flags);
    datamodel::extensions::print_effective(&effective_flags);

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
                    "[dew] check {}: ok (entry: {})",
                    manifest.id,
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

/// Copy an applet into the per-user store, keyed by its own manifest id, and
/// mark it enabled. `path` is either a directory or a `.dewpkg` file --
/// `package::install_from_archive` extracts the latter to a temporary
/// directory and hands it to the exact same `installed::install` a directory
/// reaches, so the two never diverge in what they produce.
///
/// THIS ONLY AFFECTS THE NEXT COORDINATOR STARTUP. An already-running Dew
/// service keeps whatever it loaded when it started; toggling a live one is
/// not something this sprint answers, and the printed line below says so
/// rather than leaving that a silent surprise.
fn execute_install(path: PathBuf, force: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (path, force);
        Err("'install' is Windows-only: an installed applet loads into the coordinator's window, which only runs on Windows".to_string())
    }

    #[cfg(windows)]
    {
        let id = if path.is_dir() {
            installed::install(&path, force)?
        } else if path.is_file() {
            package::install_from_archive(&path, force)?
        } else {
            return Err(format!("{}: no such file or directory", path.display()));
        };
        println!("[dew] installed '{id}' from {}", path.display());
        println!(
            "[dew] this loads the next time the Dew service starts; a service already running keeps what it started with"
        );
        sync_after_install(&id);
        Ok(())
    }
}

/// Records a successful install in the signed-in account's synced list,
/// if there is one. Silent when signed out entirely, since sync is
/// optional, not a requirement to use Dew at all; a soft warning, not a
/// failure of the install that already succeeded, if a session exists
/// but the sync request itself does not go through.
#[cfg(windows)]
fn sync_after_install(id: &str) {
    if platform::load_session().is_none() {
        return;
    }
    if let Err(e) = platform::add_synced(id) {
        println!("[dew] warning: installed, but could not sync '{id}': {e}");
    }
}

/// Zip an applet directory into a `.dewpkg` file that `dew install` accepts
/// as a source. The manifest sits at the archive's own root, and the same
/// directories `installed::install` never copies are excluded here too.
fn execute_package(dir: PathBuf, output: Option<PathBuf>) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (dir, output);
        Err("'package' is Windows-only for now, alongside 'install'".to_string())
    }

    #[cfg(windows)]
    {
        let out = package::package(&dir, output)?;
        println!("[dew] wrote {}", out.display());
        Ok(())
    }
}

/// Remove an installed applet from the store. If it is currently running in
/// an active coordinator, that coordinator is asked to unload it live
/// first -- the same request the dashboard's own Library tab makes -- so
/// `dew uninstall` never leaves a running applet with no installed copy
/// behind it, and never needs a person to close it by hand first either.
fn execute_uninstall(id: String) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = id;
        Err("'uninstall' is Windows-only: an installed applet loads into the coordinator's window, which only runs on Windows".to_string())
    }

    #[cfg(windows)]
    {
        // THE SAME FUNCTION THE DASHBOARD'S UNINSTALL BUTTON CALLS, not a
        // second copy of "ask for a live unload, wait, then delete" here --
        // see `library::uninstall`'s own doc comment for why that function
        // has to reach `UNLOAD_QUEUE` through a pipe now, unconditionally,
        // rather than special-casing this CLI call as the one caller
        // outside the coordinator's own process.
        library::uninstall(&id)?;
        println!("[dew] uninstalled '{id}'");
        Ok(())
    }
}

/// Prompts for an email and a masked password, then signs in against the
/// deployed platform's identity provider and persists the session. See
/// `platform.rs`'s own header for why this is a native form rather than a
/// browser redirect.
fn execute_login() -> Result<(), String> {
    #[cfg(not(windows))]
    {
        Err("'login' is Windows-only, alongside the rest of the marketplace commands".to_string())
    }

    #[cfg(windows)]
    {
        let (email, password) = platform::prompt_credentials()?;
        let session = platform::login(&email, &password)?;
        println!("[dew] signed in as {}", session.email);
        Ok(())
    }
}

/// Creates a new account. Does not sign in: Supabase's project requires
/// email confirmation first, so this only ever starts that.
fn execute_signup() -> Result<(), String> {
    #[cfg(not(windows))]
    {
        Err("'signup' is Windows-only, alongside the rest of the marketplace commands".to_string())
    }

    #[cfg(windows)]
    {
        let (email, password) = platform::prompt_credentials()?;
        platform::signup(&email, &password)?;
        println!("[dew] check {email} for a confirmation link, then run `dew login`");
        Ok(())
    }
}

fn execute_logout() -> Result<(), String> {
    #[cfg(not(windows))]
    {
        Err("'logout' is Windows-only, alongside the rest of the marketplace commands".to_string())
    }

    #[cfg(windows)]
    {
        platform::clear_session();
        println!("[dew] signed out");
        Ok(())
    }
}

fn execute_whoami() -> Result<(), String> {
    #[cfg(not(windows))]
    {
        Err("'whoami' is Windows-only, alongside the rest of the marketplace commands".to_string())
    }

    #[cfg(windows)]
    {
        match platform::load_session() {
            Some(session) => println!("[dew] signed in as {}", session.email),
            None => println!("[dew] not signed in; run `dew login`"),
        }
        Ok(())
    }
}

fn execute_discover() -> Result<(), String> {
    #[cfg(not(windows))]
    {
        Err(
            "'discover' is Windows-only, alongside the rest of the marketplace commands"
                .to_string(),
        )
    }

    #[cfg(windows)]
    {
        let packages = platform::discover()?;
        if packages.is_empty() {
            println!("[dew] no public packages yet");
            return Ok(());
        }
        for package in packages {
            println!(
                "[dew] @{}/{}  (published {})",
                package.owner_user_id, package.applet_id, package.uploaded_at
            );
        }
        println!("[dew] install one with `dew install @<owner>/<applet_id>`");
        Ok(())
    }
}

/// Restores the signed-in account's own synced applets that are missing
/// locally. Fetches each by applet id alone (`fetch_own_package`),
/// resolved to that account's own package on `dew-platform`'s side; an
/// id that came from installing someone ELSE'S public package has no
/// owner recorded in the sync list (milestone 16's own scope boundary,
/// unchanged here) and is reported as needing a manual reinstall rather
/// than silently skipped or treated as a failure of the sprint's own
/// scope.
/// Zips an applet directory with `package::package` (unchanged, the same
/// zipper `dew package` already uses) and uploads it. No new way of
/// producing a `.dewpkg` is added here, only the upload.
fn execute_publish(path: PathBuf, public: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (path, public);
        Err("'publish' is Windows-only, alongside the rest of the marketplace commands".to_string())
    }

    #[cfg(windows)]
    {
        let temp_path = std::env::temp_dir().join(format!(
            "dew-publish-{}-{}.dewpkg",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let zip_and_read = package::package(&path, Some(temp_path.clone()))
            .and_then(|zipped| std::fs::read(&zipped).map_err(|e| e.to_string()));
        let bytes = match zip_and_read {
            Ok(bytes) => bytes,
            Err(e) => {
                let _ = std::fs::remove_file(&temp_path);
                return Err(e);
            }
        };
        let _ = std::fs::remove_file(&temp_path);

        let published = platform::publish(bytes, public)?;
        println!(
            "[dew] published '{}' ({})",
            published.applet_id,
            if published.public {
                "public"
            } else {
                "private"
            }
        );
        Ok(())
    }
}

fn execute_sync() -> Result<(), String> {
    #[cfg(not(windows))]
    {
        Err("'sync' is Windows-only, alongside the rest of the marketplace commands".to_string())
    }

    #[cfg(windows)]
    {
        let synced = platform::list_synced()?;
        let already_installed: std::collections::HashSet<String> = installed::list()
            .into_iter()
            .map(|entry| entry.id)
            .collect();

        let mut restored = 0;
        let mut needs_manual_attention = Vec::new();

        for entry in synced {
            if already_installed.contains(&entry.applet_id) {
                continue;
            }

            match platform::fetch_own_package(&entry.applet_id) {
                Ok(bytes) => {
                    let temp_path = std::env::temp_dir().join(format!(
                        "dew-sync-{}-{}-{}.dewpkg",
                        entry.applet_id,
                        std::process::id(),
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_nanos())
                            .unwrap_or(0)
                    ));
                    let write_and_install = std::fs::write(&temp_path, &bytes)
                        .map_err(|e| e.to_string())
                        .and_then(|()| package::install_from_archive(&temp_path, false));
                    let _ = std::fs::remove_file(&temp_path);

                    match write_and_install {
                        Ok(id) => {
                            println!("[dew] restored '{id}'");
                            restored += 1;
                        }
                        Err(e) => needs_manual_attention.push(format!("{}: {e}", entry.applet_id)),
                    }
                }
                Err(_) => needs_manual_attention.push(format!(
                    "{}: not this account's own published package; reinstall it manually",
                    entry.applet_id
                )),
            }
        }

        println!("[dew] restored {restored} applet(s)");
        for line in &needs_manual_attention {
            println!("[dew] could not restore {line}");
        }
        Ok(())
    }
}

/// Fetches a public package's bytes and hands them to
/// `package::install_from_archive`, exactly the path a local `.dewpkg`
/// file already takes: a marketplace fetch is a second way of producing
/// a `&Path` on disk, never a fork of install itself, per ADR-015.
fn execute_install_from_marketplace(
    owner_user_id: String,
    applet_id: String,
    force: bool,
) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (owner_user_id, applet_id, force);
        Err("'install' is Windows-only: an installed applet loads into the coordinator's window, which only runs on Windows".to_string())
    }

    #[cfg(windows)]
    {
        let bytes = platform::fetch_package(&owner_user_id, &applet_id)?;

        let temp_path = std::env::temp_dir().join(format!(
            "dew-fetch-{applet_id}-{}-{}.dewpkg",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&temp_path, &bytes).map_err(|e| e.to_string())?;

        let result = package::install_from_archive(&temp_path, force);
        let _ = std::fs::remove_file(&temp_path);
        let id = result?;

        println!("[dew] installed '{id}' from @{owner_user_id}/{applet_id}");
        println!(
            "[dew] this loads the next time the Dew service starts; a service already running keeps what it started with"
        );
        sync_after_install(&id);
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
fn scaffold_manifest(name: &str, display_name: &str, surface: manifest::Permission) -> String {
    format!(
        r#"# {display_name}
#
# Generated by `dew init`. Everything here is yours to edit.

id = "{name}"
name = "{display_name}"
description = "A Dew applet"

# WHAT THIS APPLET MAY REACH. A surface is one of these, and an applet that asks
# for no surface has nowhere to draw and is refused at mount. Add `storage`,
# `clipboard` and the rest as you need them, and no sooner.
permissions = ["{surface_name}"]
"#,
        display_name = display_name,
        name = name,
        surface_name = surface.name(),
    )
}

fn execute_init(name: String, surface: String, size: (u32, u32)) -> Result<(), String> {
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

    let manifest_text = scaffold_manifest(&name, &display_name, surface_grant);

    let manifest_path = target_dir.join("dew.toml");
    std::fs::write(&manifest_path, manifest_text)
        .map_err(|e| format!("could not write {}: {e}", manifest_path.display()))?;

    let entry_code = format!(
        r#"--!strict
--[[
	{display_name} -- a Dew applet.
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
    );

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

        // Test runner provides a steppable clock via desktop.Clock.Step(dt)
        // so transition tests can step simulated time. This is strictly isolated
        // to `dew test` and absent in guest mods run via `dew run` / `dew snapshot`.
        let step_clock = Arc::clone(&clock);
        if let Ok(desktop) = vm.lua().globals().get::<mlua::Table>("desktop") {
            if let Ok(dew_clock) = desktop.get::<mlua::Table>("Clock") {
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
            println!("Usage: dew snapshot [OPTIONS] <DIR | SCRIPT | APPLET-ID>");
            println!();
            println!("Render a mod or standalone script headlessly to a PNG image, or capture");
            println!("a running applet's frame by its id.");
            println!();
            println!("Options:");
            println!(
                "  --applet, -m <ID>        Mod to snapshot (defaults to first available mod)"
            );
            println!("  --script, -s <PATH>   Standalone script to execute and snapshot");
            println!("  --size <WxH>          Dimensions for standalone script (default: 400x300)");
            println!("  --output, -o <PATH>   Output PNG path (default: dew.png)");
            println!("  --after <SECONDS>     Run an applet's frames this long before drawing");
            println!("  --fresh               With an applet id, render it instead of capturing it running");
            println!();
            println!("A directory is rendered headlessly; so is a script. An applet's id instead");
            println!("captures the frame it is showing if it is running (Windows), and otherwise");
            println!("renders its installed copy.");
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
        Some("start") => {
            println!("Usage: dew start [OPTIONS]");
            println!();
            println!("Start the service with no applet named: whatever is installed and enabled");
            println!("loads on its own (Windows only). `dew run <path>` stays the way to see one");
            println!("specific applet, installed or not.");
            println!();
            println!("Options:");
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
            println!("Usage: dew install <PATH> [OPTIONS]");
            println!("       dew install @<owner_user_id>/<applet_id> [OPTIONS]");
            println!();
            println!("Copy an applet into the per-user store, so a Dew service loads it on its next start.");
            println!("The @owner/id form fetches a public package from the marketplace (dew login first).");
            println!();
            println!("Arguments:");
            println!(
                "  <PATH>                Applet directory or .dewpkg file (its dew.toml names its id)"
            );
            println!(
                "  @<owner>/<id>         A public package's owner and applet id, from `dew discover`"
            );
            println!();
            println!("Options:");
            println!("  --force               Replace an already-installed applet with this id");
        }
        Some("package") => {
            println!("Usage: dew package <DIR> [OPTIONS]");
            println!();
            println!("Zip an applet directory into a .dewpkg file that dew install accepts.");
            println!();
            println!("Arguments:");
            println!("  <DIR>                 Applet directory (its dew.toml names its id)");
            println!();
            println!("Options:");
            println!("  --output, -o <PATH>   Output file (defaults to <manifest id>.dewpkg)");
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
        Some("login") => {
            println!("Usage: dew login");
            println!();
            println!("Sign in with a Dew account (email and password), prompted interactively.");
            println!("Persists a session under %LOCALAPPDATA%\\Dew for later commands to use.");
        }
        Some("signup") => {
            println!("Usage: dew signup");
            println!();
            println!("Create a Dew account. Sends a confirmation email; run `dew login` after");
            println!("clicking the link it contains.");
        }
        Some("logout") => {
            println!("Usage: dew logout");
            println!();
            println!("Clear the stored session.");
        }
        Some("whoami") => {
            println!("Usage: dew whoami");
            println!();
            println!("Show which account, if any, is currently signed in.");
        }
        Some("discover") => {
            println!("Usage: dew discover");
            println!();
            println!("List public packages on the deployed marketplace (dew login first).");
        }
        Some("sync") => {
            println!("Usage: dew sync");
            println!();
            println!("Restore this account's own synced applets that are missing locally.");
            println!("An applet synced from someone else's public package cannot be restored this");
            println!("way and is reported for manual reinstall instead (dew login first).");
        }
        Some("publish") => {
            println!("Usage: dew publish <PATH> [OPTIONS]");
            println!();
            println!("Zip an applet directory and upload it to the marketplace (dew login first).");
            println!();
            println!("Arguments:");
            println!("  <PATH>                Applet directory (its dew.toml names its id)");
            println!();
            println!("Options:");
            println!("  --public              Make the package discoverable via `dew discover`");
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
            println!(
                "  start            Start the service with whatever is installed (Windows only)"
            );
            println!("  snapshot <PATH|ID>  Render an applet or a script to a PNG, or capture a running one");
            println!("  check [PATH...]  Validate manifests and entry points, or this directory");
            println!(
                "  install <PATH>   Copy an applet (directory or .dewpkg) into the per-user store (Windows only)"
            );
            println!(
                "  package <DIR>    Zip an applet directory into a .dewpkg file (Windows only)"
            );
            println!("  uninstall <ID>   Remove an applet from the per-user store (Windows only)");
            println!("  login            Sign in with a Dew account (Windows only)");
            println!("  signup           Create a Dew account (Windows only)");
            println!("  logout           Clear the stored session (Windows only)");
            println!("  whoami           Show which account is signed in (Windows only)");
            println!("  discover         List public packages on the marketplace (Windows only)");
            println!("  sync             Restore this account's own synced applets (Windows only)");
            println!(
                "  publish <PATH>   Zip and upload an applet to the marketplace (Windows only)"
            );
            println!("  init <NAME>      Scaffold a new applet");
            println!("  test             Run Luau test suites against Dew's DataModel");
            println!("  conformance      Run the layout conformance suite");
            println!("  compat [--check] Regenerate the examples compatibility table, or check it");
            println!("  help [COMMAND]   Show help for a command");
            println!();
            println!("Options:");
            println!("  --version, -V    Show the build identifier and exit");
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
        Command::Start { stats, bench } => execute_start(stats, bench),
        Command::Snapshot {
            target,
            output,
            after,
        } => execute_snapshot(target, output, after),
        Command::Check { targets } => execute_check(targets),
        Command::Install { path, force } => execute_install(path, force),
        Command::InstallFromMarketplace {
            owner_user_id,
            applet_id,
            force,
        } => execute_install_from_marketplace(owner_user_id, applet_id, force),
        Command::Package { dir, output } => execute_package(dir, output),
        Command::Uninstall { id } => execute_uninstall(id),
        Command::Login => execute_login(),
        Command::Signup => execute_signup(),
        Command::Logout => execute_logout(),
        Command::Whoami => execute_whoami(),
        Command::Discover => execute_discover(),
        Command::Sync => execute_sync(),
        Command::Publish { path, public } => execute_publish(path, public),
        Command::Init {
            name,
            surface,
            size,
        } => execute_init(name, surface, size),
        Command::Test { filter, dir } => execute_test(filter, dir),
        Command::Conformance {
            filter,
            dir,
            pixel,
            generate_goldens,
            run_unsupported,
        } => execute_conformance(filter, dir, pixel, generate_goldens, run_unsupported),
        Command::Compat { check } => examples_compat::execute(check),
        Command::Help { subcommand } => {
            execute_help(subcommand);
            Ok(())
        }
        Command::Version => {
            println!("{BUILD_IDENTIFIER}");
            Ok(())
        }
    }
}

/// Prints the build identifier before Rust's own panic message, so a crash
/// report carries it without whoever files one needing to know to run `dew
/// --version` separately -- which may not even be possible if the crash is a
/// hang rather than a return.
///
/// READS ONLY `BUILD_IDENTIFIER`, A `const`. A hook that panics while
/// formatting its own message turns one panic into a much worse, harder to
/// diagnose double panic, so there is nothing here that can fail: no lock,
/// no lookup, just a string cargo baked in at compile time.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("dew {BUILD_IDENTIFIER}: about to panic");
        default_hook(info);
    }));
}

/// Declares this process per-monitor-v2 DPI aware, before anything creates a
/// window.
///
/// WHAT THIS FIXES: every window Dew creates is sized and positioned in raw
/// pixels a widget author chose -- `width = 260, height = 120` means exactly
/// that, on any monitor, the same way a Rainmeter skin is pixel-precise
/// rather than scaled. A process that never declares a DPI awareness level
/// defaults to one Windows itself compensates for: it renders once, at
/// whichever monitor's DPI the process started on, and Windows silently
/// bitmap-stretches the presented window whenever it ends up on a monitor
/// with a different scale factor. That reads as "the content stretches when
/// I drag the window," because that is exactly what is happening to it.
///
/// NOTHING ELSE CHANGES. This crate has never scaled anything by DPI and
/// still does not -- once declared aware, Windows stops compensating on
/// this process's behalf, and every window simply keeps the exact physical
/// pixel size it already had. `WM_DPICHANGED` is deliberately left
/// unhandled everywhere: the default behavior for an unhandled one is to do
/// nothing, which is the pixel-precise behavior this process wants, not an
/// oversight to fill in later.
///
/// BEST-EFFORT: `SetProcessDpiAwarenessContext` was added in the Creators
/// Update (1703). A failure here means an older Windows, where the process
/// falls back to whatever default it already had -- worth being silent
/// about rather than refusing to start over a display setting nobody but a
/// multi-monitor, mixed-DPI setup would ever notice.
#[cfg(windows)]
fn declare_dpi_awareness() {
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[cfg(not(windows))]
fn declare_dpi_awareness() {}

fn main() -> ExitCode {
    declare_dpi_awareness();
    install_panic_hook();
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
        crate::manifest::Permission::System,
    ];
    let grant = crate::capabilities::SurfaceGrant {
        requested: Arc::new(Mutex::new(None)),
        root: Some(mlua::IntoLua::into_lua(handle.clone(), lua)?),
        title: "test".to_string(),
    };
    let desktop = crate::capabilities::build(lua, &granted, state, &grant)?;
    lua.globals().set("desktop", desktop)?;

    let harness = lua.create_table()?;
    harness.set("Root", handle)?;

    //  RECORDED AND DELIVERED, in the order the renderer uses. A framework polls
    //  `services` for where the pointer is and connects to the input service for
    //  what happened; driving one without the other leaves half of it reading a
    //  pointer that never moved.
    //
    //  ONE POINTER FOR THE WHOLE SUITE, as the window loop keeps one. A release
    //  completes a click only on the element its press landed on, and the
    //  pointer is what remembers where that was; a fresh one per call forgot
    //  every press, so a plain `GuiButton` never fired `Activated` here.
    let shared = Arc::new(Mutex::new(dew_host::datamodel::input::Pointer::default()));
    //  THE SIZE THE LAST `Settle` LAID OUT AT, which the pointer hit-tests
    //  at. At any other size it finds elements where the test never saw them.
    let settled = Arc::new(Mutex::new((0.0f32, 0.0f32)));
    let pointer = lua.create_table()?;
    let surface_dom = dom.clone();
    let moving = shared.clone();
    let moving_size = settled.clone();
    pointer.set(
        "Move",
        lua.create_function(move |lua, (x, y): (f32, f32)| {
            crate::services::pointer_moved(x, y);
            let surface = dew_host::datamodel::input::Surface {
                lua,
                dom: &surface_dom,
                root,
                size: *moving_size.lock().expect("size"),
            };
            let _ = moving.lock().expect("pointer").moved(&surface, x, y);
            Ok(())
        })?,
    )?;

    for (name, down) in [("Down", true), ("Up", false)] {
        let surface_dom = dom.clone();
        let pressing = shared.clone();
        let pressing_size = settled.clone();
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
                    size: *pressing_size.lock().expect("size"),
                };
                let kind = match button {
                    1 => dew_host::datamodel::input::Button::Right,
                    2 => dew_host::datamodel::input::Button::Middle,
                    _ => dew_host::datamodel::input::Button::Left,
                };
                let mut p = pressing.lock().expect("pointer");
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
            *settled.lock().expect("size") = (w, h);
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
        let raw = scaffold_manifest("my_applet", "My_applet", manifest::Permission::Window);
        let parsed = manifest::Manifest::parse(&raw, "dew.toml")
            .unwrap_or_else(|e| panic!("init wrote a manifest that will not parse: {e}"));

        assert_eq!(parsed.id, "my_applet");
        assert!(parsed.unknown.is_empty());
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
            let raw = scaffold_manifest("a", "A", surface);
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
            Command::Snapshot { target, output, .. } => {
                assert_eq!(output, "out.png");
                match target {
                    SnapshotTarget::Applet(got) => assert_eq!(got, dir),
                    _ => panic!("a directory should read as an applet"),
                }
            }
            _ => panic!("expected Snapshot command"),
        }
    }

    #[test]
    fn snapshot_takes_an_applet_id() {
        // A BARE WORD THAT IS NOT A FILE OR DIRECTORY HERE names an applet,
        // captured running by default and rendered fresh with `--fresh`.
        let args = ["snapshot", "monitor", "-o", "monitor.png"]
            .into_iter()
            .map(|s| s.to_string());
        match parse_args(args, false).unwrap() {
            Command::Snapshot { target, output, .. } => {
                assert_eq!(output, "monitor.png");
                assert_eq!(
                    target,
                    SnapshotTarget::Installed {
                        id: "monitor".to_string(),
                        fresh: false
                    }
                );
            }
            _ => panic!("expected Snapshot command"),
        }

        let args = ["snapshot", "monitor", "--fresh", "--after", "5"]
            .into_iter()
            .map(|s| s.to_string());
        match parse_args(args, false).unwrap() {
            Command::Snapshot { target, after, .. } => {
                assert_eq!(
                    target,
                    SnapshotTarget::Installed {
                        id: "monitor".to_string(),
                        fresh: true
                    }
                );
                assert_eq!(after, Some(std::time::Duration::from_secs(5)));
            }
            _ => panic!("expected Snapshot command"),
        }
    }

    #[test]
    fn snapshot_still_reports_a_mistyped_path() {
        // A PATH THAT IS NOT THERE IS STILL A TYPO, not an id: a separator or
        // a script's extension says it was meant as a path.
        for typo in ["examples/clok", "app.luau", "shot.png"] {
            let args = ["snapshot", typo].into_iter().map(|s| s.to_string());
            let Err(message) = parse_args(args, false) else {
                panic!("{typo} is a missing path, not an applet id");
            };
            assert!(message.contains("no such file or directory"), "{message}");
        }

        let dir = std::env::temp_dir().join("dew-cli-test-applet-fresh");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let args = ["snapshot", dir.to_str().expect("utf-8"), "--fresh"]
            .into_iter()
            .map(|s| s.to_string());
        assert!(
            parse_args(args, false).is_err(),
            "--fresh means nothing for a directory"
        );
    }

    #[test]
    fn snapshot_after_runs_an_applet_and_refuses_a_script() {
        let dir = std::env::temp_dir().join("dew-cli-test-applet-after");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let args = ["snapshot", dir.to_str().expect("utf-8"), "--after", "2.5"]
            .into_iter()
            .map(|s| s.to_string());
        match parse_args(args, false).unwrap() {
            Command::Snapshot { after, .. } => {
                assert_eq!(after, Some(std::time::Duration::from_millis(2500)))
            }
            _ => panic!("expected Snapshot command"),
        }

        let bad = ["snapshot", dir.to_str().expect("utf-8"), "--after", "-1"]
            .into_iter()
            .map(|s| s.to_string());
        assert!(
            parse_args(bad, false).is_err(),
            "a negative wait is refused"
        );

        let script = std::env::temp_dir().join("dew-cli-test-after.luau");
        std::fs::write(&script, "").expect("temp script");
        let args = ["snapshot", script.to_str().expect("utf-8"), "--after", "1"]
            .into_iter()
            .map(|s| s.to_string());
        let Err(message) = parse_args(args, false) else {
            panic!("a script has no frames to run");
        };
        assert!(message.contains("--after"), "{message}");
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
            Command::Snapshot { target, output, .. } => {
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
        let args = ["init", "my_mod", "--size", "400x200"]
            .iter()
            .map(|s| s.to_string());
        let cmd = parse_args(args, false).unwrap();
        match cmd {
            Command::Init {
                name,
                surface: _,
                size,
            } => {
                assert_eq!(name, "my_mod");
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
    fn version_short_and_long_flags_agree() {
        for spelling in ["version", "--version", "-V"] {
            let words = [spelling];
            let args = words.iter().map(|s| s.to_string());
            assert_eq!(
                parse_args(args, false).unwrap(),
                Command::Version,
                "{spelling:?} should parse as Command::Version"
            );
        }
    }

    #[test]
    fn login_signup_logout_whoami_parse_with_no_arguments() {
        for (word, expected) in [
            ("login", Command::Login),
            ("signup", Command::Signup),
            ("logout", Command::Logout),
            ("whoami", Command::Whoami),
        ] {
            let words = [word];
            let args = words.iter().map(|s| s.to_string());
            assert_eq!(
                parse_args(args, false).unwrap(),
                expected,
                "{word:?} should parse with no arguments"
            );
        }
    }

    #[test]
    fn discover_parses_with_no_arguments() {
        let words = ["discover"];
        let args = words.iter().map(|s| s.to_string());
        assert_eq!(parse_args(args, false).unwrap(), Command::Discover);
    }

    #[test]
    fn sync_parses_with_no_arguments() {
        let words = ["sync"];
        let args = words.iter().map(|s| s.to_string());
        assert_eq!(parse_args(args, false).unwrap(), Command::Sync);
    }

    #[test]
    fn publish_defaults_to_private() {
        let words = ["publish", "examples/host/basic-widget"];
        let args = words.iter().map(|s| s.to_string());
        assert_eq!(
            parse_args(args, false).unwrap(),
            Command::Publish {
                path: PathBuf::from("examples/host/basic-widget"),
                public: false,
            }
        );
    }

    #[test]
    fn publish_accepts_the_public_flag() {
        let words = ["publish", "examples/host/basic-widget", "--public"];
        let args = words.iter().map(|s| s.to_string());
        assert_eq!(
            parse_args(args, false).unwrap(),
            Command::Publish {
                path: PathBuf::from("examples/host/basic-widget"),
                public: true,
            }
        );
    }

    #[test]
    fn start_parses_with_no_arguments() {
        let words = ["start"];
        let args = words.iter().map(|s| s.to_string());
        assert_eq!(
            parse_args(args, true).unwrap(),
            Command::Start {
                stats: false,
                bench: false,
            }
        );
    }

    #[test]
    fn start_accepts_stats_and_bench() {
        let words = ["start", "--stats", "--bench"];
        let args = words.iter().map(|s| s.to_string());
        assert_eq!(
            parse_args(args, true).unwrap(),
            Command::Start {
                stats: true,
                bench: true,
            }
        );
    }

    #[test]
    fn start_refuses_an_applet_path() {
        let words = ["start", "examples/host/basic-widget"];
        let args = words.iter().map(|s| s.to_string());
        assert!(parse_args(args, false).is_err());
    }

    #[test]
    fn start_rejected_off_windows() {
        let words = ["start"];
        let args = words.iter().map(|s| s.to_string());
        let err = parse_args(args, false).unwrap_err();
        assert_eq!(
            err,
            "'start' is Windows-only: it drives the window loop; use --snapshot to render headlessly"
        );
    }

    #[test]
    fn install_with_an_at_prefix_parses_as_a_marketplace_reference() {
        let words = [
            "install",
            "@00000000-0000-0000-0000-000000000000/calculator",
        ];
        let args = words.iter().map(|s| s.to_string());
        assert_eq!(
            parse_args(args, false).unwrap(),
            Command::InstallFromMarketplace {
                owner_user_id: "00000000-0000-0000-0000-000000000000".to_string(),
                applet_id: "calculator".to_string(),
                force: false,
            }
        );
    }

    #[test]
    fn install_without_an_at_prefix_still_parses_as_a_path() {
        let words = ["install", "examples/host/basic-widget"];
        let args = words.iter().map(|s| s.to_string());
        assert_eq!(
            parse_args(args, false).unwrap(),
            Command::Install {
                path: PathBuf::from("examples/host/basic-widget"),
                force: false,
            }
        );
    }

    #[test]
    fn install_with_an_at_prefix_but_no_slash_is_refused() {
        let words = ["install", "@no-slash-here"];
        let args = words.iter().map(|s| s.to_string());
        assert!(parse_args(args, false).is_err());
    }

    #[test]
    fn the_build_identifier_carries_the_cargo_version_and_a_build_number() {
        // NOT ASSERTING AN EXACT COUNT -- it moves every commit. What must
        // hold regardless: the Cargo version is the prefix, a `.` introduces
        // a fourth, purely numeric segment after it -- the dotted,
        // Roblox-shaped form `--version`'s real output and the sprint record
        // both rely on -- and that segment parses as a number rather than
        // carrying a stray letter the way the old `g<sha>` form did.
        assert!(BUILD_IDENTIFIER.starts_with(env!("CARGO_PKG_VERSION")));
        let (_, build_number) = BUILD_IDENTIFIER
            .rsplit_once('.')
            .expect("a fourth, dot-separated segment");
        assert!(
            build_number.parse::<u64>().is_ok(),
            "{build_number:?} should be a plain number"
        );
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

    /// `DewTest.Pointer` hit-tests at the size the last `Settle` laid out at.
    /// A button placed by scale sits at the middle of that size, and only
    /// there, so a click on it lands only when the pointer uses that size.
    #[test]
    fn the_test_pointer_hits_at_the_settled_size() {
        let lua = mlua::Lua::new();
        let dom = datamodel::SharedDom::default();
        datamodel::install(&lua, &dom).expect("install");
        datamodel::install_vocabulary(&lua).expect("vocabulary");
        let state: crate::capabilities::Shared =
            Arc::new(Mutex::new(crate::capabilities::HostState::default()));
        install_test_surface(&lua, &dom, &state).expect("test surface");

        let clicked: bool = lua
            .load(
                r#"
                local button = Instance.new("TextButton")
                button.AnchorPoint = Vector2.new(0.5, 0.5)
                button.Position = UDim2.fromScale(0.5, 0.5)
                button.Size = UDim2.fromOffset(80, 40)
                button.Parent = DewTest.Root
                local clicked = false
                button.Activated:Connect(function()
                    clicked = true
                end)
                DewTest.Settle({ width = 400, height = 300 })
                DewTest.Pointer.Move(200, 150)
                DewTest.Pointer.Down(200, 150)
                DewTest.Pointer.Up(200, 150)
                return clicked
            "#,
            )
            .eval()
            .expect("click");
        assert!(clicked, "a click at the button's centre did not reach it");
    }

    #[test]
    #[cfg(windows)]
    fn test_multi_monitor_place_widget_negative_and_seam_avoidance() {
        use dew_window::{DisplayTopology, MonitorInfo, Rect};

        // Monitor 1: Secondary to the left (-1920..0, 0..1080) with taskbar on bottom (rcWork -1920..0, 0..1040)
        let secondary = MonitorInfo {
            device_name: r"\\.\DISPLAY1".to_string(),
            monitor_rect: Rect::new(-1920, 0, 0, 1080),
            work_area: Rect::new(-1920, 0, 0, 1040),
            is_primary: false,
        };
        // Monitor 2: Primary landscape (0..1920, 0..1080) with taskbar on bottom (rcWork 0..1920, 0..1040)
        let primary = MonitorInfo {
            device_name: r"\\.\DISPLAY2".to_string(),
            monitor_rect: Rect::new(0, 0, 1920, 1080),
            work_area: Rect::new(0, 0, 1920, 1040),
            is_primary: true,
        };
        let topology = DisplayTopology {
            monitors: vec![secondary.clone(), primary.clone()],
        };

        let widget_size = (300, 200);

        // 1. Clamping to secondary monitor with negative coordinates
        // Candidate (-1000, 500) inside secondary
        let placed = place_widget((-1000, 500), widget_size, true, true, &topology);
        assert_eq!(placed, (-1000, 500));

        // Candidate (-2000, 500) extending past secondary left edge
        // Should clamp to work.left (-1920)
        let placed = place_widget((-2000, 500), widget_size, false, true, &topology);
        assert_eq!(placed.0, -1920);

        // 2. Snapping to outer left edge on secondary monitor
        let placed = place_widget((-1915, 500), widget_size, true, true, &topology);
        assert_eq!(placed.0, -1920);

        // 3. Seam snapping: when snap_to_edges is true, candidate right edge near x = 0 (e.g. x = -305, right = -5)
        // snaps cleanly to the seam (x = -300, so right is exactly 0).
        let placed = place_widget((-305, 500), widget_size, true, true, &topology);
        assert_eq!(
            placed.0, -300,
            "snaps cleanly to right edge of secondary monitor"
        );

        // 4. Inter-monitor straddling: when snap_to_edges is false, widget can sit freely
        // halfway across the boundary (e.g. x = -150: 150px on secondary, 150px on primary).
        let placed = place_widget((-150, 500), widget_size, false, true, &topology);
        assert_eq!(
            placed.0, -150,
            "allows free placement straddling two monitors when snap_to_edges is false"
        );

        // 5. Clamping on Primary monitor bottom boundary (taskbar at 1040)
        // Candidate at (500, 900), height is 200 -> bottom would be 1100 (past 1040).
        let placed = place_widget((500, 900), widget_size, false, true, &topology);
        assert_eq!(
            placed.1,
            1040 - 200,
            "must clamp to primary work area bottom"
        );

        // 6. L-shaped / multi-monitor void protection:
        // Consider an L-shaped topology where secondary monitor is taller (-444..1604)
        // while primary is only (0..1080).
        // A candidate at (500, -200) lies inside the union bounding box [-1920..1920, -444..1604],
        // but completely outside the primary monitor (0..1920, 0..1040) where it has centroid affinity.
        let l_secondary = MonitorInfo {
            device_name: r"\\.\DISPLAY1".to_string(),
            monitor_rect: Rect::new(-1920, -444, 0, 1604),
            work_area: Rect::new(-1920, -444, 0, 1604),
            is_primary: false,
        };
        let l_topology = DisplayTopology {
            monitors: vec![l_secondary, primary.clone()],
        };
        let placed = place_widget((500, -200), widget_size, false, true, &l_topology);
        assert!(
            l_topology.intersects_any_work_area(&Rect::new(
                placed.0,
                placed.1,
                placed.0 + widget_size.0 as i32,
                placed.1 + widget_size.1 as i32
            )),
            "widget must not remain in dead space void outside all monitor work areas"
        );
        assert_eq!(
            placed.1, 0,
            "clamped back to top of primary monitor work area"
        );

        // Candidate partially dragged off the top of the middle monitor (e.g. y = -50)
        // With snapToEdges = false and keepOnScreen = true, because Edge::Top has no monitor above it,
        // it must clamp cleanly to work.top (0), rather than floating into the void above.
        let placed_top_drag = place_widget((500, -50), widget_size, false, true, &l_topology);
        assert_eq!(
            placed_top_drag.1, 0,
            "partially dragged off top of middle monitor clamps to work.top"
        );

        // 7. Corner seam sliding:
        // Widget of size (300, 200). Suppose secondary monitor spans Y: 0..1040,
        // and candidate on primary monitor is at (-100, 950).
        // Since Y: 950..1150 exceeds secondary monitor's bottom (1040), the widget cannot
        // stay at Y=950 while straddling X=-100.
        // Instead of snapping discontinuously across the centroid, it slides to the closest
        // valid on-screen location: Y clamps to 840 (so bottom is 1040), allowing X to remain at -100!
        let staggered_secondary = MonitorInfo {
            device_name: r"\\.\DISPLAY1".to_string(),
            monitor_rect: Rect::new(-1920, 0, 0, 1040),
            work_area: Rect::new(-1920, 0, 0, 1040),
            is_primary: false,
        };
        let staggered_topology = DisplayTopology {
            monitors: vec![staggered_secondary, primary.clone()],
        };
        // Candidate (-100, 950): overlaps seam to the left, but bottom is 1150 (past 1040).
        let placed_corner =
            place_widget((-100, 950), widget_size, false, true, &staggered_topology);
        assert_eq!(placed_corner.0, -100, "widget slides smoothly on x axis");
        assert_eq!(
            placed_corner.1,
            1040 - 200,
            "widget y clamps smoothly to 840 so the entire widget stays on screen"
        );

        // 8. Dragging along top edge towards a taller monitor:
        // Left monitor spans (-1920..0, -444..1604), Middle monitor spans (0..1920, 0..1040).
        // Candidate (-100, -100) or (-250, -100): The user is dragging clamped along the top edge of Middle monitor (y = 0),
        // moving leftwards towards the taller Left monitor.
        // As long as the widget still partially overlaps Middle monitor (x + w > 0), it is clamped vertically
        // to Middle monitor's top edge (y = 0), and X slides completely freely without being clamped horizontally to -300!
        let placed_l_corner = place_widget((-100, -100), widget_size, false, true, &l_topology);
        assert_eq!(placed_l_corner.0, -100);
        assert_eq!(placed_l_corner.1, 0);

        // Candidate (-250, -100) is closer to the vertical wall at x = -300 (dist 50) than to ceiling y = 0 (dist 100),
        // so it clamps horizontally to x = -300 without snapping down onto middle monitor:
        let placed_l_wall = place_widget((-250, -100), widget_size, false, true, &l_topology);
        assert_eq!(placed_l_wall.0, -300);
        assert_eq!(placed_l_wall.1, -100);

        // Once the widget has completely cleared Middle monitor's top edge (x + w <= 0, e.g. x = -300),
        // it is 100% within the taller Left monitor and can move vertically upwards (y < 0):
        let placed_cleared = place_widget((-350, -100), widget_size, false, true, &l_topology);
        assert_eq!(placed_cleared.0, -350);
        assert_eq!(placed_cleared.1, -100);
    }

    #[test]
    #[cfg(windows)]
    fn test_drag_instrumentation_user_monitors() {
        let display1 = dew_window::MonitorInfo {
            device_name: r"\\.\DISPLAY1".to_string(),
            monitor_rect: Rect::new(-1152, -444, 0, 1604),
            work_area: Rect::new(-1152, -444, 0, 1604),
            is_primary: false,
        };
        let display3 = dew_window::MonitorInfo {
            device_name: r"\\.\DISPLAY3".to_string(),
            monitor_rect: Rect::new(0, 0, 2560, 1440),
            work_area: Rect::new(0, 0, 2560, 1440),
            is_primary: true,
        };
        let display2 = dew_window::MonitorInfo {
            device_name: r"\\.\DISPLAY2".to_string(),
            monitor_rect: Rect::new(2560, 162, 4480, 1242),
            work_area: Rect::new(2560, 162, 4480, 1242),
            is_primary: false,
        };
        let topology = DisplayTopology {
            monitors: vec![display1, display3, display2],
        };
        let size = (300, 695);

        let check_path = |name: &str, start: (i32, i32), end: (i32, i32)| -> Vec<String> {
            let mut snaps = Vec::new();
            let steps = (end.0 - start.0).abs().max((end.1 - start.1).abs());
            let mut prev = place_widget(start, size, false, true, &topology);
            for i in 1..=steps {
                let cand_x = start.0 + (end.0 - start.0) * i / steps;
                let cand_y = start.1 + (end.1 - start.1) * i / steps;
                let curr = place_widget_with_history(
                    (cand_x, cand_y),
                    size,
                    false,
                    true,
                    &topology,
                    Some(prev),
                );
                let dx = (curr.0 - prev.0).abs();
                let dy = (curr.1 - prev.1).abs();
                if dx > 1 || dy > 1 {
                    snaps.push(format!(
                        "[{name}] step {i}/{steps} at cand ({cand_x}, {cand_y}): jumped from {prev:?} to {curr:?}, delta=({dx}, {dy})"
                    ));
                }
                prev = curr;
            }
            snaps
        };

        // 1. Sliding left along top edge of middle monitor into tall left monitor (y clamped at 0)
        // Along the top edge from 300 down to -299, there must be ZERO snaps (smooth sliding along ceiling at y = 0)
        let snaps1_ceiling = check_path(
            "Slide Left along Middle Top to Seam",
            (300, -50),
            (-299, -50),
        );
        assert!(
            snaps1_ceiling.is_empty(),
            "snaps1_ceiling had {} snaps: {:#?}",
            snaps1_ceiling.len(),
            snaps1_ceiling
        );
        // At x = -300, the widget completely clears Middle monitor's top edge and snaps vertically to candidate y = -50:
        let placed_300 =
            place_widget_with_history((-300, -50), size, false, true, &topology, Some((-299, 0)));
        assert_eq!(
            placed_300,
            (-300, -50),
            "widget snaps vertically only once cleared at x = -300"
        );

        // 2. Dragging right against seam wall in tall left monitor at y = -100 (where middle monitor does not exist)
        // Window must clamp to x = -300 without snapping down onto middle monitor (y remains -100)
        let snaps2 = check_path("Push Right into Seam at y=-100", (-500, -100), (200, -100));
        assert!(
            snaps2.is_empty(),
            "snaps2 had {} snaps: {:#?}",
            snaps2.len(),
            snaps2
        );

        // 2b. Dragging right from the top-right corner of vertical monitor at y = -50:
        // Window must remain clamped at x = -300 and y = -50 without re-engaging vertical clamping at y = 0
        let snaps2_top_right = check_path(
            "Push Right from Top-Right Corner at y=-50",
            (-300, -50),
            (200, -50),
        );
        assert!(
            snaps2_top_right.is_empty(),
            "snaps2_top_right had {} snaps: {:#?}",
            snaps2_top_right.len(),
            snaps2_top_right
        );

        // 3. Sliding left along bottom edge of middle monitor towards left monitor (bottom = 1440, so y = 745)
        // Along the bottom edge from 300 down to -299, there must be ZERO snaps (smooth sliding along floor at y = 745)
        let snaps3_floor = check_path(
            "Slide Left along Middle Bottom to Seam",
            (300, 800),
            (-299, 800),
        );
        assert!(
            snaps3_floor.is_empty(),
            "snaps3_floor had {} snaps: {:#?}",
            snaps3_floor.len(),
            snaps3_floor
        );
        let placed_bottom_300 =
            place_widget_with_history((-300, 800), size, false, true, &topology, Some((-299, 745)));
        assert_eq!(
            placed_bottom_300,
            (-300, 800),
            "widget snaps vertically only once cleared at bottom x = -300"
        );

        // 3b. Dragging right from the bottom-right corner of vertical monitor at y = 800:
        // Window must remain clamped at x = -300 and y = 800 without re-engaging vertical clamping at y = 745
        let snaps3_bottom_right = check_path(
            "Push Right from Bottom-Right Corner at y=800",
            (-300, 800),
            (200, 800),
        );
        assert!(
            snaps3_bottom_right.is_empty(),
            "snaps3_bottom_right had {} snaps: {:#?}",
            snaps3_bottom_right.len(),
            snaps3_bottom_right
        );

        // 4. Sliding right along top edge of middle monitor towards right monitor (which starts at y = 162)
        // From 2000 to 2260, window slides smoothly along top edge at y = 0, clamping against right monitor's wall
        let snaps4 = check_path(
            "Slide Right along Middle Top towards Right",
            (2000, 50),
            (2500, 50),
        );
        assert!(
            snaps4.is_empty(),
            "snaps4 had {} snaps: {:#?}",
            snaps4.len(),
            snaps4
        );

        // 5. Sliding right along bottom edge of middle monitor towards right monitor (which ends at y = 1242)
        let snaps5 = check_path(
            "Slide Right along Middle Bottom towards Right",
            (2000, 600),
            (2500, 600),
        );
        assert!(
            snaps5.is_empty(),
            "snaps5 had {} snaps: {:#?}",
            snaps5.len(),
            snaps5
        );
    }
}
