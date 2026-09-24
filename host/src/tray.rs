//! Dew's tray icon and its menu.
//!
//! A MESSAGE-ONLY WINDOW OF OUR OWN, not the widget's. A tray icon delivers its
//! clicks to a window procedure, and `dew_window` owns the widget's — it is a
//! surface for painting Aether, and giving it a second job would put Dew's
//! furniture inside the crate that must stay useful to hosts which have none.
//!
//! `HWND_MESSAGE` creates a window that is never shown, never in the taskbar, and
//! exists only to receive. The coordinator's own `poll()` pumps every message for
//! its thread, so this window's procedure runs there without a second pump — see
//! `coordinator.rs` for why the tray moved to a thread of its own rather than
//! staying on whichever applet happened to be first.
//!
//! WHAT IS TEMPORARY HERE, and should be said out loud: the menu is a frame-rate
//! cap, the loaded applets, and an exit. It is scaffolding for reaching settings
//! that do not have a UI yet, and the moment Dew has a settings surface most of
//! this should move into it — a tray menu is a bad place to configure anything
//! you cannot see the effect of.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
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

/// What the menu currently lists as loaded, set by the coordinator whenever its
/// registry changes.
///
/// A `Mutex<Vec<_>>` RATHER THAN SOMETHING RICHER, because nothing here reads it
/// except `show_menu`, built fresh on every click — there is no state to keep in
/// step between updates, only the latest snapshot.
static APPLETS: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());

/// Which applet id a menu command index named, filled in by the same `show_menu`
/// call that assigned the ids. `WM_COMMAND` only ever hands back a `usize`, so
/// this is what turns "the third applet item was clicked" back into an id the
/// coordinator's registry actually uses.
static MENU_APPLET_ORDER: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// Applet ids the menu's "Unload" asked for, drained once per coordinator
/// iteration by `take_unload_requests`.
static UNLOAD_QUEUE: Mutex<Vec<u32>> = Mutex::new(Vec::new());

const TRAY_CALLBACK: u32 = WM_APP + 1;
const ID_EXIT: usize = 1000;
const ID_MANAGE: usize = 1001;
const ID_ABOUT: usize = 1002;
const ID_DASHBOARD: usize = 1003;
/// Where the per-applet unload entries start. Clear of `CAPS` (2000-2005) and
/// `ID_EXIT`/`ID_MANAGE`, with room for far more loaded applets than the menu
/// could ever show usefully before the low word of `WM_COMMAND`'s `wParam`
/// runs out.
const ID_APPLET_BASE: usize = 3000;

/// The offered caps. `None` is uncapped and is the default.
///
/// UNCAPPED NO LONGER MEANS "DO NOT SLEEP AT ALL." It used to, and an idle
/// widget spun a whole core doing nothing -- `Renderer::frame` returns false
/// with nothing to paint, and an applet polling `desktop.Clock.OnFrame` (see
/// examples/host/widget-behaviors) re-ran that poll again immediately, as
/// fast as the CPU could manage. The frame loop's own `None` arm now calls
/// `DwmFlush` instead of skipping the wait entirely: uncapped means "as fast
/// as the desktop compositor can actually show a new frame," the same ceiling
/// a real swap-chain present would impose, not "as fast as the CPU can spin."
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

/// Tell the tray what is currently loaded, by id and display name.
///
/// CALLED WHENEVER THE COORDINATOR'S REGISTRY CHANGES -- an applet joining,
/// leaving, or getting its display name once its manifest has loaded. The menu
/// itself is only ever built at click time, so this just replaces the snapshot
/// it will read next.
pub fn set_applets(applets: Vec<(u32, String)>) {
    *APPLETS.lock().expect("tray applets") = applets;
}

/// Applet ids the menu's "Unload" asked for since the last call. Drained, not
/// peeked -- the coordinator's loop calls this once per iteration and acts on
/// whatever it finds.
pub fn take_unload_requests() -> Vec<u32> {
    std::mem::take(&mut *UNLOAD_QUEUE.lock().expect("tray unload queue"))
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
            } else if id == ID_MANAGE {
                crate::manage::open_or_focus();
            } else if id == ID_DASHBOARD {
                crate::dashboard::open_or_focus();
            } else if id == ID_ABOUT {
                show_about();
            } else if let Some((_, _, us)) = CAPS.iter().find(|(cap_id, _, _)| *cap_id == id) {
                FRAME_BUDGET_US.store(*us, Ordering::Relaxed);
            } else if id >= ID_APPLET_BASE {
                let order = MENU_APPLET_ORDER.lock().expect("tray menu order");
                if let Some(&applet_id) = order.get(id - ID_APPLET_BASE) {
                    UNLOAD_QUEUE
                        .lock()
                        .expect("tray unload queue")
                        .push(applet_id);
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// The About box's body text. Split out from `show_about` so it can be
/// asserted on directly -- there is no way to click-drive a real
/// `MessageBoxW` from a test, but there is no reason the string that goes
/// into one should be untestable along with it.
fn about_text() -> String {
    format!("Dew {}", crate::BUILD_IDENTIFIER)
}

/// Shows the build identifier in a message box.
///
/// ITS OWN THREAD, THE SAME PATTERN `manage::open_or_focus` USES. `tray_proc`
/// runs on the coordinator's own thread, which its `poll()` also uses to
/// drive every loaded applet's frame loop -- `MessageBoxW` is modal and does
/// not return until dismissed, so calling it here directly would freeze
/// every applet's rendering for as long as the box stays open, not just the
/// tray menu.
fn show_about() {
    std::thread::spawn(|| unsafe {
        let title = wide("About Dew");
        let text = wide(&about_text());
        MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_OK);
    });
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

    let _ = AppendMenuW(
        menu,
        MF_STRING,
        ID_MANAGE,
        PCWSTR(wide("Manage applets").as_ptr()),
    );

    // NATIVE, ALONGSIDE THE WIN32 ONE ABOVE, not in place of it (milestone
    // 23). `manage.rs`'s window keeps sign-in and the installed-applet
    // list until the native surface has actually been used for a while.
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        ID_DASHBOARD,
        PCWSTR(wide("Dashboard (preview)").as_ptr()),
    );

    let _ = AppendMenuW(
        menu,
        MF_STRING,
        ID_ABOUT,
        PCWSTR(wide("About Dew").as_ptr()),
    );

    // ONE ENTRY PER LOADED APPLET, EACH ITS OWN UNLOAD -- a quick unload for
    // whatever is already running, alongside the fuller install/enable view
    // `Manage applets` opens.
    let applets = APPLETS.lock().expect("tray applets").clone();
    if !applets.is_empty() {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let mut order = Vec::with_capacity(applets.len());
        for (i, (id, name)) in applets.iter().enumerate() {
            let label = format!("Unload {name}");
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                ID_APPLET_BASE + i,
                PCWSTR(wide(&label).as_ptr()),
            );
            order.push(*id);
        }
        *MENU_APPLET_ORDER.lock().expect("tray menu order") = order;
    }

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
    /// Install the tray icon. The icon is resource id 1, compiled into
    /// `dew.exe` itself by `build.rs` (see `embed-resource` in
    /// `host/Cargo.toml`): a real downloaded release attaches only the
    /// `.exe`, so an icon sourced from a file beside it was never
    /// actually reachable outside a full source checkout. Falls back to
    /// the system's application icon if the embedded one somehow fails
    /// to load, rather than to nothing.
    ///
    /// `tooltip` is what hovering the icon says. One tray now covers every
    /// applet the coordinator has loaded, not one mod each, so this is a
    /// fixed string rather than any one mod's own description.
    pub fn new(tooltip: &str) -> Result<Tray, String> {
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

            // `1 as *const u16` IS Win32's `MAKEINTRESOURCE(1)`: an integer
            // resource id smuggled through a pointer parameter, matching
            // resource id 1 in assets/dew.rc, not a real dangling pointer.
            // `ptr::dangling` would not preserve that exact bit pattern.
            #[allow(clippy::manual_dangling_ptr)]
            let hicon = LoadIconW(Some(instance.into()), PCWSTR(1 as *const u16))
                .or_else(|_| LoadIconW(None, IDI_APPLICATION))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_about_text_carries_the_real_build_identifier() {
        assert_eq!(about_text(), format!("Dew {}", crate::BUILD_IDENTIFIER));
        // Four dot-separated segments (CARGO_PKG_VERSION's three plus the
        // build number), all numeric -- the dotted, Roblox-shaped form, not
        // the old `+g<sha>` one.
        let segments: Vec<&str> = crate::BUILD_IDENTIFIER.split('.').collect();
        assert_eq!(segments.len(), 4, "{}", about_text());
        assert!(
            segments.iter().all(|s| s.parse::<u64>().is_ok()),
            "{}",
            about_text()
        );
    }
}
