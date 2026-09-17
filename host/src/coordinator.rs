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

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, ERROR_PIPE_CONNECTED, HANDLE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_WRITE, FILE_SHARE_MODE,
    OPEN_EXISTING, PIPE_ACCESS_INBOUND,
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

/// One applet the coordinator currently has loaded.
struct Loaded {
    /// Shown in the tray menu. The directory's own name, not the manifest's
    /// display name — the manifest is read on the applet's own thread, and
    /// making the tray wait for that round trip to show anything would be a
    /// worse tray than one that names the thing you typed.
    name: String,
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

/// The coordinator's own loop. `_mutex` is held for as long as this runs — its
/// only job is to keep the named mutex alive so a later `dew run` sees it.
///
/// `first_dir` IS LOADED HERE RATHER THAN THE PROCESS STARTING EMPTY, because
/// `dew run <dir>` asks to see `<dir>` running, not to start a service and be
/// told to ask again.
pub fn run(_mutex: MutexGuard, first_dir: PathBuf, stats: bool, bench: bool) -> Result<(), String> {
    let (load_tx, load_rx) = mpsc::channel::<PathBuf>();
    spawn_pipe_server(load_tx);

    // A TRAY THAT FAILS TO CREATE DOES NOT STOP THE SERVICE. `crate::run_applet`
    // used to make the same choice for the single-applet tray it created;
    // the coordinator's one tray inherits it; a `Manage applets` window is
    // sprint 3's answer to reaching settings without one at all.
    let _tray = match crate::tray::Tray::new(
        crate::icon_path().as_deref(),
        "Dew — a desktop applet platform",
    ) {
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
    sync_tray(&registry);

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

        if changed {
            sync_tray(&registry);
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
