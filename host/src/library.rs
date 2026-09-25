//! `dew.Library` (milestone 25 sprint 1): list, launch, uninstall and
//! enable/disable an installed applet from Luau, the same operations
//! `manage.rs`'s Win32 window already offers a person from the tray.
//!
//! ITS OWN LOAD AND UNLOAD QUEUES, drained by `coordinator::run` the same
//! way as `manage.rs`'s and `dashboard.rs`'s own -- see `dashboard.rs` for
//! why a queue is not shared across surfaces that ask for different
//! reasons. The load queue is keyed by directory, like `dashboard.rs`'s
//! own: `launch`/`set_enabled(id, true)` have already resolved an applet
//! id to its directory by the time anything reaches it. The unload queue is
//! keyed by manifest id, like `manage.rs`'s own: `set_enabled(id, false)`
//! does not know or care which in-memory registry slot `id` happens to be
//! running in.

#![cfg(windows)]

use crate::manifest::Manifest;
use std::path::PathBuf;
use std::sync::Mutex;

static LOAD_QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Manifest ids `set_enabled(id, false)` has asked the coordinator to
/// unload live since the last call. BY ID, LIKE `manage.rs`'S OWN
/// `UNLOAD_QUEUE` -- this module knows an applet the way `installed::list`
/// names it, not by which registry slot it happens to be running in.
static UNLOAD_QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Directories `launch`/`set_enabled(id, true)` have asked the coordinator
/// to load live since the last call. Drained, not peeked -- the same shape
/// `dashboard.rs`'s and `manage.rs`'s own queues already use.
pub fn take_load_requests() -> Vec<PathBuf> {
    std::mem::take(&mut *LOAD_QUEUE.lock().expect("library load queue"))
}

/// Manifest ids `set_enabled(id, false)` has asked the coordinator to
/// unload live since the last call. Drained, not peeked.
pub fn take_unload_requests() -> Vec<String> {
    std::mem::take(&mut *UNLOAD_QUEUE.lock().expect("library unload queue"))
}

/// One row `dew.Library.List()` hands back: `installed::list()`'s own
/// id/dir/enabled, plus the name and description only that entry's own
/// manifest carries.
pub struct Entry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Every installed applet, enabled or not, with its manifest read for
/// display. A manifest that fails to load (removed, corrupted) falls back
/// to the bare id as its name and an empty description rather than
/// dropping the row -- the same "row survives, label degrades" choice
/// `manage.rs`'s own `row_label` already makes.
pub fn list() -> Vec<Entry> {
    crate::installed::list()
        .into_iter()
        .map(|entry| {
            let (name, description) = match Manifest::load(&entry.dir) {
                Ok(manifest) => (manifest.display_name().to_string(), manifest.description),
                Err(_) => (entry.id.clone(), String::new()),
            };
            Entry {
                id: entry.id,
                name,
                description,
                enabled: entry.enabled,
            }
        })
        .collect()
}

/// Queue `id` to load if it is not already running under this coordinator.
/// A second `launch` while it is already running is a no-op, the same
/// judgment `dashboard::open_or_focus` already makes about a second click
/// on its own tray entry -- there is no per-applet window this call could
/// bring to the front instead.
pub fn launch(id: &str) -> Result<(), String> {
    if crate::coordinator::query_running(id)? {
        return Ok(());
    }
    let dir = crate::installed::list()
        .into_iter()
        .find(|e| e.id == id)
        .map(|e| e.dir)
        .ok_or_else(|| format!("no applet installed with id '{id}'"))?;
    LOAD_QUEUE.lock().expect("library load queue").push(dir);
    Ok(())
}

/// Remove `id` from the store. Refused, not queued for later, while `id`
/// is running -- matching `dew uninstall`'s own check
/// (`coordinator::query_running`) exactly.
pub fn uninstall(id: &str) -> Result<(), String> {
    if crate::coordinator::query_running(id)? {
        return Err(format!(
            "'{id}' is currently running; exit it before uninstalling"
        ));
    }
    crate::installed::uninstall(id)
}

/// Set `id`'s enabled bit, live: disabling a running applet unloads it
/// immediately rather than only taking effect on the next service start,
/// and enabling one that is not running loads it -- the same live
/// toggle `manage.rs`'s own checkbox already makes, generalized from that
/// window's own `LOAD_QUEUE`/`UNLOAD_QUEUE` to this module's.
///
/// UNLIKE `uninstall` BELOW, THIS NEVER REFUSES ON ACCOUNT OF `id` RUNNING.
/// Uninstalling removes the directory from disk while it may still be
/// reading from it; disabling only ever asks the coordinator to close the
/// window and stop the thread, the same request `Exit` already makes of
/// every loaded applet.
pub fn set_enabled(id: &str, enabled: bool) -> Result<(), String> {
    let running = crate::coordinator::query_running(id)?;
    crate::installed::set_enabled(id, enabled)?;

    if enabled && !running {
        let dir = crate::installed::list()
            .into_iter()
            .find(|e| e.id == id)
            .map(|e| e.dir)
            .ok_or_else(|| format!("no applet installed with id '{id}'"))?;
        LOAD_QUEUE.lock().expect("library load queue").push(dir);
    } else if !enabled && running {
        UNLOAD_QUEUE
            .lock()
            .expect("library unload queue")
            .push(id.to_string());
    }

    Ok(())
}
