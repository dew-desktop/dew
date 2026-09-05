//! Dew's tray icon and its menu.
//!
//! A MESSAGE-ONLY WINDOW OF OUR OWN, not the widget's. A tray icon delivers its
//! clicks to a window procedure, and `dew_window` owns the widget's — it is a
//! surface for painting Aether, and giving it a second job would put Dew's
//! furniture inside the crate that must stay useful to hosts which have none.
//!
//! `HWND_MESSAGE` creates a window that is never shown, never in the taskbar, and
//! exists only to receive. The main loop's `poll()` pumps every message for the
//! thread, so this window's procedure runs there without a second pump.
//!
//! WHAT IS TEMPORARY HERE, and should be said out loud: the menu is a frame-rate
//! cap and an exit. It is scaffolding for reaching settings that do not have a UI
//! yet, and the moment Dew has a settings surface most of this should move into
//! it — a tray menu is a bad place to configure anything you cannot see the
//! effect of.

use std::sync::atomic::{AtomicU32, Ordering};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::*;

/// The frame budget in microseconds, or 0 for uncapped.
///
/// AN ATOMIC, because the menu writes it from inside a window procedure running
/// on the OS's stack during `poll()`, and the frame loop reads it immediately
/// after. Both are the same thread, but the borrow checker cannot see that
/// through a `wndproc` and an atomic is cheaper than proving it.
static FRAME_BUDGET_US: AtomicU32 = AtomicU32::new(0);

/// Set when the menu's Exit is chosen.
static EXIT_REQUESTED: AtomicU32 = AtomicU32::new(0);

const TRAY_CALLBACK: u32 = WM_APP + 1;
const ID_EXIT: usize = 1000;

/// The offered caps. `None` is uncapped and is the default.
///
/// UNCAPPED MEANS UNCAPPED — the loop does not sleep. That is what was asked for
/// and it is the right default for a machine with headroom, but it is worth
/// knowing that an idle widget then spins a core: `Driver::frame` returns false
/// with nothing to paint and the loop immediately asks again. A cap is the only
/// thing currently standing between Dew and 100% of one core.
const CAPS: &[(usize, &str, u32)] = &[
    (2000, "Uncapped", 0),
    (2001, "30 FPS", 33_333),
    (2002, "60 FPS", 16_667),
    (2003, "120 FPS", 8_333),
    (2004, "144 FPS", 6_944),
    (2005, "240 FPS", 4_166),
];

/// The current frame budget. `None` means do not sleep.
pub fn frame_budget() -> Option<std::time::Duration> {
    match FRAME_BUDGET_US.load(Ordering::Relaxed) {
        0 => None,
        us => Some(std::time::Duration::from_micros(us as u64)),
    }
}

pub fn exit_requested() -> bool {
    EXIT_REQUESTED.load(Ordering::Relaxed) != 0
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn tray_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        TRAY_CALLBACK => {
            // Either button opens the menu. A tray icon with a left-click action
            // that is not "show me what I can do" is a tray icon people stop
            // clicking, and Dew has no single obvious action to offer.
            let event = (lp.0 as u32) & 0xFFFF;
            if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
                show_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = wp.0 & 0xFFFF;
            if id == ID_EXIT {
                EXIT_REQUESTED.store(1, Ordering::Relaxed);
            } else if let Some((_, _, us)) = CAPS.iter().find(|(cap_id, _, _)| *cap_id == id) {
                FRAME_BUDGET_US.store(*us, Ordering::Relaxed);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };
    let Ok(fps) = CreatePopupMenu() else { return };

    let current = FRAME_BUDGET_US.load(Ordering::Relaxed);
    for (id, label, us) in CAPS {
        // A RADIO CHECK, so the menu shows what is in force. A settings menu that
        // does not report the current setting is a menu you have to guess at.
        let flags = if *us == current {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        let _ = AppendMenuW(fps, flags, *id, PCWSTR(wide(label).as_ptr()));
    }

    let _ = AppendMenuW(
        menu,
        MF_POPUP,
        fps.0 as usize,
        PCWSTR(wide("Max FPS").as_ptr()),
    );
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(menu, MF_STRING, ID_EXIT, PCWSTR(wide("Exit Dew").as_ptr()));

    let mut point = POINT::default();
    let _ = GetCursorPos(&mut point);

    // FOREGROUND FIRST, or the menu does not dismiss when you click away from it
    // — a documented Win32 quirk of tray menus, and one that looks like a hung
    // program rather than a missing call.
    let _ = SetForegroundWindow(hwnd);

    let _ = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
        point.x,
        point.y,
        None,
        hwnd,
        None,
    );

    let _ = DestroyMenu(menu);
}

pub struct Tray {
    hwnd: HWND,
    icon: NOTIFYICONDATAW,
}

impl Tray {
    /// Install the tray icon. `icon_path` is a `.ico`; a missing one falls back
    /// to the system's application icon rather than to nothing.
    ///
    /// `tooltip` is what hovering the icon says. It comes from the active mod's
    /// manifest, which is the only place `description` is read -- before this the
    /// tip was the literal string "Dew", so every mod's tray icon described the
    /// host rather than the thing running in it.
    pub fn new(icon_path: Option<&std::path::Path>, tooltip: &str) -> Result<Tray, String> {
        unsafe {
            let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
            let class = wide("DewTray");

            let wc = WNDCLASSW {
                lpfnWndProc: Some(tray_proc),
                hInstance: instance.into(),
                lpszClassName: PCWSTR(class.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(class.as_ptr()),
                PCWSTR(wide("Dew").as_ptr()),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                // MESSAGE-ONLY: never shown, never in the taskbar, never painted.
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            )
            .map_err(|e| e.to_string())?;

            let hicon = icon_path
                .and_then(|path| {
                    let wide_path = wide(&path.to_string_lossy());
                    LoadImageW(
                        None,
                        PCWSTR(wide_path.as_ptr()),
                        IMAGE_ICON,
                        0,
                        0,
                        LR_LOADFROMFILE | LR_DEFAULTSIZE,
                    )
                    .ok()
                })
                .map(|h| HICON(h.0))
                .or_else(|| LoadIconW(None, IDI_APPLICATION).ok())
                .unwrap_or_default();

            // 128 WIDE CHARS INCLUDING THE TERMINATOR, which is a Win32 limit
            // and not a choice. `take(127)` leaves the trailing zero the array
            // already has; a description longer than that is truncated rather
            // than refused.
            let mut tip = [0u16; 128];
            for (i, c) in wide(tooltip).into_iter().take(127).enumerate() {
                tip[i] = c;
            }

            let icon = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: TRAY_CALLBACK,
                hIcon: hicon,
                szTip: tip,
                ..Default::default()
            };

            if !Shell_NotifyIconW(NIM_ADD, &icon).as_bool() {
                return Err("could not add the tray icon".into());
            }

            Ok(Tray { hwnd, icon })
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // REMOVE IT EXPLICITLY. Windows keeps a dead tray icon on screen until
        // something makes it repaint, so skipping this leaves a ghost that does
        // nothing when clicked.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.icon);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
