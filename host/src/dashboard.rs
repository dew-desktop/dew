//! Opening the bundled dashboard applet from the tray.
//!
//! `bundled.rs` brings the dashboard's copy in the bundled directory up to
//! date with this build on every open; this module only asks the
//! coordinator to load it.
//!
//! ITS OWN LOAD QUEUE, drained by `coordinator::run` the same way as
//! `tray.rs`'s and `library.rs`'s own -- reusing one of theirs would blur
//! which surface asked for what.

#![cfg(windows)]

use std::path::PathBuf;
use std::sync::Mutex;

/// Directories this module has asked the coordinator to load live since the
/// last call. Drained, not peeked -- the same shape `tray.rs`'s own queue
/// already uses.
static LOAD_QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

pub fn take_load_requests() -> Vec<PathBuf> {
    std::mem::take(&mut *LOAD_QUEUE.lock().expect("dashboard load queue"))
}

/// Open the dashboard, or do nothing if it is already running.
///
/// CALLED FROM THE TRAY'S OWN WINDOW PROCEDURE, on the coordinator thread --
/// `bundled::ensure` is a handful of small file comparisons, not a network call
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
    match crate::bundled::ensure("dashboard") {
        Ok(dir) => LOAD_QUEUE.lock().expect("dashboard load queue").push(dir),
        Err(e) => eprintln!("[dew] dashboard: {e}"),
    }
}
