//! `Content`, `ContentId` and `Font` -- the last thirteen properties.
//!
//! WHAT THIS SLICE IS, AND WHAT IT IS NOT
//! This is the VALUE layer: a guest can hold a `Content`, assign it to `Image` or
//! `ImageContent`, and read it back. It does not fetch anything, cache anything,
//! or decide whether a scheme is one this host will resolve.
//!
//! That split is deliberate and it is the ADR-003 rule applied to ourselves.
//! **Resolution has no consumer yet.** Dew's renderer does not read the DataModel
//! -- mods build Aether's tree and Aether emits the display list -- so a scheme
//! registry, a permission gate and a content-addressed cache written today would
//! be three mechanisms nothing calls. That is the exact shape this milestone has
//! spent its length removing: `rbx_reflection` sat in `Cargo.toml` for months
//! described in the present tense with no importer, and the manifest accepted
//! `hotkeys` nothing ever bound. Building the gate before the thing it gates
//! would be the same mistake with a fresher comment on it.
//!
//! So `Permission` gains no variant here. It gains one in the sprint that can
//! demonstrate a fetch being refused.
//!
//! WHAT THE STANDARD ALREADY DECIDES, and this file obeys
//! Assignment ACCEPTS any well-formed URI. ADR-003: "an unresolvable `Content` is
//! a rendering outcome, not a property error -- the assignment succeeds and the
//! host reports why nothing was drawn." An engine application moved to Dew with a
//! grant withheld is a correct application missing an image, not a broken one, and
//! a host that refused the assignment would make it a broken one.
//!
//! TWO GENERATIONS OF ONE IDEA. `Image` is the legacy `ContentId`, a bare string;
//! `ImageContent` is the modern `Content`, a URI. The backlog paired them all the
//! way down -- `TopImage`/`TopImageContent`, `HoverImage`/`HoverImageContent` --
//! and a host has to take both.

use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use rbx_types::{Content, ContentType, Font, FontStyle, FontWeight};

use super::enums::LuaEnumItem;

/// A `Content` value a guest holds.
#[derive(Clone, PartialEq)]
pub struct LuaContent(pub Content);

/// A `Font` value a guest holds.
#[derive(Clone, PartialEq)]
pub struct LuaFont(pub Font);

impl LuaContent {
    pub fn from_value(value: &LuaValue) -> Option<Content> {
        // A PLAIN STRING IS A `Content`, because the engine coerces one and a
        // guest written against it writes `ImageContent = "rbxassetid://123"`
        // without thinking about the type. Refusing that would make correct
        // The engine code fail here for a reason no message could usefully explain.
        if let Some(text) = value.as_string() {
            let text = text.to_string_lossy();
            return Some(if text.is_empty() {
                Content::none()
            } else {
                Content::from_uri(text)
            });
        }
        let LuaValue::UserData(ud) = value else {
            return None;
        };
        ud.borrow::<LuaContent>().ok().map(|c| c.0.clone())
    }
}

impl LuaFont {
    pub fn from_value(value: &LuaValue) -> Option<Font> {
        let LuaValue::UserData(ud) = value else {
            return None;
        };
        ud.borrow::<LuaFont>().ok().map(|f| f.0.clone())
    }
}

/// The `Enum.ContentSourceType` member naming what a `Content` holds.
fn source_type(content: &Content) -> &'static str {
    match content.value() {
        ContentType::None => "None",
        ContentType::Uri(_) => "Uri",
        ContentType::Object(_) => "Object",
        // `ContentType` is `#[non_exhaustive]`: the engine adds source types, and a
        // host that matched exhaustively would stop compiling on a crate bump
        // rather than reporting an unknown one.
        _ => "Unknown",
    }
}

impl UserData for LuaContent {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "Content");
        fields.add_field_method_get("Uri", |_, this| Ok(this.0.as_uri().map(str::to_owned)));
        fields.add_field_method_get("SourceType", |_, this| Ok(source_type(&this.0)));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(this.0.as_uri().unwrap_or("").to_string())
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaContent::from_value(&other).as_ref() == Some(&this.0))
        });
    }
}

impl UserData for LuaFont {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "Font");
        fields.add_field_method_get("Family", |_, this| Ok(this.0.family.clone()));
        fields.add_field_method_get("Weight", |_, this| Ok(this.0.weight.as_u16()));
        fields.add_field_method_get("Style", |_, this| Ok(this.0.style.as_u8()));
        // `Bold` is what a guest actually branches on, and the engine has it.
        fields.add_field_method_get("Bold", |_, this| {
            Ok(this.0.weight.as_u16() >= FontWeight::Bold.as_u16())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "Font {{ Family = {}, Weight = {}, Style = {} }}",
                this.0.family,
                this.0.weight.as_u16(),
                this.0.style.as_u8()
            ))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaFont::from_value(&other).as_ref() == Some(&this.0))
        });
    }
}

/// One enum argument, accepted as a member or as its number, like the engine.
fn enum_argument(value: &LuaValue, want: &str, what: &str) -> LuaResult<Option<u32>> {
    match value {
        LuaValue::Nil => Ok(None),
        LuaValue::UserData(ud) => {
            let item = ud
                .borrow::<LuaEnumItem>()
                .map_err(|_| LuaError::runtime(format!("{what} expects an Enum.{want}")))?;
            if item.ty != want {
                return Err(LuaError::runtime(format!(
                    "{what} expects an Enum.{want}, got an Enum.{}",
                    item.ty
                )));
            }
            Ok(Some(item.value))
        }
        other => match super::number(other) {
            Some(n) => Ok(Some(super::whole_i32(n, what)? as u32)),
            None => Err(LuaError::runtime(format!(
                "{what} expects an Enum.{want}, got {}",
                other.type_name()
            ))),
        },
    }
}

pub fn install(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();

    let content = lua.create_table()?;
    content.set(
        "fromUri",
        lua.create_function(|_, uri: String| Ok(LuaContent(Content::from_uri(uri))))?,
    )?;
    content.set("none", LuaContent(Content::none()))?;
    // `Content.fromObject` is deliberately absent. It references another instance
    // by referent, and this host has no referent to hand out yet -- an
    // `InstanceRef` is an arena index, not an `rbx_types::Ref`. Absent and
    // erroring at the call site beats present and wrong.
    globals.set("Content", content)?;

    let font = lua.create_table()?;
    font.set(
        "new",
        lua.create_function(|_, (family, weight, style): (String, LuaValue, LuaValue)| {
            let weight = match enum_argument(&weight, "FontWeight", "Font.new's weight")? {
                Some(raw) => FontWeight::from_u16(raw as u16).ok_or_else(|| {
                    LuaError::runtime(format!("there is no Enum.FontWeight numbered {raw}"))
                })?,
                None => FontWeight::default(),
            };
            let style = match enum_argument(&style, "FontStyle", "Font.new's style")? {
                Some(raw) => FontStyle::from_u8(raw as u8).ok_or_else(|| {
                    LuaError::runtime(format!("there is no Enum.FontStyle numbered {raw}"))
                })?,
                None => FontStyle::default(),
            };
            Ok(LuaFont(Font::new(&family, weight, style)))
        })?,
    )?;
    globals.set("Font", font)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm() -> Lua {
        let lua = Lua::new();
        super::super::enums::install(&lua).expect("enums");
        install(&lua).expect("content");
        lua
    }

    fn eval<T: FromLuaMulti>(src: &str) -> LuaResult<T> {
        vm().load(src).eval()
    }

    #[test]
    fn content_and_font_report_their_own_names_to_typeof() {
        for (expression, expected) in [
            (r#"Content.fromUri("rbxassetid://1")"#, "Content"),
            ("Content.none", "Content"),
            (r#"Font.new("Inter")"#, "Font"),
        ] {
            let got: String = eval(&format!("return typeof({expression})")).expect("eval");
            assert_eq!(got, expected, "typeof({expression})");
        }
    }

    #[test]
    fn a_content_carries_its_uri_and_source_type() {
        let got: (String, String) = eval(
            r#"
            local c = Content.fromUri("rbxassetid://12345")
            return c.Uri, c.SourceType
        "#,
        )
        .expect("eval");
        assert_eq!(got.0, "rbxassetid://12345");
        assert_eq!(got.1, "Uri");
    }

    #[test]
    fn an_empty_content_has_no_uri() {
        let got: (Option<String>, String) =
            eval("return Content.none.Uri, Content.none.SourceType").expect("eval");
        assert_eq!(got.0, None);
        assert_eq!(got.1, "None");
    }

    #[test]
    fn a_plain_string_is_a_content() {
        // The engine coerces one, and a guest written against it assigns a string
        // without thinking about the type.
        let value = LuaContent::from_value(&LuaValue::Nil);
        assert!(value.is_none());

        let lua = vm();
        let text = lua.create_string("rbxassetid://7").expect("string");
        let got = LuaContent::from_value(&LuaValue::String(text)).expect("coerced");
        assert_eq!(got.as_uri(), Some("rbxassetid://7"));
    }

    #[test]
    fn an_empty_string_is_the_empty_content_rather_than_an_empty_uri() {
        let lua = vm();
        let text = lua.create_string("").expect("string");
        let got = LuaContent::from_value(&LuaValue::String(text)).expect("coerced");
        assert_eq!(got, Content::none());
    }

    #[test]
    fn a_font_defaults_its_weight_and_style() {
        let got: (String, u32, u32) = eval(
            r#"
            local f = Font.new("Inter")
            return f.Family, f.Weight, f.Style
        "#,
        )
        .expect("eval");
        assert_eq!(got.0, "Inter");
        assert_eq!(got.1, 400);
        assert_eq!(got.2, 0);
    }

    #[test]
    fn a_font_takes_enum_members_for_weight_and_style() {
        let got: (u32, u32, bool) = eval(
            r#"
            local f = Font.new("Inter", Enum.FontWeight.Bold, Enum.FontStyle.Italic)
            return f.Weight, f.Style, f.Bold
        "#,
        )
        .expect("eval");
        assert_eq!(got.0, 700);
        assert_eq!(got.1, 1);
        assert!(got.2);
    }

    #[test]
    fn a_font_refuses_a_member_of_the_wrong_enum() {
        let err = vm()
            .load(r#"return Font.new("Inter", Enum.FontStyle.Italic)"#)
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("expects an Enum.FontWeight"), "{err}");
        assert!(err.contains("got an Enum.FontStyle"), "{err}");
    }

    #[test]
    fn a_font_weight_the_enum_does_not_have_is_refused() {
        // 450 is between Regular and Medium and is not a member.
        let err = vm()
            .load(r#"return Font.new("Inter", 450)"#)
            .exec()
            .unwrap_err()
            .to_string();
        assert!(err.contains("no Enum.FontWeight numbered 450"), "{err}");
    }
}
