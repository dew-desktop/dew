//! The "Manage applets" window, opened from the tray menu.
//!
//! A PLAIN WIN32 WINDOW ON A THREAD OF ITS OWN, not a modal dialog on the
//! coordinator's. `coordinator::run`'s loop has to keep pumping the tray and
//! draining the load queue, the unload queue and every installed applet's
//! own thread the whole time this window might be open, and a modal dialog
//! (`DialogBoxParamW`) blocks the thread that opened it until it closes. A
//! dedicated thread with its own message loop sidesteps that without asking
//! the coordinator to do anything but glance at two small queues once per
//! iteration -- the same shape `tray.rs`'s `UNLOAD_QUEUE` already uses.
//!
//! ONE ROW PER INSTALLED APPLET, fixed for the window's lifetime. Install and
//! uninstall stay CLI-only (see `installed.rs`), so the set of rows cannot
//! change while this window is open -- only each row's enabled bit and
//! running state can, and those are refreshed on a timer plus immediately
//! after a click.
//!
//! THE CHECKBOX IS THE ONLY WRITE PATH. Clicking a row writes the enabled bit
//! to disk via `installed::set_enabled` and, if that changes whether the
//! applet should be running, pushes a request onto `LOAD_QUEUE` or
//! `UNLOAD_QUEUE` for `coordinator::run` to act on. This window never spawns
//! or closes an applet itself -- the applet's own thread lives and dies on
//! the coordinator thread, same as every other applet.

#![cfg(windows)]

use crate::manifest::Manifest;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use std::thread;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

const BST_CHECKED: usize = 1;
const BST_UNCHECKED: usize = 0;

/// Where the per-row checkbox ids start. Clear of anything this window's own
/// class uses, since a child control's id only has to be unique within its
/// parent.
const ROW_BASE: i32 = 100;
const ROW_HEIGHT: i32 = 28;
const ROW_X: i32 = 12;
const ROW_WIDTH: i32 = 380;
const ROW_Y0: i32 = 12;
const TIMER_ID: usize = 1;

/// The window handle of the management window, or 0 when none is open.
///
/// AN ISIZE, NOT AN `HWND` -- `HWND` wraps a raw pointer and is not `Send`,
/// and this is written by the window's own thread and read by the tray's
/// thread deciding whether to open a second one. The bit pattern round-trips
/// through `isize` without meaning anything on its own between those two
/// reads.
static DIALOG_HWND: AtomicIsize = AtomicIsize::new(0);

/// Directories the window has asked the coordinator to load live. Drained by
/// `coordinator::run`'s own loop, the same shape as `tray::UNLOAD_QUEUE`.
static LOAD_QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Manifest ids the window has asked the coordinator to unload live. BY
/// MANIFEST ID rather than the in-memory `u32` `tray::UNLOAD_QUEUE` uses --
/// this window knows an applet the way `installed::list` names it, not by
/// which registry slot it happens to be running in.
static UNLOAD_QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Every row as built when the window opened: the id `installed::list` knows
/// it by, and the directory to hand the coordinator if the row is switched
/// on. Read by the window procedure on the window's own thread only.
static ROWS: Mutex<Vec<(String, PathBuf)>> = Mutex::new(Vec::new());

/// Directories the window's own procedure has asked the coordinator to load
/// live since the last call. Drained, not peeked.
pub fn take_load_requests() -> Vec<PathBuf> {
    std::mem::take(&mut *LOAD_QUEUE.lock().expect("manage load queue"))
}

/// Manifest ids the window's own procedure has asked the coordinator to
/// unload live since the last call. Drained, not peeked.
pub fn take_unload_requests() -> Vec<String> {
    std::mem::take(&mut *UNLOAD_QUEUE.lock().expect("manage unload queue"))
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Open the management window, or bring the one already open to the front.
///
/// CALLED FROM THE TRAY'S OWN WINDOW PROCEDURE, itself running on the
/// coordinator thread inside `Pump::poll`. Spawning a thread here rather
/// than creating the window inline is what keeps that call from blocking.
pub fn open_or_focus() {
    let existing = DIALOG_HWND.load(Ordering::SeqCst);
    if existing != 0 {
        unsafe {
            let hwnd = HWND(existing as *mut _);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
        return;
    }
    thread::spawn(|| unsafe { run_window() });
}

/// A row's label: the manifest's display name when its directory still has a
/// readable manifest, the bare id otherwise, plus whether it is currently
/// running under this coordinator.
fn row_label(id: &str, dir: &Path, running: &HashSet<String>) -> String {
    let name = Manifest::load(dir)
        .map(|m| m.display_name().to_string())
        .unwrap_or_else(|_| id.to_string());
    let status = if running.contains(id) {
        "running"
    } else {
        "not running"
    };
    format!("{name} ({id}) - {status}")
}

/// Re-read disk and live state and push it into every row's control. Called
/// once after each click, for instant feedback, and once per timer tick so a
/// toggle the coordinator has not yet acted on catches up once it does.
unsafe fn refresh(hwnd: HWND) {
    let rows = ROWS.lock().expect("manage rows").clone();
    let enabled: std::collections::HashMap<String, bool> = crate::installed::list()
        .into_iter()
        .map(|e| (e.id, e.enabled))
        .collect();
    let running = crate::coordinator::running_snapshot();

    for (i, (id, dir)) in rows.iter().enumerate() {
        let Ok(ctrl) = GetDlgItem(Some(hwnd), ROW_BASE + i as i32) else {
            continue;
        };
        let is_enabled = enabled.get(id).copied().unwrap_or(false);
        let text = wide(&row_label(id, dir, &running));
        let _ = SetWindowTextW(ctrl, PCWSTR(text.as_ptr()));
        let want = if is_enabled {
            BST_CHECKED
        } else {
            BST_UNCHECKED
        };
        let _ = SendMessageW(ctrl, BM_SETCHECK, Some(WPARAM(want)), None);
    }
}

unsafe extern "system" fn manage_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            refresh(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xFFFF) as i32;
            let notification = (wp.0 >> 16) & 0xFFFF;
            if notification == BN_CLICKED as usize && id >= ROW_BASE {
                let idx = (id - ROW_BASE) as usize;
                let row = ROWS.lock().expect("manage rows").get(idx).cloned();
                if let Some((applet_id, dir)) = row {
                    let checked = GetDlgItem(Some(hwnd), id)
                        .map(|ctrl| SendMessageW(ctrl, BM_GETCHECK, None, None).0 as usize)
                        .unwrap_or(BST_UNCHECKED)
                        == BST_CHECKED;

                    if let Err(e) = crate::installed::set_enabled(&applet_id, checked) {
                        eprintln!("[dew] {applet_id}: {e}");
                    }

                    let running = crate::coordinator::running_snapshot();
                    if checked && !running.contains(&applet_id) {
                        LOAD_QUEUE.lock().expect("manage load queue").push(dir);
                    } else if !checked && running.contains(&applet_id) {
                        UNLOAD_QUEUE
                            .lock()
                            .expect("manage unload queue")
                            .push(applet_id);
                    }
                }
                refresh(hwnd);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(Some(hwnd), TIMER_ID);
            DIALOG_HWND.store(0, Ordering::SeqCst);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn run_window() {
    let entries = crate::installed::list();

    let Ok(instance) = GetModuleHandleW(None) else {
        return;
    };
    let class = wide("DewManage");

    let wc = WNDCLASSW {
        lpfnWndProc: Some(manage_proc),
        hInstance: instance.into(),
        lpszClassName: PCWSTR(class.as_ptr()),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut _),
        ..Default::default()
    };
    let _ = RegisterClassW(&wc);

    // ROOM FOR AT LEAST ONE ROW, so an empty store still shows a sensible
    // window rather than a sliver with nothing in it.
    let row_count = entries.len().max(1) as i32;
    let width = ROW_X * 2 + ROW_WIDTH + 16;
    let height = ROW_Y0 * 2 + row_count * ROW_HEIGHT + 40;

    // WS_VISIBLE AT CREATION, not a separate `ShowWindow` call afterward --
    // an ordinary captioned window handed to `ShowWindow` only once it
    // already exists can end up on screen without ever being marked visible,
    // depending on the desktop session. Baking the bit into the style this
    // window is created with does not depend on a second call landing.
    let title = wide("Manage applets");
    let Ok(hwnd) = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        PCWSTR(class.as_ptr()),
        PCWSTR(title.as_ptr()),
        WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0 | WS_VISIBLE.0),
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        width,
        height,
        None,
        None,
        Some(instance.into()),
        None,
    ) else {
        return;
    };

    DIALOG_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

    if entries.is_empty() {
        let text = wide("No applets installed. Use `dew install <dir>` from a terminal.");
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(wide("STATIC").as_ptr()),
            PCWSTR(text.as_ptr()),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0),
            ROW_X,
            ROW_Y0,
            ROW_WIDTH,
            ROW_HEIGHT,
            Some(hwnd),
            None,
            Some(instance.into()),
            None,
        );
    } else {
        let running = crate::coordinator::running_snapshot();
        let mut rows = Vec::with_capacity(entries.len());
        for (i, entry) in entries.iter().enumerate() {
            let y = ROW_Y0 + i as i32 * ROW_HEIGHT;
            let text = wide(&row_label(&entry.id, &entry.dir, &running));
            let ctrl = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(wide("BUTTON").as_ptr()),
                PCWSTR(text.as_ptr()),
                WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_AUTOCHECKBOX as u32),
                ROW_X,
                y,
                ROW_WIDTH,
                ROW_HEIGHT - 4,
                Some(hwnd),
                Some(HMENU((ROW_BASE + i as i32) as isize as *mut _)),
                Some(instance.into()),
                None,
            );
            if let Ok(ctrl) = ctrl {
                let want = if entry.enabled {
                    BST_CHECKED
                } else {
                    BST_UNCHECKED
                };
                let _ = SendMessageW(ctrl, BM_SETCHECK, Some(WPARAM(want)), None);
                rows.push((entry.id.clone(), entry.dir.clone()));
            }
        }
        *ROWS.lock().expect("manage rows") = rows;
    }

    let _ = SetTimer(Some(hwnd), TIMER_ID, 500, None);
    let _ = SetForegroundWindow(hwnd);

    // THIS WINDOW'S OWN PUMP, ON THIS THREAD. Blocking here is fine and
    // intended -- it is what keeps the window responsive between timer
    // ticks and clicks, and it is a thread the coordinator never waits on.
    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    DIALOG_HWND.store(0, Ordering::SeqCst);
}
