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
//! THREAD-LOCAL FLAG STATE, MATCHING HOW APPLETS ARE ALREADY ISOLATED. Each
//! applet runs entirely on its own OS thread with its own Lua VM
//! (`coordinator::spawn_applet`), so "which experimental flags are enabled
//! for this applet" is naturally a thread-local, the same shape
//! `crates/window/src/win32.rs` already uses for its per-thread event queue.
//! `describe`/`class_exists`/`describe_enum` need no signature change to
//! consult it, and the isolation it rides on is the same isolation that
//! keeps two applets' DataModels apart.

use rbx_reflection::{DataType, EnumDescriptor, PropertyDescriptor, Scriptability};
use rbx_types::{Enum, Variant, VariantType};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
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

thread_local! {
    /// Which experimental flags are enabled for the applet running on this
    /// thread. Empty by default, which is what makes an example that never
    /// calls `set_enabled_flags` see zero observable difference.
    static ENABLED_FLAGS: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
}

/// Enable exactly this set of flags for whatever runs on the calling thread
/// from here on. Called once, before an applet's own Luau runs -- see
/// `applets::load`.
pub fn set_enabled_flags(flags: &[(&'static str, u32)]) {
    ENABLED_FLAGS.with(|cell| {
        let mut set = cell.borrow_mut();
        set.clear();
        set.extend(flags.iter().map(|(flag, _)| *flag));
    });
}

/// Is this exact flag enabled for the calling thread's applet?
pub fn is_enabled(flag: &str) -> bool {
    ENABLED_FLAGS.with(|cell| cell.borrow().contains(flag))
}

/// Is this entry visible right now: always for `DewOnly`, only when its flag
/// is enabled for `Experimental`?
fn visible(tier: &Tier) -> bool {
    match tier {
        Tier::DewOnly => true,
        Tier::Experimental { flag, .. } => is_enabled(flag),
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

/// Every experimental flag this registry defines, deduplicated across
/// `PROPERTIES` and `ENUMS` -- a property and an enum belonging to the same
/// feature would share one flag, and this must not print it twice.
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

/// `dirs::data_local_dir()/Dew/DewAppSettings.json` -- a flat
/// `{ "FFlagName": true }` map, mirroring Roblox's own
/// `ClientAppSettings.json` convention exactly. A local, host-level override
/// independent of what any applet declares: the "flip it on while I'm
/// personally testing" path, the equivalent of Studio's beta-features
/// toggle.
///
/// SAME READ/WRITE SHAPE AS `positions.rs`: `dirs::data_local_dir()`,
/// create-dir-all, read-or-default. This file is read-only from here --
/// nothing in this sprint writes it, a person edits it by hand -- so only
/// the read half is needed.
fn local_overrides_path() -> Option<std::path::PathBuf> {
    let dir = dirs::data_local_dir()?.join("Dew");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("DewAppSettings.json"))
}

fn read_local_overrides() -> BTreeMap<String, bool> {
    let Some(path) = local_overrides_path() else {
        return BTreeMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// The union of a manifest's own `experimentalDatamodel` flags and whatever
/// `DewAppSettings.json` enables locally. The manifest says what an applet
/// USES; the local file is a host-level override for testing one without
/// editing the applet -- so the effective set for a run is always both,
/// never one replacing the other.
pub fn effective_flags(declared: &[(&'static str, u32)]) -> Vec<(&'static str, u32)> {
    let mut effective: Vec<(&'static str, u32)> = declared.to_vec();
    let overrides = read_local_overrides();
    for (flag, revision) in known_flags() {
        let locally_on = overrides.get(flag).copied().unwrap_or(false);
        if locally_on && !effective.iter().any(|(f, _)| *f == flag) {
            effective.push((flag, revision));
        }
    }
    effective
}

/// Print every flag in effect for this run, unconditionally -- the whole
/// point of an experimental surface is that nobody using it can plausibly
/// not know. Silent when the set is empty, so the common case (no flags at
/// all) manufactures no noise.
pub fn print_effective(flags: &[(&'static str, u32)]) {
    for (flag, revision) in flags {
        println!("[dew] experimental: {flag} (revision {revision})");
    }
}

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
}
