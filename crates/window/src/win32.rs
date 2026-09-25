//! The Win32 implementation.

use crate::gpu::Presenter;
use crate::{Button, Event, Surface, ZOrder};
use std::cell::RefCell;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EndPaint, GetDC,
    ReleaseDC, ScreenToClient, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, PAINTSTRUCT,
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

type LiveResizeHook = Box<dyn FnMut(u32, u32)>;

thread_local! {
    /// A repaint to run synchronously from inside `WM_NCCALCSIZE`, during a
    /// live border-drag resize.
    ///
    /// WHY THIS EXISTS, AND WHY `EVENTS` ABOVE IS NOT ENOUGH: grabbing a
    /// window's border and dragging it enters a modal loop inside
    /// `DefWindowProcW`'s own handling of the resize hit-test, and that
    /// loop does not return to whoever called `DispatchMessageW` until the
    /// drag ends -- which means `Pump::poll`'s own loop, and therefore
    /// draining `EVENTS`, is blocked for the whole drag. Windows keeps
    /// sending real resize messages to this window procedure throughout
    /// that time regardless, synchronously, nested inside the call that
    /// never returned; a repaint that only happens when `Event::Resized`
    /// is drained later is a repaint that happens once, when the mouse
    /// comes up, not while the drag is happening.
    ///
    /// `WM_NCCALCSIZE`, NOT `WM_SIZE` -- see the `wndproc` match arm's own
    /// comment for why triggering from `WM_SIZE` left the content one
    /// frame behind the border on a fast drag.
    ///
    /// ONE HOOK PER THREAD, matching `EVENTS`: one applet runs one window
    /// on one thread, so there is exactly one hook to call.
    static LIVE_RESIZE: RefCell<Option<LiveResizeHook>> = const { RefCell::new(None) };
}

/// Install the repaint `WM_NCCALCSIZE` calls synchronously during a live resize.
///
/// SEPARATE FROM THE EVENT QUEUE ON PURPOSE. This still fires beside a
/// queued `Event::Resized`, not instead of it -- the queued one is what
/// keeps the caller's own tracked size and layout correct once the pump
/// resumes, and this one is only for what appears on screen while it does
/// not.
pub fn set_live_resize_hook(hook: impl FnMut(u32, u32) + 'static) {
    LIVE_RESIZE.with(|cell| *cell.borrow_mut() = Some(Box::new(hook)));
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
        WM_NCCALCSIZE => {
            // THE LIVE REPAINT FIRES HERE, NOT FROM `WM_SIZE` BELOW. Both
            // messages arrive synchronously inside the same live-drag modal
            // loop, but `WM_NCCALCSIZE` is sent FIRST, to compute the new
            // client rect BEFORE Windows visually moves the border to match
            // it -- `WM_SIZE` arrives one step later, after the border has
            // already moved. A repaint triggered from `WM_SIZE` is therefore
            // always a frame behind the border during a fast drag: the
            // content the user sees was rendered for a size the border has
            // already left behind, which reads as content sliding relative
            // to the frame rather than tracking it. Repainting from
            // `WM_NCCALCSIZE` instead closes that gap by rendering for the
            // size the border is ABOUT to have, not the one it just had.
            let result = DefWindowProcW(hwnd, msg, wp, lp);
            if wp.0 != 0 {
                // `wp` NONZERO means `lp` is an `NCCALCSIZE_PARAMS*`, whose
                // `rgrc[0]` the call above already rewrote in place from
                // "proposed new window rect" to "resulting new client
                // rect" -- exactly the size a repaint needs, computed
                // without hand-rolling the border/caption math `AdjustWindowRect`
                // already owns elsewhere.
                let params = &*(lp.0 as *const NCCALCSIZE_PARAMS);
                let rect = params.rgrc[0];
                let width = (rect.right - rect.left).max(0) as u32;
                let height = (rect.bottom - rect.top).max(0) as u32;
                if width > 0 && height > 0 {
                    LIVE_RESIZE.with(|cell| {
                        if let Some(hook) = cell.borrow_mut().as_mut() {
                            hook(width, height);
                        }
                    });
                }
            }
            result
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
    /// `true` for a surface that presents through `present_layered`'s
    /// `UpdateLayeredWindow` path instead of `presenter`'s `DirectComposition`
    /// one.
    ///
    /// NOT "every `Widget`/`Overlay`", the way milestone 24 sprint 2 first
    /// drew this line. `DirectComposition` genuinely has no equivalent of
    /// `UpdateLayeredWindow`'s native, per-pixel alpha-based hit-testing --
    /// measured with `WindowFromPoint` and then with real clicks, not
    /// assumed: `WS_EX_TRANSPARENT` plus `WM_NCHITTEST` returning
    /// `HTTRANSPARENT` does not forward a click on a
    /// `WS_EX_NOREDIRECTIONBITMAP` window, only *reports* that it should via
    /// `WindowFromPoint`, which is a documented, separate gap. So this is
    /// `true` for `Overlay` always (its whole identity is clicks falling
    /// through wherever it did not paint, which only `UpdateLayeredWindow`'s
    /// own hit-testing provides) and for a `Widget` that asked for
    /// `click_through` specifically -- a `Widget` that did not ask for it
    /// stays on `presenter`, since plain visual transparency (soft edges,
    /// rounded corners) is confirmed working there and is not the part that
    /// broke.
    layered: bool,
    presenter: Option<Presenter>,
}

impl Window {
    pub fn new(surface: &Surface, width: u32, height: u32) -> Result<Window, String> {
        unsafe {
            let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
            let class = wide("AetherWindow");

            // Registering twice returns an error that is not one — a second window
            // of the same class is fine and the class is already there. The
            // result is deliberately discarded rather than checked.
            //
            // GEOMETRY ONLY -- a popup's outer rectangle IS its client area
            // regardless of which presentation mechanism it ends up on, so
            // this stays a plain surface-kind check. Which mechanism it
            // actually gets is `layered`, computed after the match below,
            // once each surface kind's own `click_through` is in scope.
            let is_popup = matches!(surface, Surface::Widget { .. } | Surface::Overlay { .. });

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
            // `WS_EX_TOOLWINDOW` is the one that keeps it out of Alt-Tab and
            // the taskbar; without it a desktop clock is a window you can tab
            // to, which is not what a widget is.
            //
            // `WS_EX_LAYERED` OR `WS_EX_NOREDIRECTIONBITMAP`, NOT ALWAYS THE
            // SAME ONE -- see `Window::layered`'s own doc for why an
            // `Overlay` always needs the former and a `Widget` only needs it
            // when it asked for `click_through`. Getting this wrong the other
            // way (`WS_EX_NOREDIRECTIONBITMAP` on a surface that needed real
            // click-through) is the bug this match now exists to not repeat.
            let (style, ex_style, x, y, z_order, layered) = match surface {
                // `WS_EX_NOREDIRECTIONBITMAP`: an ordinary window presents
                // through `DirectComposition` (`crate::gpu`), not a blit
                // into the window's own device context. Without this style
                // Windows still allocates the normal GDI redirection
                // surface behind the composition visual, which is the
                // exact per-HWND surface a flip-model swap chain bound to
                // it would fight over during a live resize.
                Surface::Window { .. } => (
                    WS_OVERLAPPEDWINDOW,
                    WS_EX_NOREDIRECTIONBITMAP,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    None,
                    false,
                ),
                Surface::Widget {
                    x,
                    y,
                    click_through,
                    z_order,
                } => {
                    let mut ex = WS_EX_TOOLWINDOW;
                    ex |= if *click_through {
                        WS_EX_LAYERED
                    } else {
                        WS_EX_NOREDIRECTIONBITMAP
                    };
                    // `WS_EX_TOPMOST` AT CREATION MATCHES THE COMMON CASE, and
                    // `SetWindowPos` below is what actually enforces all three
                    // tiers — this extended style alone has no way to express
                    // "bottom".
                    if *z_order == ZOrder::Topmost {
                        ex |= WS_EX_TOPMOST;
                    }
                    if *click_through {
                        // TRANSPARENT means hit-testing falls through to whatever
                        // is behind. It is a property of the window, not of the
                        // painting, so a widget can be fully opaque and still be
                        // clicked through -- but only paired with `WS_EX_LAYERED`
                        // above, which is what makes the hit-test actually skip
                        // it rather than merely reporting that it would.
                        ex |= WS_EX_TRANSPARENT;
                    }
                    (WS_POPUP, ex, *x, *y, Some(*z_order), *click_through)
                }
                Surface::Overlay {
                    z_order,
                    click_through,
                } => {
                    // ALWAYS `WS_EX_LAYERED`, REGARDLESS OF `click_through` --
                    // an overlay's own default (clicks fall through wherever
                    // it did not paint) IS `UpdateLayeredWindow`'s native
                    // alpha hit-testing, not something its `click_through`
                    // flag turns on. See `Window::layered`'s own doc.
                    let mut ex = WS_EX_LAYERED | WS_EX_TOOLWINDOW;
                    if *z_order == ZOrder::Topmost {
                        ex |= WS_EX_TOPMOST;
                    }
                    if *click_through {
                        ex |= WS_EX_TRANSPARENT;
                    }
                    // The caller sized this to the screen; it starts at its origin.
                    (WS_POPUP, ex, 0, 0, Some(*z_order), true)
                }
            };

            // Only an ordinary window has chrome to account for. A popup's
            // outer rectangle IS its client area, and adjusting one would make
            // the widget larger than the surface it presents.
            if !is_popup {
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

            // THE EXTENDED STYLE ALONE CANNOT PLACE A WINDOW AT THE BOTTOM of
            // the z-order — `WS_EX_TOPMOST` only ever says "above everything
            // else". `SetWindowPos`'s `hwndInsertAfter` is the one mechanism
            // that reaches all three tiers, so it runs for every widget and
            // overlay rather than only the topmost ones. `SetWindowPos` works
            // on a hidden window, so this does not need `ShowWindow` first.
            if let Some(z_order) = z_order {
                let insert_after = match z_order {
                    ZOrder::Bottom => HWND_BOTTOM,
                    ZOrder::Normal => HWND_NOTOPMOST,
                    ZOrder::Topmost => HWND_TOPMOST,
                };
                let _ = SetWindowPos(
                    hwnd,
                    Some(insert_after),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }

            // `None` FOR A `layered` SURFACE, which never touches `Presenter`
            // at all -- `present_layered` composites through
            // `UpdateLayeredWindow` against the screen's own DC instead. A
            // non-click-through `Widget` still passes `transparent: true`
            // here: it needs `Presenter`'s premultiplied alpha for its own
            // soft edges and rounded corners, just not a backdrop, which is
            // exactly what `transparent` already skips.
            let presenter = if layered {
                None
            } else {
                Some(Presenter::new(hwnd, width, height, is_popup)?)
            };

            // SHOWN ONLY NOW, AFTER THE PRESENTER (WHEN THERE IS ONE) EXISTS
            // AND HAS COMMITTED ITS FIRST FRAME. `Presenter::new` creates a
            // `wgpu` adapter and device and sets up `DirectComposition`'s own
            // device, target and visual tree -- real, measured, one-time
            // setup cost. A window shown before any of that finishes has no
            // composition content at all yet, `WS_EX_NOREDIRECTIONBITMAP`
            // having opted it out of the ordinary redirection surface that
            // would otherwise paper over the gap -- which is what showed up
            // as a window that appears see-through for however long setup
            // took, every time, on every open.
            let _ = ShowWindow(hwnd, SW_SHOW);

            Ok(Window {
                hwnd,
                width,
                height,
                layered,
                presenter,
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

    /// Move an already-created window to a new screen position, without
    /// touching its size, z-order or focus.
    ///
    /// THE ONLY NEW CAPABILITY THIS CRATE NEEDS for dragging. Threshold
    /// detection, clamping to the screen and snapping to its edges are all
    /// policy, and belong in the shell that drives the pump rather than in this
    /// thin platform layer.
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
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
        if let Some(presenter) = self.presenter.as_mut() {
            presenter.configure(width, height);
        }
    }

    /// Put a BGRA buffer on screen, whichever kind of surface this is.
    ///
    /// A `layered` surface takes `present_layered`, where the buffer's ALPHA
    /// becomes the window's shape AND its hit-test -- the only mechanism
    /// Windows actually provides for that combination, see `Window::layered`'s
    /// own doc for why `DirectComposition` does not. Everything else
    /// presents through `Presenter`.
    pub fn present(&mut self, bgra: &[u8], width: u32, height: u32) {
        if self.layered {
            self.present_layered(bgra, width, height);
        } else if let Some(presenter) = self.presenter.as_mut() {
            presenter.present(bgra, width, height);
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
    use crate::{Surface, ZOrder};

    /// A widget surface parked offscreen, so a test never flashes at the user.
    fn offscreen() -> Surface {
        Surface::Widget {
            x: -4000,
            y: -4000,
            click_through: false,
            z_order: ZOrder::Topmost,
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
