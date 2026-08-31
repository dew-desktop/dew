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

/// Properties a UI host is not expected to implement, and why.
///
/// EXCLUSION IS A DECISION, so it is a list rather than a heuristic. Each name
/// here is a claim that a conformant implementation may ignore it, and a claim
/// that can be argued with in a diff. The alternative -- pattern-matching on
/// prefixes -- silently absolves whatever happens to match.
///
/// Engine bookkeeping: replication, localisation, studio and asset plumbing.
/// None of it affects what is drawn or where.
const NOT_UI: &[&str] = &[
    "Archivable", "RobloxLocked", "AutoLocalize", "RootLocalizationTable", "Attributes",
    "AttributesReplicate", "AttributesSerialize", "SourceAssetId", "Sandboxed", "Capabilities",
    "Name", "Parent", "ClassName", "UniqueId", "HistoryId", "ActiveQueryNames",
];

/// Input devices this host does not have. A desktop widget is driven by a
/// pointer and a keyboard; gamepad focus traversal, selection groups and haptics
/// describe a console.
///
/// This is the one group most likely to move. If Dew ever grows gamepad support
/// these stop being out of scope and become backlog, which is exactly why they
/// are listed separately rather than lumped in above.
const INPUT_DEVICE: &[&str] = &[
    "NextSelectionUp", "NextSelectionDown", "NextSelectionLeft", "NextSelectionRight",
    "Selectable", "SelectionImageObject", "SelectionOrder", "SelectionGroup",
    "SelectionBehaviorUp", "SelectionBehaviorDown", "SelectionBehaviorLeft",
    "SelectionBehaviorRight", "GamepadInputEnabled", "HoverHapticEffect", "PressHapticEffect",
    // Touch and the on-screen keyboard: devices, not decisions.
    "TouchInputEnabled", "ShowNativeInput",
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
    // `--markdown` emits the standard's scope section instead of a report. The
    // document is generated rather than written for the reason d4 gives: the
    // datamodel surface has a machine-readable source, and a hand-maintained
    // list of 160 properties would be wrong within one Roblox release.
    let markdown = std::env::args().any(|a| a == "--markdown");
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

    per_class.sort_by(|a, b| b.2.cmp(&a.2));
    let not_ui: BTreeSet<&str> = NOT_UI.iter().copied().collect();
    let input_device: BTreeSet<&str> = INPUT_DEVICE.iter().copied().collect();

    let covered_total = all_props.iter().filter(|p| implemented.contains(*p)).count();
    let excluded: Vec<&str> = all_props
        .iter()
        .copied()
        .filter(|p| !implemented.contains(p) && (not_ui.contains(p) || input_device.contains(p)))
        .collect();
    let backlog: Vec<&str> = all_props
        .iter()
        .copied()
        .filter(|p| !implemented.contains(p) && !not_ui.contains(p) && !input_device.contains(p))
        .collect();

    // THE DENOMINATOR IS THE POINT. Against every writable property the figure
    // is meaningless, because it counts things nobody intends to implement.
    // Against what is in scope it is a completion percentage someone can act on.
    let in_scope_total = covered_total + backlog.len();

    if markdown {
        emit_markdown(
            &db.version, &ui_classes, &per_class, covered_total, in_scope_total,
            &excluded, &backlog,
        );
        return;
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

    println!("{:<28} {:>9}  {}", "CLASS", "COVERED", "OF");
    for (name, covered, total) in per_class.iter().take(18) {
        println!("{name:<28} {covered:>9}  {total}");
    }

    println!(
        "
IN SCOPE:  {covered_total} of {in_scope_total} ({:.0}%)",
        100.0 * covered_total as f64 / in_scope_total as f64
    );
    println!(
        "EXCLUDED:  {} ({} engine bookkeeping, {} input devices this host lacks)",
        excluded.len(),
        excluded.iter().filter(|p| not_ui.contains(*p)).count(),
        excluded.iter().filter(|p| input_device.contains(*p)).count()
    );

    println!(
        "
BACKLOG ({}) -- what conformance actually requires:",
        backlog.len()
    );
    for chunk in backlog.chunks(6) {
        println!("  {}", chunk.join(", "));
    }
}

fn emit_markdown(
    version: &[u32],
    ui_classes: &BTreeMap<&str, &rbx_reflection::ClassDescriptor>,
    per_class: &[(String, usize, usize)],
    covered: usize,
    in_scope: usize,
    excluded: &[&str],
    backlog: &[&str],
) {
    let v = version.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".");
    println!("# DataModel Standard: scope
");
    println!("<!-- GENERATED. Regenerate with:");
    println!("       cargo run --manifest-path host/Cargo.toml --bin datamodel_surface -- --markdown");
    println!("     Do not edit by hand; edit the classification lists in the tool. -->
");
    println!("Measured against Roblox **{v}**, from the reflection database that ships");
    println!("with `rbx_reflection_database`. It tracks Roblox releases, so re-running this");
    println!("after an update is how the standard notices the platform moved.
");
    println!("**{covered} of {in_scope} in-scope properties implemented.** {} more are excluded by", excluded.len());
    println!("decision, and {} classes under `GuiObject` are in scope.
", ui_classes.len() - OUT_OF_SCOPE.len());

    println!("## Coverage by class
");
    println!("| Class | Implemented | In the class |");
    println!("| :--- | ---: | ---: |");
    for (name, c, t) in per_class.iter().take(20) {
        println!("| `{name}` | {c} | {t} |");
    }

    println!("
## Out of scope
");
    println!("Excluded by decision rather than by oversight. Each is a claim that a");
    println!("conformant implementation may ignore it.
");
    println!("**Classes.** Video, viewports and chat windows are engine features rather than");
    println!("layout: {}.
", OUT_OF_SCOPE.iter().map(|c| format!("`{c}`")).collect::<Vec<_>>().join(", "));
    println!("**Properties.** {}
", excluded.iter().map(|p| format!("`{p}`")).collect::<Vec<_>>().join(", "));

    println!("## Backlog
");
    println!("What conformance actually requires, and nothing else.
");
    for chunk in backlog.chunks(8) {
        println!("- {}", chunk.iter().map(|p| format!("`{p}`")).collect::<Vec<_>>().join(", "));
    }
}
