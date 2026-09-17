//! Where a mod's float goes, as the mod declares it.
//!
//! ONE `mount`, NOT TWO APIs.
//!
//! It is tempting to give floats and windows separate entry points, since they
//! feel like different things. They are not: the mod builds the SAME tree either
//! way and it produces the same display list. What differs is the surface that
//! display list is presented on — chrome or none, in the taskbar or not, blitted
//! into a rectangle or composited by its own alpha — and every one of those is a
//! property of the window rather than of the float.
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

/// Which corner a float measures its offset from.
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

    /// Resolve to a screen position for a float of this size.
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

/// What a mod declared, before a screen size is known.
#[derive(Debug, Clone)]
pub enum Declared {
    Window {
        title: String,
    },
    Float {
        anchor: Anchor,
        offset: (i32, i32),
        click_through: bool,
    },
    Overlay {
        topmost: bool,
        click_through: bool,
    },
}

impl Declared {
    /// Read the `surface` field of a mod's declaration.
    ///
    /// A mod that declares none gets a FLOAT, because that is what Dew is for.
    /// A desktop applet platform whose default is an ordinary window would be
    /// making every author opt in to the thing they came for.
    /// Which permission this surface needs.
    ///
    /// ONE PER KIND (ADR-012), because they differ in weight. A float draws in a
    /// corner; an overlay that is topmost and click-through can draw over
    /// everything on screen while the user does not know it is there.
    pub fn permission(&self) -> Permission {
        match self {
            Declared::Float { .. } => Permission::Float,
            Declared::Window { .. } => Permission::Window,
            Declared::Overlay { .. } => Permission::Overlay,
        }
    }

    pub fn from_declaration(declaration: &LuaTable, fallback_title: &str) -> Declared {
        let Ok(surface) = declaration.get::<LuaTable>("surface") else {
            return Declared::default_float();
        };

        let kind: String = surface.get("kind").unwrap_or_else(|_| "float".to_string());
        match kind.as_str() {
            "overlay" => Declared::Overlay {
                //--- ON TOP BY DEFAULT, because an overlay behind everything is one
                //--- you never see.
                topmost: surface.get("topmost").unwrap_or(true),
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

                Declared::Float {
                    anchor,
                    offset,
                    click_through: surface.get("clickThrough").unwrap_or(false),
                }
            }
        }
    }

    fn default_float() -> Declared {
        Declared::Float {
            anchor: Anchor::TopRight,
            offset: (24, 24),
            click_through: false,
        }
    }

    /// Turn the declaration into a concrete surface for a screen of this size.
    #[cfg(windows)]
    pub fn resolve(&self, screen: (i32, i32), size: (u32, u32)) -> Surface {
        match self {
            Declared::Window { title } => Surface::Window {
                title: title.clone(),
            },
            Declared::Overlay {
                topmost,
                click_through,
            } => Surface::Overlay {
                topmost: *topmost,
                click_through: *click_through,
            },
            Declared::Float {
                anchor,
                offset,
                click_through,
            } => {
                let (x, y) = anchor.resolve(screen, (size.0 as i32, size.1 as i32), *offset);
                Surface::Float {
                    x,
                    y,
                    click_through: *click_through,
                }
            }
        }
    }

    /// Whether this surface is composited from its own alpha.
    ///
    /// The frame must be cleared TRANSPARENT for one and opaque for the other,
    /// and that decision belongs with the surface rather than with the painter —
    /// a float cleared opaque is a rectangle, and a window cleared transparent
    /// is a hole.
    pub fn is_transparent(&self) -> bool {
        matches!(self, Declared::Float { .. } | Declared::Overlay { .. })
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
            Declared::Overlay { topmost, .. } => {
                format!("overlay{}", if *topmost { ", topmost" } else { "" })
            }
            Declared::Float {
                anchor,
                click_through,
                ..
            } => format!(
                "float {:?}{}",
                anchor,
                if *click_through {
                    ", click-through"
                } else {
                    ""
                }
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
/// rather than a return value because `dew.Float` has to hand the applet its
/// root, which is what it is really for; the host reads what was asked out of
/// here once the module has finished.
pub type Requested = Arc<Mutex<Option<Request>>>;

impl Request {
    /// Read `dew.Float{ ... }` and friends, which take one options table.
    ///
    /// EVERY FIELD IS OPTIONAL. An applet that calls `dew.Float{}` with nothing
    /// in it has said the only thing that matters, which is that it wants a
    /// float, and the defaults are the ones a declaration got.
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
                topmost: options
                    .as_ref()
                    .and_then(|o| o.get::<bool>("topmost").ok())
                    .unwrap_or(true),
                click_through: options
                    .as_ref()
                    .and_then(|o| o.get::<bool>("clickThrough").ok())
                    .unwrap_or(false),
            },
            _ => Declared::Float {
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
            },
        };

        Request { surface, size }
    }
}
