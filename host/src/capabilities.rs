//! Building the `dew` table a mod receives.
//!
//! THIS FILE IS THE SECURITY BOUNDARY, and it is deliberately boring to read:
//! a match on a declared permission, and a table with exactly the fields that
//! permission covers. What a mod can do is the value this function returns —
//! printable, diffable against `mod.json`, and assertable in a test.
//!
//! Nothing here is installed as a global. The table is handed to the mod's
//! `mount` as an argument, so a capability that was not granted is not merely
//! guarded against, it is ABSENT — a mod that reaches for it gets a nil index at
//! its own call site rather than a permission check somewhere far away.

use crate::manifest::Permission;
use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Host-side state a granted capability reads or writes.
#[derive(Default)]
pub struct HostState {
    pub storage: HashMap<String, String>,
    pub clipboard: String,
}

pub type Shared = Arc<Mutex<HostState>>;

/// Build the capability table for one mod.
///
/// `time` is ungated on purpose. Reading a clock discloses nothing a mod could
/// not obtain by counting frames, and gating it would put a permission in every
/// manifest that exists to say "this mod is allowed to know what time it is".
pub fn build(lua: &Lua, granted: &[Permission], state: &Shared) -> LuaResult<LuaTable> {
    let dew = lua.create_table()?;

    let time = lua.create_table()?;
    time.set(
        "now",
        lua.create_function(|_, ()| {
            Ok(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0))
        })?,
    )?;
    dew.set("time", time)?;

    for permission in granted {
        match permission {
            Permission::Storage => {
                let storage = lua.create_table()?;

                let get_state = Arc::clone(state);
                storage.set(
                    "get",
                    lua.create_function(move |_, key: String| {
                        Ok(get_state.lock().unwrap().storage.get(&key).cloned())
                    })?,
                )?;

                let set_state = Arc::clone(state);
                storage.set(
                    "set",
                    lua.create_function(move |_, (key, value): (String, String)| {
                        set_state.lock().unwrap().storage.insert(key, value);
                        Ok(())
                    })?,
                )?;
                dew.set("storage", storage)?;
            }

            Permission::Clipboard => {
                let clipboard = lua.create_table()?;

                let read_state = Arc::clone(state);
                clipboard.set(
                    "read",
                    lua.create_function(move |_, ()| {
                        Ok(read_state.lock().unwrap().clipboard.clone())
                    })?,
                )?;

                let write_state = Arc::clone(state);
                clipboard.set(
                    "write",
                    lua.create_function(move |_, text: String| {
                        write_state.lock().unwrap().clipboard = text;
                        Ok(())
                    })?,
                )?;
                dew.set("clipboard", clipboard)?;
            }

            Permission::Notifications => {
                let notifications = lua.create_table()?;
                notifications.set(
                    "show",
                    lua.create_function(|_, message: String| {
                        // Stdout until there is a tray to raise these through.
                        // Named as unfinished rather than left looking complete.
                        println!("[dew:notify] {message}");
                        Ok(())
                    })?,
                )?;
                dew.set("notifications", notifications)?;
            }

            Permission::Audio => {
                let audio = lua.create_table()?;
                audio.set(
                    "chime",
                    lua.create_function(|_, name: String| {
                        println!("[dew:chime] {name}");
                        Ok(())
                    })?,
                )?;
                dew.set("audio", audio)?;
            }

            Permission::RbxAssetId => {
                // Host-level capability for Content resolution; exposes no guest Lua table.
            }
        }
    }

    Ok(dew)
}

/// What was actually granted, for a log line or a test.
pub fn describe(granted: &[Permission]) -> String {
    if granted.is_empty() {
        return "nothing".to_string();
    }
    granted
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join(", ")
}
