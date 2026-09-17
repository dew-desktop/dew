//! `dew.Text`, `dew.Clock`, `dew.Pointer`, `dew.Input`: the host facts a guest
//! framework needs and no guest can compute.
//!
//! WHERE THIS FILE LIVES, AND WHY IT MOVED BEFORE IT WAS EVER COMMITTED HERE
//! It was written as `datamodel/services.rs` and `verify_boundaries` refused it,
//! correctly: section 3 lets only the render bridge reach `dew_raster`, on the
//! grounds that a DataModel module deferring to those crates would put a second
//! implementation of the standard inside one host. Measuring a string needs a
//! face, so the import was real -- and the right conclusion was that the FILE was
//! in the wrong place rather than that the rule was too tight. Nothing here is a
//! DataModel member; see the next paragraph, which said so before the checker
//! did. It sits beside `capabilities.rs` instead, which is the other file
//! answering "what is a mod given".
//!
//! WHAT THESE ARE, AND WHY THEY ARE NOT PROPERTIES
//! Everything else in this module is a DataModel: instances, properties, events,
//! a tree. These are not. "How wide is this string in the face this host will
//! actually draw with" and "call me when the host draws a frame" are questions
//! ABOUT THE HOST, and no amount of tree makes them answerable from Luau. On
//! the engine they are `TextService:GetTextSize` and `RunService.Heartbeat`, reached
//! through `game`, and Dew has to spell them itself -- `game` would not help even
//! if it were here, because these are services behind it rather than the tree it
//! names. (`game` itself came back into scope on 2026-09-04, for vide's gate and
//! nothing else; see the roadmap's "Why `game` was dropped from step F, and why
//! it came back". This comment used to cite the half of that section that was
//! later corrected, for a claim the section never had to carry.)
//!
//! WHY MEMBERS OF `dew`, AND UNGATED ONES. `capabilities.rs` is a security
//! boundary, and a capability is a thing a mod may be DENIED. There is no
//! coherent "no" here: a layout pass refused text metrics cannot lay out, and a
//! mod refused frames cannot animate -- neither degrades, both stop. A
//! permission with only one sound answer is not a permission, it is ceremony. So
//! `install` and `install_pointer` below add their members directly to the same
//! `dew` table `capabilities::build` returns, on the same terms `Time` was
//! always on: present for every guest, whichever of the two functions runs
//! first, ungated.
//!
//! WHY DEW'S OWN NAMES, AND NOT `TextService`/`RunService`. Naming these
//! `TextService` and `RunService` would LOOK like the engine and be a lie in the
//! direction that costs most: on the engine those are not globals, so code
//! written against them would run HERE and nowhere else. That is parity
//! theatre. `Text` and `Clock`, read off `dew` the way `Time` already is, cannot
//! be mistaken for portable.
//!
//! THE STANDARD QUESTION IS ANSWERED, AND THE SHAPE IS STILL DEW'S. Whether a
//! conforming host MUST expose text metrics and a frame clock was flagged here
//! and answered YES on 2026-09-04, in `docs/host_services.md` -- its own
//! hand-written half of the standard, not a section of the generated
//! `datamodel_scope.md`, because neither service is a member of any class and
//! regenerating that document after these landed produced a byte-identical file.
//!
//! What the standard requires is the SHAPE: synchronous measurement, a frame
//! subscription that returns a disposer, a monotonic `now`, and no way for a
//! guest to step a host that drives its own frames. It requires no name, so
//! `dew.Text` and `dew.Clock` conform as they stand and are still expected to
//! be revisited only if the standard ever does specify one.

use dew_raster::Font;
use mlua::prelude::*;
use mlua::WeakLua;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

// -- Text --------------------------------------------------------------------

/// The face this host measures AND draws with, loaded once.
///
/// ONE FACE, ONE ID, and the memo is the point rather than a micro-optimisation.
/// `Font::load` pushes the file into the rasteriser's process-wide store and
/// hands back a fresh id every time it is called, so a measurement path that
/// loaded its own would hold a second copy of a multi-megabyte file AND could
/// measure in a face the painter is not drawing in. `main.rs` takes its painter
/// font from here for that second reason: a measurement that does not describe
/// the pixels is worse than no measurement at all.
pub fn face() -> Option<Font> {
    static FACE: OnceLock<Option<Font>> = OnceLock::new();
    *FACE.get_or_init(|| {
        let path = dew_runtime::font::system_font()?;
        Font::load(&path.to_string_lossy(), 0)
    })
}

/// Measure a string in the face this host draws with.
///
/// SYNCHRONOUS, AND THAT IS THE CONTRACT RATHER THAN AN IMPLEMENTATION DETAIL.
/// Layout runs inside a frame and cannot await -- which is why Aether's
/// `Host.Text` is shaped this way, and why the engine host reaches for
/// `GetTextSize` rather than for anything that yields. There is nothing to await
/// here in any case: the face is in memory and this is arithmetic over glyph
/// advances.
///
/// ERRORS RATHER THAN ANSWERING ZERO when there is no face. A zero width does not
/// look like a failure, it looks like an empty string, and it COLLAPSES the
/// element that asked -- so a container with no font would lay out as though
/// every label in it were blank. An error is recoverable and a collapse is not:
/// Aether's `Text.Measure` catches a failing provider and falls back to its
/// bundled advance table, which is approximate and visible, which is right.
/// LINE HEIGHT IS 1.5x TEXTSIZE per line, and that is a rule of the DataModel
/// layout standard (`LAYOUT.md` section 7) rather than of the underlying font:
/// in Studio and in conformance, a single line of text in an auto-sized element
/// resolves to exactly 1.5 x TextSize, while raw font typographic metrics
/// vary by font.
pub fn measure(text: &str, size: f32) -> Result<(f32, f32), String> {
    let Some(font) = face() else {
        return Err("this host has no font, so it cannot measure text".into());
    };
    let width = if text.is_empty() {
        0.0
    } else {
        let mut max_w = 0.0_f32;
        for line in text.split('\n') {
            let w = font
                .width(size, line)
                .ok_or("the measuring face did not parse")?;
            max_w = max_w.max(w);
        }
        max_w
    };
    let lines = text.split('\n').count().max(1) as f32;
    let height = lines * size * 1.5;
    Ok((width, height))
}

/// Measure a string with word wrapping against an available width constraint.
///
/// When `TextWrapped = true`, breaks at word boundaries against `max_width`,
/// and at glyph/character boundaries when a single word exceeds `max_width`.
/// Multi-line inputs separated by '\n' are preserved as distinct paragraphs.
/// Total height is resolved line count * size * 1.5.
pub fn measure_wrapped(text: &str, size: f32, max_width: f32) -> Result<(f32, f32), String> {
    let Some(font) = face() else {
        return Err("this host has no font, so it cannot measure text".into());
    };
    if text.is_empty() {
        return Ok((0.0, size * 1.5));
    }
    if max_width <= 0.0 {
        return measure(text, size);
    }

    let mut total_lines = 0usize;
    let mut max_observed_w = 0.0_f32;

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            total_lines += 1;
            continue;
        }

        let mut current_line = String::new();
        let words: Vec<&str> = paragraph.split(' ').collect();

        for word in words {
            if word.is_empty() {
                continue;
            }

            let word_w = font
                .width(size, word)
                .ok_or("the measuring face did not parse")?;

            if word_w <= max_width {
                if current_line.is_empty() {
                    current_line.push_str(word);
                } else {
                    let mut candidate = current_line.clone();
                    candidate.push(' ');
                    candidate.push_str(word);
                    let cand_w = font
                        .width(size, &candidate)
                        .ok_or("the measuring face did not parse")?;
                    if cand_w <= max_width {
                        current_line = candidate;
                    } else {
                        let line_w = font
                            .width(size, &current_line)
                            .ok_or("the measuring face did not parse")?;
                        max_observed_w = max_observed_w.max(line_w);
                        total_lines += 1;
                        current_line.clear();
                        current_line.push_str(word);
                    }
                }
            } else {
                if !current_line.is_empty() {
                    let line_w = font
                        .width(size, &current_line)
                        .ok_or("the measuring face did not parse")?;
                    max_observed_w = max_observed_w.max(line_w);
                    total_lines += 1;
                    current_line.clear();
                }

                for ch in word.chars() {
                    let mut candidate = current_line.clone();
                    candidate.push(ch);
                    let cand_w = font
                        .width(size, &candidate)
                        .ok_or("the measuring face did not parse")?;
                    if cand_w <= max_width || current_line.is_empty() {
                        current_line = candidate;
                    } else {
                        let line_w = font
                            .width(size, &current_line)
                            .ok_or("the measuring face did not parse")?;
                        max_observed_w = max_observed_w.max(line_w);
                        total_lines += 1;
                        current_line.clear();
                        current_line.push(ch);
                    }
                }
            }
        }

        if !current_line.is_empty() {
            let line_w = font
                .width(size, &current_line)
                .ok_or("the measuring face did not parse")?;
            max_observed_w = max_observed_w.max(line_w);
            total_lines += 1;
        }
    }

    let line_count = total_lines.max(1) as f32;
    let height = line_count * size * 1.5;
    Ok((max_observed_w, height))
}

// -- The clock ---------------------------------------------------------------

/// Frame subscriptions for one guest VM.
///
/// A LIST WITH IDS RATHER THAN A MAP, for the reason `Node::connections` keeps
/// one: firing order is subscribe order, and this is the only place that order is
/// recorded. A guest that subscribes a sampler and then a mutator is relying on
/// it.
#[derive(Default)]
pub struct Clock {
    listeners: Vec<(usize, LuaFunction)>,
    next: usize,
    /// Set on the first tick, not at construction. Time starts when frames do, so
    /// a mod that spent 40ms mounting does not read 0.04 from its first `Now`.
    started: Option<Instant>,
    elapsed: f64,
}

pub type SharedClock = Arc<Mutex<Clock>>;

/// Where the pointer is, so a guest can ask instead of being told.
///
/// WHY A HOST-OWNED CELL (ADR-010). A guest that decides hover by geometry rather
/// than by events needs to ask where the pointer is on any frame, including frames
/// with no input at all: a list can scroll or a window can open under a stationary
/// cursor, and the element beneath it changes with nothing to announce it.
///
/// The host used to push pointer events into a session it obtained by performing
/// the guest's mount ceremony. That is the inversion milestone 9 removes. The host
/// answers the question now, and holds nothing.
#[derive(Default)]
pub struct PointerState {
    /// None until the pointer has been seen over a surface at all. A guest that
    /// reads this before then must not be handed `(0, 0)`, which is a corner.
    at: Option<(f32, f32)>,
    /// Left, right, middle.
    held: [bool; 3],
    /// Who to tell when input happens, by signal.
    ///
    /// A POLL ANSWERS "WHERE IS IT" AND NOTHING ELSE. A framework that wants to
    /// know a press HAPPENED has to be told, because the moment between a press
    /// and its release is not something a per-frame read can recover: press and
    /// release inside one frame poll as never pressed at all.
    began: Vec<Listener>,
    ended: Vec<Listener>,
    changed: Vec<Listener>,
    next_listener: usize,
}

/// A subscribed guest function, and a weak handle on the state it came from.
///
/// THE STATE IS WHY THE HANDLE IS HERE. This list is process level while a Lua
/// state is not: a guest that subscribes and is then dropped leaves its function
/// behind, and calling one whose state has gone aborts the process from inside
/// mlua rather than erroring. Holding the state weakly says which entries are
/// still callable, and upgrading it before the call keeps it callable until the
/// call returns.
struct Listener {
    id: usize,
    f: LuaFunction,
    state: WeakLua,
}

/// The callable listeners, with the ones whose state has gone dropped.
fn live(list: &mut Vec<Listener>) -> Vec<(LuaFunction, Lua)> {
    let mut out = Vec::new();
    list.retain(|listener| match listener.state.try_upgrade() {
        Some(state) => {
            out.push((listener.f.clone(), state));
            true
        }
        None => false,
    });
    out
}

pub type SharedPointer = Arc<Mutex<PointerState>>;

/// The one the host feeds and guests read.
///
/// PROCESS LEVEL, LIKE THE FONT ABOVE. The alternative is threading a cell from
/// the window loop through `Renderer`, the snapshot path and every test driver,
/// and `crates/runtime` cannot hold it because the runtime does not depend on the
/// host. One cursor exists, so one cell is the truthful shape.
pub fn pointer() -> &'static SharedPointer {
    static POINTER: OnceLock<SharedPointer> = OnceLock::new();
    POINTER.get_or_init(|| Arc::new(Mutex::new(PointerState::default())))
}

/// Forget the pointer entirely: where it was, which buttons are held, and every
/// listener connected to it.
///
/// FOR A PROCESS THAT RUNS MORE THAN ONE GUEST. The cell is process level because
/// one cursor exists, which is right while one guest is running and wrong the
/// moment a second one loads: its listeners join the first one's, and a Lua
/// function whose state has since been dropped panics the host when the next
/// press is delivered. `framework-coverage` loads two VMs per example and found
/// this the first time an example connected to the service at all.
pub fn forget_pointer() {
    *pointer().lock().expect("pointer") = PointerState::default();
}

/// Record where the pointer went. Called wherever input enters the host.
pub fn pointer_moved(x: f32, y: f32) {
    let listeners = {
        let mut guard = pointer().lock().expect("pointer");
        guard.moved(x, y);
        live(&mut guard.changed)
    };
    //  FIRED OUTSIDE THE LOCK. A listener reads `Position` while it runs, and
    //  holding the mutex across a guest call deadlocks on the first one that
    //  asks where the pointer is.
    deliver(listeners, "MouseMovement", x, y, 0.0);
}

/// Record a button going down or up.
pub fn pointer_button(button: usize, down: bool) {
    let (listeners, at) = {
        let mut guard = pointer().lock().expect("pointer");
        guard.set_button(button, down);
        let at = guard.position().unwrap_or((0.0, 0.0));
        let list = if down {
            live(&mut guard.began)
        } else {
            live(&mut guard.ended)
        };
        (list, at)
    };
    //  ONLY THE LEFT BUTTON IS NAMED, because it is the only one the enum entry
    //  below exists for here. A right or middle press records its state, which a
    //  poll can still read, and announces nothing.
    let Some(kind) = button_name(button) else {
        return;
    };
    deliver(listeners, kind, at.0, at.1, 0.0);
}

/// Record a wheel turn. Nothing is held; the delta is the whole event.
pub fn pointer_wheel(x: f32, y: f32, delta: f32) {
    let listeners = {
        let mut guard = pointer().lock().expect("pointer");
        live(&mut guard.changed)
    };
    deliver(listeners, "MouseWheel", x, y, delta);
}

fn button_name(button: usize) -> Option<&'static str> {
    match button {
        0 => Some("MouseButton1"),
        1 => Some("MouseButton2"),
        2 => Some("MouseButton3"),
        _ => None,
    }
}

/// Hand each listener an input object and let it fail on its own.
///
/// ONE LISTENER ERRORING DOES NOT SILENCE THE REST, which is what a signal does
/// on the engine. It is reported rather than swallowed: a handler that throws
/// every frame should be findable.
fn deliver(listeners: Vec<(LuaFunction, Lua)>, kind: &'static str, x: f32, y: f32, z: f32) {
    //  THE STATE IS HELD FOR THE LENGTH OF THE CALL, which is the whole reason it
    //  travels alongside the function rather than being checked and dropped.
    for (listener, _state) in listeners {
        if let Err(e) = listener.call::<()>((InputObject { kind, x, y, z },)) {
            eprintln!("[dew] an input listener errored: {e}");
        }
    }
}

/// What a guest receives for one input event.
///
/// THE TWO FIELDS THE FRAMEWORK READS, and no more. `UserInputType` decides which
/// branch a router takes and `Position.Z` carries a wheel delta, which is the
/// engine's own arrangement rather than this host's invention.
struct InputObject {
    kind: &'static str,
    x: f32,
    y: f32,
    z: f32,
}

impl LuaUserData for InputObject {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("UserInputType", |_, this| {
            //  NAMED FROM THE SAME DATABASE the guest's `Enum` reads, so an item
            //  this host hands over and one the guest names compare equal. A
            //  made-up item would answer false against the real one.
            Ok(crate::datamodel::enums::item_by_name(
                "UserInputType",
                this.kind,
            ))
        });
        fields.add_field_method_get("Position", |lua, this| {
            let position = lua.create_table()?;
            position.set("X", this.x)?;
            position.set("Y", this.y)?;
            position.set("Z", this.z)?;
            Ok(position)
        });
    }
}

impl PointerState {
    pub fn moved(&mut self, x: f32, y: f32) {
        self.at = Some((x, y));
    }

    pub fn set_button(&mut self, button: usize, down: bool) {
        if let Some(slot) = self.held.get_mut(button) {
            *slot = down;
        }
    }

    pub fn position(&self) -> Option<(f32, f32)> {
        self.at
    }

    pub fn is_down(&self, button: usize) -> bool {
        self.held.get(button).copied().unwrap_or(false)
    }
}

/// Put the pointer on `dew` so a guest can poll it.
///
/// SEPARATE FROM `install` BECAUSE THE STATE HAS A DIFFERENT OWNER. The clock is
/// the host's; the pointer belongs to whatever is feeding input, which is the
/// window loop on Windows and a test driver everywhere else. Both call this with
/// the cell they are updating.
fn install_pointer(lua: &Lua, pointer: &SharedPointer) -> LuaResult<()> {
    let dew: LuaTable = match lua.globals().get("dew") {
        Ok(existing) => existing,
        Err(_) => {
            let fresh = lua.create_table()?;
            lua.globals().set("dew", fresh.clone())?;
            fresh
        }
    };

    let api = lua.create_table()?;

    let reader = Arc::clone(pointer);
    api.set(
        "Position",
        // TWO RETURNS OR NONE. A guest that has never seen the pointer gets
        // nothing back rather than `(0, 0)`, which is a real corner of the screen
        // and would read as the pointer being there.
        lua.create_function(
            move |_, ()| match reader.lock().expect("pointer").position() {
                Some((x, y)) => Ok((Some(x), Some(y))),
                None => Ok((None, None)),
            },
        )?,
    )?;

    let reader = Arc::clone(pointer);
    api.set(
        "IsDown",
        lua.create_function(move |_, button: Option<usize>| {
            Ok(reader.lock().expect("pointer").is_down(button.unwrap_or(0)))
        })?,
    )?;

    dew.set("Pointer", api)?;

    //  AND THE SERVICE THAT DELIVERS, beside the one that answers.
    //
    //  A poll says where the pointer IS. A framework also has to learn that a
    //  press HAPPENED, and cannot recover that from polling: a press and its
    //  release inside one frame read as never pressed. On the engine both come
    //  from `UserInputService`, so both come from here.
    let input = lua.create_table()?;
    for (name, which) in [
        ("InputBegan", Signal::Began),
        ("InputEnded", Signal::Ended),
        ("InputChanged", Signal::Changed),
    ] {
        input.set(name, signal(lua, pointer, which)?)?;
    }
    dew.set("Input", input)?;

    Ok(())
}

/// Which list a `Connect` adds to.
#[derive(Clone, Copy)]
enum Signal {
    Began,
    Ended,
    Changed,
}

/// One signal, with the `Connect` returning a disconnectable the engine returns.
///
/// THE SHAPE IS THE ENGINE'S because the consumer is written against the engine:
/// `signal:Connect(fn)` answering something with `Disconnect`. A different shape
/// here would mean every guest branching on which host it is running under, which
/// is the thing this whole direction removes.
fn signal(lua: &Lua, pointer: &SharedPointer, which: Signal) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    let owner = Arc::clone(pointer);

    table.set(
        "Connect",
        lua.create_function(move |lua, (_this, f): (LuaValue, LuaFunction)| {
            let id = {
                let mut guard = owner.lock().expect("pointer");
                let id = guard.next_listener;
                guard.next_listener += 1;
                let listener = Listener {
                    id,
                    f,
                    state: lua.weak(),
                };
                match which {
                    Signal::Began => guard.began.push(listener),
                    Signal::Ended => guard.ended.push(listener),
                    Signal::Changed => guard.changed.push(listener),
                }
                id
            };

            let connection = lua.create_table()?;
            connection.set("Connected", true)?;
            let dropper = Arc::clone(&owner);
            connection.set(
                "Disconnect",
                lua.create_function(move |_, this: LuaTable| {
                    let mut guard = dropper.lock().expect("pointer");
                    match which {
                        Signal::Began => guard.began.retain(|l| l.id != id),
                        Signal::Ended => guard.ended.retain(|l| l.id != id),
                        Signal::Changed => guard.changed.retain(|l| l.id != id),
                    }
                    this.set("Connected", false)?;
                    Ok(())
                })?,
            )?;
            Ok(connection)
        })?,
    )?;

    Ok(table)
}

impl Clock {
    /// Is anyone listening?
    ///
    /// THE FRAME LOOP ASKS THIS FIRST, and it is what keeps a clock off the idle
    /// path: a mod that never subscribed costs one uncontended lock per frame and
    /// nothing else -- no Lua call, no allocation, and above all no `touch`.
    pub fn idle(&self) -> bool {
        self.listeners.is_empty()
    }

    pub fn now(&self) -> f64 {
        self.started.map_or(self.elapsed, |t| {
            t.elapsed().as_secs_f64().max(self.elapsed)
        })
    }
}

/// Call every listener once, with `dt` in seconds.
///
/// THREE PROPERTIES THIS FUNCTION EXISTS TO HOLD, and each is load bearing.
///
/// THE LOCK IS DROPPED BEFORE ANY LISTENER RUNS. A frame handler subscribing or
/// unsubscribing -- its own disposal, or a sibling's -- is the ordinary thing for
/// one to do, and this `Mutex` is no more reentrant than the arena's. Holding it
/// across the call deadlocks on the first well-behaved listener.
///
/// THE LIST IS SNAPSHOTTED. Dropping the lock is not enough by itself: a listener
/// that disposes a sibling mid-frame would otherwise shorten the list being
/// walked. Aether's the engine host clones its listener table inside the Heartbeat
/// handler for exactly this reason, and a host that made the property impossible
/// to keep would be forcing a regression on the implementation above it.
///
/// A FAILING LISTENER DOES NOT TAKE THE FRAME DOWN. It is reported and the rest
/// still run -- the answer `mods.rs` gives a mod that fails to load, for the same
/// reason: one guest's bug is not the host's crash.
///
/// NOTHING HERE TOUCHES THE ARENA. That is the whole of milestone 1's sprint 9
/// preserved: a tick is not a change, so a mod subscribing to frames and
/// animating nothing leaves the tree clean and the frame unpainted. A listener
/// that DOES assign dirties the tree on the same path every other write takes,
/// which is exactly right -- the paint follows the change rather than the tick.
pub fn tick(clock: &SharedClock, dt: f32) {
    let batch = {
        let mut guard = clock.lock().expect("clock");
        if guard.started.is_none() {
            guard.started = Some(Instant::now());
        }
        guard.elapsed += dt as f64;
        if guard.listeners.is_empty() {
            return;
        }
        guard
            .listeners
            .iter()
            .map(|(_, f)| f.clone())
            .collect::<Vec<_>>()
    };
    for listener in batch {
        if let Err(e) = listener.call::<()>(dt) {
            eprintln!("[dew] a frame listener errored: {e}");
        }
    }
}

// -- Installation ------------------------------------------------------------

/// Install `dew.Text` and `dew.Clock` into a guest VM.
///
/// FOR EVERY GUEST, whichever runtime it declared. A DataModel mod reaches these
/// directly; an Aether mod reaches them through the `Host.Text` and `Host.Clock`
/// seams in sprint 8. Neither is a reason to install conditionally, and doing it
/// for one runtime only would make the shape of the platform depend on which
/// framework a mod happened to choose.
///
/// SAFE TO INSTALL BESIDE AETHER, unlike the vocabulary. Aether publishes its own
/// `UDim2` and the rest with `if rawget(g, name) == nil` -- first writer wins --
/// so a partial host vocabulary BLOCKS it rather than merging with it, which is
/// why `install_vocabulary` is still not called for an Aether mod. Nothing in
/// Aether or vide is called `dew`, and a name unmistakably this host's is the
/// reason there is nothing to collide with.
///
/// REUSES THE EXISTING `dew` GLOBAL IF ONE IS ALREADY THERE, the same
/// get-or-create `install_pointer` below already did for the pointer: this may
/// run before or after `capabilities::build` set `dew` up with a mod's granted
/// permissions, and either order has to add its members to the one table
/// rather than each clobbering the other's.
pub fn install(lua: &Lua, clock: &SharedClock) -> LuaResult<()> {
    let dew: LuaTable = match lua.globals().get("dew") {
        Ok(existing) => existing,
        Err(_) => {
            let fresh = lua.create_table()?;
            lua.globals().set("dew", fresh.clone())?;
            fresh
        }
    };

    // ---- Text ----
    let text = lua.create_table()?;
    text.set(
        "Measure",
        // THE THIRD ARGUMENT IS ACCEPTED AND REPORTED, NOT IGNORED. Aether's
        // `Host.Text` contract passes a font name and this host has one face, so
        // the honest answer is to measure in the face it will draw with and SAY
        // that the name selected nothing. Sprint 4's rule: `Slice` and `Tile` are
        // stretched and reported by name, because silently doing the wrong thing
        // was that sprint's named failure mode. Taking the argument and dropping
        // it quietly would make a mod look styled when it is not.
        lua.create_function(|_, (text, size, font): (String, f32, Option<String>)| {
            if let Some(name) = font {
                note_once_font(&name);
            }
            measure(&text, size).map_err(LuaError::runtime)
        })?,
    )?;
    text.set(
        "MeasureWrapped",
        lua.create_function(
            |_, (text, size, max_width, font): (String, f32, f32, Option<String>)| {
                if let Some(name) = font {
                    note_once_font(&name);
                }
                measure_wrapped(&text, size, max_width).map_err(LuaError::runtime)
            },
        )?,
    )?;
    dew.set("Text", text)?;

    // ---- Clock ----
    let dew_clock = lua.create_table()?;

    // THE NAME IS THE HOST'S, and Aether's `Clock.Name` exists to be reported.
    // "Heartbeat" in-engine, "Simulated" under Lune, and this here -- a suite or a
    // log line that says which clock drove a frame is the cheapest way to catch a
    // mod animating against the wrong one.
    dew_clock.set("Name", "DewFrame")?;

    let subscribe = Arc::clone(clock);
    dew_clock.set(
        "OnFrame",
        lua.create_function(move |lua, f: LuaFunction| {
            let id = {
                let mut guard = subscribe.lock().expect("clock");
                let id = guard.next;
                guard.next += 1;
                guard.listeners.push((id, f));
                id
            };
            // AN UNSUBSCRIBE, AND IT IS NOT DECORATION. Aether's the engine host keeps
            // ONE Heartbeat connection for N listeners and takes it down by
            // dropping them; returning a disposer is what lets the implementation
            // above keep exactly that shape here -- one Dew subscription for N of
            // its own listeners -- rather than one subscription per hook.
            //
            // BY ID, NOT BY FUNCTION IDENTITY, so subscribing the same function
            // twice yields two subscriptions that dispose independently, as it
            // does on the engine. And IDEMPOTENT, because disposal is the path
            // most likely to run twice: once from a cleanup, once from a caller
            // being careful.
            let unsubscribe = Arc::clone(&subscribe);
            lua.create_function(move |_, ()| {
                unsubscribe
                    .lock()
                    .expect("clock")
                    .listeners
                    .retain(|(this, _)| *this != id);
                Ok(())
            })
        })?,
    )?;

    let reader = Arc::clone(clock);
    dew_clock.set(
        "Now",
        // MONOTONIC SECONDS SINCE THE FIRST FRAME, AND NOT `dew.Time.now`. That
        // one is wall-clock seconds since the epoch, which is what a mod showing
        // the time of day wants. This is what Aether's the engine host answers with
        // `os.clock()`: a process clock that never steps backwards and whose zero
        // is arbitrary. Animation subtracts two readings, and a wall clock that a
        // daylight-saving change moved would make a spring jump.
        lua.create_function(move |_, ()| Ok(reader.lock().expect("clock").now()))?,
    )?;

    // NO `Step`. Aether's `Clock` declares four members and this host offers
    // three, deliberately. `Step(dt)` advances frames BY HAND: it is what a
    // simulated clock under Lune needs, and what a DRIVEN one must never offer.
    // On the engine it is present and empty for that reason -- the Heartbeat already
    // drives frames and stepping would run each one twice. Dew's frame loop drives
    // this one, so a `Step` here would be a way for a guest to double every frame
    // it touched. Sprint 8 fills the member with the engine host's own no-op,
    // which is the truthful implementation on any host that drives its own frames.

    dew.set("Clock", dew_clock)?;

    // THE POINTER COMES WITH THE HOST, not from a second call every caller has to
    // remember. The cell is process level, so there is nothing to thread and
    // nothing a caller can get wrong by forgetting.
    install_pointer(lua, pointer())?;
    Ok(())
}

/// Said once per font name, not once per measurement.
///
/// Layout measures the same strings every frame, so a per-call warning would
/// print thousands of lines a second and bury itself. `Assets::note_once` keeps
/// its set on the `Dom` because assets are per mod; a face is per process, so
/// this set is too.
fn note_once_font(name: &str) {
    static SAID: Mutex<Option<BTreeSet<String>>> = Mutex::new(None);
    let mut guard = SAID.lock().expect("said");
    if guard
        .get_or_insert_with(BTreeSet::new)
        .insert(name.to_string())
    {
        eprintln!(
            "[dew] text measured in this host's one face; the font named \
             `{name}` selected nothing"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE HOST ANSWERS, AND SAYS NOTHING WHEN IT HAS NOTHING TO SAY.
    ///
    /// `(0, 0)` is a real corner of the screen, so a guest that has never seen the
    /// pointer must not be handed it. Two returns or none.
    #[test]
    fn the_pointer_is_unknown_until_it_moves() {
        let lua = Lua::new();
        let clock: SharedClock = Arc::new(Mutex::new(Clock::default()));
        install(&lua, &clock).expect("install");

        let before: Option<f32> = lua
            .load("local x, y = dew.Pointer.Position(); return x")
            .eval()
            .expect("read");
        assert!(
            before.is_none(),
            "unknown should read as nothing, got {before:?}"
        );

        pointer_moved(12.0, 34.0);
        let (x, y): (f32, f32) = lua
            .load("local x, y = dew.Pointer.Position(); return x, y")
            .eval()
            .expect("read");
        assert_eq!((x, y), (12.0, 34.0));

        assert!(!lua
            .load("return dew.Pointer.IsDown(0)")
            .eval::<bool>()
            .unwrap());
        pointer_button(0, true);
        assert!(lua
            .load("return dew.Pointer.IsDown(0)")
            .eval::<bool>()
            .unwrap());
        pointer_button(0, false);
    }

    fn vm() -> (Lua, SharedClock) {
        let lua = Lua::new();
        let clock: SharedClock = Arc::new(Mutex::new(Clock::default()));
        install(&lua, &clock).expect("install");
        (lua, clock)
    }

    /// Whether this machine has a face at all. Every measurement assertion below
    /// is conditional on it: CI without fonts must report "no font" rather than
    /// failing a test about text, which is the distinction `system_font`'s own
    /// comment insists on.
    fn has_face() -> bool {
        face().is_some()
    }

    #[test]
    fn a_string_measures_to_a_width_and_a_height() {
        if !has_face() {
            eprintln!("no system font; skipping");
            return;
        }
        let (lua, _clock) = vm();
        let (w, h): (f32, f32) = lua
            .load(r#"return dew.Text.Measure("hello", 14)"#)
            .eval()
            .expect("measure");
        assert!(w > 0.0, "width was {w}");
        assert!(h > 0.0, "height was {h}");
    }

    #[test]
    fn a_longer_string_measures_wider_at_the_same_size() {
        if !has_face() {
            return;
        }
        let (lua, _clock) = vm();
        let got: bool = lua
            .load(
                r#"
                local short = dew.Text.Measure("i", 14)
                local long = dew.Text.Measure("iiiiiiiiii", 14)
                return long > short
            "#,
            )
            .eval()
            .expect("measure");
        assert!(got);
    }

    #[test]
    fn an_empty_string_measures_zero_wide_and_still_has_a_line_height() {
        // A LINE IS STILL A LINE. An empty label occupies no width and full
        // height, and a host that answered zero for both would collapse an empty
        // field rather than leaving a gap where text will go.
        if !has_face() {
            return;
        }
        let (lua, _clock) = vm();
        let (w, h): (f32, f32) = lua
            .load(r#"return dew.Text.Measure("", 14)"#)
            .eval()
            .expect("measure");
        assert_eq!(w, 0.0);
        assert!(h > 0.0, "height was {h}");
    }

    #[test]
    fn measuring_is_a_pure_function_of_its_arguments() {
        // Called twice with the same arguments it answers the same thing, which is
        // what lets a layout pass measure inside a frame without caching.
        if !has_face() {
            return;
        }
        let (lua, _clock) = vm();
        let got: bool = lua
            .load(
                r#"
                local a, b = dew.Text.Measure("Dew", 18)
                local c, d = dew.Text.Measure("Dew", 18)
                return a == c and b == d
            "#,
            )
            .eval()
            .expect("measure");
        assert!(got);
    }

    #[test]
    fn wrapped_text_grows_height_when_exceeding_width() {
        if !has_face() {
            return;
        }
        let long = "The quick brown fox jumps over the lazy dog and keeps going well past the edge";
        let (_, single_h) = measure("One line", 18.0).expect("measure");
        let (wrap_w, wrap_h) = measure_wrapped(long, 18.0, 200.0).expect("measure_wrapped");
        assert!(wrap_w <= 200.0, "observed width {wrap_w} exceeded 200");
        let ratio = wrap_h / single_h;
        assert!(ratio >= 1.9, "ratio {ratio} was less than 1.9");
        let nearest = (ratio + 0.5).floor();
        assert!(
            (ratio - nearest).abs() < 0.02,
            "ratio {ratio} was not integral"
        );
    }

    #[test]
    fn wrapped_text_respects_single_line_when_fitting() {
        if !has_face() {
            return;
        }
        let short = "One line";
        let (_, unwrapped_h) = measure(short, 18.0).expect("measure");
        let (_, wrapped_h) = measure_wrapped(short, 18.0, 200.0).expect("measure_wrapped");
        assert_eq!(wrapped_h, unwrapped_h);
    }

    #[test]
    fn wrapped_oversized_words_break_at_glyph_boundaries() {
        if !has_face() {
            return;
        }
        let long_word = "Supercalifragilisticexpialidocious";
        let (wrap_w, wrap_h) = measure_wrapped(long_word, 18.0, 50.0).expect("measure_wrapped");
        assert!(wrap_w <= 50.0 || wrap_w < 60.0);
        let (_, single_h) = measure("a", 18.0).expect("measure");
        assert!(wrap_h > single_h);
    }

    #[test]
    fn a_listener_is_called_with_the_delta() {
        let (lua, clock) = vm();
        lua.load(
            r#"
            seen = {}
            dew.Clock.OnFrame(function(dt) table.insert(seen, dt) end)
        "#,
        )
        .exec()
        .expect("subscribe");

        tick(&clock, 0.5);
        tick(&clock, 0.25);

        let got: Vec<f32> = lua.load("return seen").eval().expect("seen");
        assert_eq!(got, vec![0.5, 0.25]);
    }

    #[test]
    fn unsubscribing_stops_the_calls() {
        let (lua, clock) = vm();
        lua.load(
            r#"
            calls = 0
            stop = dew.Clock.OnFrame(function() calls += 1 end)
        "#,
        )
        .exec()
        .expect("subscribe");

        tick(&clock, 0.016);
        lua.load("stop()").exec().expect("stop");
        tick(&clock, 0.016);
        // TWICE, because a disposer is the code path most likely to run twice.
        lua.load("stop()").exec().expect("stop again");
        tick(&clock, 0.016);

        let got: u32 = lua.load("return calls").eval().expect("calls");
        assert_eq!(got, 1);
    }

    #[test]
    fn a_listener_may_unsubscribe_itself_mid_frame() {
        // THE SNAPSHOT, AND THE NON-REENTRANT LOCK, IN ONE TEST. Without the
        // clone this walks a list it is mutating; without the `drop` it deadlocks
        // rather than failing, so a hang here is the assertion.
        let (lua, clock) = vm();
        lua.load(
            r#"
            calls = 0
            local stop
            stop = dew.Clock.OnFrame(function()
                calls += 1
                stop()
            end)
        "#,
        )
        .exec()
        .expect("subscribe");

        tick(&clock, 0.016);
        tick(&clock, 0.016);

        let got: u32 = lua.load("return calls").eval().expect("calls");
        assert_eq!(got, 1);
    }

    #[test]
    fn a_listener_may_dispose_a_sibling_mid_frame() {
        // The case a snapshot exists for and a `drop` alone does not cover: the
        // first listener removes the second, and the second must not be skipped
        // THIS frame -- it was subscribed when the frame began.
        let (lua, clock) = vm();
        lua.load(
            r#"
            second = 0
            local stopSecond
            dew.Clock.OnFrame(function() stopSecond() end)
            stopSecond = dew.Clock.OnFrame(function() second += 1 end)
        "#,
        )
        .exec()
        .expect("subscribe");

        tick(&clock, 0.016);
        let after_first: u32 = lua.load("return second").eval().expect("second");
        assert_eq!(after_first, 1);

        tick(&clock, 0.016);
        let after_second: u32 = lua.load("return second").eval().expect("second");
        assert_eq!(after_second, 1);
    }

    #[test]
    fn a_listener_may_subscribe_another_mid_frame() {
        // The other half of the reentrancy case: a handler that ADDS. The new
        // listener joins from the next frame, since this one was already
        // snapshotted.
        let (lua, clock) = vm();
        lua.load(
            r#"
            late = 0
            local added = false
            dew.Clock.OnFrame(function()
                if not added then
                    added = true
                    dew.Clock.OnFrame(function() late += 1 end)
                end
            end)
        "#,
        )
        .exec()
        .expect("subscribe");

        tick(&clock, 0.016);
        let after_first: u32 = lua.load("return late").eval().expect("late");
        assert_eq!(after_first, 0);

        tick(&clock, 0.016);
        let after_second: u32 = lua.load("return late").eval().expect("late");
        assert_eq!(after_second, 1);
    }

    #[test]
    fn the_same_function_subscribed_twice_is_two_subscriptions() {
        let (lua, clock) = vm();
        lua.load(
            r#"
            calls = 0
            local function f() calls += 1 end
            first = dew.Clock.OnFrame(f)
            dew.Clock.OnFrame(f)
        "#,
        )
        .exec()
        .expect("subscribe");

        tick(&clock, 0.016);
        lua.load("first()").exec().expect("stop one");
        tick(&clock, 0.016);

        let got: u32 = lua.load("return calls").eval().expect("calls");
        assert_eq!(got, 3);
    }

    #[test]
    fn listeners_fire_in_subscribe_order() {
        let (lua, clock) = vm();
        lua.load(
            r#"
            order = {}
            dew.Clock.OnFrame(function() table.insert(order, "a") end)
            dew.Clock.OnFrame(function() table.insert(order, "b") end)
        "#,
        )
        .exec()
        .expect("subscribe");

        tick(&clock, 0.016);

        let got: Vec<String> = lua.load("return order").eval().expect("order");
        assert_eq!(got, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn a_failing_listener_does_not_stop_the_others() {
        let (lua, clock) = vm();
        lua.load(
            r#"
            reached = false
            dew.Clock.OnFrame(function() error("deliberate") end)
            dew.Clock.OnFrame(function() reached = true end)
        "#,
        )
        .exec()
        .expect("subscribe");

        tick(&clock, 0.016);

        let got: bool = lua.load("return reached").eval().expect("reached");
        assert!(got);
    }

    #[test]
    fn a_clock_nobody_subscribed_to_is_idle() {
        // WHAT THE FRAME LOOP KEYS ON. If this ever answered false for an
        // untouched VM, every mod would be paying for a clock it never asked for.
        let (lua, clock) = vm();
        assert!(clock.lock().expect("clock").idle());

        lua.load("stop = dew.Clock.OnFrame(function() end)")
            .exec()
            .expect("subscribe");
        assert!(!clock.lock().expect("clock").idle());

        lua.load("stop()").exec().expect("stop");
        assert!(clock.lock().expect("clock").idle());
    }

    #[test]
    fn now_advances_and_never_goes_backwards() {
        let (lua, clock) = vm();
        // Zero before the first frame: the clock starts when frames do.
        let before: f64 = lua.load("return dew.Clock.Now()").eval().expect("now");
        assert_eq!(before, 0.0);

        tick(&clock, 0.016);
        let a: f64 = lua.load("return dew.Clock.Now()").eval().expect("now");
        let b: f64 = lua.load("return dew.Clock.Now()").eval().expect("now");
        assert!(b >= a, "{b} < {a}");
    }

    #[test]
    fn there_is_no_step() {
        // ASSERTED, NOT ASSUMED. `Step` is the one member of Aether's `Clock` this
        // host must not offer -- a guest calling it on a driven clock would run
        // every frame twice -- and the way that mistake arrives is somebody adding
        // it for symmetry with the Lune host.
        let (lua, _clock) = vm();
        let got: bool = lua
            .load("return dew.Clock.Step == nil")
            .eval()
            .expect("step");
        assert!(got);
    }

    #[test]
    fn the_clock_names_itself() {
        let (lua, _clock) = vm();
        let got: String = lua.load("return dew.Clock.Name").eval().expect("name");
        assert_eq!(got, "DewFrame");
    }
}

#[cfg(test)]
mod input_is_delivered {
    use super::*;

    /// A guest connects and is told, rather than having to poll.
    ///
    /// THE GAP THIS CLOSES: a poll answers where the pointer is. A press and its
    /// release inside one frame read as never pressed, so a framework that only
    /// polls cannot see a click at all.
    #[test]
    fn a_press_reaches_a_connected_listener() {
        let lua = Lua::new();
        let state: SharedPointer = Arc::new(Mutex::new(PointerState::default()));
        install_pointer(&lua, &state).expect("install");

        lua.load(
            r#"
            seen = {}
            dew.Input.InputBegan:Connect(function(input)
                table.insert(seen, {
                    kind = tostring(input.UserInputType),
                    x = input.Position.X,
                    y = input.Position.Y,
                })
            end)
            "#,
        )
        .exec()
        .expect("connect");

        {
            let mut guard = state.lock().expect("pointer");
            guard.moved(40.0, 12.0);
        }
        let listeners = live(&mut state.lock().expect("pointer").began);
        deliver(listeners, "MouseButton1", 40.0, 12.0, 0.0);

        let count: usize = lua.load("return #seen").eval().expect("count");
        assert_eq!(count, 1, "the listener should have been told once");

        let kind: String = lua.load("return seen[1].kind").eval().expect("kind");
        assert!(
            kind.contains("MouseButton1"),
            "the input should name the button, got {kind}"
        );
        let x: f32 = lua.load("return seen[1].x").eval().expect("x");
        assert_eq!(x, 40.0);
    }

    /// The enum item a guest compares against is the same one.
    ///
    /// A MADE-UP ITEM WOULD ANSWER FALSE against the real one, and the router
    /// branches on exactly this comparison, so an item that merely prints right
    /// would leave every press unhandled.
    #[test]
    fn the_input_type_compares_equal_to_the_guests_own() {
        let lua = Lua::new();
        crate::datamodel::install_vocabulary(&lua).expect("vocabulary");
        let state: SharedPointer = Arc::new(Mutex::new(PointerState::default()));
        install_pointer(&lua, &state).expect("install");

        lua.load(
            r#"
            matched = false
            dew.Input.InputBegan:Connect(function(input)
                matched = input.UserInputType == Enum.UserInputType.MouseButton1
            end)
            "#,
        )
        .exec()
        .expect("connect");

        let listeners = live(&mut state.lock().expect("pointer").began);
        deliver(listeners, "MouseButton1", 1.0, 2.0, 0.0);

        let matched: bool = lua.load("return matched").eval().expect("matched");
        assert!(
            matched,
            "the item this host hands over must equal Enum.UserInputType.MouseButton1"
        );
    }

    /// Disconnecting stops delivery, and says so.
    #[test]
    fn a_disconnected_listener_is_not_told() {
        let lua = Lua::new();
        let state: SharedPointer = Arc::new(Mutex::new(PointerState::default()));
        install_pointer(&lua, &state).expect("install");

        lua.load(
            r#"
            calls = 0
            connection = dew.Input.InputBegan:Connect(function() calls += 1 end)
            connection:Disconnect()
            "#,
        )
        .exec()
        .expect("connect");

        assert!(
            state.lock().expect("pointer").began.is_empty(),
            "Disconnect should remove the listener"
        );
        let connected: bool = lua
            .load("return connection.Connected")
            .eval()
            .expect("flag");
        assert!(!connected, "and should say it is no longer connected");
    }
}
