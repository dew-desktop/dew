//! Datamodel surface the reflection database does not carry.
//!
//! WHY THIS EXISTS. Two shapes of property/enum land here rather than in
//! `rbx_reflection_database`: something genuinely Dew-only with no Roblox
//! equivalent expected ever, and a plausible pre-implementation of a feature
//! Roblox might ship later, under its real, final, unprefixed name. The
//! design mirrors Roblox's own FFlag system on purpose: an experimental entry
//! is invisible from Luau until its flag is enabled for the run, the way a
//! real Roblox feature is invisible until its FFlag rolls out. The flag
//! governs EXISTENCE -- whether `describe` finds the property at all -- never
//! the value a guest chooses once it is unlocked.
//!
//! `GuiObject.BlendingMode` IS THE FIRST EXPERIMENTAL ROW. A `UIGradient.Type`/
//! `Enum.GradientType` entry was drafted here and pulled before landing:
//! `rbx_reflection_database` 3.0.0+roblox-728, the version this workspace
//! actually pins, already ships `UIGradient.Type` as a real, `ReadWrite`,
//! unflagged property with a three-member `GradientType` enum. `BlendingMode`
//! was checked the same way and is genuinely absent: no property or enum by
//! that name or a similar one exists anywhere in 3.0.0+roblox-728. It is
//! gated behind `FFlagDewGuiObjectBlendingMode` until Roblox ships the real
//! thing, at which point this row is deleted rather than renamed.
//!
//! ONE REGISTRY, NOT TWO. `InputActionLabel`/`InputAction` used to be three
//! `if class == "InputActionLabel"` blocks hand-written into
//! `datamodel/mod.rs` before this file existed, because the engine shipped
//! that class in 0.736 and `rbx_reflection_database` still carried 0.728.
//! It is a permanent, always-on (`Tier::DewOnly`) row here now, on the same
//! table a future experimental entry will be a row of, so there will be
//! exactly one place this pattern lives.
//!
//! FLAG ENABLEMENT LIVES IN `crate::flags`, NOT HERE (milestone 12 sprint
//! 1). This module owns the DataModel-specific rows -- `Tier`, `PROPERTIES`,
//! `ENUMS`, `CLASSES`, and resolving a `"Class.Property"` string to the flag
//! it's gated by -- and calls into `flags` for storage, the thread-local
//! enabled set, and `DewAppSettings.json`'s local override. That module
//! knows nothing about a `class.property` shape, so a future non-DataModel
//! flag (a coordinator or rendering-pipeline change) calls it directly
//! rather than adding a second copy of this bookkeeping. See `flags.rs`'s
//! header for why the storage itself is thread-local.

use rbx_reflection::{DataType, EnumDescriptor, PropertyDescriptor, Scriptability};
use rbx_types::{Enum, Variant, VariantType};
use std::sync::OnceLock;

/// How long an entry lives before, or instead of, appearing in upstream
/// reflection data.
#[derive(Clone, Copy)]
pub enum Tier {
    /// No Roblox equivalent is ever expected. Permanent; no flag, always on.
    DewOnly,
    /// A plausible pre-implementation of a feature Roblox might ship later,
    /// under its real, final, unprefixed name. Invisible unless its flag is
    /// enabled for the current run. `revision` bumps if the shape changes
    /// before graduation, so a stale flag-enabler gets a clear diagnostic
    /// instead of silently different behavior.
    Experimental { flag: &'static str, revision: u32 },
}

/// A property the reflection database does not carry.
pub struct ExtensionProperty {
    pub class: &'static str,
    pub name: &'static str,
    pub data_type: DataType<'static>,
    pub default: Variant,
    pub scriptability: Scriptability,
    pub tier: Tier,
    /// Scaffolding for this sprint's own tests, never a real feature --
    /// `generate-datamodel-types` skips it, so it never reaches
    /// `types/datamodel.d.luau`, the one artifact every entry here
    /// otherwise feeds unconditionally (see that generator's comment on why
    /// `Tier::Experimental` rows still generate a type: the flag gates
    /// behavior, not what the type-checker can see). `false` for every real
    /// row.
    pub test_only: bool,
}

/// An enum the reflection database does not carry.
pub struct ExtensionEnum {
    pub name: &'static str,
    pub items: &'static [(&'static str, u32)],
    pub tier: Tier,
}

/// A whole class the reflection database has no entry for at all.
///
/// SEPARATE FROM `PROPERTIES`, because `class_exists` and `describe`'s
/// chain-walk both need to answer "does this class exist" independently of
/// any one property on it -- `InputActionLabel` must exist even for a guest
/// that never touches `InputAction`, since it also inherits `TextLabel` and
/// `ImageLabel` properties `datamodel::mod`'s `describe` resolves separately.
pub struct ExtensionClass {
    pub name: &'static str,
    pub tier: Tier,
}

/// Every extension property, Dew-only and experimental together.
pub static PROPERTIES: &[ExtensionProperty] = &[
    // InputActionLabel is introduced in the engine 0.736 and is not yet in
    // rbx_reflection_database 0.728. It inherits from GuiObject and declares
    // text and image properties matching TextLabel and ImageLabel; those are
    // resolved by falling back through the reflection database directly in
    // `datamodel::describe`, so only the class's own property needs a row
    // here.
    ExtensionProperty {
        class: "InputActionLabel",
        name: "InputAction",
        data_type: DataType::Value(VariantType::String),
        default: Variant::String(String::new()),
        scriptability: Scriptability::ReadWrite,
        tier: Tier::DewOnly,
        test_only: false,
    },
    // Declared on GuiObject, the base every visual class inherits from, so
    // `describe`'s ancestor walk (`datamodel::mod`'s `describe`) finds it for
    // `Frame`, `TextLabel`, `ImageLabel` and everything else that descends
    // from it -- the same walk that already resolves `Instance.Name` for
    // `Frame` without a row of its own.
    ExtensionProperty {
        class: "GuiObject",
        name: "BlendingMode",
        data_type: DataType::Enum("BlendMode"),
        default: Variant::Enum(Enum::from_u32(0)),
        scriptability: Scriptability::ReadWrite,
        tier: Tier::Experimental {
            flag: "FFlagDewGuiObjectBlendingMode",
            revision: 1,
        },
        test_only: false,
    },
    // TEST-ONLY. Not a real feature and never will graduate -- the class
    // exists nowhere else, `test_only: true` keeps it out of
    // `types/datamodel.d.luau`, and no applet ships declaring it. Exists
    // purely so revision-mismatch resolution (`resolve_declaration`) can be
    // exercised end to end, through a real `dew.toml` and the real
    // registry, rather than asserted against a mock -- see `manifest.rs`'s
    // `a_mismatched_revision_refuses_with_a_specific_message`. Frozen at
    // revision 2 to stand in for "a row whose shape already changed once",
    // without needing to invent a shape change for `GuiObject.BlendingMode`,
    // the one real experimental row, that never happened.
    ExtensionProperty {
        class: "DewSprintTestOnly",
        name: "RevisionProbe",
        data_type: DataType::Value(VariantType::String),
        default: Variant::String(String::new()),
        scriptability: Scriptability::ReadWrite,
        tier: Tier::Experimental {
            flag: "FFlagDewSprintTestOnlyRevisionProbe",
            revision: 2,
        },
        test_only: true,
    },
];

/// Every extension enum.
pub static ENUMS: &[ExtensionEnum] = &[ExtensionEnum {
    name: "BlendMode",
    items: &[("Alpha", 0), ("Additive", 1), ("Multiply", 2)],
    tier: Tier::Experimental {
        flag: "FFlagDewGuiObjectBlendingMode",
        revision: 1,
    },
}];

/// Every class synthesized whole.
pub static CLASSES: &[ExtensionClass] = &[ExtensionClass {
    name: "InputActionLabel",
    tier: Tier::DewOnly,
}];

/// Re-exported so existing callers (`applets::load`, this module's own
/// tests) keep naming it `extensions::set_enabled_flags` -- the storage it
/// touches now lives in `crate::flags`, but which module OWNS enablement is
/// an implementation detail callers outside this file don't need to track.
pub use crate::flags::set_enabled_flags;

/// Is this entry visible right now: always for `DewOnly`, only when its flag
/// is enabled for `Experimental`?
fn visible(tier: &Tier) -> bool {
    match tier {
        Tier::DewOnly => true,
        Tier::Experimental { flag, .. } => crate::flags::is_enabled(flag),
    }
}

/// The synthesized descriptors, built once and cached -- `PropertyDescriptor`
/// carries a `HashSet`/`PropertyKind` that cannot be built in a `static`
/// initializer, the same reason the old `InputAction` descriptor was kept in
/// a `OnceLock` rather than a `const`.
fn descriptors() -> &'static [PropertyDescriptor<'static>] {
    static CACHE: OnceLock<Vec<PropertyDescriptor<'static>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        PROPERTIES
            .iter()
            .map(|p| {
                let mut descriptor = PropertyDescriptor::new(p.name, p.data_type.clone());
                descriptor.scriptability = p.scriptability;
                descriptor
            })
            .collect()
    })
}

fn enum_descriptors() -> &'static [EnumDescriptor<'static>] {
    static CACHE: OnceLock<Vec<EnumDescriptor<'static>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        ENUMS
            .iter()
            .map(|e| {
                let mut descriptor = EnumDescriptor::new(e.name);
                for (name, value) in e.items {
                    descriptor.items.insert(name, *value);
                }
                descriptor
            })
            .collect()
    })
}

/// A property descriptor for `class.property`, if this registry has one AND
/// it is visible right now. Consulted by `datamodel::describe` on every step
/// of its ancestor walk, the same as a reflection-database class.
pub fn describe(class: &str, property: &str) -> Option<&'static PropertyDescriptor<'static>> {
    let index = PROPERTIES
        .iter()
        .position(|p| p.class == class && p.name == property && visible(&p.tier))?;
    Some(&descriptors()[index])
}

/// The default value for `class.property`, if this registry has one AND it
/// is visible right now.
pub fn default_for(class: &str, property: &str) -> Option<Variant> {
    PROPERTIES
        .iter()
        .find(|p| p.class == class && p.name == property && visible(&p.tier))
        .map(|p| p.default.clone())
}

/// Does this class exist as a whole synthesized entry, and is it visible?
pub fn class_exists(class: &str) -> bool {
    CLASSES.iter().any(|c| c.name == class && visible(&c.tier))
}

/// An enum descriptor for `name`, if this registry has one AND it is visible
/// right now. Consulted by `enums::category`, which is the one place every
/// enum lookup in `enums.rs` and the Luau `Enum` global funnel through.
pub fn describe_enum(name: &str) -> Option<&'static EnumDescriptor<'static>> {
    let index = ENUMS
        .iter()
        .position(|e| e.name == name && visible(&e.tier))?;
    Some(&enum_descriptors()[index])
}

/// Resolve one `"Class.Property"` string from `dew.toml`'s
/// `experimentalDatamodel` array to the flag that gates it.
///
/// `None` MEANS "not a known experimental property" -- a typo, or a feature
/// that graduated into the reflection database and lost its row here -- and
/// `Manifest::experimental_flags` turns that into a refusal to load rather
/// than a manifest entry that silently does nothing.
///
/// TAKES A BARE KEY, NO `@revision` -- `resolve_declaration` is what
/// `dew.toml` actually calls (milestone 12 sprint 2); this stays the plain
/// name-to-flag lookup underneath it, and its own test
/// (`flag_for_refuses_an_unknown_key_and_a_dew_only_one`) is exactly the
/// coverage `resolve_declaration`'s `UnknownProperty` case builds on.
pub fn flag_for(key: &str) -> Option<(&'static str, u32)> {
    let (class, property) = key.split_once('.')?;
    PROPERTIES.iter().find_map(|p| {
        if p.class != class || p.name != property {
            return None;
        }
        match p.tier {
            Tier::Experimental { flag, revision } => Some((flag, revision)),
            Tier::DewOnly => None,
        }
    })
}

/// Why a `dew.toml` `experimentalDatamodel` entry could not be resolved.
/// `Display` gives the reason on its own, without the entry itself or the
/// applet's id -- `Manifest::experimental_flags` prepends both, since it is
/// the one that has them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationError {
    /// No `@revision` suffix. A bare `"Class.Property"` used to be enough
    /// (before milestone 12 sprint 2); it no longer is, because a
    /// declaration that never pins a revision can never mismatch one --
    /// exactly the silent break the Origin-Trials framing in
    /// `vision/overview.md` exists to rule out. `describe`/`class_exists`/
    /// etc. don't care WHY a flag got enabled, only that it did, so this
    /// refusal happens once, here, rather than at every call site that
    /// reads a property.
    MissingRevision,
    /// The text after `@` was not a plain non-negative integer.
    InvalidRevision(String),
    /// The part before `@` does not resolve to a known `Tier::Experimental`
    /// row -- the same condition `flag_for` reports as `None`: a typo, a
    /// `Tier::DewOnly` entry (which has no revision to pin), or a feature
    /// that graduated out of this registry.
    UnknownProperty,
    /// The key is real, but the registry has moved past the revision this
    /// declaration pinned -- the row's shape changed since the applet was
    /// written against it.
    RevisionMismatch { declared: u32, current: u32 },
}

impl std::fmt::Display for DeclarationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeclarationError::MissingRevision => write!(
                f,
                "has no @revision -- write it as \"Class.Property@N\", pinning the revision \
                 this mod was written against"
            ),
            DeclarationError::InvalidRevision(text) => {
                write!(f, "has a revision that is not a plain integer: {text:?}")
            }
            DeclarationError::UnknownProperty => {
                write!(f, "is not a known experimental property")
            }
            DeclarationError::RevisionMismatch { declared, current } => write!(
                f,
                "was written against revision {declared}, but the registry is now at revision \
                 {current} -- the property's shape changed; update the mod for the new behavior \
                 and declare @{current}"
            ),
        }
    }
}

/// Resolve one `"Class.Property@N"` string from `dew.toml`'s
/// `experimentalDatamodel` array to the flag it unlocks, refusing unless the
/// declared revision matches the registry's current one for that row.
///
/// SPLITS ON THE LAST `@` BEFORE SPLITTING ON `.`. `flag_for` already splits
/// `key` on the first `.` to separate class from property; if this split
/// class/property first and looked for `@` inside the trailing piece, a
/// property name that happened to contain a `.` (none do today) would put
/// the `@` split on the wrong side of it. Stripping the revision suffix
/// first, with `rsplit_once`, means `flag_for` never sees it and this
/// function never has to know how `flag_for` parses its half.
pub fn resolve_declaration(declaration: &str) -> Result<(&'static str, u32), DeclarationError> {
    let (key, revision_text) = declaration
        .rsplit_once('@')
        .ok_or(DeclarationError::MissingRevision)?;
    let declared_revision: u32 = revision_text
        .parse()
        .map_err(|_| DeclarationError::InvalidRevision(revision_text.to_string()))?;
    let (flag, current_revision) = flag_for(key).ok_or(DeclarationError::UnknownProperty)?;
    if declared_revision != current_revision {
        return Err(DeclarationError::RevisionMismatch {
            declared: declared_revision,
            current: current_revision,
        });
    }
    Ok((flag, current_revision))
}

/// Every experimental flag this registry defines, deduplicated across
/// `PROPERTIES` and `ENUMS` -- a property and an enum belonging to the same
/// feature would share one flag, and this must not print it twice. This is
/// the DataModel's own idea of "known"; `crate::flags::effective_flags`
/// takes it as a parameter rather than assuming it, since a future
/// non-DataModel flag would have its own list drawn from its own rows.
fn known_flags() -> Vec<(&'static str, u32)> {
    let mut out: Vec<(&'static str, u32)> = Vec::new();
    let mut note = |flag: &'static str, revision: u32| {
        if !out.iter().any(|(f, _)| *f == flag) {
            out.push((flag, revision));
        }
    };
    for p in PROPERTIES {
        if let Tier::Experimental { flag, revision } = p.tier {
            note(flag, revision);
        }
    }
    for e in ENUMS {
        if let Tier::Experimental { flag, revision } = e.tier {
            note(flag, revision);
        }
    }
    out
}

/// The union of a manifest's own `experimentalDatamodel` flags and whatever
/// `DewAppSettings.json` enables locally, out of this module's own registry.
/// See `crate::flags::effective_flags` for what "declared" and "local
/// override" mean; this is a thin wrapper supplying `known_flags()` so
/// callers here keep the old one-argument shape.
pub fn effective_flags(declared: &[(&'static str, u32)]) -> Vec<(&'static str, u32)> {
    crate::flags::effective_flags(declared, &known_flags())
}

/// Print every flag in effect for this run. See `crate::flags::print_effective`.
pub use crate::flags::print_effective;

#[cfg(test)]
mod tests {
    use super::*;

    /// Flags are thread-local; clear before each test so ordering between
    /// tests in this module cannot leak enabled state between them.
    fn reset() {
        set_enabled_flags(&[]);
    }

    #[test]
    fn dew_only_is_always_visible() {
        reset();
        assert!(describe("InputActionLabel", "InputAction").is_some());
        assert!(class_exists("InputActionLabel"));
    }

    /// THE TIER MECHANISM ITSELF, tested directly against a `Tier` value
    /// rather than a row in `PROPERTIES` -- there is no real `Experimental`
    /// entry to exercise this through yet (see this module's top comment),
    /// but `visible` is the one function every lookup above funnels through,
    /// so this is still a real test of the gate rather than of a fixture.
    #[test]
    fn experimental_is_invisible_until_its_flag_is_set() {
        reset();
        let tier = Tier::Experimental {
            flag: "FFlagTestOnlyDoesNotGateAnything",
            revision: 1,
        };
        assert!(!visible(&tier));

        set_enabled_flags(&[("FFlagTestOnlyDoesNotGateAnything", 1)]);
        assert!(visible(&tier));
        reset();
    }

    #[test]
    fn flag_for_refuses_an_unknown_key_and_a_dew_only_one() {
        assert_eq!(flag_for("UIGradient.NotAThing"), None);
        assert_eq!(flag_for("no-dot-here"), None);
        // A DewOnly entry has no flag to resolve to -- it is always visible.
        assert_eq!(flag_for("InputActionLabel.InputAction"), None);
    }

    /// A REAL EXPERIMENTAL ROW, exercised directly against the registry rather
    /// than through the generic `Tier` value the test above uses.
    #[test]
    fn blending_mode_is_invisible_until_its_flag_is_set() {
        reset();
        assert!(describe("GuiObject", "BlendingMode").is_none());
        assert!(describe_enum("BlendMode").is_none());
        assert_eq!(
            flag_for("GuiObject.BlendingMode"),
            Some(("FFlagDewGuiObjectBlendingMode", 1))
        );

        set_enabled_flags(&[("FFlagDewGuiObjectBlendingMode", 1)]);
        assert!(describe("GuiObject", "BlendingMode").is_some());
        let items = describe_enum("BlendMode").expect("enum visible once flagged");
        assert_eq!(items.items.get("Alpha"), Some(&0));
        assert_eq!(items.items.get("Additive"), Some(&1));
        assert_eq!(items.items.get("Multiply"), Some(&2));
        reset();
    }

    #[test]
    fn effective_flags_is_at_least_the_declared_set() {
        let declared = vec![("FFlagTestOnlyDoesNotGateAnything", 1)];
        let effective = effective_flags(&declared);
        assert!(effective.contains(&("FFlagTestOnlyDoesNotGateAnything", 1)));
    }

    #[test]
    fn resolve_declaration_requires_a_revision_suffix() {
        assert!(matches!(
            resolve_declaration("GuiObject.BlendingMode"),
            Err(DeclarationError::MissingRevision)
        ));
    }

    #[test]
    fn resolve_declaration_rejects_a_non_integer_revision() {
        assert!(matches!(
            resolve_declaration("GuiObject.BlendingMode@one"),
            Err(DeclarationError::InvalidRevision(text)) if text == "one"
        ));
    }

    #[test]
    fn resolve_declaration_rejects_an_unknown_property_even_with_a_revision() {
        assert!(matches!(
            resolve_declaration("UIGradient.NotAThing@1"),
            Err(DeclarationError::UnknownProperty)
        ));
    }

    #[test]
    fn resolve_declaration_accepts_a_matching_revision() {
        assert_eq!(
            resolve_declaration("GuiObject.BlendingMode@1"),
            Ok(("FFlagDewGuiObjectBlendingMode", 1))
        );
    }

    /// THE MISMATCH, PRODUCED FOR REAL against a genuine registry row
    /// (`DewSprintTestOnly.RevisionProbe`, frozen at revision 2 for exactly
    /// this purpose) rather than asserted against a mock.
    #[test]
    fn resolve_declaration_refuses_a_stale_revision() {
        assert_eq!(
            resolve_declaration("DewSprintTestOnly.RevisionProbe@1"),
            Err(DeclarationError::RevisionMismatch {
                declared: 1,
                current: 2,
            })
        );
    }
}
