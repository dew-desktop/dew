//! The Win32 implementation.

use crate::{Button, Event, Surface};
use std::cell::RefCell;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EndPaint, GetDC,
    ReleaseDC, ScreenToClient, SelectObject, StretchDIBits, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, PAINTSTRUCT, SRCCOPY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN,
    VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Which surface an event came from.
///
/// THE HWND ITSELF, not an index into a table the shell keeps. An event is
/// produced in the window procedure, where the only identity available is the
/// handle, and anything else would need a lookup that can be stale exactly when
/// it matters: during the teardown of the window that just sent the event.
///
/// Opaque on purpose. The shell compares these and nothing else, so the fact
/// that it is a handle stays inside this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceId(isize);

thread_local! {
    /// Events collected by the window procedure, drained by the pump.
    ///
    /// THREAD-LOCAL RATHER THAN A POINTER IN `GWLP_USERDATA`, because a Win32
    /// window is owned by the thread that created it and its procedure only ever
    /// runs there. A queue per thread is therefore exactly a queue per window
    /// set, with no lifetime to get wrong and no cast from an integer back to a
    /// reference that would be unsound if a message arrived after a drop.
    ///
    /// EVERY ENTRY IS TAGGED. One queue for every window on the thread is the
    /// right structure, and it was the right structure when there was one
    /// window too, but without the tag a click on a widget and a click on its
    /// popover were the same value and neither could be routed.
    static EVENTS: RefCell<Vec<(SurfaceId, Event)>> = const { RefCell::new(Vec::new()) };
}

fn push(hwnd: HWND, event: Event) {
    EVENTS.with(|e| e.borrow_mut().push((SurfaceId(hwnd.0 as isize), event)));
}

fn xy(lparam: LPARAM) -> (f32, f32) {
    // The coordinates are SIGNED 16-bit halves. Reading them as unsigned puts the
    // pointer at ~65000 the moment it leaves the window's left or top edge while
    // a button is held, which is exactly when a drag wants to keep tracking.
    let raw = lparam.0 as u32;
    let x = (raw & 0xFFFF) as i16;
    let y = ((raw >> 16) & 0xFFFF) as i16;
    (x as f32, y as f32)
}

/// A named key for the ones that produce no character.
///
/// Only the keys a text field acts on. Anything else is left to `WM_CHAR`, which
/// already handles layout, dead keys and modifiers correctly — re-deriving a
/// character from a virtual-key code is how a host ends up typing the wrong thing
/// on a non-US keyboard.
fn key_name(vk: u32) -> Option<&'static str> {
    Some(match VIRTUAL_KEY(vk as u16) {
        VK_BACK => "Backspace",
        VK_DELETE => "Delete",
        VK_LEFT => "Left",
        VK_RIGHT => "Right",
        VK_UP => "Up",
        VK_DOWN => "Down",
        VK_HOME => "Home",
        VK_END => "End",
        VK_RETURN => "Return",
        VK_TAB => "Tab",
        VK_ESCAPE => "Escape",
        _ => return None,
    })
}

fn modifier(vk: VIRTUAL_KEY) -> bool {
    // The high bit means "down now"; the low bit is the toggle state and is why
    // testing the whole value treats a released Caps Lock as a held key.
    unsafe { (GetKeyState(vk.0 as i32) as u16 & 0x8000) != 0 }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEMOVE => {
            let (x, y) = xy(lp);
            push(hwnd, Event::PointerMove { x, y });
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            let (x, y) = xy(lp);
            let button = match msg {
                WM_RBUTTONDOWN => Button::Right,
                WM_MBUTTONDOWN => Button::Middle,
                _ => Button::Left,
            };
            // CAPTURE, so a drag that leaves the window still reports its release.
            // Without it a press-drag-out-release leaves the UI stuck held down.
            let _ = SetCapture(hwnd);
            push(hwnd, Event::PointerDown { x, y, button });
            LRESULT(0)
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
            let (x, y) = xy(lp);
            let button = match msg {
                WM_RBUTTONUP => Button::Right,
                WM_MBUTTONUP => Button::Middle,
                _ => Button::Left,
            };
            let _ = ReleaseCapture();
            push(hwnd, Event::PointerUp { x, y, button });
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // WM_MOUSEWHEEL CARRIES SCREEN COORDINATES, unlike every other mouse
            // message. Forwarding them unconverted scrolls whatever happens to be
            // under that point in the wrong space — and on a window near the
            // bottom-right of a large display, that is nothing at all.
            let (sx, sy) = xy(lp);
            let mut point = windows::Win32::Foundation::POINT {
                x: sx as i32,
                y: sy as i32,
            };
            let _ = ScreenToClient(hwnd, &mut point);
            let delta = ((wp.0 >> 16) as i16) as f32 / 120.0;
            push(
                hwnd,
                Event::Wheel {
                    x: point.x as f32,
                    y: point.y as f32,
                    delta,
                },
            );
            LRESULT(0)
        }
        WM_CHAR => {
            if let Some(c) = char::from_u32(wp.0 as u32) {
                // Control codes arrive here too; Backspace and Return are already
                // reported as named keys by WM_KEYDOWN, and passing them again as
                // characters would apply each twice.
                if !c.is_control() {
                    push(hwnd, Event::Char(c));
                }
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if let Some(name) = key_name(wp.0 as u32) {
                push(
                    hwnd,
                    Event::Key {
                        name: name.to_string(),
                        shift: modifier(VK_SHIFT),
                        ctrl: modifier(VK_CONTROL),
                    },
                );
            }
            LRESULT(0)
        }
        WM_SIZE => {
            let raw = lp.0 as u32;
            let width = (raw & 0xFFFF) as u32;
            let height = ((raw >> 16) & 0xFFFF) as u32;
            if width > 0 && height > 0 {
                push(hwnd, Event::Resized { width, height });
            }
            LRESULT(0)
        }
        WM_PAINT => {
            // VALIDATE THE REGION, or Windows re-posts WM_PAINT forever and the
            // pump spins at 100% doing nothing. The frame loop repaints anyway;
            // this only records that the surface can no longer be patched.
            let mut ps = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
            push(hwnd, Event::Exposed);
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            // CLAIM IT, so Windows does not clear to the class brush before every
            // paint. Left to the default, the window flashes its background
            // between frames.
            LRESULT(1)
        }
        WM_CLOSE => {
            push(hwnd, Event::CloseRequested);
            LRESULT(0)
        }
        // NO `PostQuitMessage` HERE. It posts `WM_QUIT`, which stops the pump for
        // the whole thread, so with more than one surface open the first one to
        // close took the process with it. A tooltip is a popover that closes
        // every time the pointer leaves, which would have made this look like a
        // random crash rather than a lifetime rule.
        //
        // Whether Dew should exit is the shell's call: "the last surface closed"
        // and "this applet is finished" are different claims, and only the shell
        // knows either.
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// The primary display's size in pixels.
///
/// `SM_CXSCREEN` is the PRIMARY monitor, not the virtual desktop spanning all of
/// them. Proper multi-monitor placement is a real feature and this is not it —
/// but a widget anchored to the primary display's corner is right far more often
/// than one anchored to the bounding box of every display, which on a
/// two-monitor setup puts it off the edge of the one you are looking at.
pub fn screen_size() -> (i32, i32) {
    unsafe {
        let (w, h) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        if w > 0 && h > 0 {
            (w, h)
        } else {
            // A headless or remote session can report zero. A plausible desktop
            // beats a widget positioned at the origin of a screen of size zero.
            (1920, 1080)
        }
    }
}

/// The thread's message queue, drained once for every window on it.
///
/// ONE PUMP, NOT ONE PER WINDOW. `PeekMessageW` with a null window takes
/// messages for every window the thread owns, so a `poll` that hung off one
/// window was already consuming the others': whichever was polled first ate the
/// rest's input, and with a single window that could never be observed.
///
/// ## Events, not callbacks, still
///
/// The pump drains into a `Vec` and hands it back rather than dispatching
/// through a closure, for the reason the crate docs give: the guest must not run
/// inside `DispatchMessage`. That is also what makes a surface created from
/// inside a guest call safe, because the guest is on the shell's stack and not
/// on the OS's.
#[derive(Default)]
pub struct Pump {
    _private: (),
}

impl Pump {
    pub fn new() -> Pump {
        Pump { _private: () }
    }

    /// Drain every event the thread's windows have seen since the last call.
    ///
    /// Each event says which surface produced it. Returns `None` only on
    /// `WM_QUIT`, which nothing in this crate posts any more; it is left
    /// honoured so an outside request to end the thread still ends it.
    pub fn poll(&mut self) -> Option<Vec<(SurfaceId, Event)>> {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return None;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        Some(EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut())))
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct Window {
    hwnd: HWND,
    width: u32,
    height: u32,
    layered: bool,
}

impl Window {
    pub fn new(surface: &Surface, width: u32, height: u32) -> Result<Window, String> {
        unsafe {
            let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
            let class = wide("AetherWindow");

            // Registering twice returns an error that is not one — a second window
            // of the same class is fine and the class is already there. The
            // result is deliberately discarded rather than checked.
            let layered = matches!(surface, Surface::Widget { .. } | Surface::Overlay { .. });

            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                lpszClassName: PCWSTR(class.as_ptr()),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            // SIZE THE CLIENT AREA, not the window. `CreateWindowEx` takes the
            // outer rectangle, so passing the wanted size directly yields a client
            // area smaller by the border and caption — and every frame then
            // renders into a surface that does not match the window.
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            // BOUND TO A LOCAL, not written inline. `PCWSTR(wide(title).as_ptr())`
            // drops the Vec at the end of the enclosing expression, so the window
            // is created from a pointer into freed memory — which usually
            // "works", occasionally shows a garbage title, and is undefined
            // behaviour every time.
            let title_w = wide(match surface {
                Surface::Window { title } => title.as_str(),
                Surface::Widget { .. } | Surface::Overlay { .. } => "",
            });

            // A WIDGET IS BORDERLESS, TOPMOST, AND OUT OF THE TASKBAR.
            //
            // `WS_EX_TOOLWINDOW` is the one that keeps it out of Alt-Tab and the
            // taskbar; without it a desktop clock is a window you can tab to,
            // which is not what a widget is. `WS_EX_LAYERED` is what makes
            // `UpdateLayeredWindow` available, and therefore per-pixel alpha.
            let (style, ex_style, x, y) = match surface {
                Surface::Window { .. } => (
                    WS_OVERLAPPEDWINDOW,
                    WINDOW_EX_STYLE::default(),
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                ),
                Surface::Widget {
                    x,
                    y,
                    click_through,
                } => {
                    let mut ex = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
                    if *click_through {
                        // TRANSPARENT means hit-testing falls through to whatever
                        // is behind. It is a property of the window, not of the
                        // painting, so a widget can be fully opaque and still be
                        // clicked through.
                        ex |= WS_EX_TRANSPARENT;
                    }
                    (WS_POPUP, ex, *x, *y)
                }
                Surface::Overlay {
                    topmost,
                    click_through,
                } => {
                    let mut ex = WS_EX_LAYERED | WS_EX_TOOLWINDOW;
                    if *topmost {
                        ex |= WS_EX_TOPMOST;
                    }
                    if *click_through {
                        ex |= WS_EX_TRANSPARENT;
                    }
                    // The caller sized this to the screen; it starts at its origin.
                    (WS_POPUP, ex, 0, 0)
                }
            };

            // Only an ordinary window has chrome to account for. A popup's
            // outer rectangle IS its client area, and adjusting one would make
            // the widget larger than the surface it presents.
            if !layered {
                let _ = AdjustWindowRect(&mut rect, style, false);
            }

            let hwnd = CreateWindowExW(
                ex_style,
                PCWSTR(class.as_ptr()),
                PCWSTR(title_w.as_ptr()),
                style,
                x,
                y,
                rect.right - rect.left,
                rect.bottom - rect.top,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .map_err(|e| e.to_string())?;

            let _ = ShowWindow(hwnd, SW_SHOW);

            Ok(Window {
                hwnd,
                width,
                height,
                layered,
            })
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Which surface this is, for matching against a polled event.
    pub fn id(&self) -> SurfaceId {
        SurfaceId(self.hwnd.0 as isize)
    }

    /// Take a size this window was told it now has.
    ///
    /// THE SHELL APPLIES THIS, because the shell is what reads the event. The
    /// window used to notice its own `Resized` while draining its own queue;
    /// once draining belongs to the pump, a window that kept updating itself
    /// would be reading another window's resize as its own.
    pub fn resized(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Put a BGRA buffer on screen, whichever kind of surface this is.
    ///
    /// A widget takes the layered path, where the buffer's ALPHA becomes the
    /// window's shape; an ordinary window takes the blit, where it is ignored.
    /// The caller does not choose — it painted a frame, and how that reaches the
    /// screen is a property of the window it asked for.
    pub fn present(&self, bgra: &[u8], width: u32, height: u32) {
        if self.layered {
            self.present_layered(bgra, width, height);
        } else {
            self.blit(bgra, width, height);
        }
    }

    /// Composite a premultiplied BGRA buffer as the window itself.
    ///
    /// `UpdateLayeredWindow` takes the bitmap AND the window's size and position
    /// in one call — the window has no client area being painted into, it simply
    /// IS this bitmap. That is what makes the alpha real: a pixel with alpha 0 is
    /// not a transparent pixel drawn over a background, it is a pixel the window
    /// does not occupy, and the desktop behind it is what shows and what receives
    /// the click.
    ///
    /// PREMULTIPLIED is required, not preferred: `AC_SRC_ALPHA` says the colour
    /// channels are already scaled by alpha. `dew_raster` produces exactly
    /// that, so nothing converts on the way.
    fn present_layered(&self, bgra: &[u8], width: u32, height: u32) {
        if bgra.len() < (width * height * 4) as usize {
            return;
        }
        unsafe {
            let screen = GetDC(None);
            if screen.is_invalid() {
                return;
            }
            let mem = CreateCompatibleDC(Some(screen));
            if mem.is_invalid() {
                ReleaseDC(None, screen);
                return;
            }

            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    // Negative for top-down, as in the blit path.
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let bitmap: HBITMAP =
                match CreateDIBSection(Some(mem), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
                    Ok(b) if !bits.is_null() => b,
                    _ => {
                        let _ = DeleteDC(mem);
                        ReleaseDC(None, screen);
                        return;
                    }
                };

            std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
            let old = SelectObject(mem, bitmap.into());

            let mut size = SIZE {
                cx: width as i32,
                cy: height as i32,
            };
            let mut src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let _ = UpdateLayeredWindow(
                self.hwnd,
                Some(screen),
                None,
                Some(&mut size),
                Some(mem),
                Some(&mut src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            SelectObject(mem, old);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
        }
    }

    /// Put a BGRA buffer on screen. `bgra` must be `width * height * 4` bytes.
    fn blit(&self, bgra: &[u8], width: u32, height: u32) {
        if bgra.len() < (width * height * 4) as usize {
            return;
        }
        unsafe {
            let hdc: HDC = GetDC(Some(self.hwnd));
            if hdc.is_invalid() {
                return;
            }

            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    // NEGATIVE, for a TOP-DOWN bitmap. A DIB is bottom-up by
                    // default, so a positive height presents every frame flipped
                    // vertically — which reads as a renderer bug and is a header
                    // field.
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            StretchDIBits(
                hdc,
                0,
                0,
                self.width as i32,
                self.height as i32,
                0,
                0,
                width as i32,
                height as i32,
                Some(bgra.as_ptr() as *const _),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );

            ReleaseDC(Some(self.hwnd), hdc);
        }
    }
}

impl Drop for Window {
    /// A SURFACE THAT GOES OUT OF SCOPE LEAVES THE SCREEN. This was here before
    /// anything needed it, when the process exited with its only window. It
    /// carries real weight now: a popover is dropped every time it closes, and
    /// one that stayed on screen would be a leak the user can see.
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Surface;

    /// A widget surface parked offscreen, so a test never flashes at the user.
    fn offscreen() -> Surface {
        Surface::Widget {
            x: -4000,
            y: -4000,
            click_through: false,
        }
    }

    /// Post a mouse move to a window without touching the real cursor.
    ///
    /// `PostMessageW` RATHER THAN `SendInput`, because the test is about routing
    /// and not about the platform: a synthesised real click would go to whatever
    /// is under the pointer on the developer's desktop.
    fn post_move(window: &Window, x: i16, y: i16) {
        let lparam = LPARAM(((y as u16 as u32) << 16 | (x as u16 as u32)) as isize);
        unsafe {
            let _ = PostMessageW(Some(window.hwnd), WM_MOUSEMOVE, WPARAM(0), lparam);
        }
    }

    /// An event says which surface it came from.
    ///
    /// THE WHOLE POINT OF THE TAG. With one window this was unobservable, which
    /// is why the queue went untagged for as long as it did; with two, an
    /// untagged queue routes a click on a popover to the widget behind it.
    #[test]
    fn two_surfaces_do_not_share_their_events() {
        let mut pump = Pump::new();
        let first = Window::new(&offscreen(), 100, 100).expect("first window");
        let second = Window::new(&offscreen(), 100, 100).expect("second window");
        assert_ne!(first.id(), second.id(), "two windows, two ids");

        // Anything the creation itself produced is not what is under test.
        let _ = pump.poll();

        post_move(&first, 11, 12);
        post_move(&second, 21, 22);

        let events = pump.poll().expect("the pump is still running");

        let for_first: Vec<_> = events
            .iter()
            .filter(|(id, _)| *id == first.id())
            .map(|(_, e)| e.clone())
            .collect();
        let for_second: Vec<_> = events
            .iter()
            .filter(|(id, _)| *id == second.id())
            .map(|(_, e)| e.clone())
            .collect();

        assert!(
            for_first.contains(&Event::PointerMove { x: 11.0, y: 12.0 }),
            "the first surface should have seen its own move, saw {for_first:?}"
        );
        assert!(
            for_second.contains(&Event::PointerMove { x: 21.0, y: 22.0 }),
            "the second surface should have seen its own move, saw {for_second:?}"
        );
        assert!(
            !for_first.contains(&Event::PointerMove { x: 21.0, y: 22.0 }),
            "the first surface received an event meant for the second"
        );
    }

    /// Closing one surface leaves the pump running and the others alive.
    ///
    /// THE BUG THIS PINS: `WM_DESTROY` posted `WM_QUIT`, which ends the thread's
    /// message loop, so the first surface to close took every other surface with
    /// it. A popover closes every time the pointer leaves it, so this would have
    /// presented as Dew exiting when a tooltip went away.
    #[test]
    fn closing_one_surface_does_not_stop_the_pump() {
        let mut pump = Pump::new();
        let first = Window::new(&offscreen(), 100, 100).expect("first window");
        let second = Window::new(&offscreen(), 100, 100).expect("second window");
        let _ = pump.poll();

        drop(first);

        // The destroy has to be dispatched before its consequences are visible,
        // and `WM_QUIT` (if one were posted) arrives in this same drain.
        let after_close = pump.poll();
        assert!(
            after_close.is_some(),
            "destroying one surface stopped the thread's pump"
        );

        post_move(&second, 33, 44);
        let events = pump.poll().expect("the pump is still running");
        assert!(
            events
                .iter()
                .any(|(id, e)| *id == second.id() && *e == Event::PointerMove { x: 33.0, y: 44.0 }),
            "the surviving surface stopped receiving input, saw {events:?}"
        );
    }
}
