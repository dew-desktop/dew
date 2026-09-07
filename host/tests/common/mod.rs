//! What a shell has to put in a VM before an Aether application will load.
//!
//! SHARED BECAUSE FIVE SUITES NEED IT AND ONE DOES NOT. `image.rs` builds
//! `Node`s in Rust and never loads the framework, so it stayed in
//! `crates/runtime`. Everything that loads Aether needs a DataModel, and
//! `dew_host` is what grants one.
//!
//! The order matches `dew_host::mods`, which is the path a real mod takes, and
//! the order matters: the DataModel host's `available()` probe REQUIRES the
//! value vocabulary, so the vocabulary is a precondition rather than a
//! convenience.
//!
//! # What this does NOT settle
//!
//! INSTALLING A DATAMODEL CHANGES WHICH AETHER HOST `detect()` SELECTS, and that
//! is a behaviour change rather than a relocation. `crates/runtime/tests/render.rs`
//! asserts on PIXELS -- it counts near-white pixels to prove glyphs reached the
//! surface -- and under a DataModel host that fixture draws no glyphs at all. It
//! fails the same way against the OLD pin, so the cause is the DataModel, not the
//! Aether version.
//!
//! So `render.rs` has not moved and the pesde pin has not been bumped. Four
//! suites that only needed to LOAD are here; the one that asserts what was
//! PAINTED stays where it was until somebody understands why the two hosts
//! disagree. Moving it would have traded a red test for a silent divergence,
//! which is the trade this project exists to refuse.

use dew_host::datamodel::{self, SharedDom};
use dew_host::services::{self, Clock, SharedClock};
use dew_runtime::vm::Vm;
use mlua::Result as LuaResult;
use std::sync::{Arc, Mutex};

/// Install everything Aether probes for, exactly as `mods.rs` does for a mod.
pub fn install_host(vm: &Vm) -> LuaResult<()> {
    let dom: SharedDom = Arc::new(Mutex::new(Default::default()));
    datamodel::install(vm.lua(), &dom)?;

    let clock: SharedClock = Arc::new(Mutex::new(Clock::default()));
    services::install(vm.lua(), &clock)?;

    datamodel::install_vocabulary(vm.lua())?;

    // VIDE'S GATE, WHICH IS WHAT `game` IS AND ALL IT IS.
    //
    // vide reaches `typeof`, `Instance`, `Enum` and `Color3` through
    // `game and typeof or require "../test/mock"`, so with `game` nil the
    // expression takes a fallback requiring a module the shipped package does
    // not carry, and seven of vide's modules fail to load however real those
    // four globals are.
    //
    // Installed for the AETHER runtime, which is what these suites load, and for
    // nothing else -- the same scoping `mods.rs::install_gate` documents. If
    // anything other than vide's gate starts reading it, that is a regression of
    // ADR-001's amendment rather than a convenience.
    vm.lua().globals().set("game", true)?;

    // The DOM and clock outlive this call because the VM holds handles into
    // them; dropping them here pulls the arena out from under the application
    // before its first frame.
    std::mem::forget(dom);
    std::mem::forget(clock);
    Ok(())
}
