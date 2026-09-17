//! Where a mod's widget goes, as the mod declares it.
//!
//! ONE `mount`, NOT TWO APIs.
//!
//! It is tempting to give widgets and windows separate entry points, since they
//! feel like different things. They are not: the mod builds the SAME tree either
//! way and it produces the same display list. What differs is the surface that
//! display list is presented on — chrome or none, in the taskbar or not, blitted
//! into a rectangle or composited by its own alpha — and every one of those is a
//! property of the window rather than of the widget.
//!
//! Two APIs would mean two paths through the loader for a difference that is
//! entirely window-creation flags, and would force a mod that wants both a HUD
//! and a settings panel to be two mods.
//!
//! A TAGGED UNION, though, and not a bag of optional fields. `resizable` means
//! nothing to a floating surface and `anchor` means nothing to a window; a flat
//! table would let a mod set either and have it silently ignored, which is
//! exactly how `dew.toml`'s permissions were decoration before they were
//! enforced.

use crate::manifest::Permission;
#[cfg(windows)]
use dew_window::Surface;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};

/// Which corner a widget measures its offset from.
///
/// ANCHORS, NOT RAW COORDINATES, because a desktop is not one size. A clock
/// pinned 24px from the top-right stays in the corner when the display changes;
/// one at x=1872 is in the corner of the display it was written on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

impl Anchor {
    fn parse(name: &str) -> Option<Anchor> {
        Some(match name {
            "top-left" => Anchor::TopLeft,
            "top-right" => Anchor::TopRight,
            "bottom-left" => Anchor::BottomLeft,
            "bottom-right" => Anchor::BottomRight,
            "center" => Anchor::Center,
            _ => return None,
        })
    }

    /// Resolve to a screen position for a widget of this size.
    fn resolve(self, screen: (i32, i32), size: (i32, i32), offset: (i32, i32)) -> (i32, i32) {
        let (sw, sh) = screen;
        let (w, h) = size;
        let (ox, oy) = offset;
        match self {
            Anchor::TopLeft => (ox, oy),
            Anchor::TopRight => (sw - w - ox, oy),
            Anchor::BottomLeft => (ox, sh - h - oy),
            Anchor::BottomRight => (sw - w - ox, sh - h - oy),
            Anchor::Center => ((sw - w) / 2 + ox, (sh - h) / 2 + oy),
        }
    }
}

/// A window's tier in the desktop's z-order.
///
/// THREE VALUES, NOT A BOOL. `Bottom` sits below every normal window and above
/// the wallpaper — NOT the same as Rainmeter's "OnDesktop", which needs
/// `WorkerW`-parenting and is out of scope here. `Normal` is ordinary z-order,
/// which can be covered. `Topmost` is always on top, which is today's only
/// behaviour and stays the default, so an existing applet's behaviour does not
/// change by omission.
///
/// DEFINED HERE RATHER THAN REUSING `dew_window::ZOrder` DIRECTLY, even though
/// they say the same three things. `dew_window` is `#[cfg(windows)]` for its
/// whole crate, so a type living there does not exist on a target that is not
/// Windows — and `Declared` is not itself platform-gated, because CI type-checks
/// this host on Linux too (see `.github/workflows/ci.yml`'s `linux` job). Read
/// the same way `Anchor` already crosses this boundary: as a plain scalar pair,
/// produced only inside `resolve`, which IS `#[cfg(windows)]`. This mirrors
/// that — a host-side enum, converted to `dew_window::ZOrder` only at the same
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZOrder {
    Bottom,
    Normal,
    #[default]
    Topmost,
}

impl ZOrder {
    fn parse(name: &str) -> Option<ZOrder> {
        Some(match name {
            "bottom" => ZOrder::Bottom,
            "normal" => ZOrder::Normal,
            "topmost" => ZOrder::Topmost,
            _ => return None,
        })
    }

    fn read(table: &LuaTable) -> ZOrder {
        table
            .get::<String>("zOrder")
            .ok()
            .and_then(|z| ZOrder::parse(&z))
            .unwrap_or_default()
    }
}

/// What a mod declared, before a screen size is known.
#[derive(Debug, Clone)]
pub enum Declared {
    Window {
        title: String,
    },
    Widget {
        anchor: Anchor,
        offset: (i32, i32),
        click_through: bool,
        z_order: ZOrder,
        /// Grab the widget's body and move it, the way a Rainmeter skin does
        /// with no title bar of its own. HOST-DRIVEN (see `main.rs`'s frame
        /// loop), not scriptable — an applet's own code never sees a drag.
        draggable: bool,
        /// After a drag, keep the widget's full rectangle within some visible
        /// display rather than letting it end up off every one of them.
        keep_on_screen: bool,
        /// While dragging, snap to a screen edge within a small threshold.
        snap_to_edges: bool,
        /// A dragged position survives to the next launch, overriding the
        /// declared anchor until the widget is dragged again.
        save_position: bool,
    },
    Overlay {
        z_order: ZOrder,
        click_through: bool,
    },
}

impl Declared {
    /// Read the `surface` field of a mod's declaration.
    ///
    /// A mod that declares none gets a WIDGET, because that is what Dew is for.
    /// A desktop applet platform whose default is an ordinary window would be
    /// making every author opt in to the thing they came for.
    /// Which permission this surface needs.
    ///
    /// ONE PER KIND (ADR-012), because they differ in weight. A widget draws in a
    /// corner; an overlay that is topmost and click-through can draw over
    /// everything on screen while the user does not know it is there.
    pub fn permission(&self) -> Permission {
        match self {
            Declared::Widget { .. } => Permission::Widget,
            Declared::Window { .. } => Permission::Window,
            Declared::Overlay { .. } => Permission::Overlay,
        }
    }

    pub fn from_declaration(declaration: &LuaTable, fallback_title: &str) -> Declared {
        let Ok(surface) = declaration.get::<LuaTable>("surface") else {
            return Declared::default_widget();
        };

        let kind: String = surface.get("kind").unwrap_or_else(|_| "widget".to_string());
        match kind.as_str() {
            //--- ON TOP BY DEFAULT (`ZOrder::read`'s own default), because an
            //--- overlay behind everything is one you never see.
            "overlay" => Declared::Overlay {
                z_order: ZOrder::read(&surface),
                click_through: surface.get("clickThrough").unwrap_or(false),
            },
            "window" => Declared::Window {
                title: surface
                    .get::<String>("title")
                    .unwrap_or_else(|_| fallback_title.to_string()),
            },
            _ => {
                let anchor = surface
                    .get::<String>("anchor")
                    .ok()
                    .and_then(|a| Anchor::parse(&a))
                    .unwrap_or(Anchor::TopRight);

                let offset = surface
                    .get::<LuaTable>("offset")
                    .map(|o| (o.get("x").unwrap_or(24), o.get("y").unwrap_or(24)))
                    .unwrap_or((24, 24));

                Declared::Widget {
                    anchor,
                    offset,
                    click_through: surface.get("clickThrough").unwrap_or(false),
                    z_order: ZOrder::read(&surface),
                    draggable: surface.get("draggable").unwrap_or(false),
                    keep_on_screen: surface.get("keepOnScreen").unwrap_or(true),
                    snap_to_edges: surface.get("snapToEdges").unwrap_or(false),
                    save_position: surface.get("savePosition").unwrap_or(false),
                }
            }
        }
    }

    fn default_widget() -> Declared {
        Declared::Widget {
            anchor: Anchor::TopRight,
            offset: (24, 24),
            click_through: false,
            z_order: ZOrder::Topmost,
            draggable: false,
            keep_on_screen: true,
            snap_to_edges: false,
            save_position: false,
        }
    }

    /// Turn the declaration into a concrete surface for a screen of this size.
    #[cfg(windows)]
    pub fn resolve(&self, screen: (i32, i32), size: (u32, u32)) -> Surface {
        fn to_window_z_order(z: ZOrder) -> dew_window::ZOrder {
            match z {
                ZOrder::Bottom => dew_window::ZOrder::Bottom,
                ZOrder::Normal => dew_window::ZOrder::Normal,
                ZOrder::Topmost => dew_window::ZOrder::Topmost,
            }
        }

        match self {
            Declared::Window { title } => Surface::Window {
                title: title.clone(),
            },
            Declared::Overlay {
                z_order,
                click_through,
            } => Surface::Overlay {
                z_order: to_window_z_order(*z_order),
                click_through: *click_through,
            },
            Declared::Widget {
                anchor,
                offset,
                click_through,
                z_order,
                ..
            } => {
                let (x, y) = anchor.resolve(screen, (size.0 as i32, size.1 as i32), *offset);
                Surface::Widget {
                    x,
                    y,
                    click_through: *click_through,
                    z_order: to_window_z_order(*z_order),
                }
            }
        }
    }

    /// Whether this surface is composited from its own alpha.
    ///
    /// The frame must be cleared TRANSPARENT for one and opaque for the other,
    /// and that decision belongs with the surface rather than with the painter —
    /// a widget cleared opaque is a rectangle, and a window cleared transparent
    /// is a hole.
    pub fn is_transparent(&self) -> bool {
        matches!(self, Declared::Widget { .. } | Declared::Overlay { .. })
    }

    /// An overlay is sized by the SCREEN, not by the mod.
    ///
    /// A mod cannot know the display it will land on, and one that guessed would
    /// be wrong on every machine but the author's. So `size` in the declaration
    /// is ignored for this kind rather than being a value nobody can supply
    /// correctly.
    pub fn fills_screen(&self) -> bool {
        matches!(self, Declared::Overlay { .. })
    }

    pub fn describe(&self) -> String {
        match self {
            Declared::Window { .. } => "window".to_string(),
            Declared::Overlay {
                z_order,
                click_through,
            } => format!(
                "overlay {:?}{}",
                z_order,
                if *click_through {
                    ", click-through"
                } else {
                    ""
                }
            ),
            Declared::Widget {
                anchor,
                click_through,
                z_order,
                draggable,
                ..
            } => format!(
                "widget {:?} {:?}{}{}",
                anchor,
                z_order,
                if *click_through {
                    ", click-through"
                } else {
                    ""
                },
                if *draggable { ", draggable" } else { "" }
            ),
        }
    }
}

/// What an applet asked the host for, and how big.
///
/// SEPARATE FROM `Declared` BECAUSE IT CARRIES A SIZE. A declaration was read
/// from a returned table alongside a `size` field, so the two travelled together
/// without either naming the other. A request arrives as one call, and the size
/// is part of what was asked for.
#[derive(Debug, Clone)]
pub struct Request {
    pub surface: Declared,
    pub size: Option<(u32, u32)>,
}

/// Where a surface request lands between the applet asking and the host reading.
///
/// THE APPLET ASKS WHILE IT RUNS, and the host wants the answer after. A cell
/// rather than a return value because `dew.Widget` has to hand the applet its
/// root, which is what it is really for; the host reads what was asked out of
/// here once the module has finished.
pub type Requested = Arc<Mutex<Option<Request>>>;

impl Request {
    /// Read `dew.Widget{ ... }` and friends, which take one options table.
    ///
    /// EVERY FIELD IS OPTIONAL. An applet that calls `dew.Widget{}` with nothing
    /// in it has said the only thing that matters, which is that it wants a
    /// widget, and the defaults are the ones a declaration got.
    pub fn from_options(
        kind: Permission,
        options: Option<LuaTable>,
        fallback_title: &str,
    ) -> Request {
        let size = options.as_ref().and_then(|o| {
            let width: u32 = o.get("width").ok()?;
            let height: u32 = o.get("height").ok()?;
            Some((width.max(1), height.max(1)))
        });

        let surface = match kind {
            Permission::Window => Declared::Window {
                title: options
                    .as_ref()
                    .and_then(|o| o.get::<String>("title").ok())
                    .unwrap_or_else(|| fallback_title.to_string()),
            },
            Permission::Overlay => Declared::Overlay {
                z_order: options.as_ref().map(ZOrder::read).unwrap_or_default(),
                click_through: options
                    .as_ref()
                    .and_then(|o| o.get::<bool>("clickThrough").ok())
                    .unwrap_or(false),
            },
            _ => Declared::Widget {
                anchor: options
                    .as_ref()
                    .and_then(|o| o.get::<String>("anchor").ok())
                    .and_then(|a| Anchor::parse(&a))
                    .unwrap_or(Anchor::TopRight),
                offset: options
                    .as_ref()
                    .and_then(|o| o.get::<LuaTable>("offset").ok())
                    .map(|o| (o.get("x").unwrap_or(24), o.get("y").unwrap_or(24)))
                    .unwrap_or((24, 24)),
                click_through: options
                    .as_ref()
                    .and_then(|o| o.get::<bool>("clickThrough").ok())
                    .unwrap_or(false),
                z_order: options.as_ref().map(ZOrder::read).unwrap_or_default(),
                draggable: options
                    .as_ref()
                    .and_then(|o| o.get::<bool>("draggable").ok())
                    .unwrap_or(false),
                keep_on_screen: options
                    .as_ref()
                    .and_then(|o| o.get::<bool>("keepOnScreen").ok())
                    .unwrap_or(true),
                snap_to_edges: options
                    .as_ref()
                    .and_then(|o| o.get::<bool>("snapToEdges").ok())
                    .unwrap_or(false),
                save_position: options
                    .as_ref()
                    .and_then(|o| o.get::<bool>("savePosition").ok())
                    .unwrap_or(false),
            },
        };

        Request { surface, size }
    }
}
