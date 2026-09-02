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
//! `Vector3`, `CFrame` and the sequence types. This standard is 2D UI: a host
//! that implemented `CFrame` would be describing Roblox rather than describing a
//! UI, which is the same test `OUT_OF_SCOPE` applies to classes.
//!
//! NUMBERS ARE NOT ROUNDED FOR YOU. `UDim.Offset` is an `i32` in the engine, and
//! a guest passing 10.7 gets an error rather than a 10 -- silently truncating a
//! layout value produces a UI that is subtly wrong everywhere and blames nobody.

use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use rbx_types::{Color3, Rect, UDim, UDim2, Vector2};

use super::{number, whole_i32};

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
        // `Width` and `Height` are what a UI author reaches for, and Roblox has
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
        // SCALAR ONLY, either way round. Roblox also multiplies two Vector2s
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

/// Install the constructors as globals.
///
/// GLOBALS, like `Instance`, and for the same reason: this is the language of
/// the platform rather than a capability. A guest that had to be handed `UDim2`
/// would not be the guest that runs on Roblox.
pub fn install(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();

    let udim = lua.create_table()?;
    udim.set(
        "new",
        lua.create_function(|_, (scale, offset): (Option<f32>, Option<f64>)| {
            Ok(LuaUDim(UDim::new(
                scale.unwrap_or(0.0),
                whole_i32(offset.unwrap_or(0.0), "UDim.new's offset")?,
            )))
        })?,
    )?;
    globals.set("UDim", udim)?;

    let udim2 = lua.create_table()?;
    // FOUR NUMBERS OR TWO UDims, because Roblox accepts both and a guest written
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
                UDim::new(
                    number(0) as f32,
                    whole_i32(number(1), "UDim2.new's X offset")?,
                ),
                UDim::new(
                    number(2) as f32,
                    whole_i32(number(3), "UDim2.new's Y offset")?,
                ),
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
                UDim::new(0.0, whole_i32(x.unwrap_or(0.0), "UDim2.fromOffset's X")?),
                UDim::new(0.0, whole_i32(y.unwrap_or(0.0), "UDim2.fromOffset's Y")?),
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

    Ok(())
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
    fn a_fractional_offset_is_refused_rather_than_truncated() {
        // Truncating silently produces a UI that is subtly wrong everywhere and
        // blames nobody.
        let err = vm()
            .load("return UDim.new(0, 10.7)")
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("whole number"), "{err}");
        assert!(err.contains("10.7"), "{err}");
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
