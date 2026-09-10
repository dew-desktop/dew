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
use dew_host::services::{self, Clock, SharedClock};
use dew_runtime::{modules, Capabilities, Vm};
use mlua::{Lua, Table as LuaTable, Value as LuaValue};
use std::path::Path;
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

    if self_test {
        println!();
        println!(
            "  SELF TEST: a chunk touching three symbols recorded {}",
            touched.len()
        );
        for t in &touched {
            println!("    {t}");
        }
        if touched.len() != 3 {
            eprintln!(
                "the proxy recorded {} reads for a chunk that made three; it is not measuring what ran",
                touched.len()
            );
            std::process::exit(1);
        }
    }
}
