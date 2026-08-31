//! What the DataModel Standard has to cover, measured rather than guessed.
//!
//! Walks Roblox's own reflection database for everything under `GuiObject` plus
//! the UI modifiers, and reports it against what this stack actually implements.
//! The answer is a number rather than an impression, and it is regenerated when
//! the database tracks a new Roblox build.
//!
//! Run with `cargo run --bin datamodel-surface`.

use rbx_reflection::{DataType, Scriptability};
use std::collections::{BTreeMap, BTreeSet};

/// What `Aether/src/host/Layout.luau` declares it reads, verbatim from
/// `Layout.Inputs`. Kept here rather than parsed: a hand-copied list that drifts
/// is visible in a diff, and a parser that silently matches nothing is not.
const IMPLEMENTED: &[&str] = &[
    "AnchorPoint", "AutomaticSize", "CanvasPosition", "FillDirection", "LayoutOrder", "Padding",
    "PaddingBottom", "PaddingLeft", "PaddingRight", "PaddingTop", "Position", "Scale", "Size",
    "Text", "TextSize", "Visible",
    // Read through accessors rather than by name, so absent from Layout.Inputs'
    // Properties list but no less implemented.
    "ClipsDescendants", "ZIndex", "Parent", "Name", "ClassName",
    // Carried by the display list rather than by layout.
    "BackgroundColor3", "BackgroundTransparency", "TextColor3", "TextTransparency",
    "TextXAlignment", "TextYAlignment", "CornerRadius", "Image", "ImageColor3",
    "ImageTransparency", "Color", "Thickness", "Transparency", "Rotation", "Offset",
];

/// Classes that exist under GuiObject and are not UI a host has to draw.
///
/// Video, viewports and chat windows are engine features rather than layout, and
/// a standard that demanded them would be describing Roblox rather than
/// describing a UI. Named explicitly so the exclusion is a decision in a diff
/// rather than a silent filter.
const OUT_OF_SCOPE: &[&str] = &[
    "VideoFrame", "VideoDisplay", "ViewportFrame", "TextChannelWindow", "RelativeGui",
];

/// The classes a UI actually uses. `GuiObject` descendants come from the
/// hierarchy; these are the modifiers, which are `UIComponent` rather than
/// `GuiObject` and so are not found by walking superclasses.
const MODIFIERS: &[&str] = &[
    "UICorner", "UIPadding", "UIListLayout", "UIGridLayout", "UIPageLayout", "UITableLayout",
    "UIGradient", "UIStroke", "UIScale", "UIAspectRatioConstraint", "UISizeConstraint",
    "UITextSizeConstraint", "UIFlexItem",
];

fn main() {
    let db = rbx_reflection_database::get().expect("bundled reflection database");
    let implemented: BTreeSet<&str> = IMPLEMENTED.iter().copied().collect();

    // Everything that is, or descends from, GuiObject.
    let mut ui_classes: BTreeMap<&str, &rbx_reflection::ClassDescriptor> = BTreeMap::new();
    for (name, class) in &db.classes {
        let mut cursor = Some(class);
        while let Some(c) = cursor {
            if c.name == "GuiObject" {
                ui_classes.insert(name.as_ref(), class);
                break;
            }
            cursor = c.superclass.as_ref().and_then(|s| db.classes.get(s));
        }
    }
    for m in MODIFIERS {
        if let Some(c) = db.classes.get(*m) {
            ui_classes.insert(m, c);
        }
    }

    let mut all_props: BTreeSet<&str> = BTreeSet::new();
    let mut per_class: Vec<(String, usize, usize)> = Vec::new();

    for (name, class) in &ui_classes {
        if OUT_OF_SCOPE.contains(name) {
            continue;
        }
        // INPUTS ONLY. A standard describes what an implementation must ACCEPT,
        // not what it reports back: `AbsolutePosition` and `AbsoluteSize` are
        // read-only results of layout, and demanding them of an implementation
        // that has not laid out yet is incoherent. Deprecated properties are
        // excluded for the same reason nobody should implement against them.
        let mut props: Vec<&str> = Vec::new();
        let mut cursor = Some(*class);
        while let Some(c) = cursor {
            for (pname, p) in &c.properties {
                if p.tags.contains(&rbx_reflection::PropertyTag::Deprecated) {
                    continue;
                }
                if !matches!(p.data_type, DataType::Value(_) | DataType::Enum(_)) {
                    continue;
                }
                if !matches!(p.scriptability, Scriptability::ReadWrite | Scriptability::Write) {
                    continue;
                }
                props.push(pname.as_ref());
            }
            cursor = c.superclass.as_ref().and_then(|s| db.classes.get(s));
        }
        props.sort_unstable();
        props.dedup();
        let covered = props.iter().filter(|p| implemented.contains(*p)).count();
        for p in &props {
            all_props.insert(p);
        }
        per_class.push((name.to_string(), covered, props.len()));
    }

    let in_scope = ui_classes.len()
        - OUT_OF_SCOPE.iter().filter(|c| ui_classes.contains_key(*c)).count();

    println!(
        "DataModel surface, from Roblox {}
",
        db.version.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".")
    );
    println!(
        "{in_scope} classes in scope of {} under GuiObject, {} writable properties
",
        ui_classes.len(),
        all_props.len()
    );

    per_class.sort_by(|a, b| b.2.cmp(&a.2));
    println!("{:<28} {:>9}  {}", "CLASS", "COVERED", "OF");
    for (name, covered, total) in per_class.iter().take(18) {
        println!("{name:<28} {covered:>9}  {total}");
    }

    let covered_total = all_props.iter().filter(|p| implemented.contains(*p)).count();
    println!(
        "\nOVERALL: {covered_total} of {} distinct properties ({:.0}%)",
        all_props.len(),
        100.0 * covered_total as f64 / all_props.len() as f64
    );

    // The list nobody has written down: what a second implementation would have
    // to add. Printed sorted so a diff between runs is readable.
    let missing: Vec<&str> = all_props
        .iter()
        .copied()
        .filter(|p| !implemented.contains(p))
        .collect();
    println!("\nNOT IMPLEMENTED ({}):", missing.len());
    for chunk in missing.chunks(6) {
        println!("  {}", chunk.join(", "));
    }
}
