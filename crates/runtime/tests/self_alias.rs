//! `@self` resolving under a Dew host.
//!
//! WHY THIS IS ITS OWN FIXTURE RATHER THAN VIDE. vide's `init.luau` is the
//! module that found this, and it cannot test it: its first line asserts on
//! `game` and fails before a require is ever attempted, so the two defects hide
//! each other and neither can be measured through the other. The fixture is that
//! module with everything but the `@self` require removed.
//!
//! WHAT `@self` MEANS, because the fix only makes sense against it. Luau's
//! navigator treats `@self/x` as a CHILD of the requiring module -- reset to the
//! requirer, then descend -- with no step to the parent, unlike `./x`, which
//! resets, goes up, then descends. That is only coherent for an `init.luau`,
//! which stands for its own directory: `@self/lib` from `pkg/init.luau` is
//! `pkg/lib`. It is the one form a package can use to reach its own files
//! without knowing what its directory is called.

use dew_runtime::{modules, Capabilities, Vm};
use mlua::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Load one entry module under a guest granted `roots` and `aliases`, and read
/// the marker back out of what it returned.
///
/// The marker is pulled out HERE rather than by the caller because the returned
/// table borrows the `Vm`, and a helper that handed the table back would drop the
/// VM at the end of this function -- which shows up as "Lua instance is
/// destroyed" from inside `mlua`, pointing at the assertion rather than at the
/// lifetime.
fn marker(
    entry: &Path,
    roots: Vec<PathBuf>,
    aliases: HashMap<String, PathBuf>,
) -> LuaResult<String> {
    let caps = Capabilities {
        require_roots: roots,
        print: false,
        aliases,
    };
    let vm = Vm::new(caps.clone())?;
    modules::install(&vm, &caps)?;
    let module: LuaTable = modules::load_entry(&vm, entry)?.call(())?;
    module.get("marker")
}

/// The entry point IS the `init.luau`, which is how `upstream-probe` loads a
/// package's modules and how a mod whose entry point is an `init.luau` arrives.
#[test]
fn self_resolves_from_an_entry_init() {
    let pkg = fixtures().join("self_require");
    let found = marker(&pkg.join("init.luau"), vec![pkg.clone()], HashMap::new())
        .expect("`@self/lib` from an entry `init.luau`");
    assert_eq!(found, "self-require-fixture");
}

/// The same package reached through a host alias. A different chunk name arrives
/// at `reset`, so this is a genuinely different path and not a restatement.
#[test]
fn self_resolves_through_a_host_alias() {
    let root = fixtures();
    let aliases = HashMap::from([("pkg".to_string(), root.join("self_require"))]);
    let found = marker(&root.join("via_alias.luau"), vec![root.clone()], aliases)
        .expect("`@self/lib` from a package required by alias");
    assert_eq!(found, "self-require-fixture");
}

/// The defect was never confined to `@self`: EVERY require form resets to the
/// requiring context first, so an entry `init.luau` could not resolve `./x`
/// either. This asserts the general property, and it pins the direction `./`
/// goes from an `init.luau` -- out to a sibling of the package, which is the
/// surprise that makes `@self` necessary in the first place.
#[test]
fn a_relative_require_from_an_entry_init_reaches_the_packages_sibling() {
    let root = fixtures();
    let entry = root.join("relative_init/init.luau");
    let found = marker(&entry, vec![root.clone()], HashMap::new())
        .expect("`./sibling` from an entry `init.luau`");
    assert_eq!(found, "sibling-of-the-package");
}
