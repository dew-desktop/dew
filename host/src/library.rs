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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Bumped every time anything this module knows of would change what
/// `list()` returns. `dew.Library.OnChange` (`capabilities.rs`) is a
/// generation counter comparison, not a drained queue -- see that arm's own
/// doc comment for why a queue would starve every registration but the
/// first to read it. THREE THINGS BUMP THIS, on purpose:
///
/// 1. `launch`/`uninstall`/`set_enabled` below, directly, in-process --
///    covers `dew.Library`'s own calls, from whichever applet made them,
///    including in a hermetic test where nothing else here runs at all.
/// 2. `coordinator.rs`'s own registry-change point, for a start/stop with no
///    paired disk write (a crash, a plain tray unload, the moment `launch`'s
///    own queued load actually lands).
/// 3. `installed::install`/`uninstall`/`set_enabled`, via a ping to the
///    `DewLibraryChanged` pipe `coordinator.rs` owns -- the one path that
///    reaches this from a SEPARATE PROCESS (`dew install`/`dew uninstall`
///    run from a terminal against an already-running service).
///
/// (1) and (3) overlap for a `dew.Library` call that goes through
/// `installed::*` (most of them): the generation bumps twice for one
/// change. Harmless -- a registration that sees the same "changed" answer
/// on its next two checks just re-renders once more than strictly needed,
/// which is not observable next to the alternative of a missed one.
static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn bump_generation() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
}

pub fn generation() -> u64 {
    GENERATION.load(Ordering::SeqCst)
}

/// How long `uninstall` and `set_enabled` wait for a live load/unload they
/// queued to actually land before giving up. A queued request is drained
/// on the coordinator's own thread and noticed at most one
/// `coordinator::run` tick later (~15ms), so the ordinary case is a
/// handful of `TRANSITION_POLL_INTERVAL` steps, not this ceiling -- it
/// exists for a coordinator that never notices at all. Half a second is
/// already ~30x that ordinary case; there is no real scenario this is
/// meant to tolerate that takes longer; a coordinator loop that has not
/// ticked in half a second has bigger problems than this wait.
const TRANSITION_TIMEOUT: Duration = Duration::from_millis(500);
const TRANSITION_POLL_INTERVAL: Duration = Duration::from_millis(20);

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

/// Queue `id` for a live unload. THE ONLY WRITER OF `UNLOAD_QUEUE`, called
/// from exactly one place: `coordinator::spawn_unload_server`'s pipe
/// receipt handler (milestone 26). `uninstall`/`set_enabled` below reach
/// this indirectly, through `coordinator::request_unload` -- even when
/// they are running in the same process as the coordinator (`dew.Library`'s
/// own calls), not just when a separate `dew uninstall` invocation is. One
/// path for populating this queue, used identically by every caller, is
/// simpler to reason about than two.
pub(crate) fn queue_unload_request(id: String) {
    UNLOAD_QUEUE.lock().expect("library unload queue").push(id);
}

/// One row `dew.Library.List()` hands back: `installed::list()`'s own
/// id/dir/enabled, plus the name and description only that entry's own
/// manifest carries, plus whether it is actually running under this
/// coordinator right now.
///
/// `enabled` AND `running` ARE TWO DIFFERENT AXES, ON PURPOSE (see the
/// design note on `dashboard.luau`'s own Library toggle for the full
/// rationale). `enabled` is `installed.json`'s own persisted "load this
/// automatically at the next service start"; `running` is
/// `coordinator::running_snapshot()`'s transient, in-memory "is it loaded
/// right now." They usually agree -- `set_enabled` below keeps them in
/// sync live -- but they can drift: an applet stays `enabled` after its
/// own window is closed by hand (the OS close button, not this API), or a
/// crash. `running` is what a caller displaying this list actually wants
/// to show; `enabled` alone cannot answer "is this up right now."
pub struct Entry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub running: bool,
}

/// Every installed applet, enabled or not, with its manifest read for
/// display. A manifest that fails to load (removed, corrupted) falls back
/// to the bare id as its name and an empty description rather than
/// dropping the row -- the same "row survives, label degrades" choice
/// `manage.rs`'s own `row_label` already makes.
///
/// `running_snapshot()` IS READ ONCE FOR THE WHOLE LIST, not once per
/// entry -- it is an in-process read of a `Mutex<Vec<String>>`
/// (`coordinator.rs`), cheap enough that there is no reason to turn an
/// O(installed applets) list into that many separate reads of the same
/// answer.
pub fn list() -> Vec<Entry> {
    let running = crate::coordinator::running_snapshot();
    crate::installed::list()
        .into_iter()
        .map(|entry| {
            let (name, description) = match Manifest::load(&entry.dir) {
                Ok(manifest) => (manifest.display_name().to_string(), manifest.description),
                Err(_) => (entry.id.clone(), String::new()),
            };
            let is_running = running.contains(&entry.id);
            Entry {
                id: entry.id,
                name,
                description,
                enabled: entry.enabled,
                running: is_running,
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
    bump_generation();
    Ok(())
}

/// Remove `id` from the store. If it is currently running, it is unloaded
/// live first -- the same request `set_enabled(id, false)` makes -- and
/// this call waits for that to actually land before touching its
/// directory on disk, rather than deleting files out from under a thread
/// that might still be reading them.
///
/// THIS IS `dew uninstall` (the CLI) TOO, NOW (milestone 26 follow-up) --
/// `main.rs`'s `execute_uninstall` calls this function directly rather than
/// its own copy of the same dance, which is what makes `coordinator::
/// request_unload` go through a pipe even for `dew.Library`'s own in-process
/// callers: the CLI is a different process and has no other way to reach
/// `UNLOAD_QUEUE`, so this function cannot special-case "am I in the same
/// process as the coordinator or not" and still be one function.
pub fn uninstall(id: &str) -> Result<(), String> {
    if crate::coordinator::query_running(id)? {
        crate::coordinator::request_unload(id)?;
        wait_until_running_is(id, false)
            .map_err(|_| format!("'{id}' did not stop in time to uninstall; try again"))?;
    }
    crate::installed::uninstall(id)?;
    bump_generation();
    Ok(())
}

/// Polls `query_running` until it agrees with `want_running` or
/// `TRANSITION_TIMEOUT` elapses. Used by both `uninstall` (waiting for a
/// live unload to land before it is safe to delete the directory) and
/// `set_enabled` (waiting for a live load/unload to land before `List`'s
/// very next call could otherwise still report the old state).
fn wait_until_running_is(id: &str, want_running: bool) -> Result<(), String> {
    let deadline = Instant::now() + TRANSITION_TIMEOUT;
    while Instant::now() < deadline {
        if crate::coordinator::query_running(id)? == want_running {
            return Ok(());
        }
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
    Err(format!("'{id}' did not reach the expected state in time"))
}

/// Set `id`'s enabled bit, live: disabling a running applet unloads it
/// immediately rather than only taking effect on the next service start,
/// and enabling one that is not running loads it -- the same live
/// toggle `manage.rs`'s own checkbox already makes, generalized from that
/// window's own `LOAD_QUEUE`/`UNLOAD_QUEUE` to this module's.
///
/// WAITS FOR THE LOAD/UNLOAD TO LAND, THE SAME WAY `uninstall` DOES, so
/// that a `List()` called the instant this returns already sees the new
/// `running` state -- a caller rebuilding a row's own label right after
/// toggling it (`dashboard.luau`'s Library tab does exactly this) would
/// otherwise show the state from just before the click for one more
/// frame, which reads as the click having done nothing.
pub fn set_enabled(id: &str, enabled: bool) -> Result<(), String> {
    let running = crate::coordinator::query_running(id)?;
    crate::installed::set_enabled(id, enabled)?;
    // BUMPED HERE, BEFORE THE WAIT BELOW -- the disk write already
    // succeeded, so a listener's `List()` already has a new answer even if
    // the wait below times out. A live load/unload actually landing bumps
    // it again, from `coordinator.rs`'s own registry-change point.
    bump_generation();

    if enabled && !running {
        let dir = crate::installed::list()
            .into_iter()
            .find(|e| e.id == id)
            .map(|e| e.dir)
            .ok_or_else(|| format!("no applet installed with id '{id}'"))?;
        LOAD_QUEUE.lock().expect("library load queue").push(dir);
        wait_until_running_is(id, true)?;
    } else if !enabled && running {
        crate::coordinator::request_unload(id)?;
        wait_until_running_is(id, false)?;
    }

    Ok(())
}
