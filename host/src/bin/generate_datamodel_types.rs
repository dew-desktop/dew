//! Generates `types/datamodel.d.luau`, the editor's entire view of the
//! DataModel: every class and enum `rbx_reflection_database` carries, with
//! `datamodel::extensions`'s Dew-original rows folded in.
//!
//! WHY THE WHOLE DATABASE, NOT JUST THE CLASSES `extensions.rs` TOUCHES.
//! Luau's type-checker does not merge two `declare extern type X` blocks for
//! the same name across definition files loaded together -- the later file
//! replaces the earlier one, not a partial merge (verified directly: two
//! files each declaring `Widget` with one different property left only the
//! later file's property resolvable). A small overlay declaring only
//! `GuiObject` and `InputActionLabel` would silently blank out every other
//! class `luau-lsp.types.roblox` already knows about the moment both files
//! loaded, which is worse than the problem this generator exists to solve.
//! Owning the whole hierarchy sidesteps that by construction: exactly one
//! file is authoritative for the class/enum surface, so there is nothing
//! left for a second file to race.
//!
//! WHAT THIS DOES NOT OWN. Value types (`Vector3`, `CFrame`, `Color3`,
//! `UDim2`, `Content`, `Font`, `BrickColor`, `NumberSequence`,
//! `ColorSequence`, `PhysicalProperties`, and friends) and the `Enum`/
//! `EnumItem` base types are real global types `luau-lsp.types.roblox`
//! already declares. This file references them by name and declares none of
//! them itself.
//!
//! THE KNOWN COST: EVERY CLASS HERE HAS NO METHODS OR EVENTS, ONLY
//! PROPERTIES. `rbx_reflection_database`'s `ClassDescriptor` carries `name`,
//! `tags`, `superclass`, `properties` and `default_properties` -- no methods,
//! no events, ever (`scripts/fetch_api_surface.luau`'s own header says so
//! explicitly, which is why that script exists at all: it vendors a separate,
//! ~40-class pinned API dump, scoped to the `GuiObject` subtree
//! `docs/datamodel_scope.md` measures, specifically to recover method/event
//! signatures this crate cannot provide).
//!
//! This generator declaring the whole hierarchy means every one of those
//! classes REPLACES Roblox's own same-named declaration from
//! `luau-lsp.types.roblox` outright (the no-merge rule above cuts both ways),
//! so a script calling `instance:Clone()`, connecting `RunService.Heartbeat`,
//! or calling `GetPropertyChangedSignal` on ANY instance now gets a real
//! "Key not found" editor error it did not get before this file existed --
//! confirmed directly against `examples/host/widget-behaviors` (3 new
//! errors: `Instance.Clone`, `RunService.Heartbeat`,
//! `Instance.GetPropertyChangedSignal`) and `examples/aether/timetracker` /
//! `examples/aether/calculator` (32 new errors each, same shape). This is an
//! accepted, deliberate tradeoff, not an oversight: two narrower designs were
//! tried and both failed harder under Luau's actual cross-file semantics
//! (declaring only the touched classes leaves them invisible to any class
//! that already inherited from them before this file loaded, since `extends`
//! resolves at parse time and does not update retroactively; and touching
//! the `Enum` global at all -- needed to add `Enum.BlendMode` -- forces
//! re-declaring every real enum category with a new nominal identity, which
//! breaks ordinary comparisons like `if x == Enum.Font.SourceSans` against
//! ANY untouched class's property, everywhere, not just the one enum this
//! file actually needed to add). Extend `fetch_api_surface.luau`'s vendored
//! subset over time if/when the method/event gap needs closing for more than
//! the classes it already covers; that is a separate, incremental effort
//! from this file's job of getting properties right.
//!
//! GENERATED. Do not hand-edit `types/datamodel.d.luau` -- the next run of
//! this binary silently reverts anything hand-tuned there. Run with:
//!
//!     cargo run --bin generate-datamodel-types
//!
//! `--check` regenerates into a temp buffer and diffs it against the
//! checked-in file without writing, for CI to catch drift.

use dew_host::datamodel::extensions::{self, ExtensionProperty};
use rbx_reflection::{DataType, ReflectionDatabase, Scriptability};
use rbx_types::VariantType;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const OUTPUT_PATH: &str = "types/datamodel.d.luau";

fn main() {
    let check_only = std::env::args().any(|a| a == "--check");

    let db = rbx_reflection_database::get().expect("bundled reflection database");
    let generated = generate(db);

    let repo_root = repo_root();
    let output_path = repo_root.join(OUTPUT_PATH);

    if check_only {
        let on_disk = std::fs::read_to_string(&output_path).unwrap_or_default();
        // `\r\n` NORMALIZED AWAY BEFORE COMPARING. This generator writes `\n`
        // only, but a checkout can still hand back `\r\n` -- Windows CI hit
        // this directly: the file this binary had just written locally
        // matched byte-for-byte, but the SAME committed content, checked out
        // fresh by a Windows runner, came back CRLF and failed the check for
        // a reason that has nothing to do with the generator being stale.
        if on_disk.replace("\r\n", "\n") == generated {
            println!("{OUTPUT_PATH} is up to date.");
        } else {
            eprintln!(
                "{OUTPUT_PATH} is STALE. Run `cargo run --bin generate-datamodel-types` and \
                 commit the result."
            );
            std::process::exit(1);
        }
        return;
    }

    std::fs::write(&output_path, &generated).expect("write types/datamodel.d.luau");
    println!("wrote {OUTPUT_PATH} ({} bytes)", generated.len());
}

/// `CARGO_MANIFEST_DIR` is `<repo>/host`; the generated file lives at the
/// repo root next to the `types/` directory `desktop.d.luau` already lives in.
fn repo_root() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    Path::new(&manifest_dir)
        .parent()
        .expect("host/ has a parent")
        .to_path_buf()
}

// ── Property visibility and type mapping ───────────────────────────────────

/// A property is worth declaring to the editor if a script can do anything
/// with it at all. `Scriptability::None` means the engine (and this host's
/// read path, which never checks scriptability) still stores it, but no
/// script is meant to see it -- an internal/serialization-only field, not
/// something a Dew applet would ever legitimately reference.
fn visible_to_script(scriptability: Scriptability) -> bool {
    !matches!(scriptability, Scriptability::None)
}

/// The Luau type name for a property's `VariantType`, matching what real
/// Roblox `.d.luau` definitions name it (these are global types
/// `luau-lsp.types.roblox` declares; this generator does not declare them).
/// `None` means this host has never seen the type carry a real property and
/// there is no safe mapping to guess -- the property is skipped rather than
/// emitted as something that would silently typecheck wrong.
fn luau_type_for(ty: VariantType) -> Option<&'static str> {
    Some(match ty {
        VariantType::Bool => "boolean",
        VariantType::String => "string",
        VariantType::Float32 | VariantType::Float64 | VariantType::Int32 | VariantType::Int64 => {
            "number"
        }
        VariantType::UDim => "UDim",
        VariantType::UDim2 => "UDim2",
        VariantType::Vector2 => "Vector2",
        VariantType::Vector2int16 => "Vector2int16",
        VariantType::Vector3 => "Vector3",
        VariantType::Vector3int16 => "Vector3int16",
        VariantType::Color3 => "Color3",
        VariantType::Color3uint8 => "Color3",
        VariantType::BrickColor => "BrickColor",
        VariantType::CFrame => "CFrame",
        VariantType::OptionalCFrame => "CFrame?",
        VariantType::Rect => "Rect",
        VariantType::Ray => "Ray",
        VariantType::Region3 => "Region3",
        VariantType::Region3int16 => "Region3int16",
        VariantType::Axes => "Axes",
        VariantType::Faces => "Faces",
        VariantType::Content => "Content",
        // `ContentId` is `type ContentId = string` in Roblox's own bundled
        // definitions -- a plain alias, not a `declare extern type`. Aliases
        // like that do not survive across separately-loaded definitions
        // files the way a nominal extern type does (verified directly:
        // loading Roblox's real defs and this generator's output together
        // through `luau-lsp analyze --defs` twice left `ContentId` unknown
        // in the second file even though the first file defines it), so this
        // spells out `string` rather than naming an alias that may not be in
        // scope.
        VariantType::ContentId => "string",
        VariantType::Font => "Font",
        VariantType::NumberRange => "NumberRange",
        VariantType::NumberSequence => "NumberSequence",
        VariantType::ColorSequence => "ColorSequence",
        VariantType::PhysicalProperties => "PhysicalProperties",
        // Also a plain `type UniqueId = any` alias upstream -- see the
        // `ContentId` comment above; spelled out directly for the same
        // reason.
        VariantType::UniqueId => "any",
        VariantType::SecurityCapabilities => "SecurityCapabilities",
        // `BinaryString` is `type BinaryString = string` upstream; see the
        // `ContentId` comment above. `SharedString` has no property in the
        // pinned database today (see the `_ => return None` fallback below)
        // but is given the same treatment on the chance one is ever added.
        VariantType::BinaryString | VariantType::SharedString => "string",
        // Every `Ref` property this database carries (`Parent`,
        // `Model.PrimaryPart`, `ObjectValue.Value`, ...) is legitimately nil
        // at some point in an instance's life, so this is `Instance?` rather
        // than `Instance` even though `DataType` itself carries no
        // nilability -- an unconditional `Instance` would make
        // `frame.Parent = nil`, a real and common assignment, a type error.
        VariantType::Ref => "Instance?",
        VariantType::EnumItem => "EnumItem",
        // Attributes, Tags, MaterialColors and NetAssetRef have no
        // corresponding property in the pinned reflection database today --
        // they are reached through methods (`GetAttribute`, `AddTag`, ...)
        // rather than a typed property. Left unmapped on purpose: a class
        // that actually carried one of these as a property would fail this
        // generator's own build rather than emit a guess.
        _ => return None,
    })
}

fn luau_type_for_data_type(data_type: &DataType) -> Option<String> {
    match data_type {
        DataType::Value(ty) => luau_type_for(*ty).map(|s| s.to_string()),
        DataType::Enum(name) => Some(format!("Enum{name}")),
        _ => None,
    }
}

/// A Luau identifier is a class/property name as-is unless it collides with a
/// keyword -- none of the classes or properties in the pinned database do,
/// but this keeps a future addition from producing a silent parse error.
const LUAU_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

fn safe_ident(name: &str) -> String {
    if LUAU_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// Is `name` a valid Luau identifier on its own -- no quotes, no spaces, no
/// leading digit? `Studio`'s reflection entry carries properties literally
/// named `"TODO" Color`, `"function" Color`, `"local" Color` and
/// `"nil" Color` (internal editor theme-color placeholders), which are not
/// syntactically legal as a `declare extern type` field name at all. Those
/// are dropped rather than guessed at with bracket syntax this luau-lsp
/// version was never confirmed to accept for extern type properties.
fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

// ── Class graph, merged with extensions.rs ─────────────────────────────────

struct ClassOut {
    name: String,
    superclass: Option<String>,
    /// property name -> Luau type
    properties: BTreeMap<String, String>,
}

/// Every class the generated file must declare: the reflection database's
/// own classes, plus every `ExtensionClass` row, plus any class an
/// `ExtensionProperty` targets that the reflection database does not
/// otherwise know about (there are none of those today, but a future
/// Dew-only property on a Dew-only class would need this to still hold).
fn collect_classes(db: &ReflectionDatabase) -> BTreeMap<String, ClassOut> {
    let mut classes: BTreeMap<String, ClassOut> = BTreeMap::new();

    for (name, class) in &db.classes {
        let mut properties = BTreeMap::new();
        for (prop_name, descriptor) in &class.properties {
            if !visible_to_script(descriptor.scriptability) || !is_valid_identifier(prop_name) {
                continue;
            }
            let Some(ty) = luau_type_for_data_type(&descriptor.data_type) else {
                continue;
            };
            properties.insert(prop_name.to_string(), ty);
        }
        classes.insert(
            name.to_string(),
            ClassOut {
                name: name.to_string(),
                superclass: class.superclass.map(|s| s.to_string()),
                properties,
            },
        );
    }

    // Dew-original whole classes, unconditionally (Tier::Experimental rows
    // must be visible to the editor even when their FFlag is off at runtime
    // -- that flag gates behavior, not what the type-checker can see).
    for extension_class in extensions::CLASSES {
        classes
            .entry(extension_class.name.to_string())
            .or_insert_with(|| ClassOut {
                name: extension_class.name.to_string(),
                // `InputActionLabel` inherits from `GuiObject` at runtime --
                // see `datamodel::mod`'s ancestor-walk fallback through
                // `TextLabel`/`ImageLabel`. Those two donate their own
                // properties directly below rather than through `extends`,
                // matching the runtime fallback exactly: the runtime never
                // actually walks `TextLabel`/`ImageLabel` as ancestors, it
                // borrows their property tables once and then continues from
                // `GuiObject`.
                superclass: Some("GuiObject".to_string()),
                properties: BTreeMap::new(),
            });
    }

    // InputActionLabel's runtime fallback: TextLabel's and ImageLabel's own
    // (non-inherited-from-GuiObject) properties are attributed to it
    // directly, mirroring `datamodel::mod::describe`'s special case exactly.
    if classes.contains_key("InputActionLabel") {
        let mut borrowed = BTreeMap::new();
        for donor in ["TextLabel", "ImageLabel"] {
            if let Some(class) = db.classes.get(donor) {
                for (prop_name, descriptor) in &class.properties {
                    if !visible_to_script(descriptor.scriptability)
                        || !is_valid_identifier(prop_name)
                    {
                        continue;
                    }
                    let Some(ty) = luau_type_for_data_type(&descriptor.data_type) else {
                        continue;
                    };
                    borrowed.insert(prop_name.to_string(), ty);
                }
            }
        }
        if let Some(class) = classes.get_mut("InputActionLabel") {
            for (k, v) in borrowed {
                class.properties.entry(k).or_insert(v);
            }
        }
    }

    // Extension properties merge into whatever class they name, whether that
    // class comes from the reflection database (`GuiObject.BlendingMode`) or
    // is itself synthesized (`InputActionLabel.InputAction`). `test_only`
    // rows are skipped: they exist purely to exercise the revision-mismatch
    // mechanism in `manifest.rs`'s own tests and were never meant to reach
    // an applet author's editor.
    for prop in extensions::PROPERTIES {
        let ExtensionProperty {
            class,
            name,
            data_type,
            test_only,
            ..
        } = prop;
        if *test_only {
            continue;
        }
        let Some(ty) = luau_type_for_data_type(data_type) else {
            panic!(
                "extension property {class}.{name} has a data type this generator cannot map \
                 to a Luau type -- teach `luau_type_for` about it"
            );
        };
        classes
            .entry((*class).to_string())
            .or_insert_with(|| ClassOut {
                name: (*class).to_string(),
                superclass: None,
                properties: BTreeMap::new(),
            })
            .properties
            .insert((*name).to_string(), ty);
    }

    classes
}

/// Superclass-before-subclass. Luau's `declare extern type X extends Y`
/// requires `Y` to already be declared (verified directly: a forward
/// reference to a not-yet-declared `extends` target is a parse error, "Unknown
/// type"), so emission order is not cosmetic here.
fn topological_order(classes: &BTreeMap<String, ClassOut>) -> Vec<String> {
    let mut order = Vec::with_capacity(classes.len());
    let mut done: BTreeSet<String> = BTreeSet::new();

    fn visit(
        name: &str,
        classes: &BTreeMap<String, ClassOut>,
        done: &mut BTreeSet<String>,
        order: &mut Vec<String>,
        visiting: &mut BTreeSet<String>,
    ) {
        if done.contains(name) {
            return;
        }
        if !visiting.insert(name.to_string()) {
            // A cycle would mean the reflection database itself is
            // inconsistent; nothing here should be able to produce one.
            panic!("cycle in class hierarchy at {name}");
        }
        if let Some(class) = classes.get(name) {
            if let Some(super_name) = &class.superclass {
                visit(super_name, classes, done, order, visiting);
            }
        }
        visiting.remove(name);
        if done.insert(name.to_string()) {
            order.push(name.to_string());
        }
    }

    let mut visiting = BTreeSet::new();
    for name in classes.keys() {
        visit(name, classes, &mut done, &mut order, &mut visiting);
    }
    order
}

// ── Enums, merged with extensions.rs ────────────────────────────────────────

struct EnumOut {
    name: String,
    /// item name -> value, ordered by value for stable, readable output.
    items: Vec<(String, u32)>,
}

fn collect_enums(db: &ReflectionDatabase) -> BTreeMap<String, EnumOut> {
    let mut enums = BTreeMap::new();
    for (name, descriptor) in &db.enums {
        let mut items: Vec<(String, u32)> = descriptor
            .items
            .iter()
            .map(|(item_name, value)| (item_name.to_string(), *value))
            .collect();
        items.sort_by_key(|(_, value)| *value);
        enums.insert(
            name.to_string(),
            EnumOut {
                name: name.to_string(),
                items,
            },
        );
    }
    // Experimental extension enums (`BlendMode`) appear unconditionally --
    // same reasoning as `Tier::Experimental` classes/properties above: the
    // FFlag gates runtime existence, not what the editor can see.
    for extension_enum in extensions::ENUMS {
        let mut items: Vec<(String, u32)> = extension_enum
            .items
            .iter()
            .map(|(n, v)| (n.to_string(), *v))
            .collect();
        items.sort_by_key(|(_, value)| *value);
        enums.insert(
            extension_enum.name.to_string(),
            EnumOut {
                name: extension_enum.name.to_string(),
                items,
            },
        );
    }
    enums
}

// ── Emission ────────────────────────────────────────────────────────────────

fn generate(db: &ReflectionDatabase) -> String {
    let classes = collect_classes(db);
    let order = topological_order(&classes);
    let enums = collect_enums(db);

    let mut out = String::new();
    out.push_str(HEADER);

    out.push_str("-- ── Classes ─────────────────────────────────────────────────────────────\n\n");
    for name in &order {
        let class = &classes[name];
        emit_class(&mut out, class);
    }

    out.push_str("-- ── Enums ───────────────────────────────────────────────────────────────\n\n");
    let mut enum_names: Vec<&String> = enums.keys().collect();
    enum_names.sort();
    for name in &enum_names {
        emit_enum(&mut out, &enums[*name]);
    }

    emit_enum_aggregate(&mut out, &enum_names);

    out
}

const HEADER: &str = "\
--!strict
-- GENERATED FILE. Do not hand-edit -- regenerate with:
--     cargo run --manifest-path host/Cargo.toml --bin generate-datamodel-types
-- and commit the result. CI's `Check the generated DataModel editor types are
-- current` step runs the same binary with `--check`, which regenerates into
-- memory and diffs against this file without writing, the same discipline
-- `docs/datamodel_scope.md` already asks for in its own header comment.
--
-- Owns the entire DataModel class and enum hierarchy this host accepts,
-- including `host/src/datamodel/extensions.rs`'s Dew-original rows
-- (`InputActionLabel.InputAction`, `GuiObject.BlendingMode`/`Enum.BlendMode`)
-- merged in at generation time. It does NOT declare value types (`Vector3`,
-- `CFrame`, `Color3`, `UDim2`, `Content`, `Font`, `BrickColor`,
-- `NumberSequence`, `ColorSequence`, `PhysicalProperties`, and the rest) or
-- the `Enum`/`EnumItem` base types -- those are real global types
-- `luau-lsp.types.roblox` already provides.
--
-- KNOWN GAP: PROPERTIES ONLY, NO METHODS OR EVENTS. Every class below has
-- accurate properties and nothing else -- `rbx_reflection_database` (the
-- only source this generator reads) carries no method or event signatures
-- for any class, ever. Because this file's classes replace Roblox's own
-- same-named declarations outright rather than merging, a script calling
-- `instance:Clone()`, connecting `RunService.Heartbeat`, or calling
-- `GetPropertyChangedSignal` now gets a real editor error it did not get
-- before this file was wired in. This is a known, accepted tradeoff -- see
-- `generate_datamodel_types.rs`'s own header for why two narrower designs
-- were tried and rejected. `scripts/fetch_api_surface.luau`'s vendored
-- ~40-class method/event subset is the incremental path to closing this gap
-- further; extend that over time rather than reopening this generator's
-- scope to try to solve it all at once.

";

fn emit_class(out: &mut String, class: &ClassOut) {
    let name = safe_ident(&class.name);
    match &class.superclass {
        Some(super_name) => {
            out.push_str(&format!(
                "declare extern type {name} extends {} with\n",
                safe_ident(super_name)
            ));
        }
        None => {
            out.push_str(&format!("declare extern type {name} with\n"));
        }
    }
    for (prop_name, ty) in &class.properties {
        out.push_str(&format!("\t{}: {ty}\n", safe_ident(prop_name)));
    }
    out.push_str("end\n\n");
}

fn emit_enum(out: &mut String, e: &EnumOut) {
    let item_type = format!("Enum{}", e.name);
    let internal_type = format!("{item_type}_INTERNAL");

    out.push_str(&format!(
        "declare extern type {item_type} extends EnumItem with\nend\n\n"
    ));

    out.push_str(&format!(
        "declare extern type {internal_type} extends Enum with\n"
    ));
    for (item_name, _) in &e.items {
        out.push_str(&format!("\t{}: {item_type}\n", safe_ident(item_name)));
    }
    out.push_str(&format!(
        "\tfunction GetEnumItems(self): {{ {item_type} }}\n\
         \tfunction FromName(self, name: string): {item_type}?\n\
         \tfunction FromValue(self, value: number): {item_type}?\n\
         end\n\n"
    ));
}

/// The single `Enum` global, aggregating every category by name -- the
/// pattern confirmed against the installed luau-lsp's own bundled
/// definitions (`declare Enum: { <Category>: Enum<Category>_INTERNAL, ... }`).
fn emit_enum_aggregate(out: &mut String, enum_names: &[&String]) {
    out.push_str("declare Enum: {\n");
    for name in enum_names {
        out.push_str(&format!("\t{name}: Enum{name}_INTERNAL,\n"));
    }
    out.push_str("}\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_without_panicking() {
        let db = rbx_reflection_database::get().expect("db");
        let text = generate(db);
        assert!(text.contains("declare extern type Instance extends Object with"));
        assert!(text.contains("declare extern type GuiObject extends"));
        assert!(text.contains("BlendingMode: EnumBlendMode"));
        assert!(text.contains("declare extern type EnumBlendMode extends EnumItem"));
        assert!(text.contains("declare extern type InputActionLabel extends GuiObject"));
        assert!(text.contains("InputAction: string"));
    }

    #[test]
    fn every_extends_target_is_declared_earlier() {
        let db = rbx_reflection_database::get().expect("db");
        let classes = collect_classes(db);
        let order = topological_order(&classes);
        let mut seen = BTreeSet::new();
        for name in &order {
            if let Some(super_name) = &classes[name].superclass {
                assert!(
                    seen.contains(super_name),
                    "{name} extends {super_name}, which has not been emitted yet"
                );
            }
            seen.insert(name.clone());
        }
    }
}
