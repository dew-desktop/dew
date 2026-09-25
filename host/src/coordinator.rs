//! One Dew process for every applet, regardless of how many `dew run`
//! invocations asked for one.
//!
//! THE SHAPE: one **coordinator thread** — whichever invocation got here
//! first — and one **thread per loaded applet**. The coordinator owns the
//! single [`crate::tray::Tray`] and the [`Pump`] that services its
//! message-only window; each applet thread owns its own window, its own
//! [`Pump`], and its own Luau VM, entirely on that thread and never handed to
//! another one. `crate::run_applet` is today's single-applet frame loop,
//! unchanged in shape; this module is what decides how many times it runs and
//! on which thread.
//!
//! SINGLE-INSTANCE, BY A NAMED MUTEX. `CreateMutexW` with a well-known name
//! either creates the mutex (this invocation becomes the coordinator) or
//! finds it already there (`ERROR_ALREADY_EXISTS`, this invocation is a
//! second `dew run` and hands its applet to the first over a named pipe
//! rather than starting a window of its own).
//!
//! WHY A PIPE AND NOT A MUTEX ALONE. The mutex only ever answers "is a
//! coordinator running" — it carries no payload. A second invocation still
//! has to say WHICH directory it wants loaded, and a named pipe is the
//! ordinary way one Windows process hands a short message to another it does
//! not otherwise share memory with.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, ERROR_PIPE_CONNECTED, HANDLE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_SHARE_MODE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, PIPE_ACCESS_INBOUND,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
    PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::Win32::System::Threading::CreateMutexW;

use dew_window::Pump;

/// A well-known name rather than anything derived, because the whole point is
/// that a second, unrelated `dew run` invocation finds the SAME object. `Local`
/// scopes it to this login session, which is the right scope for a per-user
/// desktop service.
const MUTEX_NAME: &str = "Local\\DewSingleInstance";
const PIPE_NAME: &str = r"\\.\pipe\Dew";

/// A second pipe, separate from `PIPE_NAME`, because a query needs an answer
/// and `PIPE_NAME`'s server only ever reads -- `PIPE_ACCESS_INBOUND` cannot
/// write a response back. Kept apart from the load pipe rather than made
/// duplex, so the fire-and-forget load path this pipe has served since the
/// single-instance mutex was added stays exactly as it was.
const QUERY_PIPE_NAME: &str = r"\\.\pipe\DewQuery";

/// A third pipe, inbound only like `PIPE_NAME` and for the same reason its
/// own writer never wants an answer back: a ping saying "`installed.json`
/// changed", not a payload worth reading. `installed::install`/`uninstall`/
/// `set_enabled` write to it (`notify_library_changed`) after a successful
/// write of their own, from ANY process -- this is the one channel that
/// lets a plain terminal running `dew install`/`dew uninstall` reach an
/// already-running coordinator at all (milestone 26). What arrives is never
/// inspected; a successful connection is itself the whole message, and
/// `library::bump_generation` is the only thing a receipt does.
const LIBRARY_CHANGED_PIPE_NAME: &str = r"\\.\pipe\DewLibraryChanged";

/// A fourth pipe, inbound like `PIPE_NAME` and `LIBRARY_CHANGED_PIPE_NAME`.
/// Carries an applet id to unload live, straight into `library.rs`'s own
/// `UNLOAD_QUEUE` -- the same queue `coordinator::run`'s main loop already
/// drains for `dew.Library.SetEnabled`/`Uninstall`'s in-process calls.
/// `request_unload` (below) is what `library::uninstall`/`set_enabled` call
/// now, INSTEAD OF pushing to that queue directly, so the identical
/// mechanism serves a caller in the same process (the dashboard) and one in
/// a different process (`dew uninstall`, run from a terminal) without
/// either needing to know which it is.
const UNLOAD_PIPE_NAME: &str = r"\\.\pipe\DewUnload";

/// Whether this invocation is the one Dew process, or a request handed to it.
pub enum Role {
    /// The first `dew run` reaching this machine. Holds the mutex alive for
    /// the process's whole lifetime — dropping it is what lets a LATER
    /// invocation see the name as free again.
    Primary(MutexGuard),
    /// A Dew service is already running. This invocation's applet belongs to
    /// that one.
    Secondary,
}

/// The named mutex that marks this process as the coordinator. Nothing here
/// ever waits on it or owns it in the mutual-exclusion sense — it exists to be
/// SEEN by the next invocation's `CreateMutexW`, not to guard anything.
pub struct MutexGuard(HANDLE);

impl Drop for MutexGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Decide whether this invocation is the coordinator or a request to one.
///
/// `CreateMutexW` SUCCEEDS EITHER WAY — creating the object or opening the
/// existing one — and `ERROR_ALREADY_EXISTS` is reported through
/// `GetLastError`, not through the `Result`. That is the classic Win32
/// single-instance idiom and the reason the error is checked after a
/// successful call rather than instead of one.
pub fn acquire() -> Result<Role, String> {
    unsafe {
        let name = wide(MUTEX_NAME);
        let handle = CreateMutexW(None, false, PCWSTR(name.as_ptr())).map_err(|e| e.to_string())?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            Ok(Role::Secondary)
        } else {
            Ok(Role::Primary(MutexGuard(handle)))
        }
    }
}

/// Hand `dir` to the coordinator already running, and return without opening
/// a window of this invocation's own.
pub fn send_to_running(dir: &Path) -> Result<(), String> {
    let full = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let pipe_name = wide(PIPE_NAME);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(pipe_name.as_ptr()),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_MODE::default(),
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|e| format!("could not reach the running Dew service: {e}"))?;

    let bytes = full.to_string_lossy().into_owned().into_bytes();
    let wrote = unsafe { WriteFile(handle, Some(&bytes), None, None) };
    unsafe {
        let _ = CloseHandle(handle);
    }
    wrote.map_err(|e| format!("could not reach the running Dew service: {e}"))?;

    println!("[dew] {} handed to the running Dew service", full.display());
    Ok(())
}

/// Best-effort tell a running coordinator that `installed.json` changed.
/// Called from `installed::install`/`uninstall`/`set_enabled`, from ANY
/// process -- including one that is not the coordinator and never will be,
/// like the CLI. NO LISTENER MEANS NOTHING NEEDS TELLING, the same
/// tolerance `query_running` below already has, and for the identical
/// reason: `dew install` run before any Dew service has ever started must
/// not start failing, or even printing, because of this. Fire-and-forget
/// from the writer's side too -- these three functions have never before
/// had a reason to know or care whether a coordinator exists, and this must
/// not give them one to fail over.
pub fn notify_library_changed() {
    let pipe_name = wide(LIBRARY_CHANGED_PIPE_NAME);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(pipe_name.as_ptr()),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_MODE::default(),
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };
    let Ok(handle) = handle else {
        return;
    };
    // THE BYTES ARE NEVER READ BACK. A successful write is the whole
    // message; see `spawn_library_changed_server`.
    let _ = unsafe { WriteFile(handle, Some(b"changed"), None, None) };
    unsafe {
        let _ = CloseHandle(handle);
    }
}

/// Ask a running coordinator to unload `id` live. UNLIKE
/// `notify_library_changed`, a missing listener here IS an error --
/// `library::uninstall`/`set_enabled` only ever call this after
/// `query_running(id)` already said a coordinator has `id` loaded, so a
/// connection failure now means that answer changed underneath the caller
/// (the coordinator exited in the gap between the two calls), not "nothing
/// needs telling." The caller still has to wait for the unload to actually
/// land (`library::wait_until_running_is`) -- this only delivers the ask.
pub fn request_unload(id: &str) -> Result<(), String> {
    let pipe_name = wide(UNLOAD_PIPE_NAME);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(pipe_name.as_ptr()),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_MODE::default(),
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|e| format!("could not reach the running Dew service to unload '{id}': {e}"))?;

    let wrote = unsafe { WriteFile(handle, Some(id.as_bytes()), None, None) };
    unsafe {
        let _ = CloseHandle(handle);
    }
    wrote.map_err(|e| format!("could not reach the running Dew service to unload '{id}': {e}"))?;
    Ok(())
}

/// Ask a running coordinator whether `id` is one of its currently loaded
/// applets. `Ok(false)` covers both "no, it is not loaded" and "there is no
/// coordinator to ask" -- `dew uninstall` treats those the same way, since
/// neither leaves anything running for it to orphan.
pub fn query_running(id: &str) -> Result<bool, String> {
    let pipe_name = wide(QUERY_PIPE_NAME);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(pipe_name.as_ptr()),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_MODE::default(),
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };
    let Ok(handle) = handle else {
        // NO COORDINATOR IS LISTENING, so nothing is running under it.
        return Ok(false);
    };

    let wrote = unsafe { WriteFile(handle, Some(id.as_bytes()), None, None) };
    if wrote.is_err() {
        unsafe {
            let _ = CloseHandle(handle);
        }
        return Ok(false);
    }

    let mut buf = [0u8; 16];
    let mut read = 0u32;
    let got = unsafe { ReadFile(handle, Some(&mut buf), Some(&mut read), None) };
    unsafe {
        let _ = CloseHandle(handle);
    }

    Ok(got.is_ok() && read > 0 && buf[0] == b'1')
}

/// Listen for `send_to_running` calls forever, forwarding each directory to
/// `tx`. Runs on a thread of its own so a slow or absent client never blocks
/// the coordinator's own tray pump.
fn spawn_pipe_server(tx: mpsc::Sender<PathBuf>) {
    thread::spawn(move || loop {
        let name = wide(PIPE_NAME);
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_INBOUND,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                0,
                4096,
                0,
                None,
            )
        };
        if handle.is_invalid() {
            // SOMETHING IS BADLY WRONG rather than merely busy — a normal
            // "another instance already has one pending" is not how named
            // pipe instances work with `PIPE_UNLIMITED_INSTANCES`. Back off
            // rather than spin a core retrying a call that will not change
            // its mind.
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        let connected = unsafe { ConnectNamedPipe(handle, None) };
        let ok = connected.is_ok() || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
        if ok {
            let mut buf = [0u8; 4096];
            let mut read = 0u32;
            let got = unsafe { ReadFile(handle, Some(&mut buf), Some(&mut read), None) };
            if got.is_ok() && read > 0 {
                if let Ok(s) = std::str::from_utf8(&buf[..read as usize]) {
                    let _ = tx.send(PathBuf::from(s));
                }
            }
        }

        unsafe {
            let _ = DisconnectNamedPipe(handle);
            let _ = CloseHandle(handle);
        }
    });
}

/// The manifest ids of every applet this coordinator currently has loaded.
/// Written by `sync_running` whenever the registry changes, and read by
/// `query_running`'s pipe server, and by `manage.rs`'s management window on
/// its own thread -- both need the same fact and neither is the coordinator
/// thread that actually knows it first-hand.
static RUNNING: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The manifest ids currently running under this coordinator, as of the last
/// registry change. Read by the management window to decide whether toggling
/// an entry on or off is a live spawn/unload or just a disk write.
pub fn running_snapshot() -> HashSet<String> {
    RUNNING
        .lock()
        .map(|guard| guard.iter().cloned().collect())
        .unwrap_or_default()
}

/// Answer `query_running` calls forever, from `RUNNING` rather than anything
/// passed in -- the set of loaded applet ids changes on the coordinator's own
/// thread and this is only ever a reader of it.
fn spawn_query_server() {
    thread::spawn(move || loop {
        let name = wide(QUERY_PIPE_NAME);
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                0,
                None,
            )
        };
        if handle.is_invalid() {
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        let connected = unsafe { ConnectNamedPipe(handle, None) };
        let ok = connected.is_ok() || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
        if ok {
            let mut buf = [0u8; 4096];
            let mut read = 0u32;
            let got = unsafe { ReadFile(handle, Some(&mut buf), Some(&mut read), None) };
            if got.is_ok() && read > 0 {
                if let Ok(id) = std::str::from_utf8(&buf[..read as usize]) {
                    let is_running = RUNNING
                        .lock()
                        .map(|list| list.iter().any(|running_id| running_id == id))
                        .unwrap_or(false);
                    let response: &[u8] = if is_running { b"1" } else { b"0" };
                    let _ = unsafe { WriteFile(handle, Some(response), None, None) };
                }
            }
        }

        unsafe {
            let _ = DisconnectNamedPipe(handle);
            let _ = CloseHandle(handle);
        }
    });
}

/// Answer `notify_library_changed` pings forever. INBOUND ONLY, like
/// `spawn_pipe_server` -- a ping is not a question, and nothing here ever
/// writes back. What arrives is never inspected (see
/// `notify_library_changed`'s own doc comment for why); a successful
/// connection is the whole of the message, and bumping the generation is
/// the only thing a receipt does.
///
/// `pub(crate)`, NOT PRIVATE, so a hermetic test can start this exact
/// function directly rather than the whole of `coordinator::run` (which
/// acquires the single-instance mutex and would collide with a real `dew`
/// process on the same machine) just to prove the pipe's receiving half
/// works.
pub(crate) fn spawn_library_changed_server() {
    thread::spawn(move || loop {
        let name = wide(LIBRARY_CHANGED_PIPE_NAME);
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_INBOUND,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                0,
                64,
                0,
                None,
            )
        };
        if handle.is_invalid() {
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        let connected = unsafe { ConnectNamedPipe(handle, None) };
        let ok = connected.is_ok() || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
        if ok {
            let mut buf = [0u8; 64];
            let mut read = 0u32;
            let got = unsafe { ReadFile(handle, Some(&mut buf), Some(&mut read), None) };
            if got.is_ok() {
                crate::library::bump_generation();
            }
        }

        unsafe {
            let _ = DisconnectNamedPipe(handle);
            let _ = CloseHandle(handle);
        }
    });
}

/// Answer `request_unload` calls forever. What arrives IS inspected here,
/// unlike `LIBRARY_CHANGED_PIPE_NAME`'s server -- it is the applet id to
/// unload, pushed straight into `library::queue_unload_request`, the same
/// queue `coordinator::run`'s main loop already drains for `dew.Library`'s
/// own in-process calls to `SetEnabled`/`Uninstall`.
fn spawn_unload_server() {
    thread::spawn(move || loop {
        let name = wide(UNLOAD_PIPE_NAME);
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_INBOUND,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                0,
                4096,
                0,
                None,
            )
        };
        if handle.is_invalid() {
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        let connected = unsafe { ConnectNamedPipe(handle, None) };
        let ok = connected.is_ok() || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
        if ok {
            let mut buf = [0u8; 4096];
            let mut read = 0u32;
            let got = unsafe { ReadFile(handle, Some(&mut buf), Some(&mut read), None) };
            if got.is_ok() && read > 0 {
                if let Ok(id) = std::str::from_utf8(&buf[..read as usize]) {
                    crate::library::queue_unload_request(id.to_string());
                }
            }
        }

        unsafe {
            let _ = DisconnectNamedPipe(handle);
            let _ = CloseHandle(handle);
        }
    });
}

/// One applet the coordinator currently has loaded.
struct Loaded {
    /// Shown in the tray menu. The directory's own name, not the manifest's
    /// display name — the manifest is read on the applet's own thread, and
    /// making the tray wait for that round trip to show anything would be a
    /// worse tray than one that names the thing you typed.
    name: String,
    /// This applet's own manifest id, read here on the coordinator thread
    /// rather than waited for from the applet's thread, so a query or an
    /// auto-load decision never blocks on a VM that has not started yet.
    /// `None` when the manifest could not be read at all -- `run_applet`
    /// reports that failure properly; the coordinator just has no id to
    /// track for it.
    applet_id: Option<String>,
    /// Checked once a frame by the applet's own loop. Set to unload it
    /// without ending the process.
    close: Arc<AtomicBool>,
    handle: thread::JoinHandle<()>,
}

/// Start one applet on a thread of its own, tracked in `registry` under `id`.
/// `closed_tx` is how that thread tells the coordinator it has ended, whether
/// from its window closing, `close` being set, or a load failure.
#[allow(clippy::too_many_arguments)]
fn spawn_applet(
    dir: PathBuf,
    stats: bool,
    bench: bool,
    id: u32,
    closed_tx: mpsc::Sender<u32>,
    registry: &mut std::collections::HashMap<u32, Loaded>,
) {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string());

    // READ HERE, BEFORE THE THREAD EXISTS. `run_applet` reads the same
    // manifest again on the applet's own thread to actually mount it; this
    // second read is what lets the coordinator know an applet's id in time
    // to answer `query_running` and to skip loading an installed applet a
    // second time, neither of which can wait on a VM that has not started.
    let applet_id = crate::manifest::Manifest::load(&dir).ok().map(|m| m.id);

    let close = Arc::new(AtomicBool::new(false));
    let close_for_thread = Arc::clone(&close);

    let handle = thread::spawn(move || {
        // THE VM IS BUILT AND USED ENTIRELY ON THIS THREAD. `crate::run_applet`
        // calls `load_applet`, which creates the Luau VM, from inside the
        // closure that thread::spawn just moved onto this thread -- nothing
        // Lua-shaped ever crosses a thread boundary.
        if let Err(e) = crate::run_applet(&dir, stats, bench, &close_for_thread) {
            eprintln!("[dew] {}: {e}", dir.display());
        }
        let _ = closed_tx.send(id);
    });

    registry.insert(
        id,
        Loaded {
            name,
            applet_id,
            close,
            handle,
        },
    );
}

/// Push the registry's current names to the tray menu.
fn sync_tray(registry: &std::collections::HashMap<u32, Loaded>) {
    let mut list: Vec<(u32, String)> = registry
        .iter()
        .map(|(id, a)| (*id, a.name.clone()))
        .collect();
    list.sort_by_key(|(id, _)| *id);
    crate::tray::set_applets(list);
}

/// Refresh `RUNNING` after the registry changes.
fn sync_running(registry: &std::collections::HashMap<u32, Loaded>) {
    let ids: Vec<String> = registry
        .values()
        .filter_map(|a| a.applet_id.clone())
        .collect();
    if let Ok(mut guard) = RUNNING.lock() {
        *guard = ids;
    }
}

/// The coordinator's own loop. `_mutex` is held for as long as this runs — its
/// only job is to keep the named mutex alive so a later `dew run` sees it.
///
/// `first_dir` IS LOADED HERE RATHER THAN THE PROCESS STARTING EMPTY, because
/// `dew run <dir>` asks to see `<dir>` running, not to start a service and be
/// told to ask again.
pub fn run(_mutex: MutexGuard, first_dir: PathBuf, stats: bool, bench: bool) -> Result<(), String> {
    let (load_tx, load_rx) = mpsc::channel::<PathBuf>();
    spawn_pipe_server(load_tx);

    spawn_query_server();

    spawn_library_changed_server();

    spawn_unload_server();

    // A TRAY THAT FAILS TO CREATE DOES NOT STOP THE SERVICE. `crate::run_applet`
    // used to make the same choice for the single-applet tray it created;
    // the coordinator's one tray inherits it.
    let _tray = match crate::tray::Tray::new("Dew") {
        Ok(tray) => Some(tray),
        Err(message) => {
            eprintln!("[dew] no tray icon: {message}");
            None
        }
    };

    let mut registry: std::collections::HashMap<u32, Loaded> = std::collections::HashMap::new();
    let mut next_id: u32 = 1;
    let (closed_tx, closed_rx) = mpsc::channel::<u32>();

    spawn_applet(
        first_dir,
        stats,
        bench,
        next_id,
        closed_tx.clone(),
        &mut registry,
    );
    next_id += 1;

    // EVERY INSTALLED, ENABLED APPLET LOADS ALONGSIDE `first_dir`, UNLESS IT
    // IS `first_dir` ITSELF UNDER ANOTHER NAME. `dew run` against a
    // directory that happens to be an installed applet's own copy is not
    // asking for that applet twice.
    let already: HashSet<String> = registry
        .values()
        .filter_map(|loaded| loaded.applet_id.clone())
        .collect();
    for (id, dir) in crate::installed::enabled() {
        if already.contains(&id) {
            continue;
        }
        spawn_applet(dir, false, false, next_id, closed_tx.clone(), &mut registry);
        next_id += 1;
    }

    sync_tray(&registry);
    sync_running(&registry);

    // THE TRAY'S OWN PUMP, ON THIS THREAD. `Tray::new` created a message-only
    // window here, on the coordinator thread, and a window's messages are
    // only ever pumped by the thread that created it -- so this is the one
    // pump that has to exist for `Shell_NotifyIconW`'s callback message ever
    // to arrive.
    let mut pump = Pump::new();
    loop {
        if pump.poll().is_none() {
            // WM_QUIT, which nothing here posts. Left honoured in case
            // something outside this process ever does.
            break;
        }

        let mut changed = false;

        while let Ok(dir) = load_rx.try_recv() {
            spawn_applet(dir, false, false, next_id, closed_tx.clone(), &mut registry);
            next_id += 1;
            changed = true;
        }

        while let Ok(id) = closed_rx.try_recv() {
            if let Some(loaded) = registry.remove(&id) {
                let _ = loaded.handle.join();
                changed = true;
            }
        }

        for id in crate::tray::take_unload_requests() {
            if let Some(loaded) = registry.get(&id) {
                loaded.close.store(true, Ordering::Relaxed);
            }
        }

        // THE MANAGE-APPLETS WINDOW'S OWN TWO QUEUES, drained the same way as
        // the tray's -- see `manage.rs` for why toggling a row there never
        // spawns or closes an applet directly.
        for dir in crate::manage::take_load_requests() {
            spawn_applet(dir, false, false, next_id, closed_tx.clone(), &mut registry);
            next_id += 1;
            changed = true;
        }
        for applet_id in crate::manage::take_unload_requests() {
            for loaded in registry.values() {
                if loaded.applet_id.as_deref() == Some(applet_id.as_str()) {
                    loaded.close.store(true, Ordering::Relaxed);
                }
            }
        }

        // THE DASHBOARD'S OWN QUEUE (milestone 23), drained the same way --
        // see `dashboard.rs` for why it is not `manage.rs`'s.
        for dir in crate::dashboard::take_load_requests() {
            spawn_applet(dir, false, false, next_id, closed_tx.clone(), &mut registry);
            next_id += 1;
            changed = true;
        }

        // `dew.Library`'S OWN QUEUES (milestone 25), drained the same way
        // -- see `library.rs` for why they are not `dashboard.rs`'s or
        // `manage.rs`'s.
        for dir in crate::library::take_load_requests() {
            spawn_applet(dir, false, false, next_id, closed_tx.clone(), &mut registry);
            next_id += 1;
            changed = true;
        }
        for applet_id in crate::library::take_unload_requests() {
            for loaded in registry.values() {
                if loaded.applet_id.as_deref() == Some(applet_id.as_str()) {
                    loaded.close.store(true, Ordering::Relaxed);
                }
            }
        }

        if changed {
            sync_tray(&registry);
            sync_running(&registry);
            // THE SECOND OF `dew.Library.OnChange`'S THREE PRODUCERS (see
            // `library::bump_generation`'s own doc comment) -- a start or
            // stop that changed the registry with no paired disk write:
            // `launch`'s own queued load actually landing, a crash, a plain
            // tray unload.
            crate::library::bump_generation();
        }

        if crate::tray::exit_requested() {
            // EVERY LOADED APPLET IS TOLD, THEN WAITED FOR. A per-applet
            // unload only sets one flag; Exit sets all of them, so the
            // process does not end while a window is still on screen.
            for loaded in registry.values() {
                loaded.close.store(true, Ordering::Relaxed);
            }
            for (_, loaded) in registry {
                let _ = loaded.handle.join();
            }
            break;
        }

        // THE SERVICE PERSISTS AT ZERO APPLETS, so this loop cannot simply
        // block until something arrives -- it has to keep pumping the tray
        // and polling the pipe either way. A short sleep is what keeps that
        // from costing a spinning core while nothing is happening; nothing
        // about the tray or the pipe is frame-rate sensitive the way an
        // applet's own render loop is.
        thread::sleep(Duration::from_millis(15));
    }

    Ok(())
}
