//! How much of Aether does anything in this repository actually exercise?
//!
//! THE DENOMINATOR IS ASKED FOR. Aether's `api` is required under a real Dew host
//! and its keys are read, so a symbol added upstream shows up here without
//! anybody editing a list. `dew_host::framework` classifies them; the namespaces
//! are excluded with a reason each.
//!
//! THE NUMERATOR IS WHAT RAN, NOT WHAT WAS WRITTEN. A grep over demo source would
//! count `Aether.Combobox` in a comment exactly as it counts a call, and this
//! project has made that mistake twice -- milestone 7 opened believing 7 of 48
//! modules were framework-free because the measurement counted comments and a
//! string literal, and the true figure was 16.
//!
//! Aether is a table, so a demo is handed a proxy over it and `__index` records
//! every key read. A symbol mentioned in a comment records nothing.
//!
//! `--self-test` PROVES THE PROXY RECORDS, by running a chunk that touches
//! exactly three known symbols and refusing any answer but three. A counter
//! nobody watched count is worth what a gate nobody watched fail is worth.

use dew_host::datamodel::{self, SharedDom};
use dew_host::framework;
use dew_host::gallery;
use dew_host::services::{self, Clock, SharedClock};
use dew_runtime::{modules, Application, Capabilities, Pointer, Vm};
use mlua::{Lua, Table as LuaTable, Value as LuaValue};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn install_host(vm: &Vm) -> mlua::Result<()> {
    let dom: SharedDom = Arc::new(Mutex::new(Default::default()));
    datamodel::install(vm.lua(), &dom)?;
    let clock: SharedClock = Arc::new(Mutex::new(Clock::default()));
    services::install(vm.lua(), &clock)?;
    datamodel::install_vocabulary(vm.lua())?;
    std::mem::forget(dom);
    std::mem::forget(clock);
    Ok(())
}

/// A table that answers like `real` and remembers what was asked for.
///
/// `rawset` INTO A SEPARATE RECORD RATHER THAN THE PROXY, so a read of the
/// record's own keys cannot be mistaken for a read of the framework's.
fn recording_proxy(lua: &Lua, real: LuaTable, seen: LuaTable) -> mlua::Result<LuaTable> {
    let proxy = lua.create_table()?;
    let meta = lua.create_table()?;
    let index = lua.create_function(
        move |_, (_proxy, key): (LuaTable, LuaValue)| -> mlua::Result<LuaValue> {
            if let LuaValue::String(name) = &key {
                seen.set(name.clone(), true)?;
            }
            real.get::<LuaValue>(key)
        },
    )?;
    meta.set("__index", index)?;
    proxy.set_metatable(Some(meta))?;
    Ok(proxy)
}

/// Load Aether's public table under a real host.
fn load_aether(vm: &Vm, caps: &Capabilities, root: &Path) -> mlua::Result<LuaTable> {
    modules::install(vm, caps)?;
    let chunk = modules::load_entry(vm, &root.join("src/api.luau"))?;
    chunk.call(())
}

/// One demo's verdict.
struct Demo {
    name: String,
    /// Pixels the first paint put on the surface at all.
    ///
    /// SEPARATE FROM `moved`, because "nothing rendered" and "nothing changed"
    /// are different bugs with the same symptom. A demo whose tree never reached
    /// the surface paints an empty image twice and reports a feature broken when
    /// the wiring is.
    painted: usize,
    /// Symbols the demo reached for while building, recorded by the proxy.
    used: Vec<String>,
    /// Pixels that changed between the first paint and the last.
    moved: usize,
}

/// Every directory under `demos/` that carries a manifest and an entry named
/// after itself, which is the shape `applets/` already uses.
/// WALKED RATHER THAN LISTED, because demos are grouped by runtime --
/// `demos/applets/aether/pressable`, `demos/applets/datamodel/...` (ADR-009).
/// Scanning one level found nothing after that move and printed an
/// honest-looking `0 of 56`, which is the failure a generated number is supposed
/// to prevent rather than produce.
fn demo_entries(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let child = entry.path();
            if !child.is_dir() {
                continue;
            }
            // A demo's own installed packages are not a place to look for demos.
            if child.file_name().is_some_and(|n| n == "roblox_packages") {
                continue;
            }
            let Some(name) = child.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let luau = child.join(format!("{name}.luau"));
            if luau.is_file() && child.join("pesde.toml").is_file() {
                out.push(luau);
            } else {
                stack.push(child);
            }
        }
    }
    out.sort();
    out
}

/// Run one demo: record what it used, then drive its script and see whether the
/// pixels moved.
///
/// TWO PASSES, AND TWO SEPARATE VMs. The coverage pass mounts the tree with a
/// proxy and measures; the differential pass mounts it with the real table and
/// paints. They cannot share a VM, and finding out why cost an afternoon:
///
/// `PointerRouter` is a MODULE-LEVEL SINGLETON. Measuring in the same VM mounts a
/// second live tree that registers its own pressables with the same router, and
/// the router then hands the press to whichever it resolves first. The painted
/// tree never saw it, so `Presses` read 1 while the pixels never moved -- a demo
/// reporting a working feature as broken, for a reason entirely inside the
/// harness.
/// What the demo reached for, measured in a VM of its own.
///
/// ITS OWN VM IS THE POINT. See `run_demo`: sharing one with the differential pass
/// puts two live trees behind one module-level `PointerRouter`, and the press goes
/// to whichever it resolves first.
fn measure_demo(entry: &Path, dir: &Path, name: &str) -> Result<Vec<String>, String> {
    let mut caps = Capabilities::cli(dir.to_path_buf());
    caps.require_roots = vec![dir.to_path_buf()];
    caps.aliases.clear();
    caps.print = false;

    let dom: SharedDom = Arc::new(Mutex::new(Default::default()));
    let clock: SharedClock = Arc::new(Mutex::new(Clock::default()));
    let root_id = dom
        .lock()
        .map_err(|_| "dom lock")?
        .insert("ScreenGui".into(), "DewRoot".into());

    let installer_dom = dom.clone();
    let installer_clock = clock.clone();
    let app = Application::load_with(caps, entry, move |vm: &Vm| {
        datamodel::install(vm.lua(), &installer_dom)?;
        services::install(vm.lua(), &installer_clock)?;
        datamodel::install_vocabulary(vm.lua())?;
        let root_handle = datamodel::handle(vm.lua(), &installer_dom, root_id)?;
        vm.lua().globals().set("DewRoot", root_handle)?;
        Ok(())
    })
    .map_err(|e| format!("{name}: the demo did not load for measurement: {e}"))?;

    let lua = app.vm().lua();
    let real: LuaTable = app
        .get("Aether")
        .map_err(|e| format!("{name}: a demo must export the Aether it required: {e}"))?;
    let measure: mlua::Function = app
        .get("Measure")
        .map_err(|e| format!("{name}: a demo must export Measure: {e}"))?;

    let seen = lua.create_table().map_err(|e| e.to_string())?;
    let proxy = recording_proxy(lua, real, seen.clone()).map_err(|e| e.to_string())?;
    measure
        .call::<LuaValue>(proxy)
        .map_err(|e| format!("{name}: Measure failed: {e}"))?;

    let mut used: Vec<String> = seen
        .pairs::<String, bool>()
        .flatten()
        .map(|(k, _)| k)
        .collect();
    used.sort();
    Ok(used)
}

fn run_demo(entry: &Path) -> Result<Demo, String> {
    let dir = entry
        .parent()
        .ok_or("a demo has no directory")?
        .to_path_buf();
    let name = entry
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("demo")
        .to_string();

    // Measured first, in a VM of its own, for the reason in this function's
    // header: two live trees behind one router is a press that goes to the wrong
    // one.
    let used = measure_demo(entry, &dir, &name)?;

    // THE DEMO'S OWN PACKAGES ARE ITS ONLY REQUIRE ROOT, and it gets no aliases.
    // A demo that only loads because the host handed it a framework would prove
    // nothing about a mod, which is the thing it stands in for.
    let mut caps = Capabilities::cli(dir.clone());
    caps.require_roots = vec![dir.clone()];
    caps.aliases.clear();
    caps.print = false;

    let dom: SharedDom = Arc::new(Mutex::new(Default::default()));
    let clock: SharedClock = Arc::new(Mutex::new(Clock::default()));

    // `DewRoot` BEFORE THE DEMO RUNS, because a demo parents its tree into the
    // root the host made -- the same arrangement `main.rs` gives a standalone
    // script and `mods.rs` gives a DataModel mod. A tree parented to nothing
    // lays out against nothing and paints an empty surface, which is
    // indistinguishable from a feature that did not work.
    let root_id = dom
        .lock()
        .map_err(|_| "dom lock")?
        .insert("ScreenGui".into(), "DewRoot".into());

    let installer_dom = dom.clone();
    let installer_clock = clock.clone();

    let app = Application::load_with(caps, entry, move |vm: &Vm| {
        datamodel::install(vm.lua(), &installer_dom)?;
        services::install(vm.lua(), &installer_clock)?;
        datamodel::install_vocabulary(vm.lua())?;
        let root_handle = datamodel::handle(vm.lua(), &installer_dom, root_id)?;
        vm.lua().globals().set("DewRoot", root_handle)?;
        Ok(())
    })
    .map_err(|e| format!("{name}: the demo did not load: {e}"))?;

    // ---- the differential: did the pixels move?
    let width: f32 = app
        .get("Width")
        .map_err(|e| format!("{name}: Width: {e}"))?;
    let height: f32 = app
        .get("Height")
        .map_err(|e| format!("{name}: Height: {e}"))?;
    let session = app
        .session()
        .map_err(|e| format!("{name}: no session: {e}"))?;

    let before = gallery::paint_dom(&dom, root_id, width, height)?;

    let script: LuaTable = app
        .get("Script")
        .map_err(|e| format!("{name}: a demo must export Script: {e}"))?;
    for step in script.sequence_values::<LuaTable>() {
        let step = step.map_err(|e| format!("{name}: a Script step: {e}"))?;
        if let Ok(seconds) = step.get::<f32>("step") {
            // TICK, THEN STEP -- one frame the way Dew's own loop runs one.
            //
            // `services::tick` is what drives `DewHost.Clock.OnFrame`, and
            // `PointerRouter` registers its hover pass there when no Heartbeat
            // exists. Stepping the session alone lays out and routes input but
            // never advances the clock, so hover never re-evaluates and a tooltip
            // cannot open. That was the first demo's whole failure.
            services::tick(&clock, seconds);
            session
                .step(seconds)
                .map_err(|e| format!("{name}: step: {e}"))?;
            continue;
        }
        let kind: String = step
            .get("pointer")
            .map_err(|e| format!("{name}: a Script step is neither a step nor a pointer: {e}"))?;
        let x: f32 = step.get("x").map_err(|e| format!("{name}: x: {e}"))?;
        let y: f32 = step.get("y").map_err(|e| format!("{name}: y: {e}"))?;
        let pointer = match kind.as_str() {
            "move" => Pointer::Move,
            "down" => Pointer::Down,
            "up" => Pointer::Up,
            other => return Err(format!("{name}: unknown pointer '{other}'")),
        };
        session
            .pointer(pointer, x, y)
            .map_err(|e| format!("{name}: pointer: {e}"))?;
    }

    // WHAT THE DEMO SAW, if it offers to say. A demo may export `Hovered` so a
    // static differential can distinguish "the input never arrived" from "the
    // feature ignored it" -- two bugs with one symptom.
    // WHATEVER THE DEMO OFFERS TO SAY. A demo may export readers so a static
    // differential can distinguish "the input never arrived" from "the feature
    // ignored it" -- two bugs with one symptom, and the pixel count cannot tell
    // them apart. Finding the pressable's own was worth four rounds of guessing.
    for field in ["Presses", "Hovered", "Ticks", "Opened"] {
        if let Ok(reader) = app.get::<mlua::Function>(field) {
            if let Ok(v) = reader.call::<LuaValue>(()) {
                eprintln!("  probe   {name}: {field} = {v:?}");
            }
        }
    }

    // WHAT IS ACTUALLY ON THE SURFACE. A feature can open, add instances, and
    // still paint nothing -- transparent, zero-sized, or positioned off the
    // surface. Printing the tree separates "it never appeared" from "it appeared
    // and is invisible", which no pixel count can.
    if std::env::args().any(|a| a == "--tree") {
        let guard = dom.lock().map_err(|_| "dom lock")?;
        let mut stack = vec![(root_id, 0usize)];
        while let Some((id, depth)) = stack.pop() {
            let class = guard.class_of(id).unwrap_or_default();
            let iname = guard.name_of(id).unwrap_or_default();
            eprintln!(
                "  tree    {:indent$}{class} {iname}",
                "",
                indent = depth * 2
            );
            for child in guard.children(id).into_iter().rev() {
                stack.push((child, depth + 1));
            }
        }
    }

    let after = gallery::paint_dom(&dom, root_id, width, height)?;
    let moved = gallery::pixels_differing(&before, &after);
    let painted = before.rgba.chunks(4).filter(|px| px[3] != 0).count();

    Ok(Demo {
        name,
        painted,
        used,
        moved,
    })
}

fn main() {
    let root = match dew_runtime::installed_package("aether") {
        Some(p) => p,
        None => {
            eprintln!("no installed aether -- run `pesde install`");
            std::process::exit(2);
        }
    };

    let mut caps = Capabilities::cli(root.clone());
    caps.print = false;
    caps.aliases.insert("aether".to_string(), root.join("src"));
    if let Some(vide) = dew_runtime::installed_package("vide") {
        caps.aliases.insert("vide".to_string(), vide.join("src"));
    }

    let vm = Vm::new(caps.clone()).expect("a vm");
    install_host(&vm).expect("a host");
    let aether = load_aether(&vm, &caps, &root).expect("aether should load under a Dew host");

    let mut exported: Vec<String> = Vec::new();
    for pair in aether.clone().pairs::<LuaValue, LuaValue>() {
        if let Ok((LuaValue::String(k), _)) = pair {
            exported.push(k.to_str().expect("utf8").to_string());
        }
    }
    exported.sort();

    let scope = framework::in_scope(&exported);

    let self_test = std::env::args().any(|a| a == "--self-test");
    let mut touched: Vec<String> = Vec::new();

    if self_test {
        let seen = vm.lua().create_table().expect("seen");
        let proxy = recording_proxy(vm.lua(), aether.clone(), seen.clone()).expect("proxy");
        let probe: mlua::Function = vm
            .lua()
            .load(
                r#"
                return function(A)
                    local _ = A.create
                    local _ = A.source
                    local _ = A.Presence
                end
                "#,
            )
            .eval()
            .expect("probe chunk");
        probe.call::<()>(proxy).expect("probe runs");

        for (k, _) in seen.pairs::<String, bool>().flatten() {
            touched.push(k);
        }
        touched.sort();
    }

    // ---- the demos
    let demos_root = PathBuf::from("demos");
    let mut demos: Vec<Demo> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for entry in demo_entries(&demos_root) {
        match run_demo(&entry) {
            Ok(d) => demos.push(d),
            Err(e) => failures.push(e),
        }
    }

    for d in &demos {
        for u in &d.used {
            if !touched.contains(u) {
                touched.push(u.clone());
            }
        }
    }
    touched.retain(|t| scope.contains(t));
    touched.sort();

    println!(
        "FRAMEWORK: {} of {} demonstrated",
        touched.len(),
        scope.len()
    );
    println!(
        "  {} exported, {} excluded as namespaces",
        exported.len(),
        exported.len() - scope.len()
    );
    for (name, why) in framework::NAMESPACES {
        println!("    not a feature  {name:<12} {why}");
    }

    if !demos.is_empty() {
        println!();
        for d in &demos {
            // A DEMO THAT MOVED NO PIXELS DEMONSTRATED NOTHING, whatever it
            // touched. A closed tooltip and a broken tooltip paint the same
            // image, so the number that matters is how many pixels the script
            // changed.
            let verdict = if d.moved > 0 { "moved" } else { "STATIC" };
            println!(
                "  {verdict:>6} {:>7} px moved, {:>7} px painted  {:<14} {} symbol(s)",
                d.moved,
                d.painted,
                d.name,
                d.used.len()
            );
        }
    }

    for f in &failures {
        eprintln!("  FAILED  {f}");
    }

    if self_test {
        println!();
        let probe = ["Presence", "create", "source"];
        let hits = probe
            .iter()
            .filter(|s| touched.contains(&s.to_string()))
            .count();
        println!("  SELF TEST: a chunk touching three symbols recorded {hits} of them");
        if hits != probe.len() {
            eprintln!(
                "the proxy recorded {hits} of the three symbols the probe read; it is not measuring what ran"
            );
            std::process::exit(1);
        }
    }

    if !failures.is_empty() {
        std::process::exit(1);
    }

    // A DEMO THAT STOPPED RESPONDING IS A FEATURE THAT STOPPED WORKING, and that
    // is a red build rather than a smaller number. The coverage figure itself is
    // reported and not gated: gating an honest early number teaches somebody to
    // inflate it, which is the failure `gallery.rs` names in its own header.
    let static_demos: Vec<&Demo> = demos.iter().filter(|d| d.moved == 0).collect();
    if !static_demos.is_empty() {
        eprintln!();
        for d in &static_demos {
            if d.painted == 0 {
                eprintln!(
                    "  {}: the surface is empty. The tree never reached `DewRoot`, so this is wiring rather than the feature.",
                    d.name
                );
            } else {
                eprintln!(
                    "  {}: {} px painted and none changed. The script ran and the feature did not respond.",
                    d.name, d.painted
                );
            }
        }
        std::process::exit(1);
    }
}
