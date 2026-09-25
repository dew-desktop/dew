//! Building the `desktop` table a mod receives.
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Host-side state a granted capability reads or writes.
#[derive(Default)]
pub struct HostState {
    pub storage: HashMap<String, String>,
    pub clipboard: String,
}

pub type Shared = Arc<Mutex<HostState>>;

/// What `desktop.Widget{ ... }` needs to answer an applet.
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

/// Get or create `dew.Marketplace`, the sub-table every `Capability::Host`
/// permission below adds its members to (ADR-018: `dew` grows by named
/// sub-table from its first member, not by flat sibling, the shape
/// `chrome.tabs`/`chrome.identity` already follow and `desktop`'s own flat
/// members deliberately do not need to).
#[cfg(windows)]
type DiscoverResult = Result<Vec<crate::platform::DiscoveredPackage>, String>;

#[cfg(windows)]
fn dew_subtable(lua: &Lua, name: &str) -> LuaResult<LuaTable> {
    let dew: LuaTable = match lua.globals().get("dew") {
        Ok(existing) => existing,
        Err(_) => {
            let fresh = lua.create_table()?;
            lua.globals().set("dew", fresh.clone())?;
            fresh
        }
    };
    match dew.get::<LuaTable>(name) {
        Ok(existing) => Ok(existing),
        Err(_) => {
            let fresh = lua.create_table()?;
            dew.set(name, fresh.clone())?;
            Ok(fresh)
        }
    }
}

/// The `{ ok = true }` / `{ ok = false, error = "..." }` shape every
/// fallible synchronous call in `dew.Library` returns, matching what
/// `Discovered`/`Installed` already hand back for the same two outcomes.
#[cfg(windows)]
fn result_table(lua: &Lua, result: Result<(), String>) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    match result {
        Ok(()) => table.set("ok", true)?,
        Err(message) => {
            table.set("ok", false)?;
            table.set("error", message)?;
        }
    }
    Ok(table)
}

/// Build the capability table for one mod.
///
/// REUSES THE `desktop` GLOBAL RATHER THAN CREATING A FRESH ONE, because
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
    let desktop: LuaTable = match lua.globals().get("desktop") {
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
    desktop.set("Time", time)?;

    for permission in granted {
        // A SURFACE PERMISSION PUTS NOTHING ON `desktop` YET.
        //
        // It is a grant the host checks when the applet asks for a surface, and
        // the asking arrives in the next branch (ADR-012). Skipping here rather
        // than falling through a catch-all keeps the match exhaustive, so adding
        // a permission still fails to compile until somebody decides what it
        // hands over.
        if permission.is_surface() {
            //  A GRANTED SURFACE IS A FUNCTION ON `desktop`, and an ungranted one
            //  is absent. An applet calling `desktop.Overlay{}` without the word
            //  in its manifest indexes nil at its own call site, which says where
            //  the mistake is; a permission check somewhere else would not.
            //
            //  CAPITALIZED LIKE EVERY OTHER MEMBER OF `desktop`, matching Aether's
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

            desktop.set(
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
                desktop.set("Storage", storage)?;
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
                desktop.set("Clipboard", clipboard)?;
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
                desktop.set("Notifications", notifications)?;
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
                desktop.set("Audio", audio)?;
            }

            Permission::RbxAssetId => {
                // Host-level capability for Content resolution; exposes no guest Lua table.
            }

            // `Capability::Host` permissions (ADR-017), reachable from Luau
            // under `dew.Marketplace` (ADR-018) rather than as members of
            // `desktop` -- Dew's own platform layer, not the open vocabulary.
            //
            // TRIGGER AND POLL, THE SAME SHAPE `manage.rs` ALREADY PROVED,
            // moved to Luau instead of a `Mutex<Option<_>>` a `WM_TIMER`
            // drains. `Discover`/`Install` start a background thread and
            // return immediately; `Discovered`/`Installed` read whatever has
            // landed so far, or `nil` if nothing has. Neither ever blocks the
            // calling applet's own frame, and a second trigger while one is
            // already in flight is a no-op rather than a second thread.
            //
            // A PEEKED READ, NOT A DRAINED ONE -- unlike `manage.rs`'s own
            // `SIGNIN_RESULT`/`DISCOVER_RESULT`. Those exist beside an
            // imperative Win32 message loop that mutates a widget once and
            // moves on; a reactive Luau applet wants a stable value it can
            // read every frame without racing whichever frame happened to
            // catch the one moment it was drained.
            #[cfg(windows)]
            Permission::Discover => {
                let marketplace = dew_subtable(lua, "Marketplace")?;

                let in_flight = Arc::new(AtomicBool::new(false));
                let result: Arc<Mutex<Option<DiscoverResult>>> = Arc::new(Mutex::new(None));

                let discover_result = Arc::clone(&result);
                marketplace.set(
                    "Discover",
                    lua.create_function(move |_, ()| {
                        if !in_flight.swap(true, Ordering::SeqCst) {
                            // CLEARED HERE, SYNCHRONOUSLY, not left for the
                            // spawned thread to overwrite whenever it gets
                            // around to it. A second `Discover()` call while
                            // a stale answer still sits in `discover_result`
                            // must not hand that stale answer back as if it
                            // were the new request's -- `Discovered()` goes
                            // back to `nil` the instant a fresh fetch starts.
                            *discover_result.lock().expect("discover result") = None;
                            let flight = Arc::clone(&in_flight);
                            let slot = Arc::clone(&discover_result);
                            std::thread::spawn(move || {
                                let outcome = crate::platform::discover();
                                *slot.lock().expect("discover result") = Some(outcome);
                                flight.store(false, Ordering::SeqCst);
                            });
                        }
                        Ok(())
                    })?,
                )?;

                marketplace.set(
                    "Discovered",
                    lua.create_function(move |lua, ()| {
                        match &*result.lock().expect("discover result") {
                            None => Ok(LuaValue::Nil),
                            Some(Ok(packages)) => {
                                let table = lua.create_table()?;
                                table.set("ok", true)?;
                                let list = lua.create_table()?;
                                for (i, package) in packages.iter().enumerate() {
                                    let row = lua.create_table()?;
                                    row.set("ownerUserId", package.owner_user_id.as_str())?;
                                    row.set("appletId", package.applet_id.as_str())?;
                                    row.set("uploadedAt", package.uploaded_at.as_str())?;
                                    list.set((i + 1) as i64, row)?;
                                }
                                table.set("packages", list)?;
                                Ok(LuaValue::Table(table))
                            }
                            Some(Err(message)) => {
                                let table = lua.create_table()?;
                                table.set("ok", false)?;
                                table.set("error", message.as_str())?;
                                Ok(LuaValue::Table(table))
                            }
                        }
                    })?,
                )?;
            }
            #[cfg(not(windows))]
            Permission::Discover => {}

            #[cfg(windows)]
            Permission::Install => {
                let marketplace = dew_subtable(lua, "Marketplace")?;

                let in_flight = Arc::new(AtomicBool::new(false));
                let result: Arc<Mutex<Option<Result<String, String>>>> = Arc::new(Mutex::new(None));

                let install_result = Arc::clone(&result);
                marketplace.set(
                    "Install",
                    lua.create_function(move |_, (owner_user_id, applet_id): (String, String)| {
                        if !in_flight.swap(true, Ordering::SeqCst) {
                            // Same reasoning as `Discover` above: cleared
                            // synchronously so a second `Install` call never
                            // hands back a previous install's result.
                            *install_result.lock().expect("install result") = None;
                            let flight = Arc::clone(&in_flight);
                            let slot = Arc::clone(&install_result);
                            std::thread::spawn(move || {
                                let outcome = crate::package::install_from_marketplace(
                                    &owner_user_id,
                                    &applet_id,
                                    false,
                                )
                                .inspect(|id| {
                                    if let Err(e) = crate::package::sync_after_install(id) {
                                        eprintln!("[dew] {id}: {e}");
                                    }
                                });
                                *slot.lock().expect("install result") = Some(outcome);
                                flight.store(false, Ordering::SeqCst);
                            });
                        }
                        Ok(())
                    })?,
                )?;

                marketplace.set(
                    "Installed",
                    lua.create_function(move |lua, ()| {
                        match &*result.lock().expect("install result") {
                            None => Ok(LuaValue::Nil),
                            Some(Ok(id)) => {
                                let table = lua.create_table()?;
                                table.set("ok", true)?;
                                table.set("id", id.as_str())?;
                                Ok(LuaValue::Table(table))
                            }
                            Some(Err(message)) => {
                                let table = lua.create_table()?;
                                table.set("ok", false)?;
                                table.set("error", message.as_str())?;
                                Ok(LuaValue::Table(table))
                            }
                        }
                    })?,
                )?;
            }
            #[cfg(not(windows))]
            Permission::Install => {}

            // `dew.Library` (milestone 25 sprint 1). SYNCHRONOUS, NOT
            // TRIGGER-AND-POLL LIKE `Discover`/`Install` ABOVE -- every
            // operation here is a local file read or write (`installed.rs`)
            // or a named-pipe round trip already fast enough for `dew
            // uninstall` to make on every invocation
            // (`coordinator::query_running`), never a network call worth a
            // background thread.
            #[cfg(windows)]
            Permission::Library => {
                let library = dew_subtable(lua, "Library")?;

                library.set(
                    "List",
                    lua.create_function(|lua, ()| {
                        let list = lua.create_table()?;
                        for (i, entry) in crate::library::list().into_iter().enumerate() {
                            let row = lua.create_table()?;
                            row.set("id", entry.id)?;
                            row.set("name", entry.name)?;
                            row.set("description", entry.description)?;
                            row.set("enabled", entry.enabled)?;
                            list.set((i + 1) as i64, row)?;
                        }
                        Ok(list)
                    })?,
                )?;

                library.set(
                    "Launch",
                    lua.create_function(|lua, id: String| {
                        result_table(lua, crate::library::launch(&id))
                    })?,
                )?;

                library.set(
                    "Uninstall",
                    lua.create_function(|lua, id: String| {
                        result_table(lua, crate::library::uninstall(&id))
                    })?,
                )?;

                library.set(
                    "SetEnabled",
                    lua.create_function(|lua, (id, enabled): (String, bool)| {
                        result_table(lua, crate::library::set_enabled(&id, enabled))
                    })?,
                )?;
            }
            #[cfg(not(windows))]
            Permission::Library => {}

            // `dew.Account` (milestone 25 sprint 2), `Permission::Auth`'s
            // first real content -- sign-in itself stays Win32-only in
            // `manage.rs`, but reading who is signed in and signing out are
            // both small enough not to need that surface.
            //
            // BOTH SYNCHRONOUS, LIKE `dew.Library` ABOVE AND FOR THE SAME
            // REASON. `platform::load_session()` is a local, DPAPI-encrypted
            // file read -- no network call -- so `Whoami` needs no
            // trigger-and-poll shape any more than `Discover`/`Install`'s
            // own reasoning would ask of it. `platform::clear_session()` is
            // just as local.
            #[cfg(windows)]
            Permission::Auth => {
                let account = dew_subtable(lua, "Account")?;

                account.set(
                    "Whoami",
                    lua.create_function(|lua, ()| match crate::platform::load_session() {
                        Some(session) => {
                            let table = lua.create_table()?;
                            table.set("email", session.email)?;
                            table.set("userId", session.user_id)?;
                            Ok(LuaValue::Table(table))
                        }
                        None => Ok(LuaValue::Nil),
                    })?,
                )?;

                account.set(
                    "SignOut",
                    lua.create_function(|_, ()| {
                        crate::platform::clear_session();
                        Ok(())
                    })?,
                )?;
            }
            #[cfg(not(windows))]
            Permission::Auth => {}
        }
    }

    Ok(desktop)
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
