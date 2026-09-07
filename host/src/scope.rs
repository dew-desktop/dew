//! What the DataModel Standard covers, as one set that every tool reads.
//!
//! THE DENOMINATOR HAS ONE HOME. `datamodel-surface` reports how much of the
//! surface the host ACCEPTS, and `gallery-coverage` reports how much of it a
//! rendered scene DEMONSTRATES. Those are different questions about the same
//! 139 properties, and two tools deriving that 139 separately is two tools that
//! will eventually disagree about it -- which is how this repository once
//! reported 35 of 138 for a surface the host had never implemented, and how the
//! denominator once moved from 138 to 148 because an implementation detail
//! overturned a scope decision.
//!
//! So the classification lists and the walk that applies them live here, and
//! both bins call in -- they agree by construction rather than by luck, and
//! `the_denominator_is_139` pins the number so that a change to it has to be a
//! deliberate diff rather than a silent one.
//!
//! WHAT IS NOT HERE. Whether the host accepts a property (`datamodel::accepts`)
//! and whether Aether's pipeline honours it are answered elsewhere and stay
//! elsewhere. This module answers one question: which property names is the
//! standard about.

use std::collections::{BTreeMap, BTreeSet};

/// The scriptable API surface, pinned.
///
/// Both the property half and the method/event half of the DataModel standard
/// are pinned from Roblox's API dump at `0.736.0.7361346`.
///
/// `scripts/fetch_api_surface.luau` writes this from Roblox's own API dump.
/// Embedded rather than read at runtime so the data is part of the build: a
/// missing or malformed file is a compile error rather than a tool that runs and
/// reports a smaller surface than exists.
pub const API_SURFACE: &str = include_str!("../datamodel/api_surface.json");

#[derive(serde::Deserialize)]
pub struct ApiSurface {
    pub version: String,
    pub classes: BTreeMap<String, ApiClass>,
}

#[derive(serde::Deserialize)]
pub struct ApiClass {
    pub superclass: Option<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
    /// Member name to "Function" or "Event".
    ///
    /// A MAP RATHER THAN A LIST because Lune, which writes this file, encodes an
    /// empty array and an empty map identically as `{}` -- and most classes in
    /// the chain introduce no members of their own. A list shape failed to
    /// deserialise on precisely the classes that carry nothing, which is the
    /// least useful place for a format to be ambiguous.
    pub members: BTreeMap<String, String>,
}

/// Properties that are engine bookkeeping rather than UI.
pub const NOT_UI: &[&str] = &[
    "Archivable",
    "RobloxLocked",
    "AutoLocalize",
    "RootLocalizationTable",
    "Attributes",
    "AttributesReplicate",
    "AttributesSerialize",
    "SourceAssetId",
    "Sandboxed",
    "Capabilities",
    "UniqueId",
    "HistoryId",
    "ActiveQueryNames",
];

/// Input devices this host does not have. A desktop widget is driven by a
/// pointer and a keyboard; gamepad focus traversal, selection groups and haptics
/// describe a console.
///
/// This is the one group most likely to move. If Dew ever grows gamepad support
/// these stop being out of scope and become backlog, which is exactly why they
/// are listed separately rather than lumped in above.
pub const INPUT_DEVICE: &[&str] = &[
    "NextSelectionUp",
    "NextSelectionDown",
    "NextSelectionLeft",
    "NextSelectionRight",
    "Selectable",
    "SelectionImageObject",
    "SelectionOrder",
    "SelectionGroup",
    "SelectionBehaviorUp",
    "SelectionBehaviorDown",
    "SelectionBehaviorLeft",
    "SelectionBehaviorRight",
    "GamepadInputEnabled",
    "HoverHapticEffect",
    "PressHapticEffect",
    // Touch and the on-screen keyboard: devices, not decisions.
    "TouchInputEnabled",
    "ShowNativeInput",
];

/// Classes that exist under GuiObject and are not UI a host has to draw.
///
/// Video, viewports and chat windows are engine features rather than layout, and
/// a standard that demanded them would be describing Roblox rather than
/// describing a UI. Named explicitly so the exclusion is a decision in a diff
/// rather than a silent filter.
pub const OUT_OF_SCOPE: &[&str] = &[
    "VideoFrame",
    "VideoDisplay",
    "ViewportFrame",
    "TextChannelWindow",
    "RelativeGui",
];

/// The classes a UI actually uses. `GuiObject` descendants come from the
/// hierarchy; these are the modifiers, which are `UIComponent` rather than
/// `GuiObject` and so are not found by walking superclasses.
pub const MODIFIERS: &[&str] = &[
    "UICorner",
    "UIPadding",
    "UIListLayout",
    "UIGridLayout",
    "UIPageLayout",
    "UITableLayout",
    "UIGradient",
    "UIStroke",
    "UIScale",
    "UIAspectRatioConstraint",
    "UISizeConstraint",
    "UITextSizeConstraint",
    "UIFlexItem",
];

/// Parse the pinned dump. Panics on a malformed file, which is a build-data
/// problem rather than a runtime one.
pub fn api() -> ApiSurface {
    serde_json::from_str(API_SURFACE).expect("host/datamodel/api_surface.json is malformed")
}

/// Everything that is, or descends from, `GuiObject`, plus the modifiers, minus
/// the classes excluded by decision.
pub fn ui_classes(api: &ApiSurface) -> BTreeMap<&str, &ApiClass> {
    let mut ui: BTreeMap<&str, &ApiClass> = BTreeMap::new();
    for (name, class) in &api.classes {
        if name == "GuiObject" {
            ui.insert(name.as_str(), class);
            continue;
        }
        let mut cursor = class.superclass.as_deref();
        while let Some(c_name) = cursor {
            if c_name == "GuiObject" {
                ui.insert(name.as_str(), class);
                break;
            }
            cursor = api
                .classes
                .get(c_name)
                .and_then(|c| c.superclass.as_deref());
        }
    }
    for m in MODIFIERS {
        if let Some(c) = api.classes.get(*m) {
            ui.insert(m, c);
        }
    }
    ui.retain(|name, _| !OUT_OF_SCOPE.contains(name));
    ui
}

/// Every writable property reachable on a class, walking the superclass chain.
///
/// Members are declared on the class that introduces them and are not repeated
/// on descendants, so a class's real property list is the union along its
/// ancestry.
pub fn properties_of<'a>(api: &'a ApiSurface, class: &str) -> Vec<&'a str> {
    let mut props: Vec<&str> = Vec::new();
    let mut cursor = Some(class);
    while let Some(c_name) = cursor {
        if let Some(c) = api.classes.get(c_name) {
            for pname in c.properties.keys() {
                props.push(pname.as_str());
            }
            cursor = c.superclass.as_deref();
        } else {
            break;
        }
    }
    props.sort_unstable();
    props.dedup();
    props
}

/// Is this property name excluded from the standard by decision?
pub fn excluded(property: &str) -> bool {
    NOT_UI.contains(&property) || INPUT_DEVICE.contains(&property)
}

/// THE DENOMINATOR. Every in-scope property name, once.
///
/// The census is BY NAME, matching `datamodel-surface`: a name that appears on
/// several classes counts once. That choice is load-bearing and is why
/// `in_scope_by_class` exists beside it -- a tool that wants to know whether
/// `Color` was demonstrated on `UIStroke` as well as on `UIGradient` has to ask
/// the per-class question, because the by-name answer cannot tell them apart.
pub fn in_scope_properties() -> BTreeSet<String> {
    let api = api();
    let mut names: BTreeSet<String> = BTreeSet::new();
    for class in ui_classes(&api).keys() {
        for p in properties_of(&api, class) {
            if !excluded(p) {
                names.insert(p.to_string());
            }
        }
    }
    names
}

/// Every in-scope (class, property) pair.
///
/// The by-name figure is the headline because it is the one the standard is
/// written in. This is the finer question, and the reason to keep it is that
/// counting by name is exactly how one class's answer once masked another's and
/// the surface read 100%.
pub fn in_scope_by_class() -> BTreeMap<String, BTreeSet<String>> {
    let api = api();
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for class in ui_classes(&api).keys() {
        let set: BTreeSet<String> = properties_of(&api, class)
            .into_iter()
            .filter(|p| !excluded(p))
            .map(|p| p.to_string())
            .collect();
        out.insert(class.to_string(), set);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_denominator_is_not_empty_and_excludes_what_it_says_it_does() {
        let names = in_scope_properties();
        assert!(
            names.len() > 100,
            "the in-scope set collapsed to {}",
            names.len()
        );
        // The two exclusion groups, spot-checked. `Archivable` is the one that
        // inflated the denominator from 138 to 148 when it was counted.
        assert!(!names.contains("Archivable"));
        assert!(!names.contains("Selectable"));
        // And something that is unambiguously in scope.
        assert!(names.contains("BackgroundColor3"));
        assert!(names.contains("Size"));
    }

    #[test]
    fn out_of_scope_classes_are_not_walked() {
        let api = api();
        let ui = ui_classes(&api);
        for excluded_class in OUT_OF_SCOPE {
            assert!(
                !ui.contains_key(excluded_class),
                "{excluded_class} should not be in scope"
            );
        }
    }

    /// THE DENOMINATOR IS PINNED, and a change here has to be a deliberate diff.
    ///
    /// It has moved by accident before: `accepts` happily stored `Archivable`,
    /// a replication flag the standard says a host may ignore, and the
    /// denominator went from 138 to 148 with the score behind it -- a scope
    /// decision overturned by an implementation detail. Nothing failed, because
    /// nothing was watching the number itself.
    ///
    /// If this test fails, the standard's scope changed. That is allowed and it
    /// is sometimes right -- `InputAction` legitimately took it from 138 to 139
    /// when the Roblox pin moved. Update the number here IN THE SAME COMMIT as
    /// the change that moved it, so the diff says which.
    #[test]
    fn the_denominator_is_139() {
        assert_eq!(
            in_scope_properties().len(),
            139,
            "the in-scope property count moved; see this test's comment"
        );
    }

    #[test]
    fn by_class_and_by_name_describe_the_same_set() {
        let by_name = in_scope_properties();
        let mut flattened: BTreeSet<String> = BTreeSet::new();
        for props in in_scope_by_class().values() {
            flattened.extend(props.iter().cloned());
        }
        assert_eq!(
            by_name, flattened,
            "the by-name and per-class views disagree about what is in scope"
        );
    }
}
