//! Building the `dew` table a mod receives.
//!
//! THIS FILE IS THE SECURITY BOUNDARY, and it is deliberately boring to read:
//! a match on a declared permission, and a table with exactly the fields that
//! permission covers. What a mod can do is the value this function returns —
//! printable, diffable against `dew.toml`, and assertable in a test.
//!
//! Nothing here is installed as a global. The table is handed to the mod's
//! `mount` as an argument, so a capability that was not granted is not merely
//! guarded against, it is ABSENT — a mod that reaches for it gets a nil index at
//! its own call site rather than a permission check somewhere far away.

use crate::manifest::Permission;
use crate::surface::{Request, Requested};
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

/// What `dew.Widget{ ... }` needs to answer an applet.
///
/// THE ROOT IS MADE BEFORE THE APPLET RUNS, so asking for a surface hands back
/// something that already exists rather than creating a window from inside a
/// Luau call. That distinction is what keeps this step small: the applet chooses
/// its surface, and the host still decides when a window appears.
pub struct SurfaceGrant {
    pub requested: Requested,
    pub root: Option<LuaValue>,
    pub title: String,
}

/// Build the capability table for one mod.
///
/// REUSES THE `dew` GLOBAL RATHER THAN CREATING A FRESH ONE, because
/// `services::install` may already have put `Pointer`, `Input`, `Clock` and
/// `Text` on it -- the host facts that are always present, on the same terms
/// `Time` already was. Whichever of the two runs first, the other adds its
/// members to the same table instead of the two colliding on the global name.
///
/// `Time` is ungated on purpose. Reading a clock discloses nothing a mod could
/// not obtain by counting frames, and gating it would put a permission in every
/// manifest that exists to say "this mod is allowed to know what time it is".
///
/// NOT TO BE CONFUSED WITH `Clock`: `Time.now()` is wall-clock seconds since
/// the epoch, the way a mod would show the time of day; `Clock` is the
/// per-frame service installed by `services::install`, with its own `Now()`
/// that is a monotonic seconds-since-first-frame reading. Same shape, two
/// different clocks, kept apart on purpose.
pub fn build(
    lua: &Lua,
    granted: &[Permission],
    state: &Shared,
    surface: &SurfaceGrant,
) -> LuaResult<LuaTable> {
    let dew: LuaTable = match lua.globals().get("dew") {
        Ok(existing) => existing,
        Err(_) => lua.create_table()?,
    };

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
    dew.set("Time", time)?;

    for permission in granted {
        // A SURFACE PERMISSION PUTS NOTHING ON `dew` YET.
        //
        // It is a grant the host checks when the applet asks for a surface, and
        // the asking arrives in the next branch (ADR-012). Skipping here rather
        // than falling through a catch-all keeps the match exhaustive, so adding
        // a permission still fails to compile until somebody decides what it
        // hands over.
        if permission.is_surface() {
            //  A GRANTED SURFACE IS A FUNCTION ON `dew`, and an ungranted one is
            //  absent. An applet calling `dew.Overlay{}` without the word in its
            //  manifest indexes nil at its own call site, which says where the
            //  mistake is; a permission check somewhere else would not.
            //
            //  CAPITALIZED LIKE EVERY OTHER MEMBER OF `dew`, matching Aether's
            //  own PascalCase convention -- except `Permission::Popover`, which
            //  has no surface kind behind it yet and is left exactly as it was.
            let name: &str = match permission {
                Permission::Widget => "Widget",
                Permission::Window => "Window",
                Permission::Overlay => "Overlay",
                _ => permission.name(),
            };
            let kind = *permission;
            let cell = Arc::clone(&surface.requested);
            let root = surface.root.clone();
            let title = surface.title.clone();

            dew.set(
                name,
                lua.create_function(move |_, options: Option<LuaTable>| {
                    //  LAST CALL WINS, and there is no error for a second one.
                    //  An applet asking twice has changed its mind while
                    //  starting up, which is not worth refusing; asking for two
                    //  surfaces at once is, and that arrives with popovers.
                    *cell.lock().expect("requested") =
                        Some(Request::from_options(kind, options, &title));
                    Ok(root.clone())
                })?,
            )?;
            continue;
        }
        match permission {
            Permission::Widget | Permission::Window | Permission::Overlay | Permission::Popover => {
                unreachable!("surfaces are skipped above")
            }
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
                dew.set("Storage", storage)?;
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
                dew.set("Clipboard", clipboard)?;
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
                dew.set("Notifications", notifications)?;
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
                dew.set("Audio", audio)?;
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
