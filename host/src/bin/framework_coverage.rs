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
use dew_runtime::{Application, Capabilities, Pointer, Vm};
use mlua::{Lua, Table as LuaTable, Value as LuaValue};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    /// What the framework THIS example required exports, which is what its usage
    /// is a fraction of.
    scope: Vec<String>,
    /// Keys on that framework before namespaces are excluded.
    exported: usize,
}

/// Every directory under `examples/` that carries a manifest and an entry named
/// after itself.
///
/// WALKED RATHER THAN LISTED, because examples are grouped by what they show:
/// `examples/aether/pressable` sits one level deeper than a flat scan expects.
/// A one-level scan finds nothing the moment an example sits a directory deeper,
/// and prints an honest-looking `0 of 56`, which is the failure a generated
/// number is supposed to prevent rather than produce.
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
/// A `dew` global for a VM that is measuring rather than running.
///
/// AN EXAMPLE OPENS ITS SURFACE BY CALLING `dew.Float`, so a VM without one
/// cannot load it at all: the call is at module scope and there is nothing to
/// index. This is the same answer the host gives, narrowed to what a measurement
/// needs, which is a root to parent into.
///
/// EVERY SURFACE IS PRESENT HERE, unlike in the host, where the table carries
/// only what the manifest granted. Nothing is being protected in a measuring
/// VM, and refusing one would make the measurement depend on a manifest it does
/// not otherwise read.
fn install_dew(lua: &mlua::Lua, root: mlua::AnyUserData) -> mlua::Result<()> {
    // REUSES THE EXISTING `dew` GLOBAL IF ONE IS ALREADY THERE. `services::install`
    // runs before this and already put `Text`/`Clock`/`Pointer` on it; creating a
    // fresh table here and reassigning the global clobbered every one of them.
    let dew: mlua::Table = match lua.globals().get("dew") {
        Ok(existing) => existing,
        Err(_) => {
            let fresh = lua.create_table()?;
            lua.globals().set("dew", fresh.clone())?;
            fresh
        }
    };

    let time = lua.create_table()?;
    time.set(
        "now",
        lua.create_function(|_, ()| {
            Ok(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0))
        })?,
    )?;
    dew.set("Time", time)?;

    for surface in ["Float", "Window", "Overlay", "popover"] {
        let handed = root.clone();
        dew.set(
            surface,
            lua.create_function(move |_, _options: Option<mlua::Table>| Ok(handed.clone()))?,
        )?;
    }

    Ok(())
}

/// What one example touched, and the framework it touched it in.
struct Measured {
    used: Vec<String>,
    scope: Vec<String>,
    exported: usize,
}

fn measure_demo(entry: &Path, dir: &Path, name: &str) -> Result<Option<Measured>, String> {
    //  A VM OF ITS OWN NEEDS A POINTER OF ITS OWN. The host's pointer is process
    //  level, so an example that connects to the input service leaves its
    //  listener behind for the next VM -- and calling a listener whose Lua has
    //  been dropped panics rather than misbehaving.
    services::forget_pointer();
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
        vm.lua().globals().set("DewRoot", root_handle.clone())?;
        install_dew(vm.lua(), root_handle)?;
        Ok(())
    })
    .map_err(|e| format!("{name}: the demo did not load for measurement: {e}"))?;

    let lua = app.vm().lua();
    //  A DEMO DECLARES ITSELF BY EXPORTING `Measure`, and an applet that does not
    //  is not a broken demo. They were separate trees, so every directory this
    //  walked was instrumented by construction; one tree means an ordinary Aether
    //  applet now sits beside the demos, and calling that a failure would report
    //  three every run and teach everyone to ignore the number.
    if app.get::<mlua::Function>("Measure").is_err() {
        return Ok(None);
    }

    let real: LuaTable = app.get("Aether").map_err(|e| {
        format!("{name}: a demo exporting Measure must export the Aether it required: {e}")
    })?;
    let measure: mlua::Function = app
        .get("Measure")
        .map_err(|e| format!("{name}: a demo must export Measure: {e}"))?;

    let seen = lua.create_table().map_err(|e| e.to_string())?;
    let proxy = recording_proxy(lua, real.clone(), seen.clone()).map_err(|e| e.to_string())?;
    measure
        .call::<LuaValue>(proxy)
        .map_err(|e| format!("{name}: Measure failed: {e}"))?;

    let mut used: Vec<String> = seen
        .pairs::<String, bool>()
        .flatten()
        .map(|(k, _)| k)
        .collect();
    used.sort();
    //  Every key the example's own framework exports, which is the denominator
    //  its usage should be read against.
    let mut exported: Vec<String> = Vec::new();
    for pair in real.clone().pairs::<LuaValue, LuaValue>() {
        if let Ok((LuaValue::String(k), _)) = pair {
            exported.push(k.to_str().map_err(|e| e.to_string())?.to_string());
        }
    }
    exported.sort();
    let scope = framework::in_scope(&exported);

    Ok(Some(Measured {
        used,
        scope,
        exported: exported.len(),
    }))
}

fn run_demo(entry: &Path) -> Result<Option<Demo>, String> {
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
    let Some(measured) = measure_demo(entry, &dir, &name)? else {
        return Ok(None);
    };
    let used = measured.used.clone();

    //  The measuring VM is finished with; its listeners go with it. See
    //  `measure_demo`.
    services::forget_pointer();

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
        vm.lua().globals().set("DewRoot", root_handle.clone())?;
        install_dew(vm.lua(), root_handle)?;
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
    let lua = app.vm().lua();
    let mut input_pointer = datamodel::input::Pointer::default();

    // LAY THE TREE OUT BEFORE AIMING AT IT. A demo that parents itself into its
    // surface leaves `AbsolutePosition` at the origin until something computes
    // geometry, and hit-testing against that finds every rectangle stacked at
    // (0, 0).
    let settle = |dom: &SharedDom| -> Result<(), String> {
        let mut guard = dom.lock().map_err(|_| "dom lock")?;
        datamodel::render::commit_geometry(&mut guard, root_id, width, height);
        Ok(())
    };
    settle(&dom)?;

    let before = gallery::paint_dom(&dom, root_id, width, height)?;

    let script: LuaTable = app
        .get("Script")
        .map_err(|e| format!("{name}: a demo must export Script: {e}"))?;
    for step in script.sequence_values::<LuaTable>() {
        let step = step.map_err(|e| format!("{name}: a Script step: {e}"))?;
        if let Ok(seconds) = step.get::<f32>("step") {
            // TICK, THEN SETTLE -- one frame the way Dew's own loop runs one.
            //
            // `services::tick` is what drives `dew.Clock.OnFrame`, and
            // `PointerRouter` registers its hover pass there when no Heartbeat
            // exists. Committing geometry alone never advances the clock, so
            // hover would never re-evaluate and a tooltip could not open.
            services::tick(&clock, seconds);
            settle(&dom)?;
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
        // WHICH BUTTON, DEFAULTING TO THE PRIMARY ONE. A step that says nothing
        // is the left button, which is what every script written before this said
        // and what a session means by "down".
        let button = match step.get::<Option<String>>("button") {
            Ok(Some(named)) => match named.as_str() {
                "left" => 0,
                "right" => 1,
                "middle" => 2,
                other => return Err(format!("{name}: unknown button '{other}'")),
            },
            _ => 0,
        };
        // RECORDED AS WELL AS DELIVERED, so a demo that polls the host sees the
        // same pointer the tree was told about. Without this a guest reading
        // `dew.Pointer` headlessly gets nothing while the tree gets events.
        services::pointer_moved(x, y);
        if matches!(pointer, Pointer::Down | Pointer::Up) {
            services::pointer_button(button, matches!(pointer, Pointer::Down));
        }
        // THE HIT-TEST DISPATCH HAS NO BUTTON, and a framework's router
        // arbitrates the primary one. So a secondary press is reported through
        // the input service and stops there: dispatching it as a hit-test press
        // too would make the router press whatever the pointer happened to be
        // over, and a right-click that also left-clicks is not a gesture any
        // host performs.
        if button != 0 {
            continue;
        }
        let surface = datamodel::input::Surface {
            lua,
            dom: &dom,
            root: root_id,
            size: (width, height),
        };
        match pointer {
            Pointer::Move => input_pointer
                .moved(&surface, x, y)
                .map_err(|e| format!("{name}: pointer: {e}"))?,
            Pointer::Down => input_pointer
                .down(&surface, datamodel::input::Button::Left, x, y)
                .map_err(|e| format!("{name}: pointer: {e}"))?,
            Pointer::Up => input_pointer
                .up(&surface, datamodel::input::Button::Left, x, y)
                .map_err(|e| format!("{name}: pointer: {e}"))?,
        }
    }

    // WHAT THE DEMO SAW, if it offers to say. A demo may export `Hovered` so a
    // static differential can distinguish "the input never arrived" from "the
    // feature ignored it" -- two bugs with one symptom.
    // WHATEVER THE DEMO OFFERS TO SAY. A demo may export readers so a static
    // differential can distinguish "the input never arrived" from "the feature
    // ignored it" -- two bugs with one symptom, and the pixel count cannot tell
    // them apart. Finding the pressable's own was worth four rounds of guessing.
    for field in ["Presses", "Hovered", "Ticks", "Opened", "Chosen"] {
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

    Ok(Some(Demo {
        name,
        painted,
        used,
        moved,
        scope: measured.scope,
        exported: measured.exported,
    }))
}

fn main() {
    //  NO FRAMEWORK OF THIS REPOSITORY'S OWN. Every example installs the one it
    //  requires, and the scope each is measured against now comes from that same
    //  table, so there is nothing left for a root install to answer.
    let self_test = std::env::args().any(|a| a == "--self-test");

    let mut caps = Capabilities::cli(PathBuf::from("."));
    caps.print = false;

    // ---- the demos
    //
    // `examples/aether` RATHER THAN EVERY EXAMPLE. This measures how much of the
    // framework something actually drives, and an example that brings no
    // framework has nothing to report.
    let demos_root = PathBuf::from("examples/aether");
    let mut demos: Vec<Demo> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let mut considered = 0usize;
    for entry in demo_entries(&demos_root) {
        considered += 1;
        match run_demo(&entry) {
            Ok(Some(d)) => demos.push(d),
            Ok(None) => {}
            Err(e) => failures.push(e),
        }
    }

    //  SAID OUT LOUD, because "not a demo" and "a demo that failed to load" look
    //  identical in a total, and the first is fine while the second is not.
    println!(
        "{} example(s) under {}, {} instrumented",
        considered,
        demos_root.display(),
        demos.len()
    );

    //  ONE FRACTION PER EXAMPLE, because each brought its own framework and a
    //  single total would need them to be the same one. Reporting a headline
    //  over a scope that only some of the numerators came from is what this
    //  measured before, and it was wrong whenever two examples disagreed.
    println!();
    for d in &demos {
        let mut used = d.used.clone();
        used.retain(|u| d.scope.contains(u));
        used.sort();
        println!(
            "FRAMEWORK: {} of {} demonstrated by {}",
            used.len(),
            d.scope.len(),
            d.name
        );
        println!(
            "  {} exported, {} excluded as namespaces",
            d.exported,
            d.exported - d.scope.len()
        );
    }

    //  A UNION ONLY WHERE THERE IS ONE SCOPE TO TAKE IT OVER. Examples pinning
    //  the same framework can be added up; examples pinning different ones
    //  cannot, and saying so is the honest answer rather than a number.
    let shared = demos.first().map(|d| d.scope.clone());
    let agreed = match &shared {
        Some(first) => demos.iter().all(|d| &d.scope == first),
        None => false,
    };
    if agreed && demos.len() > 1 {
        let scope = shared.expect("checked");
        let mut touched: Vec<String> = Vec::new();
        for d in &demos {
            for u in &d.used {
                if !touched.contains(u) {
                    touched.push(u.clone());
                }
            }
        }
        touched.retain(|t| scope.contains(t));
        touched.sort();
        println!();
        println!(
            "FRAMEWORK: {} of {} demonstrated in total",
            touched.len(),
            scope.len()
        );
    } else if demos.len() > 1 {
        println!();
        println!("No total: these examples do not all require the same framework.");
    }
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
            .filter(|s| demos.iter().any(|d| d.used.contains(&s.to_string())))
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
