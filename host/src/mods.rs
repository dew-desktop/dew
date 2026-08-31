//! Loading a mod: manifest, then VM, then capabilities, then its own code.
//!
//! THE ORDER IS THE POINT. Everything the host needs to decide whether a mod may
//! run is known before the mod's logic executes:
//!
//!   1. `mod.json` is read from disk. It names the mod and its permissions.
//!   2. A VM is created — deny-by-default, with no ffi, io, os or ambient `dew`.
//!   3. The `dew` table is built from the GRANTED permissions and nothing else.
//!   4. The mod's module is loaded. It returns a declaration and does nothing.
//!   5. `mount(dew)` is called once, and returns a tree.
//!
//! A registration-style API collapses 4 and 5 into "loading the mod runs the
//! mod", which puts every one of the earlier steps after the fact.

use crate::capabilities::{self, Shared};
use crate::manifest::Manifest;
use crate::surface::Declared;
use aether_runtime::{modules, Session, Vm};
use mlua::prelude::*;
use std::path::{Path, PathBuf};

pub struct Mod {
    pub manifest: Manifest,
    pub width: u32,
    pub height: u32,
    pub surface: Declared,
    pub session: Session,
    /// The VM this mod lives in. Held because dropping it takes the session's
    /// Lua handles with it — a mod is exactly as alive as its VM.
    pub vm: Vm,
}

/// Default widget size when a mod declares none.
const DEFAULT_SIZE: (u32, u32) = (380, 56);

fn size_from(declaration: &LuaTable) -> (u32, u32) {
    let Ok(size) = declaration.get::<LuaTable>("size") else {
        return DEFAULT_SIZE;
    };
    let width: u32 = size.get("width").unwrap_or(0);
    let height: u32 = size.get("height").unwrap_or(0);
    if width > 0 && height > 0 {
        (width, height)
    } else {
        DEFAULT_SIZE
    }
}

pub fn load(
    dir: &Path,
    aether_root: &Path,
    aliases: &std::collections::HashMap<String, PathBuf>,
    state: &Shared,
) -> Result<Mod, String> {
    // 1 ── the manifest, before anything of the mod's runs.
    let manifest = Manifest::load(dir)?;
    let entry = manifest
        .entry(dir)
        .ok_or_else(|| format!("{}: no {}.luau or main.luau", dir.display(), manifest.id))?;

    // 2 ── a VM that can reach the mod's own directory and Aether, and nothing
    //      else. Two roots rather than one: a mod requiring a sibling mod's files
    //      is not a thing this platform supports, and the resolver is where that
    //      is enforced rather than checked for later.
    let caps = aether_runtime::Capabilities {
        require_roots: vec![dir.to_path_buf(), aether_root.to_path_buf()],
        // A mod's `print` is the author's own debugging and goes to the console
        // the host was launched from.
        print: true,
        // Aether, and the dependencies Aether declares that this host installs.
        aliases: aliases.clone(),
    };
    let vm = Vm::new(caps.clone()).map_err(|e| format!("{}: {e}", manifest.id))?;
    modules::install(&vm, &caps).map_err(|e| format!("{}: {e}", manifest.id))?;

    // 3 ── the mount prelude: vocabulary installed, Session ready to build.
    let prelude_path = prelude_path()?;
    let prelude: LuaTable = modules::load_entry(&vm, &prelude_path)
        .and_then(|f| f.call(()))
        .map_err(|e| format!("{}: loading the mount prelude: {e}", manifest.id))?;

    // 4 ── the capability table, from the granted permissions ONLY.
    let dew = capabilities::build(vm.lua(), &manifest.permissions, state)
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    // 5 ── the mod's own module. It returns a declaration; it performs nothing.
    let declaration: LuaTable = modules::load_entry(&vm, &entry)
        .and_then(|f| f.call(()))
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    let (width, height) = size_from(&declaration);
    let surface = Declared::from_declaration(&declaration, &manifest.id);

    let mount: LuaFunction = declaration.get("mount").map_err(|_| {
        format!(
            "{}: the module returned no `mount` — a Dew mod returns \
             {{ id = …, size = …, mount = function(dew) … end }}. See docs/mod_contract.md",
            manifest.id
        )
    })?;

    // 6 ── build the tree, ONCE, inside a reactive scope the prelude opens.
    //      The mount function is handed OVER rather than called here: `derive`
    //      and `effect` refuse to run outside a stable scope, and calling it from
    //      Rust would run it outside one.
    let mount_fn: LuaFunction = prelude.get("Mount").map_err(|e| e.to_string())?;
    let mounted: LuaTable = mount_fn
        .call((mount, dew, width, height))
        .map_err(|e| format!("{}: while mounting: {e}", manifest.id))?;

    let session_tbl: LuaTable = mounted.get("Session").map_err(|e| e.to_string())?;
    let session =
        Session::from_lua(vm.lua(), &session_tbl).map_err(|e| format!("{}: {e}", manifest.id))?;

    println!(
        "[dew] loaded {} ({}x{}) — {} — granted: {}",
        manifest.id,
        width,
        height,
        surface.describe(),
        capabilities::describe(&manifest.permissions)
    );

    Ok(Mod {
        manifest,
        width,
        height,
        surface,
        session,
        vm,
    })
}

/// Where `mount.luau` lives, relative to the running executable or the source
/// tree. Looked up rather than embedded so it can be read and edited like the
/// Luau it is.
fn prelude_path() -> Result<PathBuf, String> {
    let candidates = [
        PathBuf::from("host/runtime/mount.luau"),
        PathBuf::from("runtime/mount.luau"),
        PathBuf::from("../host/runtime/mount.luau"),
        PathBuf::from("../../host/runtime/mount.luau"),
    ];
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| "could not find host/runtime/mount.luau".to_string())
}
