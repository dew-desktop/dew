//! `DewHost`: the two services a guest framework needs and no guest can compute.
//!
//! WHERE THIS FILE LIVES, AND WHY IT MOVED BEFORE IT WAS EVER COMMITTED HERE
//! It was written as `datamodel/services.rs` and `verify_boundaries` refused it,
//! correctly: section 3 lets only the render bridge reach `aether_raster`, on the
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
//! a tree. These two are not. "How wide is this string in the face this host will
//! actually draw with" and "call me when the host draws a frame" are questions
//! ABOUT THE HOST, and no amount of tree makes them answerable from Luau. On
//! Roblox they are `TextService:GetTextSize` and `RunService.Heartbeat`, reached
//! through `game`, and Dew has to spell them itself -- `game` would not help even
//! if it were here, because these are services behind it rather than the tree it
//! names. (`game` itself came back into scope on 2026-09-04, for vide's gate and
//! nothing else; see the roadmap's "Why `game` was dropped from step F, and why
//! it came back". This comment used to cite the half of that section that was
//! later corrected, for a claim the section never had to carry.)
//!
//! WHY A GLOBAL, AND NOT THE `dew` TABLE -- the design question this sprint owned
//! Three homes were possible and the argument is in the sprint record; the fact
//! that settled it is mechanical. Aether's host modules NEVER SEE `dew`.
//! `Host.detect(vide)` takes the reactive core and nothing else, and
//! `Desktop.Mount` forwards the capability table through its varargs straight to
//! the mod's own `build` -- deliberately, so the framework never learns what a
//! capability is. Put text metrics on `dew` and the DataModel host cannot reach
//! them: sprint 8 would have to thread a capability table into a framework entry
//! point, for a reason that is entirely Dew's.
//!
//! The principled half agrees with the mechanical one. `capabilities.rs` is a
//! security boundary, and a capability is a thing a mod may be DENIED. There is
//! no coherent "no" here: a layout pass refused text metrics cannot lay out, and
//! a mod refused frames cannot animate -- neither degrades, both stop. A
//! permission with only one sound answer is not a permission, it is ceremony. So
//! these go where `Instance` goes, for the reason `Instance` goes there: the
//! language of the platform, present for every guest, ungated.
//!
//! WHY `DewHost` AND NOT `TextService` -- and this is the parity trap, declined.
//! Installing globals called `TextService` and `RunService` would LOOK like
//! Roblox and be a lie in the direction that costs most: on the engine those are
//! not globals, so code written against them would run HERE and nowhere else.
//! That is parity theatre, which is what the rescope removed from this step. A
//! name that is honestly Dew's cannot be mistaken for portable, and it sits
//! beside `DewRoot`, which was named the same way for the same reason.
//!
//! THE STANDARD QUESTION IS ANSWERED, AND THE NAME IS STILL DEW'S. Whether a
//! conforming host MUST expose text metrics and a frame clock was flagged here
//! and answered YES on 2026-09-04, in `docs/host_services.md` -- its own
//! hand-written half of the standard, not a section of the generated
//! `datamodel_scope.md`, because neither service is a member of any class and
//! regenerating that document after these landed produced a byte-identical file.
//!
//! What the standard requires is the SHAPE: synchronous measurement, a frame
//! subscription that returns a disposer, a monotonic `now`, and no way for a
//! guest to step a host that drives its own frames. It requires no name, so
//! `DewHost` conforms as it stands and is still expected to be revisited only if
//! the standard ever does specify one.

use aether_raster::Font;
use mlua::prelude::*;
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
        let path = aether_runtime::font::system_font()?;
        Font::load(&path.to_string_lossy(), 0)
    })
}

/// Measure a string in the face this host draws with.
///
/// SYNCHRONOUS, AND THAT IS THE CONTRACT RATHER THAN AN IMPLEMENTATION DETAIL.
/// Layout runs inside a frame and cannot await -- which is why Aether's
/// `Host.Text` is shaped this way, and why the Roblox host reaches for
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
fn measure(text: &str, size: f32) -> Result<(f32, f32), String> {
    let Some(font) = face() else {
        return Err("this host has no font, so it cannot measure text".into());
    };
    let width = font
        .width(size, text)
        .ok_or("the measuring face did not parse")?;
    let height = font
        .line_height(size)
        .ok_or("the measuring face did not parse")?;
    Ok((width, height))
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
}

pub type SharedClock = Arc<Mutex<Clock>>;

impl Clock {
    /// Is anyone listening?
    ///
    /// THE FRAME LOOP ASKS THIS FIRST, and it is what keeps a clock off the idle
    /// path: a mod that never subscribed costs one uncontended lock per frame and
    /// nothing else -- no Lua call, no allocation, and above all no `touch`.
    pub fn idle(&self) -> bool {
        self.listeners.is_empty()
    }

    fn now(&self) -> f64 {
        self.started.map_or(0.0, |t| t.elapsed().as_secs_f64())
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
/// walked. Aether's Roblox host clones its listener table inside the Heartbeat
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

/// Install `DewHost` into a guest VM.
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
/// Aether or vide is called `DewHost`, and a name unmistakably this host's is the
/// reason there is nothing to collide with.
pub fn install(lua: &Lua, clock: &SharedClock) -> LuaResult<()> {
    let host = lua.create_table()?;

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
    host.set("Text", text)?;

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
            // AN UNSUBSCRIBE, AND IT IS NOT DECORATION. Aether's Roblox host keeps
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
        // MONOTONIC SECONDS SINCE THE FIRST FRAME, AND NOT `dew.time.now`. That
        // one is wall-clock seconds since the epoch, which is what a mod showing
        // the time of day wants. This is what Aether's Roblox host answers with
        // `os.clock()`: a process clock that never steps backwards and whose zero
        // is arbitrary. Animation subtracts two readings, and a wall clock that a
        // daylight-saving change moved would make a spring jump.
        lua.create_function(move |_, ()| Ok(reader.lock().expect("clock").now()))?,
    )?;

    // NO `Step`. Aether's `Clock` declares four members and this host offers
    // three, deliberately. `Step(dt)` advances frames BY HAND: it is what a
    // simulated clock under Lune needs, and what a DRIVEN one must never offer.
    // On Roblox it is present and empty for that reason -- the Heartbeat already
    // drives frames and stepping would run each one twice. Dew's frame loop drives
    // this one, so a `Step` here would be a way for a guest to double every frame
    // it touched. Sprint 8 fills the member with the engine host's own no-op,
    // which is the truthful implementation on any host that drives its own frames.

    host.set("Clock", dew_clock)?;

    lua.globals().set("DewHost", host)?;
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
            .load(r#"return DewHost.Text.Measure("hello", 14)"#)
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
                local short = DewHost.Text.Measure("i", 14)
                local long = DewHost.Text.Measure("iiiiiiiiii", 14)
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
            .load(r#"return DewHost.Text.Measure("", 14)"#)
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
                local a, b = DewHost.Text.Measure("Dew", 18)
                local c, d = DewHost.Text.Measure("Dew", 18)
                return a == c and b == d
            "#,
            )
            .eval()
            .expect("measure");
        assert!(got);
    }

    #[test]
    fn a_listener_is_called_with_the_delta() {
        let (lua, clock) = vm();
        lua.load(
            r#"
            seen = {}
            DewHost.Clock.OnFrame(function(dt) table.insert(seen, dt) end)
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
            stop = DewHost.Clock.OnFrame(function() calls += 1 end)
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
            stop = DewHost.Clock.OnFrame(function()
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
            DewHost.Clock.OnFrame(function() stopSecond() end)
            stopSecond = DewHost.Clock.OnFrame(function() second += 1 end)
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
            DewHost.Clock.OnFrame(function()
                if not added then
                    added = true
                    DewHost.Clock.OnFrame(function() late += 1 end)
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
            first = DewHost.Clock.OnFrame(f)
            DewHost.Clock.OnFrame(f)
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
            DewHost.Clock.OnFrame(function() table.insert(order, "a") end)
            DewHost.Clock.OnFrame(function() table.insert(order, "b") end)
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
            DewHost.Clock.OnFrame(function() error("deliberate") end)
            DewHost.Clock.OnFrame(function() reached = true end)
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

        lua.load("stop = DewHost.Clock.OnFrame(function() end)")
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
        let before: f64 = lua.load("return DewHost.Clock.Now()").eval().expect("now");
        assert_eq!(before, 0.0);

        tick(&clock, 0.016);
        let a: f64 = lua.load("return DewHost.Clock.Now()").eval().expect("now");
        let b: f64 = lua.load("return DewHost.Clock.Now()").eval().expect("now");
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
            .load("return DewHost.Clock.Step == nil")
            .eval()
            .expect("step");
        assert!(got);
    }

    #[test]
    fn the_clock_names_itself() {
        let (lua, _clock) = vm();
        let got: String = lua.load("return DewHost.Clock.Name").eval().expect("name");
        assert_eq!(got, "DewFrame");
    }
}
