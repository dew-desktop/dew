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
//! nothing to a floating widget and `anchor` means nothing to a window; a flat
//! table would let a mod set either and have it silently ignored, which is
//! exactly how `mod.json`'s permissions were decoration before they were
//! enforced.

use aether_window::Surface;
use mlua::prelude::*;

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
    },
}

impl Declared {
    /// Read the `surface` field of a mod's declaration.
    ///
    /// A mod that declares none gets a WIDGET, because that is what Dew is for.
    /// A desktop applet platform whose default is an ordinary window would be
    /// making every author opt in to the thing they came for.
    pub fn from_declaration(declaration: &LuaTable, fallback_title: &str) -> Declared {
        let Ok(surface) = declaration.get::<LuaTable>("surface") else {
            return Declared::default_widget();
        };

        let kind: String = surface.get("kind").unwrap_or_else(|_| "widget".to_string());
        match kind.as_str() {
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
                }
            }
        }
    }

    fn default_widget() -> Declared {
        Declared::Widget {
            anchor: Anchor::TopRight,
            offset: (24, 24),
            click_through: false,
        }
    }

    /// Turn the declaration into a concrete surface for a screen of this size.
    pub fn resolve(&self, screen: (i32, i32), size: (u32, u32)) -> Surface {
        match self {
            Declared::Window { title } => Surface::Window {
                title: title.clone(),
            },
            Declared::Widget {
                anchor,
                offset,
                click_through,
            } => {
                let (x, y) = anchor.resolve(screen, (size.0 as i32, size.1 as i32), *offset);
                Surface::Widget {
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
    /// a widget cleared opaque is a rectangle, and a window cleared transparent
    /// is a hole.
    pub fn is_transparent(&self) -> bool {
        matches!(self, Declared::Widget { .. })
    }

    pub fn describe(&self) -> String {
        match self {
            Declared::Window { .. } => "window".to_string(),
            Declared::Widget { anchor, click_through, .. } => format!(
                "widget {:?}{}",
                anchor,
                if *click_through { ", click-through" } else { "" }
            ),
        }
    }
}
