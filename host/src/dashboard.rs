//! Loading the bundled dashboard applet (milestone 23) on demand from the
//! tray, into the one directory ADR-017 lets hold a `Capability::Host`
//! permission.
//!
//! MATERIALIZED FRESH ON EVERY OPEN, not installed once and left to drift.
//! There is no packaged installer yet to place the dashboard's source next
//! to `dew.exe`, so this copies it from this checkout's own `dashboard/`
//! directory (found via `CARGO_MANIFEST_DIR`, baked in at compile time) --
//! a development-time stand-in, honestly named as one. The day a real
//! installer exists, this is the one function that changes: everything
//! downstream of it (the bundled directory, `applets::load`'s check, the
//! tray entry) already only cares that the directory it is handed is the
//! coordinator's own.
//!
//! ITS OWN LOAD QUEUE, drained by `coordinator::run` the same way as
//! `tray.rs`'s and `library.rs`'s own -- reusing one of theirs would blur
//! which surface asked for what.

#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Directories this module has asked the coordinator to load live since the
/// last call. Drained, not peeked -- the same shape `tray.rs`'s own queue
/// already uses.
static LOAD_QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

pub fn take_load_requests() -> Vec<PathBuf> {
    std::mem::take(&mut *LOAD_QUEUE.lock().expect("dashboard load queue"))
}

/// This checkout's own `dashboard/` directory, sibling to `host/`.
/// `CARGO_MANIFEST_DIR` is `<repo>/host`, matching
/// `generate_datamodel_types.rs`'s own `repo_root()`.
fn dev_source_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .expect("host/ has a parent")
        .join("dashboard")
}

/// Copies the dashboard's source into the coordinator's own bundled
/// directory, replacing whatever was there -- fresh on every open, so a
/// build never runs a stale copy of itself. Returns the directory
/// `applets::load` should be pointed at.
///
/// REMOVED BEFORE IT IS RECOPIED, not merged into. `copy_dir` only ever
/// adds or overwrites; a file this source used to have and no longer does
/// would otherwise linger in the bundled copy forever, which is exactly
/// the staleness this function exists to rule out.
fn ensure_installed() -> Result<PathBuf, String> {
    let dest = crate::installed::bundled_applets_dir()
        .ok_or("could not find a per-user data directory to bundle the dashboard into")?
        .join("dashboard");
    let _ = std::fs::remove_dir_all(&dest);
    crate::installed::copy_dir(&dev_source_dir(), &dest)?;
    Ok(dest)
}

/// Open the dashboard, or do nothing if it is already running.
///
/// CALLED FROM THE TRAY'S OWN WINDOW PROCEDURE, on the coordinator thread --
/// `ensure_installed` is a handful of small file copies, not a network call
/// or anything else worth spawning a thread over.
///
/// NO FOCUS-AN-EXISTING-WINDOW PATH. An ordinary loaded applet's window is
/// not tracked by id anywhere a tray click could reach it -- only
/// `running_snapshot()` says whether one is live at all -- so a second
/// click while it is already open is a no-op rather than a crash, not a
/// bring-to-front.
pub fn open_or_focus() {
    if crate::coordinator::running_snapshot().contains("dashboard") {
        return;
    }
    match ensure_installed() {
        Ok(dir) => LOAD_QUEUE.lock().expect("dashboard load queue").push(dir),
        Err(e) => eprintln!("[dew] dashboard: {e}"),
    }
}
