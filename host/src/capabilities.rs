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
/// `Discover`/`Install`'s own callbacks hand back for the same two outcomes.
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

/// Register `checker` as a `desktop.Clock.OnFrame` listener, the same list
/// an ordinary Luau `desktop.Clock.OnFrame(...)` call joins.
///
/// THIS IS THE WHOLE OF HOW A TRIGGER-WITH-CALLBACK OPERATION EVER CALLS
/// BACK INTO LUAU (ADR pending, milestone 26). `Discover`/`Install`'s own
/// background thread cannot call a `LuaFunction` directly -- Luau's VM is
/// single-threaded, and `mlua`'s handles are only ever safe to call from the
/// thread that owns the VM. What the background thread CAN do is write its
/// answer into a `Mutex<Option<_>>`; `checker` reads that, once a frame, from
/// exactly the same per-frame point `desktop.Clock`'s own listeners already
/// run from -- `services::tick`, called on the applet's own thread. Nothing
/// here is a new pump; it reuses the one `desktop.Clock.OnFrame` already
/// proved.
///
/// RELIES ON `services::install_with_pointer` HAVING ALREADY PUT `Clock` ON
/// `desktop`, which `applets::load` calls before `capabilities::build` --
/// confirmed by reading `applets.rs`, not assumed.
///
/// THE RETURNED UNSUBSCRIBE FUNCTION IS DISCARDED ON PURPOSE. This listener
/// lives exactly as long as the applet's own VM does, the same as the
/// `Discover`/`Install` capability itself -- there is no event that should
/// ever turn it off early.
#[cfg(windows)]
fn register_frame_checker(desktop: &LuaTable, checker: LuaFunction) -> LuaResult<()> {
    let clock: LuaTable = desktop.get("Clock")?;
    let on_frame: LuaFunction = clock.get("OnFrame")?;
    let _stop: LuaFunction = on_frame.call(checker)?;
    Ok(())
}

/// The `{ ok = true, packages = [...] }` / `{ ok = false, error = "..." }`
/// shape `Discover`'s callback receives.
#[cfg(windows)]
fn discover_result_table(lua: &Lua, outcome: DiscoverResult) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    match outcome {
        Ok(packages) => {
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
        }
        Err(message) => {
            table.set("ok", false)?;
            table.set("error", message)?;
        }
    }
    Ok(table)
}

/// The `{ ok = true, id = "..." }` / `{ ok = false, error = "..." }` shape
/// `Install`'s callback receives.
#[cfg(windows)]
fn install_result_table(lua: &Lua, outcome: Result<String, String>) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    match outcome {
        Ok(id) => {
            table.set("ok", true)?;
            table.set("id", id)?;
        }
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
            // TRIGGER WITH CALLBACK (milestone 26; was trigger-and-poll --
            // `Discover()` plus a separately-polled `Discovered()` -- through
            // milestone 25). `Discover`/`Install` start a background thread
            // and return immediately, same as before; the difference is what
            // happens when that thread's answer lands. It used to sit in a
            // `Mutex<Option<_>>` for the caller to remember to poll every
            // frame, peeked rather than drained so two reads of an unchanged
            // answer were never mistaken for a new one -- `dashboard.luau`'s
            // own `awaitingDiscover`/`awaitingInstall` booleans existed
            // purely to manage that. Now the caller hands over a callback
            // and the host calls it itself, exactly once, the instant the
            // answer lands -- see `register_frame_checker`'s own doc comment
            // for how that call reaches Luau without a background thread
            // ever touching the VM directly. A second trigger while one is
            // already in flight does not spawn a second thread; it replaces
            // the pending callback, the same "last call wins" precedent a
            // surface request already sets above -- the caller's most recent
            // click is the one that gets the answer, not its first one.
            #[cfg(windows)]
            Permission::Discover => {
                let marketplace = dew_subtable(lua, "Marketplace")?;

                let in_flight = Arc::new(AtomicBool::new(false));
                let result: Arc<Mutex<Option<DiscoverResult>>> = Arc::new(Mutex::new(None));
                let pending: Arc<Mutex<Option<LuaFunction>>> = Arc::new(Mutex::new(None));

                let trigger_flight = Arc::clone(&in_flight);
                let trigger_result = Arc::clone(&result);
                let trigger_pending = Arc::clone(&pending);
                marketplace.set(
                    "Discover",
                    lua.create_function(move |_, callback: LuaFunction| {
                        *trigger_pending.lock().expect("discover callback") = Some(callback);
                        if !trigger_flight.swap(true, Ordering::SeqCst) {
                            *trigger_result.lock().expect("discover result") = None;
                            let flight = Arc::clone(&trigger_flight);
                            let slot = Arc::clone(&trigger_result);
                            std::thread::spawn(move || {
                                let outcome = crate::platform::discover();
                                *slot.lock().expect("discover result") = Some(outcome);
                                flight.store(false, Ordering::SeqCst);
                            });
                        }
                        Ok(())
                    })?,
                )?;

                let checker_result = Arc::clone(&result);
                let checker_pending = Arc::clone(&pending);
                let checker = lua.create_function(move |lua, _dt: f64| {
                    let Some(outcome) = checker_result.lock().expect("discover result").take()
                    else {
                        return Ok(());
                    };
                    let Some(callback) = checker_pending.lock().expect("discover callback").take()
                    else {
                        return Ok(());
                    };
                    let table = discover_result_table(lua, outcome)?;
                    callback.call::<()>(table)
                })?;
                register_frame_checker(&desktop, checker)?;
            }
            #[cfg(not(windows))]
            Permission::Discover => {}

            #[cfg(windows)]
            Permission::Install => {
                let marketplace = dew_subtable(lua, "Marketplace")?;

                let in_flight = Arc::new(AtomicBool::new(false));
                let result: Arc<Mutex<Option<Result<String, String>>>> = Arc::new(Mutex::new(None));
                let pending: Arc<Mutex<Option<LuaFunction>>> = Arc::new(Mutex::new(None));

                let trigger_flight = Arc::clone(&in_flight);
                let trigger_result = Arc::clone(&result);
                let trigger_pending = Arc::clone(&pending);
                marketplace.set(
                    "Install",
                    lua.create_function(
                        move |_, (owner_user_id, applet_id, callback): (String, String, LuaFunction)| {
                            *trigger_pending.lock().expect("install callback") = Some(callback);
                            if !trigger_flight.swap(true, Ordering::SeqCst) {
                                *trigger_result.lock().expect("install result") = None;
                                let flight = Arc::clone(&trigger_flight);
                                let slot = Arc::clone(&trigger_result);
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
                        },
                    )?,
                )?;

                let checker_result = Arc::clone(&result);
                let checker_pending = Arc::clone(&pending);
                let checker = lua.create_function(move |lua, _dt: f64| {
                    let Some(outcome) = checker_result.lock().expect("install result").take()
                    else {
                        return Ok(());
                    };
                    let Some(callback) = checker_pending.lock().expect("install callback").take()
                    else {
                        return Ok(());
                    };
                    let table = install_result_table(lua, outcome)?;
                    callback.call::<()>(table)
                })?;
                register_frame_checker(&desktop, checker)?;
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
                            row.set("running", entry.running)?;
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

                // A PERSISTENT SUBSCRIPTION, NOT A ONE-SHOT CALLBACK LIKE
                // `Discover`/`Install` ABOVE. Those fire once per trigger and
                // are done; `OnChange` fires every time
                // `library::bump_generation()` runs again, for as long as
                // this applet lives -- the same "keep firing until told
                // otherwise" shape `desktop.Clock.OnFrame` itself already
                // has, which is fitting since this is built on it. "Last
                // call wins" on a second `OnChange` registration, the same
                // precedent a surface request already sets.
                //
                // THE BASELINE IS READ AT GRANT TIME, not zero -- a change
                // that already happened before this applet mounted is not
                // this applet's to react to; only a change from here on is.
                let pending: Arc<Mutex<Option<LuaFunction>>> = Arc::new(Mutex::new(None));
                let last_seen: Arc<Mutex<u64>> = Arc::new(Mutex::new(crate::library::generation()));

                let on_change_pending = Arc::clone(&pending);
                library.set(
                    "OnChange",
                    lua.create_function(move |_, callback: LuaFunction| {
                        *on_change_pending.lock().expect("library onchange") = Some(callback);
                        Ok(())
                    })?,
                )?;

                let checker = lua.create_function(move |_lua, _dt: f64| {
                    let Some(callback) = pending.lock().expect("library onchange").clone() else {
                        return Ok(());
                    };
                    let current = crate::library::generation();
                    let mut seen = last_seen.lock().expect("library generation seen");
                    if *seen != current {
                        *seen = current;
                        callback.call::<()>(())?;
                    }
                    Ok(())
                })?;
                register_frame_checker(&desktop, checker)?;
            }
            #[cfg(not(windows))]
            Permission::Library => {}

            // `dew.Account`. `Whoami`/`SignOut` (milestone 25 sprint 2) are
            // synchronous, like `dew.Library` above and for the same reason:
            // `platform::load_session()`/`clear_session()` are local,
            // DPAPI-encrypted file operations, no network call, so neither
            // needs a trigger-and-poll shape any more than `Discover`/
            // `Install`'s own reasoning would ask of it.
            //
            // `SignIn` (milestone 26 sprint 3) IS a network call --
            // `platform::login` -- so it is trigger-with-callback, the same
            // shape Sprint 1 gave `Discover`/`Install`, not a third shape
            // invented just for this. Sign-in stayed Win32-only in
            // `manage.rs` through milestone 25 only because this shape did
            // not exist yet to build it in.
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

                let in_flight = Arc::new(AtomicBool::new(false));
                let result: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
                let pending: Arc<Mutex<Option<LuaFunction>>> = Arc::new(Mutex::new(None));

                let trigger_flight = Arc::clone(&in_flight);
                let trigger_result = Arc::clone(&result);
                let trigger_pending = Arc::clone(&pending);
                account.set(
                    "SignIn",
                    lua.create_function(
                        move |_, (email, password, callback): (String, String, LuaFunction)| {
                            *trigger_pending.lock().expect("signin callback") = Some(callback);
                            if !trigger_flight.swap(true, Ordering::SeqCst) {
                                *trigger_result.lock().expect("signin result") = None;
                                let flight = Arc::clone(&trigger_flight);
                                let slot = Arc::clone(&trigger_result);
                                std::thread::spawn(move || {
                                    let outcome =
                                        crate::platform::login(&email, &password).map(|_| ());
                                    *slot.lock().expect("signin result") = Some(outcome);
                                    flight.store(false, Ordering::SeqCst);
                                });
                            }
                            Ok(())
                        },
                    )?,
                )?;

                let checker_result = Arc::clone(&result);
                let checker_pending = Arc::clone(&pending);
                let checker = lua.create_function(move |lua, _dt: f64| {
                    let Some(outcome) = checker_result.lock().expect("signin result").take()
                    else {
                        return Ok(());
                    };
                    let Some(callback) = checker_pending.lock().expect("signin callback").take()
                    else {
                        return Ok(());
                    };
                    let table = result_table(lua, outcome)?;
                    callback.call::<()>(table)
                })?;
                register_frame_checker(&desktop, checker)?;
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
