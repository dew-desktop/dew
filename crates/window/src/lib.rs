//! A native window, a message pump, and a blit.
//!
//! What a shell needs from a platform and nothing more. It reports input as
//! plain values and takes pixels as a byte slice, so it knows nothing about
//! Aether, Luau, or which rasteriser drew the frame.
//!
//! ## Events, not callbacks
//!
//! [`Pump::poll`] drains the queue into a `Vec<Event>` rather than dispatching
//! through a closure. A Win32 window procedure runs on the OS's stack, inside
//! `DispatchMessage`, and calling into a Luau VM from there means the guest can
//! re-enter the pump — which is how a nested modal loop ends up stepping the
//! clock twice for one frame. Draining first keeps the guest on our stack, where
//! the shell decides when it runs. It is also what lets a guest ask for a new
//! surface mid-run: the call arrives on the shell's stack, not the OS's.
//!
//! ## One pump, many surfaces
//!
//! The pump belongs to the thread, not to a window. Every event it returns says
//! which [`SurfaceId`] produced it, because a shell showing a widget and its
//! popover has to know which tree a click was meant for.

#![cfg(windows)]

mod gpu;
mod win32;

pub use win32::{screen_size, set_live_resize_hook, Pump, SurfaceId, Window};

/// A window's tier in the desktop's z-order.
///
/// THREE VALUES, NOT A BOOL. `Bottom` sits below every normal window and above
/// the wallpaper — a Rainmeter-style desktop widget — but is NOT the same as
/// Rainmeter's "OnDesktop", which parents into the desktop's `WorkerW` and is
/// out of scope here. `Normal` behaves like an ordinary window's z-order and can
/// be covered. `Topmost` is always on top, which was the only behaviour before
/// this enum existed and stays the default so an existing widget's behaviour
/// does not change by omission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZOrder {
    Bottom,
    Normal,
    Topmost,
}

/// What kind of surface a window is.
///
/// NOT A FLAG ON ONE STRUCT, because the two differ in what they can be asked.
/// A widget has an anchor and no title; a window has a title and no anchor, and
/// letting either set the other's fields means a setting that is silently
/// ignored — which is how a manifest becomes decoration.
#[derive(Debug, Clone, PartialEq)]
pub enum Surface {
    /// An ordinary window: caption, border, resizable, in the taskbar.
    ///
    /// What a developer previewing a component wants, and what a settings panel
    /// or a log viewer should be.
    Window { title: String },

    /// A floating surface: no chrome, always on top, out of the taskbar, and
    /// PER-PIXEL TRANSPARENT.
    ///
    /// The Rainmeter shape. The window's visible silhouette is whatever the tree
    /// painted — rounded corners and soft edges included — because the surface is
    /// composited from a premultiplied buffer rather than blitted into a
    /// rectangle. A `wgpu` swap chain composed as a `DirectComposition`
    /// visual's content does this, configured for premultiplied alpha
    /// instead of the opaque mode an ordinary `Window` uses --
    /// `crate::gpu`'s own doc has the reasoning.
    Widget {
        /// Screen position of the top-left corner.
        x: i32,
        y: i32,
        /// Let clicks fall through to whatever is behind. A monitor that only
        /// displays wants this; anything with a control does not.
        click_through: bool,
        /// This widget's tier in the desktop's z-order.
        z_order: ZOrder,
    },

    /// A widget the size of the desktop.
    ///
    /// The same layered surface as `Widget`, spanning the screen, so what the
    /// tree paints appears to sit directly on the desktop with nothing around
    /// it. An application that draws its own windows inside one of these is
    /// indistinguishable from one that owns several.
    ///
    /// CLICKS FALL THROUGH WHERE NOTHING WAS PAINTED, and this is the property
    /// that makes it work rather than a trick played on top of it. Windows
    /// hit-tests a layered window against its ALPHA CHANNEL: a pixel with alpha
    /// zero does not receive the click, it goes to whatever is behind. So a
    /// full-screen overlay that is transparent except where it drew is also
    /// click-through except where it drew, with no region, no hit-test hook and
    /// no second surface to keep in step.
    ///
    /// `WS_EX_TRANSPARENT` would DESTROY that — it makes the whole window
    /// click-through including the painted parts. It stays available because a
    /// purely decorative overlay wants exactly that, but it is off by default
    /// here for the opposite reason it is off for a widget.
    Overlay {
        /// This overlay's tier in the desktop's z-order. Defaults to `Topmost`
        /// — a workspace you cannot see is useless — but that does mean the
        /// overlay sits above full-screen applications by default, which is
        /// aggressive for anything that is not a shell.
        ///
        /// `Bottom` sits below every other window but above the wallpaper. That
        /// is NOT the same as Rainmeter's "OnDesktop", which parents into the
        /// desktop's `WorkerW` — a different technique entirely, and not this.
        z_order: ZOrder,
        click_through: bool,
    },
}

/// Which mouse button, spelled the way a shell wants to match on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// Something the window saw.
///
/// Coordinates are CLIENT-RELATIVE and in physical pixels — the same space the
/// display list uses, so a shell forwards them without converting. The one place
/// that is not free is the wheel, which Win32 reports in screen coordinates; that
/// conversion happens inside rather than being left as a trap for each caller.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerDown {
        x: f32,
        y: f32,
        button: Button,
    },
    PointerUp {
        x: f32,
        y: f32,
        button: Button,
    },
    /// Positive scrolls the content up, matching `Live.Session`'s own sign.
    Wheel {
        x: f32,
        y: f32,
        delta: f32,
    },
    /// A typed character, already decoded from the platform's encoding.
    Char(char),
    /// A named key that produces no character — "Backspace", "Left", "Return".
    Key {
        name: String,
        shift: bool,
        ctrl: bool,
    },
    Resized {
        width: u32,
        height: u32,
    },
    /// The surface must be fully repainted; nothing can be patched.
    Exposed,
    CloseRequested,
}
