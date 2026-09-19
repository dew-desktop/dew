//! The engine's one flag registry: FFlag-shaped, off by default, checked
//! per-run on the thread that runs a given piece of not-ready-for-everyone
//! behavior.
//!
//! Split out of `datamodel::extensions` (milestone 12 sprint 1), which was
//! the mechanism's only owner while `Tier::Experimental` was its only
//! consumer. This module owns ENABLEMENT -- storage, the local override
//! file, and the declared/local union -- and knows nothing about the
//! DataModel, properties, or `Tier`. A caller supplies its own rows (a
//! `(flag, revision)` pair) and its own idea of what "known" means for
//! `effective_flags`'s local-override union; `datamodel::extensions` is the
//! one caller today, but nothing here requires a `class.property` shape, so
//! a coordinator or rendering-pipeline flag can call straight into this
//! module without a DataModel row to hang off of.
//!
//! THREAD-LOCAL STORAGE IS LOAD-BEARING, NOT INCIDENTAL. Each applet runs on
//! its own OS thread with its own Lua VM (`coordinator::spawn_applet`), so
//! "which flags are enabled for this applet" is naturally a thread-local,
//! the same shape `crates/window/src/win32.rs` uses for its per-thread event
//! queue. Moving this storage here without keeping it thread-local would be
//! a silent behavior change a single-threaded test run would never catch --
//! see `set_enabled_flags`.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};

/// A flag name paired with the revision of the shape it currently gates.
/// Revision has no teeth yet -- milestone 12 sprint 2's job -- so today it
/// is metadata carried alongside the name, not compared against anything.
pub type Flag = (&'static str, u32);

thread_local! {
    /// Which flags are enabled for the applet running on this thread. Empty
    /// by default, which is what makes a run that never calls
    /// `set_enabled_flags` see zero observable difference.
    static ENABLED_FLAGS: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
}

/// Enable exactly this set of flags for whatever runs on the calling thread
/// from here on. Called once, before an applet's own Luau runs -- see
/// `applets::load`.
pub fn set_enabled_flags(flags: &[Flag]) {
    ENABLED_FLAGS.with(|cell| {
        let mut set = cell.borrow_mut();
        set.clear();
        set.extend(flags.iter().map(|(flag, _)| *flag));
    });
}

/// Is this exact flag enabled for the calling thread?
pub fn is_enabled(flag: &str) -> bool {
    ENABLED_FLAGS.with(|cell| cell.borrow().contains(flag))
}

/// `dirs::data_local_dir()/Dew/DewAppSettings.json` -- a flat
/// `{ "FFlagName": true }` map, mirroring Roblox's own
/// `ClientAppSettings.json` convention exactly. A local, host-level override
/// independent of what any applet declares: the "flip it on while I'm
/// personally testing" path, the equivalent of Studio's beta-features
/// toggle.
///
/// SAME READ/WRITE SHAPE AS `positions.rs`: `dirs::data_local_dir()`,
/// create-dir-all, read-or-default. This file is read-only from here --
/// nothing writes it, a person edits it by hand -- so only the read half is
/// needed.
fn local_overrides_path() -> Option<std::path::PathBuf> {
    let dir = dirs::data_local_dir()?.join("Dew");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("DewAppSettings.json"))
}

fn read_local_overrides() -> BTreeMap<String, bool> {
    let Some(path) = local_overrides_path() else {
        return BTreeMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// The union of `declared` and whatever `DewAppSettings.json` enables
/// locally, out of `known` -- the caller's own registry, so this module
/// never needs to know how a caller's flags are named or grouped. The
/// caller says what a run DECLARED (e.g. a manifest's own
/// `experimentalDatamodel` list); the local file is a host-level override
/// for testing one without editing that declaration -- so the effective set
/// is always both, never one replacing the other.
pub fn effective_flags(declared: &[Flag], known: &[Flag]) -> Vec<Flag> {
    let mut effective: Vec<Flag> = declared.to_vec();
    let overrides = read_local_overrides();
    for (flag, revision) in known {
        let locally_on = overrides.get(*flag).copied().unwrap_or(false);
        if locally_on && !effective.iter().any(|(f, _)| f == flag) {
            effective.push((flag, *revision));
        }
    }
    effective
}

/// Print every flag in effect for this run, unconditionally -- the whole
/// point of an experimental surface is that nobody using it can plausibly
/// not know. Silent when the set is empty, so the common case (no flags at
/// all) manufactures no noise.
pub fn print_effective(flags: &[Flag]) {
    for (flag, revision) in flags {
        println!("[dew] experimental: {flag} (revision {revision})");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flags are thread-local; clear before each test so ordering between
    /// tests in this module cannot leak enabled state between them.
    fn reset() {
        set_enabled_flags(&[]);
    }

    #[test]
    fn a_flag_is_invisible_until_it_is_set() {
        reset();
        assert!(!is_enabled("FFlagTestOnlyDoesNotGateAnything"));

        set_enabled_flags(&[("FFlagTestOnlyDoesNotGateAnything", 1)]);
        assert!(is_enabled("FFlagTestOnlyDoesNotGateAnything"));
        reset();
    }

    #[test]
    fn effective_flags_is_at_least_the_declared_set() {
        let declared = vec![("FFlagTestOnlyDoesNotGateAnything", 1)];
        let known: Vec<Flag> = Vec::new();
        let effective = effective_flags(&declared, &known);
        assert!(effective.contains(&("FFlagTestOnlyDoesNotGateAnything", 1)));
    }
}
