//! `Enum.AutomaticSize.Y` and the three hundred others.
//!
//! WHY THIS IS THE BIGGEST REMAINING BLOCK
//! Of the 46 properties left needing host and rendering work after the
//! vocabulary landed, most are enums: `ApplyStrokeMode`, `AspectType`,
//! `BorderMode`, `DominantAxis`, `EasingStyle`, `FlexMode`,
//! `HorizontalAlignment`, `ScaleType`, `SortOrder`, `TextTruncate` and the rest.
//! They are one mechanism, not twenty features.
//!
//! GENERATED, NOT LISTED. Every name and value comes from the same reflection
//! database the property surface is measured with, so `Enum` here and `Enum` in
//! the engine cannot disagree about what exists. A hand-written table of three
//! hundred enums would be wrong within one Roblox release, which is the argument
//! `datamodel_surface` already makes about properties.
//!
//! LAZY, because eagerly building every item of every enum would allocate
//! thousands of userdata per VM to answer a handful of lookups. `Enum` resolves a
//! category on indexing, and a category resolves an item on indexing.
//!
//! THREE TYPES, THREE `__type` ANSWERS. Roblox reports "Enums" for the root,
//! "Enum" for a category and "EnumItem" for a member, and a guest that branches
//! on `typeof` needs all three -- the same reason the vocabulary is five Rust
//! types rather than one.

use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};

/// One member, e.g. `Enum.AutomaticSize.Y`.
#[derive(Clone, Copy, PartialEq)]
pub struct LuaEnumItem {
    pub ty: &'static str,
    pub name: &'static str,
    pub value: u32,
}

/// One category, e.g. `Enum.AutomaticSize`.
#[derive(Clone, Copy)]
pub struct LuaEnumCategory {
    pub name: &'static str,
}

/// The root, `Enum`.
#[derive(Clone, Copy)]
pub struct LuaEnums;

fn database() -> LuaResult<&'static rbx_reflection::ReflectionDatabase<'static>> {
    rbx_reflection_database::get()
        .map_err(|e| LuaError::runtime(format!("the reflection database is unavailable: {e}")))
}

/// The descriptor for one enum, by name.
fn category(name: &str) -> Option<&'static rbx_reflection::EnumDescriptor<'static>> {
    rbx_reflection_database::get().ok()?.enums.get(name)
}

/// The member of `ty` with this value, if the value is one this enum has.
///
/// USED WHEN READING A PROPERTY BACK. `Variant::Enum` stores a bare `u32` with no
/// record of which enum it belongs to, so presenting it to a guest needs the
/// property's declared type from the reflection database -- the value alone is not
/// enough to name itself.
pub fn item_by_value(ty: &str, value: u32) -> Option<LuaEnumItem> {
    let descriptor = category(ty)?;
    descriptor
        .items
        .iter()
        .find(|(_, v)| **v == value)
        .map(|(name, v)| LuaEnumItem {
            ty: descriptor.name,
            name,
            value: *v,
        })
}

/// The member of `ty` with this name.
///
/// THE HOST'S OWN DIRECTION OF LOOKUP. [`item_by_value`] serves a guest reading
/// a property back, where a bare `u32` has to be given a name. This serves the
/// host handing a guest an enum item it decided on -- `Enum.UserInputType`
/// `.MouseButton1` on an `InputObject` -- where the name is what the host knows
/// and the value is the detail. Generated from the same database, so an item
/// this host names and an item the engine names cannot disagree.
///
/// A NAME THE DATABASE DOES NOT CARRY RETURNS `None` rather than a placeholder
/// item. The callers are all host-side constants, so `None` means this host
/// spelled one wrong -- and an `EnumItem` with a made-up value would travel into
/// a guest's comparison and answer false against the real one.
pub fn item_by_name(ty: &str, name: &str) -> Option<LuaEnumItem> {
    let descriptor = category(ty)?;
    descriptor
        .items
        .get_key_value(name)
        .map(|(name, value)| LuaEnumItem {
            ty: descriptor.name,
            name,
            value: *value,
        })
}

/// Is `value` a member of `ty`?
pub fn value_is_valid(ty: &str, value: u32) -> bool {
    category(ty).is_some_and(|d| d.items.values().any(|v| *v == value))
}

impl UserData for LuaEnumItem {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "EnumItem");
        fields.add_field_method_get("Name", |_, this| Ok(this.name));
        fields.add_field_method_get("Value", |_, this| Ok(this.value));
        fields.add_field_method_get("EnumType", |_, this| Ok(LuaEnumCategory { name: this.ty }));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Enum.{}.{}", this.ty, this.name))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            let LuaValue::UserData(ud) = other else {
                return Ok(false);
            };
            Ok(ud.borrow::<LuaEnumItem>().is_ok_and(|o| *o == *this))
        });
    }
}

impl UserData for LuaEnumCategory {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "Enum");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(this.name.to_string())
        });

        // EVERYTHING GOES THROUGH `__index`, INCLUDING THE METHOD. mlua composes
        // its own method dispatch with a user `Index` handler, and relying on the
        // order it does that in is relying on an implementation detail of a
        // dependency. Handling both here is a few lines and cannot change under
        // us.
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: String| {
            if key == "GetEnumItems" {
                let ty = this.name;
                return lua
                    .create_function(move |_, ()| {
                        let descriptor = category(ty).ok_or_else(|| {
                            LuaError::runtime(format!(
                                "Enum.{ty} is not in the reflection database"
                            ))
                        })?;
                        // SORTED BY VALUE, so the order is the engine's rather
                        // than a hash map's. An iteration order that changes per
                        // run makes a UI built from it change per run.
                        let mut items: Vec<LuaEnumItem> = descriptor
                            .items
                            .iter()
                            .map(|(name, value)| LuaEnumItem {
                                ty: descriptor.name,
                                name,
                                value: *value,
                            })
                            .collect();
                        items.sort_by_key(|i| i.value);
                        Ok(items)
                    })?
                    .into_lua(lua);
            }

            let descriptor = category(this.name).ok_or_else(|| {
                LuaError::runtime(format!(
                    "Enum.{} is not in the reflection database",
                    this.name
                ))
            })?;
            match descriptor.items.get_key_value(key.as_str()) {
                Some((name, value)) => LuaEnumItem {
                    ty: descriptor.name,
                    name,
                    value: *value,
                }
                .into_lua(lua),
                // NAMED, NOT NIL. `Enum.AutomaticSize.Both` is a typo for `XY`,
                // and nil would travel into a property assignment and be reported
                // there as "expects an EnumItem, got nil" -- a message about the
                // wrong line.
                None => Err(LuaError::runtime(format!(
                    "{key} is not a member of Enum.{}",
                    this.name
                ))),
            }
        });
    }
}

impl UserData for LuaEnums {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "Enums");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, _, ()| Ok("Enums"));
        methods.add_meta_method(MetaMethod::Index, |lua, _, key: String| {
            let db = database()?;
            match db.enums.get_key_value(key.as_str()) {
                Some((name, _)) => LuaEnumCategory { name }.into_lua(lua),
                None => Err(LuaError::runtime(format!("Enum.{key} does not exist"))),
            }
        });
    }
}

pub fn install(lua: &Lua) -> LuaResult<()> {
    lua.globals().set("Enum", LuaEnums)?;
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
    fn the_three_levels_report_their_own_names_to_typeof() {
        for (expression, expected) in [
            ("Enum", "Enums"),
            ("Enum.AutomaticSize", "Enum"),
            ("Enum.AutomaticSize.Y", "EnumItem"),
        ] {
            let got: String = eval(&format!("return typeof({expression})")).expect("eval");
            assert_eq!(got, expected, "typeof({expression})");
        }
    }

    #[test]
    fn an_item_carries_its_name_value_and_type() {
        let got: (String, u32, String) = eval(
            r#"
            local item = Enum.AutomaticSize.Y
            return item.Name, item.Value, tostring(item.EnumType)
        "#,
        )
        .expect("eval");
        assert_eq!(got.0, "Y");
        assert_eq!(got.2, "AutomaticSize");
        // The value is the engine's, from the reflection database, not a guess.
        assert_eq!(got.1, item_by_value("AutomaticSize", got.1).unwrap().value);
    }

    #[test]
    fn tostring_matches_the_engine_shape() {
        let got: String = eval("return tostring(Enum.FillDirection.Vertical)").expect("eval");
        assert_eq!(got, "Enum.FillDirection.Vertical");
    }

    #[test]
    fn two_references_to_one_member_compare_equal() {
        let got: bool = eval("return Enum.AutomaticSize.Y == Enum.AutomaticSize.Y").expect("eval");
        assert!(got);
    }

    #[test]
    fn members_of_different_enums_are_not_equal() {
        // Both are value 1 in their own enum, so comparing on value alone would
        // call these the same thing.
        let got: bool =
            eval("return Enum.AutomaticSize.X == Enum.FillDirection.Vertical").expect("eval");
        assert!(!got);
    }

    #[test]
    fn a_misspelled_member_is_named_rather_than_nil() {
        let err = vm()
            .load("return Enum.AutomaticSize.Both")
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("Both"), "{err}");
        assert!(err.contains("AutomaticSize"), "{err}");
    }

    #[test]
    fn a_misspelled_enum_is_named_rather_than_nil() {
        let err = vm()
            .load("return Enum.AutomaticSizes")
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("AutomaticSizes"), "{err}");
        assert!(err.contains("does not exist"), "{err}");
    }

    #[test]
    fn get_enum_items_returns_every_member_in_value_order() {
        let got: Vec<String> = eval(
            r#"
            local names = {}
            for _, item in ipairs(Enum.AutomaticSize:GetEnumItems()) do
                table.insert(names, item.Name)
            end
            return names
        "#,
        )
        .expect("eval");
        // None, X, Y, XY in the engine, and in that order because the list is
        // sorted by value rather than left in hash order.
        assert_eq!(got, vec!["None", "X", "Y", "XY"]);
    }
}
