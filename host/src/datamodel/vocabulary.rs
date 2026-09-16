//! `UDim`, `UDim2`, `Vector2`, `Color3` and `Rect` for a guest.
//!
//! WHY THESE ARE THE NEXT SLICE
//! `Size` and `Position` are `UDim2`, and they are the two properties every UI
//! sets first. The previous slice accepted primitives and refused these with a
//! message saying the host had not built them yet, which was honest and useless
//! -- an application cannot lay anything out without them.
//!
//! FIVE RUST TYPES, NOT ONE ENUM
//! The obvious shape is one `Value` enum behind one `UserData`. It does not work:
//! Luau's `typeof` reads `__type` as a string off the METATABLE, so it belongs to
//! the Rust type rather than to the value, and one wrapper would have to report a
//! single name for all five. `typeof(UDim2.new())` must be `"UDim2"` on both
//! hosts or a guest that branches on it takes the wrong path -- the same trap that
//! made `typeof(instance)` read `"InstanceRef"` until it was pinned down.
//!
//! WHAT IS DELIBERATELY NOT HERE
//! `Vector3` and `CFrame`. This standard is 2D UI: a host that implemented
//! `CFrame` would be describing the engine rather than describing a UI, which is the
//! same test `OUT_OF_SCOPE` applies to classes.
//!
//! THE SEQUENCE TYPES ARE HERE AND ARE NOT DRAWN. `ColorSequence` and
//! `NumberSequence` are the two properties this host still refuses, and a
//! gradient is step K's neighbourhood -- but a CONSTRUCTOR that exists and a
//! PROPERTY the host declines are different claims, and a module that merely
//! names the type has to be able to load. That distinction is the whole of why
//! they are here.
//!
//! ELEVEN NAMES, AND PARTIAL IS WORSE THAN ABSENT. Aether publishes its own
//! vocabulary first-writer-wins, so five of these did not merge with its eleven,
//! they blocked them -- `Color3.fromHex` went missing and three mods stopped
//! loading. Under Aether's DataModel host nothing publishes a second vocabulary
//! at all, so what is missing here is missing everywhere. `install_vocabulary`
//! in the parent module carries that argument in full.
//!
//! NUMBERS ARE TRUNCATED THE WAY THE ENGINE TRUNCATES THEM. `UDim.Offset` is an
//! `i32`, and this module used to refuse a fraction rather than convert one --
//! which is a divergence and not a stricter reading, because the engine converts.
//! See `pixel_i32` in the parent module for the measurement and for what refusing
//! turned out to cost.

use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use rbx_types::{
    Color3, ColorSequence, ColorSequenceKeypoint, Font, FontStyle, FontWeight, NumberSequence,
    NumberSequenceKeypoint, Rect, UDim, UDim2, Vector2,
};

use super::{number, pixel_i32};

macro_rules! vocabulary_type {
    ($wrapper:ident, $inner:ty, $name:literal) => {
        #[derive(Clone, Copy, PartialEq)]
        pub struct $wrapper(pub $inner);

        impl $wrapper {
            /// Pull one out of a Lua value, or `None` if it is something else.
            pub fn from_value(value: &LuaValue) -> Option<$inner> {
                let LuaValue::UserData(ud) = value else {
                    return None;
                };
                ud.borrow::<$wrapper>().ok().map(|w| w.0)
            }
        }
    };
}

vocabulary_type!(LuaUDim, UDim, "UDim");
vocabulary_type!(LuaUDim2, UDim2, "UDim2");
vocabulary_type!(LuaVector2, Vector2, "Vector2");
vocabulary_type!(LuaColor3, Color3, "Color3");
vocabulary_type!(LuaRect, Rect, "Rect");
vocabulary_type!(
    LuaNumberSequenceKeypoint,
    NumberSequenceKeypoint,
    "NumberSequenceKeypoint"
);
vocabulary_type!(
    LuaColorSequenceKeypoint,
    ColorSequenceKeypoint,
    "ColorSequenceKeypoint"
);

/// `Font`, `NumberSequence` and `ColorSequence` carry a `String` and two `Vec`s,
/// so they are not `Copy` and cannot go through the macro above. Same contract,
/// written out: a `__type` that matches the constructor's name, equality, and a
/// `from_value` that refuses anything else.
macro_rules! owned_vocabulary_type {
    ($wrapper:ident, $inner:ty) => {
        #[derive(Clone, PartialEq)]
        pub struct $wrapper(pub $inner);

        impl $wrapper {
            pub fn from_value(value: &LuaValue) -> Option<$inner> {
                let LuaValue::UserData(ud) = value else {
                    return None;
                };
                ud.borrow::<$wrapper>().ok().map(|w| w.0.clone())
            }
        }
    };
}

owned_vocabulary_type!(LuaFont, Font);
owned_vocabulary_type!(LuaNumberSequence, NumberSequence);
owned_vocabulary_type!(LuaColorSequence, ColorSequence);

impl UserData for LuaUDim {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "UDim");
        fields.add_field_method_get("Scale", |_, this| Ok(this.0.scale));
        fields.add_field_method_get("Offset", |_, this| Ok(this.0.offset));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("{}, {}", this.0.scale, this.0.offset))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaUDim::from_value(&other) == Some(this.0))
        });
        methods.add_meta_method(MetaMethod::Add, |_, this, other: LuaValue| {
            let other = LuaUDim::from_value(&other)
                .ok_or_else(|| LuaError::runtime("a UDim can only be added to a UDim"))?;
            Ok(LuaUDim(UDim::new(
                this.0.scale + other.scale,
                this.0.offset + other.offset,
            )))
        });
        methods.add_meta_method(MetaMethod::Sub, |_, this, other: LuaValue| {
            let other = LuaUDim::from_value(&other)
                .ok_or_else(|| LuaError::runtime("a UDim can only be subtracted from a UDim"))?;
            Ok(LuaUDim(UDim::new(
                this.0.scale - other.scale,
                this.0.offset - other.offset,
            )))
        });
    }
}

impl UserData for LuaUDim2 {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "UDim2");
        fields.add_field_method_get("X", |_, this| Ok(LuaUDim(this.0.x)));
        fields.add_field_method_get("Y", |_, this| Ok(LuaUDim(this.0.y)));
        // `Width` and `Height` are what a UI author reaches for, and the engine has
        // them on `UDim2` as aliases of X and Y.
        fields.add_field_method_get("Width", |_, this| Ok(LuaUDim(this.0.x)));
        fields.add_field_method_get("Height", |_, this| Ok(LuaUDim(this.0.y)));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "{{{}, {}}}, {{{}, {}}}",
                this.0.x.scale, this.0.x.offset, this.0.y.scale, this.0.y.offset
            ))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaUDim2::from_value(&other) == Some(this.0))
        });
        methods.add_meta_method(MetaMethod::Add, |_, this, other: LuaValue| {
            let other = LuaUDim2::from_value(&other)
                .ok_or_else(|| LuaError::runtime("a UDim2 can only be added to a UDim2"))?;
            Ok(LuaUDim2(UDim2::new(
                UDim::new(
                    this.0.x.scale + other.x.scale,
                    this.0.x.offset + other.x.offset,
                ),
                UDim::new(
                    this.0.y.scale + other.y.scale,
                    this.0.y.offset + other.y.offset,
                ),
            )))
        });
        methods.add_meta_method(MetaMethod::Sub, |_, this, other: LuaValue| {
            let other = LuaUDim2::from_value(&other)
                .ok_or_else(|| LuaError::runtime("a UDim2 can only be subtracted from a UDim2"))?;
            Ok(LuaUDim2(UDim2::new(
                UDim::new(
                    this.0.x.scale - other.x.scale,
                    this.0.x.offset - other.x.offset,
                ),
                UDim::new(
                    this.0.y.scale - other.y.scale,
                    this.0.y.offset - other.y.offset,
                ),
            )))
        });
    }
}

impl UserData for LuaVector2 {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "Vector2");
        fields.add_field_method_get("X", |_, this| Ok(this.0.x));
        fields.add_field_method_get("Y", |_, this| Ok(this.0.y));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("{}, {}", this.0.x, this.0.y))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaVector2::from_value(&other) == Some(this.0))
        });
        methods.add_meta_method(MetaMethod::Add, |_, this, other: LuaValue| {
            let other = LuaVector2::from_value(&other)
                .ok_or_else(|| LuaError::runtime("a Vector2 can only be added to a Vector2"))?;
            Ok(LuaVector2(Vector2::new(
                this.0.x + other.x,
                this.0.y + other.y,
            )))
        });
        methods.add_meta_method(MetaMethod::Sub, |_, this, other: LuaValue| {
            let other = LuaVector2::from_value(&other).ok_or_else(|| {
                LuaError::runtime("a Vector2 can only be subtracted from a Vector2")
            })?;
            Ok(LuaVector2(Vector2::new(
                this.0.x - other.x,
                this.0.y - other.y,
            )))
        });
        // SCALAR ONLY, either way round. The engine also multiplies two Vector2s
        // component-wise; that is added when something needs it rather than
        // guessed at now.
        methods.add_meta_method(MetaMethod::Mul, |_, this, other: LuaValue| {
            let by = number(&other)
                .ok_or_else(|| LuaError::runtime("a Vector2 can be multiplied by a number"))?
                as f32;
            Ok(LuaVector2(Vector2::new(this.0.x * by, this.0.y * by)))
        });
    }
}

impl UserData for LuaColor3 {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "Color3");
        fields.add_field_method_get("R", |_, this| Ok(this.0.r));
        fields.add_field_method_get("G", |_, this| Ok(this.0.g));
        fields.add_field_method_get("B", |_, this| Ok(this.0.b));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("{}, {}, {}", this.0.r, this.0.g, this.0.b))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaColor3::from_value(&other) == Some(this.0))
        });
    }
}

impl UserData for LuaRect {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "Rect");
        fields.add_field_method_get("Min", |_, this| Ok(LuaVector2(this.0.min)));
        fields.add_field_method_get("Max", |_, this| Ok(LuaVector2(this.0.max)));
        fields.add_field_method_get("Width", |_, this| Ok(this.0.max.x - this.0.min.x));
        fields.add_field_method_get("Height", |_, this| Ok(this.0.max.y - this.0.min.y));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "{}, {}, {}, {}",
                this.0.min.x, this.0.min.y, this.0.max.x, this.0.max.y
            ))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaRect::from_value(&other) == Some(this.0))
        });
    }
}

impl UserData for LuaFont {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "Font");
        fields.add_field_method_get("Family", |_, this| Ok(this.0.family.clone()));
        fields.add_field_method_get("Weight", |_, this| Ok(weight_item(this.0.weight)));
        fields.add_field_method_get("Style", |_, this| Ok(style_item(this.0.style)));
        // The engine's shorthand, and Aether reads it nowhere -- it is here
        // because a guest written against the engine does.
        fields.add_field_method_get("Bold", |_, this| {
            Ok(matches!(
                this.0.weight,
                FontWeight::Bold | FontWeight::ExtraBold | FontWeight::Heavy
            ))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "Font({}, {:?}, {:?})",
                this.0.family, this.0.weight, this.0.style
            ))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaFont::from_value(&other).as_ref() == Some(&this.0))
        });
    }
}

impl UserData for LuaNumberSequenceKeypoint {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "NumberSequenceKeypoint");
        fields.add_field_method_get("Time", |_, this| Ok(this.0.time));
        fields.add_field_method_get("Value", |_, this| Ok(this.0.value));
        fields.add_field_method_get("Envelope", |_, this| Ok(this.0.envelope));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "{} {} {}",
                this.0.time, this.0.value, this.0.envelope
            ))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaNumberSequenceKeypoint::from_value(&other) == Some(this.0))
        });
    }
}

impl UserData for LuaColorSequenceKeypoint {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "ColorSequenceKeypoint");
        fields.add_field_method_get("Time", |_, this| Ok(this.0.time));
        fields.add_field_method_get("Value", |_, this| Ok(LuaColor3(this.0.color)));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "{} {} {} {}",
                this.0.time, this.0.color.r, this.0.color.g, this.0.color.b
            ))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaColorSequenceKeypoint::from_value(&other) == Some(this.0))
        });
    }
}

impl UserData for LuaNumberSequence {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "NumberSequence");
        // A FRESH TABLE PER READ, and the engine does the same: `Keypoints` is a
        // copy, so a guest mutating what it got back has not edited the value.
        fields.add_field_method_get("Keypoints", |lua, this| {
            let out = lua.create_table()?;
            for (i, k) in this.0.keypoints.iter().enumerate() {
                out.set(i + 1, LuaNumberSequenceKeypoint(*k))?;
            }
            Ok(out)
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaNumberSequence::from_value(&other).as_ref() == Some(&this.0))
        });
    }
}

impl UserData for LuaColorSequence {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "ColorSequence");
        fields.add_field_method_get("Keypoints", |lua, this| {
            let out = lua.create_table()?;
            for (i, k) in this.0.keypoints.iter().enumerate() {
                out.set(i + 1, LuaColorSequenceKeypoint(*k))?;
            }
            Ok(out)
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaColorSequence::from_value(&other).as_ref() == Some(&this.0))
        });
    }
}

/// `Enum.FontWeight.Medium` for a `FontWeight`, or the raw number when the
/// reflection database does not carry that member.
///
/// THROUGH `enums::item_by_name`, NOT A LOCAL TABLE, for the reason that module
/// already argues about properties: a hand-written mapping is wrong within one
/// upstream release, and `Font.Weight` compared against `Enum.FontWeight.Bold`
/// must answer true rather than plausibly.
fn weight_item(weight: FontWeight) -> LuaEitherEnum {
    match super::enums::item_by_name("FontWeight", &format!("{weight:?}")) {
        Some(item) => LuaEitherEnum::Item(item),
        None => LuaEitherEnum::Number(weight.as_u16() as u32),
    }
}

fn style_item(style: FontStyle) -> LuaEitherEnum {
    match super::enums::item_by_name("FontStyle", &format!("{style:?}")) {
        Some(item) => LuaEitherEnum::Item(item),
        None => LuaEitherEnum::Number(style.as_u8() as u32),
    }
}

/// What a `Font`'s weight and style read back as.
pub enum LuaEitherEnum {
    Item(super::enums::LuaEnumItem),
    Number(u32),
}

impl IntoLua for LuaEitherEnum {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        match self {
            LuaEitherEnum::Item(item) => item.into_lua(lua),
            LuaEitherEnum::Number(n) => Ok(LuaValue::Integer(n as i64)),
        }
    }
}

/// Install the constructors as globals.
///
/// GLOBALS, like `Instance`, and for the same reason: this is the language of
/// the platform rather than a capability. A guest that had to be handed `UDim2`
/// would not be the guest that runs on the engine.
pub fn install(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();

    let udim = lua.create_table()?;
    udim.set(
        "new",
        lua.create_function(|_, (scale, offset): (Option<f32>, Option<f64>)| {
            Ok(LuaUDim(UDim::new(
                scale.unwrap_or(0.0),
                pixel_i32(offset.unwrap_or(0.0)),
            )))
        })?,
    )?;
    globals.set("UDim", udim)?;

    let udim2 = lua.create_table()?;
    // FOUR NUMBERS OR TWO UDims, because the engine accepts both and a guest written
    // against the engine uses whichever it likes.
    udim2.set(
        "new",
        lua.create_function(|_, args: LuaMultiValue| {
            let values: Vec<LuaValue> = args.into_iter().collect();
            if values.len() == 2 {
                if let (Some(x), Some(y)) = (
                    LuaUDim::from_value(&values[0]),
                    LuaUDim::from_value(&values[1]),
                ) {
                    return Ok(LuaUDim2(UDim2::new(x, y)));
                }
            }
            let number =
                |index: usize| -> f64 { values.get(index).and_then(number).unwrap_or(0.0) };
            Ok(LuaUDim2(UDim2::new(
                UDim::new(number(0) as f32, pixel_i32(number(1))),
                UDim::new(number(2) as f32, pixel_i32(number(3))),
            )))
        })?,
    )?;
    udim2.set(
        "fromScale",
        lua.create_function(|_, (x, y): (Option<f32>, Option<f32>)| {
            Ok(LuaUDim2(UDim2::new(
                UDim::new(x.unwrap_or(0.0), 0),
                UDim::new(y.unwrap_or(0.0), 0),
            )))
        })?,
    )?;
    udim2.set(
        "fromOffset",
        lua.create_function(|_, (x, y): (Option<f64>, Option<f64>)| {
            Ok(LuaUDim2(UDim2::new(
                UDim::new(0.0, pixel_i32(x.unwrap_or(0.0))),
                UDim::new(0.0, pixel_i32(y.unwrap_or(0.0))),
            )))
        })?,
    )?;
    globals.set("UDim2", udim2)?;

    let vector2 = lua.create_table()?;
    vector2.set(
        "new",
        lua.create_function(|_, (x, y): (Option<f32>, Option<f32>)| {
            Ok(LuaVector2(Vector2::new(x.unwrap_or(0.0), y.unwrap_or(0.0))))
        })?,
    )?;
    vector2.set("zero", LuaVector2(Vector2::new(0.0, 0.0)))?;
    vector2.set("one", LuaVector2(Vector2::new(1.0, 1.0)))?;
    globals.set("Vector2", vector2)?;

    let color3 = lua.create_table()?;
    color3.set(
        "new",
        lua.create_function(|_, (r, g, b): (Option<f32>, Option<f32>, Option<f32>)| {
            Ok(LuaColor3(Color3::new(
                r.unwrap_or(0.0),
                g.unwrap_or(0.0),
                b.unwrap_or(0.0),
            )))
        })?,
    )?;
    // 0-255 IN, 0-1 STORED. The engine does the same conversion, and a host that
    // stored bytes would disagree with every case in the conformance suite.
    color3.set(
        "fromRGB",
        lua.create_function(|_, (r, g, b): (Option<f32>, Option<f32>, Option<f32>)| {
            Ok(LuaColor3(Color3::new(
                r.unwrap_or(0.0) / 255.0,
                g.unwrap_or(0.0) / 255.0,
                b.unwrap_or(0.0) / 255.0,
            )))
        })?,
    )?;
    // HEX, AND IT IS NOT DECORATION. `Color3.fromHex` has 17 call sites in
    // `applets/` and 5 more in Aether's own source, and it is the single
    // constructor whose absence stopped all three Aether mods when Dew's
    // vocabulary was first installed into one.
    //
    // "#RGB", "#RRGGBB" and either without the hash: what the engine accepts and
    // what Aether's own PureVocabulary accepts. Anything else is an ERROR rather
    // than black, because a theme colour silently reading as black is a UI that
    // renders and is wrong.
    color3.set(
        "fromHex",
        lua.create_function(|_, hex: String| {
            let digits = hex.strip_prefix('#').unwrap_or(&hex);
            let expanded = match digits.len() {
                3 => digits.chars().flat_map(|c| [c, c]).collect::<String>(),
                6 => digits.to_string(),
                _ => {
                    return Err(LuaError::runtime(format!(
                        "Color3.fromHex expects \"#RGB\" or \"#RRGGBB\", got {hex:?}"
                    )))
                }
            };
            let channel = |at: usize| -> LuaResult<f32> {
                u8::from_str_radix(&expanded[at..at + 2], 16)
                    .map(|v| v as f32 / 255.0)
                    .map_err(|_| {
                        LuaError::runtime(format!(
                            "Color3.fromHex expects hexadecimal digits, got {hex:?}"
                        ))
                    })
            };
            Ok(LuaColor3(Color3::new(
                channel(0)?,
                channel(2)?,
                channel(4)?,
            )))
        })?,
    )?;
    // Theme code derives hover and pressed shades from a base colour with this.
    // The standard conversion, matching the engine and PureVocabulary alike.
    color3.set(
        "fromHSV",
        lua.create_function(|_, (h, s, v): (f32, f32, f32)| {
            let sector = (h * 6.0).floor();
            let f = h * 6.0 - sector;
            let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
            let (r, g, b) = match (sector as i64).rem_euclid(6) {
                0 => (v, t, p),
                1 => (q, v, p),
                2 => (p, v, t),
                3 => (p, q, v),
                4 => (t, p, v),
                _ => (v, p, q),
            };
            Ok(LuaColor3(Color3::new(r, g, b)))
        })?,
    )?;
    globals.set("Color3", color3)?;

    let rect = lua.create_table()?;
    rect.set(
        "new",
        lua.create_function(|_, args: LuaMultiValue| {
            let values: Vec<LuaValue> = args.into_iter().collect();
            if values.len() == 2 {
                if let (Some(min), Some(max)) = (
                    LuaVector2::from_value(&values[0]),
                    LuaVector2::from_value(&values[1]),
                ) {
                    return Ok(LuaRect(Rect::new(min, max)));
                }
            }
            let at =
                |index: usize| -> f32 { values.get(index).and_then(number).unwrap_or(0.0) as f32 };
            Ok(LuaRect(Rect::new(
                Vector2::new(at(0), at(1)),
                Vector2::new(at(2), at(3)),
            )))
        })?,
    )?;
    globals.set("Rect", rect)?;

    // ---- Font, and the two sequence types ----
    //
    // THE REST OF THE ELEVEN NAMES Aether's `REQUIRED_VOCABULARY` requires, and they
    // are here for completeness rather than because Dew draws with them. A
    // gradient is step K's neighbourhood and `ColorSequence`/`NumberSequence` are
    // the two properties this host still refuses.
    //
    // A CONSTRUCTOR THAT EXISTS AND A PROPERTY THE HOST DECLINES ARE DIFFERENT
    // CLAIMS, and only the first is needed to load a module that names the type.
    // What a PARTIAL vocabulary costs is the argument in `install_vocabulary`:
    // under the DataModel host `InstallVocabulary` is a no-op, so whatever is
    // missing here is missing everywhere, and it goes missing as a nil index at a
    // call site rather than as anything a reader can trace back to this file.

    let font = lua.create_table()?;
    font.set(
        "new",
        lua.create_function(
            |_, (family, weight, style): (String, Option<LuaValue>, Option<LuaValue>)| {
                Ok(LuaFont(Font::new(
                    &family,
                    font_weight(weight.as_ref())?,
                    font_style(style.as_ref())?,
                )))
            },
        )?,
    )?;
    globals.set("Font", font)?;

    let nsk = lua.create_table()?;
    nsk.set(
        "new",
        lua.create_function(|_, (time, value, envelope): (f32, f32, Option<f32>)| {
            Ok(LuaNumberSequenceKeypoint(NumberSequenceKeypoint::new(
                time,
                value,
                envelope.unwrap_or(0.0),
            )))
        })?,
    )?;
    globals.set("NumberSequenceKeypoint", nsk)?;

    let ns = lua.create_table()?;
    // A CONSTANT, A FROM/TO PAIR, OR A KEYPOINT LIST -- all three, because the
    // engine accepts all three and a guest uses whichever it likes.
    ns.set(
        "new",
        lua.create_function(|_, (first, second): (LuaValue, Option<f32>)| {
            if let LuaValue::Table(list) = &first {
                let mut keypoints = Vec::new();
                for entry in list.clone().sequence_values::<LuaValue>() {
                    keypoints.push(LuaNumberSequenceKeypoint::from_value(&entry?).ok_or_else(
                        || {
                            LuaError::runtime(
                                "NumberSequence.new expects a list of NumberSequenceKeypoint",
                            )
                        },
                    )?);
                }
                return Ok(LuaNumberSequence(NumberSequence { keypoints }));
            }
            let a = number(&first).ok_or_else(|| {
                LuaError::runtime("NumberSequence.new expects a number or a keypoint list")
            })? as f32;
            let b = second.unwrap_or(a);
            Ok(LuaNumberSequence(NumberSequence {
                keypoints: vec![
                    NumberSequenceKeypoint::new(0.0, a, 0.0),
                    NumberSequenceKeypoint::new(1.0, b, 0.0),
                ],
            }))
        })?,
    )?;
    globals.set("NumberSequence", ns)?;

    let csk = lua.create_table()?;
    csk.set(
        "new",
        lua.create_function(|_, (time, value): (f32, LuaValue)| {
            let color = LuaColor3::from_value(&value).ok_or_else(|| {
                LuaError::runtime("ColorSequenceKeypoint.new expects a time and a Color3")
            })?;
            Ok(LuaColorSequenceKeypoint(ColorSequenceKeypoint::new(
                time, color,
            )))
        })?,
    )?;
    globals.set("ColorSequenceKeypoint", csk)?;

    let cs = lua.create_table()?;
    cs.set(
        "new",
        lua.create_function(|_, (first, second): (LuaValue, Option<LuaValue>)| {
            if let LuaValue::Table(list) = &first {
                let mut keypoints = Vec::new();
                for entry in list.clone().sequence_values::<LuaValue>() {
                    keypoints.push(LuaColorSequenceKeypoint::from_value(&entry?).ok_or_else(
                        || {
                            LuaError::runtime(
                                "ColorSequence.new expects a list of ColorSequenceKeypoint",
                            )
                        },
                    )?);
                }
                return Ok(LuaColorSequence(ColorSequence { keypoints }));
            }
            let a = LuaColor3::from_value(&first).ok_or_else(|| {
                LuaError::runtime("ColorSequence.new expects a Color3 or a keypoint list")
            })?;
            let b = match second.as_ref() {
                Some(value) => LuaColor3::from_value(value).ok_or_else(|| {
                    LuaError::runtime("ColorSequence.new's second argument must be a Color3")
                })?,
                None => a,
            };
            Ok(LuaColorSequence(ColorSequence {
                keypoints: vec![
                    ColorSequenceKeypoint::new(0.0, a),
                    ColorSequenceKeypoint::new(1.0, b),
                ],
            }))
        })?,
    )?;
    globals.set("ColorSequence", cs)?;

    Ok(())
}

/// `Enum.FontWeight.Medium`, the number 500, or nothing, into a `FontWeight`.
///
/// NOTHING MEANS REGULAR, which is the engine's default and `Font::default`'s.
/// Anything else is an error rather than a default: a weight the host did not
/// understand, silently drawn regular, is a UI that renders and is wrong.
fn font_weight(value: Option<&LuaValue>) -> LuaResult<FontWeight> {
    let Some(value) = value.filter(|v| !v.is_nil()) else {
        return Ok(FontWeight::default());
    };
    let raw = match value {
        LuaValue::UserData(ud) => ud
            .borrow::<super::enums::LuaEnumItem>()
            .ok()
            .map(|item| item.value as u16),
        other => number(other).map(|n| n as u16),
    };
    raw.and_then(FontWeight::from_u16)
        .ok_or_else(|| LuaError::runtime("Font.new's weight must be an Enum.FontWeight member"))
}

fn font_style(value: Option<&LuaValue>) -> LuaResult<FontStyle> {
    let Some(value) = value.filter(|v| !v.is_nil()) else {
        return Ok(FontStyle::default());
    };
    let raw = match value {
        LuaValue::UserData(ud) => ud
            .borrow::<super::enums::LuaEnumItem>()
            .ok()
            .map(|item| item.value as u8),
        other => number(other).map(|n| n as u8),
    };
    raw.and_then(FontStyle::from_u8)
        .ok_or_else(|| LuaError::runtime("Font.new's style must be an Enum.FontStyle member"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm() -> Lua {
        let lua = Lua::new();
        install(&lua).expect("install");
        lua
    }

    fn eval<T: FromLuaMulti>(src: &str) -> LuaResult<T> {
        vm().load(src).eval()
    }

    #[test]
    fn each_type_reports_its_own_name_to_typeof() {
        // The reason these are five Rust types rather than one enum. `__type` is
        // a string on the metatable, so it belongs to the type; one wrapper would
        // have to answer the same name for all five, and a guest branching on
        // `typeof` would take the wrong path on four of them.
        for (expression, expected) in [
            ("UDim.new(0, 1)", "UDim"),
            ("UDim2.new(0, 1, 0, 2)", "UDim2"),
            ("Vector2.new(1, 2)", "Vector2"),
            ("Color3.new(1, 0, 0)", "Color3"),
            ("Rect.new(0, 0, 1, 1)", "Rect"),
        ] {
            let got: String = eval(&format!("return typeof({expression})")).expect("eval");
            assert_eq!(got, expected, "typeof({expression})");
        }
    }

    #[test]
    fn a_udim2_reads_back_its_parts() {
        let got: Vec<f64> = eval(
            r#"
            local size = UDim2.new(0.5, 10, 0.25, 20)
            return { size.X.Scale, size.X.Offset, size.Y.Scale, size.Y.Offset }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![0.5, 10.0, 0.25, 20.0]);
    }

    #[test]
    fn udim2_accepts_two_udims_as_well_as_four_numbers() {
        let same: bool = eval(
            r#"
            return UDim2.new(UDim.new(0.5, 10), UDim.new(0.25, 20))
                == UDim2.new(0.5, 10, 0.25, 20)
        "#,
        )
        .expect("eval");
        assert!(same);
    }

    #[test]
    fn width_and_height_alias_x_and_y() {
        let same: bool = eval(
            r#"
            local s = UDim2.new(0.5, 10, 0.25, 20)
            return s.Width == s.X and s.Height == s.Y
        "#,
        )
        .expect("eval");
        assert!(same);
    }

    #[test]
    fn from_scale_and_from_offset_are_the_shorthands_they_look_like() {
        let ok: bool = eval(
            r#"
            return UDim2.fromScale(1, 0.5) == UDim2.new(1, 0, 0.5, 0)
               and UDim2.fromOffset(30, 40) == UDim2.new(0, 30, 0, 40)
        "#,
        )
        .expect("eval");
        assert!(ok);
    }

    #[test]
    fn from_rgb_converts_to_the_engine_range() {
        // Bytes in, 0-to-1 stored. A host that kept bytes would disagree with
        // every colour in the conformance suite.
        let got: f32 = eval("return Color3.fromRGB(255, 0, 0).R").expect("eval");
        assert_eq!(got, 1.0);
        let got: f32 = eval("return Color3.fromRGB(51, 0, 0).R").expect("eval");
        assert!((got - 0.2).abs() < 1e-6, "{got}");
    }

    #[test]
    fn a_fractional_offset_truncates_towards_zero_the_way_the_engine_does() {
        // THIS TEST USED TO ASSERT THE OPPOSITE, and the assertion was the whole
        // of the rule: a fraction was refused, on the grounds that truncating
        // 10.7 to 10 produces a UI that is subtly wrong everywhere and blames
        // nobody. What was never checked is what the engine does, and the engine
        // converts -- measured against lune's implementation of the datatype,
        // which is built on the same `rbx_types` this host is:
        //
        //     UDim2.fromOffset(0, 8.7138671875).Y.Offset  ->   8
        //     UDim.new(0, -2.5).Offset                    ->  -2
        //
        // So the refusal was a divergence rather than a stricter reading, and it
        // made a whole class of correct program impossible: the fraction that
        // found it was a TEXT MEASUREMENT travelling through a layout solver into
        // `Host.SetBounds`, and text metrics are fractional by nature.
        let got: i32 = eval("return UDim.new(0, 10.7).Offset").expect("eval");
        assert_eq!(got, 10);
        let got: i32 = eval("return UDim2.fromOffset(0, 8.7138671875).Y.Offset").expect("eval");
        assert_eq!(got, 8);
        // TOWARDS ZERO, NOT DOWNWARDS, and the negative case is the only one that
        // can tell those apart. `floor` would answer -3 here.
        let got: i32 = eval("return UDim.new(0, -2.5).Offset").expect("eval");
        assert_eq!(got, -2);
        let got: i32 = eval("return UDim2.new(0, 1.9, 0, -1.9).X.Offset").expect("eval");
        assert_eq!(got, 1);
    }

    #[test]
    fn an_int32_property_still_refuses_a_fraction() {
        // THE HALF THAT DID NOT CHANGE, asserted so that it cannot drift with the
        // half that did. `pixel_i32` covers the vocabulary constructors, where the
        // engine's behaviour is measured; `whole_i32` still covers writing an
        // `Int32`-typed property, where it is not. Two questions that shared a
        // function until sprint 6, and only one of them has an answer.
        let lua = Lua::new();
        super::super::install(&lua, &Default::default()).expect("install");
        install(&lua).expect("vocabulary");
        let err = lua
            .load(r#"Instance.new("Frame").ZIndex = 1.5"#)
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("whole number"), "{err}");
    }

    #[test]
    fn the_vocabulary_answers_every_name_aether_requires() {
        // A SUPERSET, ASSERTED AS ONE, and the list is not decoration.
        //
        // Aether's `REQUIRED_VOCABULARY` names eleven types, and under its
        // DataModel host `InstallVocabulary` is `function() end` -- so on this
        // host these globals are the ONLY vocabulary a mod has. A partial one is
        // worse than none: `available()` asks whether six of the names exist and
        // nothing about their members, so a missing constructor passes the probe,
        // gets the host selected, and surfaces as a nil index inside a component.
        //
        // That is measured history rather than a worry. `Color3.fromHex` has 17
        // call sites in `applets/` and 5 in Aether's own source, this host had `new`
        // and `fromRGB` and not `fromHex`, and installing the partial vocabulary
        // into an Aether mod VM stopped all three mods loading.
        //
        // Aether's `DataModel.new` refuses an environment failing this list, by
        // name. This is the same list from the other side, so the two cannot drift
        // apart silently.
        for expression in [
            "UDim.new(0, 1)",
            "UDim2.new(0, 1, 0, 2)",
            "UDim2.fromScale(1, 1)",
            "UDim2.fromOffset(1, 1)",
            "Vector2.new(1, 1)",
            "Vector2.zero",
            "Vector2.one",
            "Color3.new(1, 1, 1)",
            "Color3.fromRGB(255, 255, 255)",
            "Color3.fromHex(\"#FFFFFF\")",
            "Color3.fromHSV(0.5, 0.5, 0.5)",
            "Rect.new(0, 0, 1, 1)",
            "Font.new(\"SourceSans\")",
            "NumberSequence.new(0, 1)",
            "NumberSequenceKeypoint.new(0, 1)",
            "ColorSequence.new(Color3.new(1, 1, 1))",
            "ColorSequenceKeypoint.new(0, Color3.new(1, 1, 1))",
        ] {
            let got: bool = eval(&format!("return ({expression}) ~= nil"))
                .unwrap_or_else(|e| panic!("{expression}: {e}"));
            assert!(got, "{expression} answered nil");
        }
    }

    #[test]
    fn from_hex_accepts_both_lengths_and_refuses_anything_else() {
        let got: f32 = eval("return Color3.fromHex(\"#FF0000\").R").expect("eval");
        assert_eq!(got, 1.0);
        // "#RGB" expands each digit, so "#F00" is "#FF0000" and not "#0F0000".
        let got: f32 = eval("return Color3.fromHex(\"F00\").R").expect("eval");
        assert_eq!(got, 1.0);
        let got: f32 = eval("return Color3.fromHex(\"#888888\").G").expect("eval");
        assert!((got - 0.5333).abs() < 1e-3, "{got}");
        // AN ERROR RATHER THAN BLACK. A theme colour silently reading as black is
        // a UI that renders and is wrong.
        let err = vm()
            .load("return Color3.fromHex(\"#GGGGGG\")")
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("hexadecimal"), "{err}");
        let err = vm()
            .load("return Color3.fromHex(\"#FFFF\")")
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("#RRGGBB"), "{err}");
    }

    #[test]
    fn a_font_reports_its_weight_as_an_enum_item() {
        // Through the reflection database, so `Font.Weight` compared against
        // `Enum.FontWeight.Bold` answers true rather than plausibly.
        let lua = Lua::new();
        install(&lua).expect("vocabulary");
        super::super::enums::install(&lua).expect("enums");
        let ok: bool = lua
            .load(
                r#"
                local f = Font.new("SourceSans", Enum.FontWeight.Bold, Enum.FontStyle.Italic)
                return f.Family == "SourceSans"
                    and f.Weight == Enum.FontWeight.Bold
                    and f.Style == Enum.FontStyle.Italic
                    and f.Bold == true
                    and typeof(f) == "Font"
            "#,
            )
            .eval()
            .expect("eval");
        assert!(ok);
    }

    #[test]
    fn a_sequence_carries_its_keypoints() {
        let ok: bool = eval(
            r#"
            local n = NumberSequence.new(0, 1)
            local c = ColorSequence.new(Color3.new(1, 0, 0), Color3.new(0, 0, 1))
            return #n.Keypoints == 2
                and n.Keypoints[1].Time == 0 and n.Keypoints[2].Value == 1
                and #c.Keypoints == 2
                and c.Keypoints[2].Value == Color3.new(0, 0, 1)
                and typeof(n) == "NumberSequence"
                and typeof(c.Keypoints[1]) == "ColorSequenceKeypoint"
        "#,
        )
        .expect("eval");
        assert!(ok);
    }

    #[test]
    fn udim2_arithmetic_is_componentwise() {
        let ok: bool = eval(
            r#"
            local a = UDim2.new(0.5, 10, 0.25, 20)
            local b = UDim2.new(0.5, 5, 0.25, 5)
            return (a + b) == UDim2.new(1, 15, 0.5, 25)
               and (a - b) == UDim2.new(0, 5, 0, 15)
        "#,
        )
        .expect("eval");
        assert!(ok);
    }

    #[test]
    fn adding_a_udim2_to_something_else_is_an_error() {
        let err = vm()
            .load("return UDim2.new() + 3")
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("can only be added to a UDim2"), "{err}");
    }

    #[test]
    fn a_vector2_scales_by_a_number() {
        let ok: bool = eval("return Vector2.new(2, 3) * 2 == Vector2.new(4, 6)").expect("eval");
        assert!(ok);
    }

    #[test]
    fn a_rect_reports_its_extent() {
        let got: Vec<f32> = eval(
            r#"
            local r = Rect.new(10, 20, 40, 60)
            return { r.Min.X, r.Min.Y, r.Width, r.Height }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![10.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn a_value_of_a_different_vocabulary_type_is_not_equal() {
        let equal: bool = eval("return Vector2.new(1, 2) == UDim2.new(1, 2, 0, 0)").expect("eval");
        assert!(!equal);
    }
}
