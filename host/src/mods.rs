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
use crate::datamodel;
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

    //      SAID OUT LOUD, BEFORE THE MOD RUNS. What the manifest asked for and
    //      the host will not do is reported here rather than discovered by an
    //      author wondering why their keybinding does nothing. Warnings, not
    //      errors: a mod whose hotkeys are inert still renders, and refusing to
    //      load it would be a worse answer than saying which half works.
    for problem in manifest.unhonoured() {
        eprintln!("[dew] {}: {problem}", manifest.id);
    }
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

    //      THE DATAMODEL IS PER MOD, like the VM. Two mods sharing one instance
    //      tree could reach each other's widgets by walking Parent, which is the
    //      same isolation the require roots above enforce for files. It is
    //      installed as a GLOBAL rather than passed like `dew`, because it is not
    //      a capability: it is the language of the platform, present for every
    //      guest on Roblox and on Dew alike, and an application that had to be
    //      handed it would not be the application that runs on both.
    let dom = datamodel::SharedDom::default();
    datamodel::install(vm.lua(), &dom).map_err(|e| format!("{}: {e}", manifest.id))?;

    // 3 ── the framework's own desktop ceremony. Dew ships no Luau of its own:
    //      resolving the host, installing the vocabulary, opening a reactive
    //      scope and opening a session are identical for every off-engine host,
    //      so they live in Aether where the CLI gets them too.
    let desktop: LuaTable = modules::load_entry(&vm, &aether_root.join("src/host/Desktop.luau"))
        .and_then(|f| f.call(()))
        .map_err(|e| format!("{}: loading Aether's desktop host: {e}", manifest.id))?;

    // 4 ── the capability table, from the granted permissions ONLY.
    let dew = capabilities::build(vm.lua(), &manifest.permissions, state)
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    // 5 ── the mod's own module. It returns a declaration; it performs nothing.
    let declaration: LuaTable = modules::load_entry(&vm, &entry)
        .and_then(|f| f.call(()))
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    let (width, height) = size_from(&declaration);
    // The window caption a mod gets when it declares no `title` of its own is its
    // manifest `name`, falling back to the id. An applet called "Time Tracker &
    // Pomodoro HUD" in its manifest should not present itself as `timetracker`.
    let surface = Declared::from_declaration(&declaration, manifest.display_name());

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
    //      `dew` is forwarded to the mod's `mount` through Desktop.Mount's
    //      varargs, so the framework never learns what a capability table is.
    let mount_fn: LuaFunction = desktop.get("Mount").map_err(|e| e.to_string())?;
    let mounted: LuaTable = mount_fn
        .call((mount, width, height, dew))
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
