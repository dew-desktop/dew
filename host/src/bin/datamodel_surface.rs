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

// ── The method and event surface ─────────────────────────────────────────────

/// The scriptable API surface, pinned.
///
/// NOT FROM `rbx_reflection_database`, and it cannot be. That crate's
/// `ClassDescriptor` carries exactly `name`, `tags`, `superclass`, `properties`
/// and `default_properties` -- it is built for file serialisation, and a method
/// cannot be serialised. So the property half of this tool has a machine-readable
/// source and the method half had none, which is why the API surface has never
/// been measured while the property surface has been measured since d4.
///
/// `scripts/fetch_api_surface.luau` writes this from Roblox's own API dump.
/// Embedded rather than read at runtime so the data is part of the build: a
/// missing or malformed file is a compile error rather than a tool that runs and
/// reports a smaller surface than exists.
const API_SURFACE: &str = include_str!("../../datamodel/api_surface.json");

#[derive(serde::Deserialize)]
struct ApiSurface {
    version: String,
    classes: BTreeMap<String, ApiClass>,
}

#[derive(serde::Deserialize)]
struct ApiClass {
    superclass: Option<String>,
    /// Member name to "Function" or "Event".
    ///
    /// A MAP RATHER THAN A LIST because Lune, which writes this file, encodes an
    /// empty array and an empty map identically as `{}` -- and most classes in
    /// the chain introduce no members of their own. A list shape failed to
    /// deserialise on precisely the classes that carry nothing, which is the
    /// least useful place for a format to be ambiguous.
    members: BTreeMap<String, String>,
}

// What DEW'S HOST implements is asked of the HOST, not listed here either.
//
// THIS WAS `IMPLEMENTED_MEMBERS: &[&str] = &[]`, with a comment saying the
// emptiness was the measurement. It was, right up until it stopped being -- and
// then it would have been an array somebody edited, which is exactly what the
// property half was before `accepts` existed and exactly how this document came
// to report 35 of 138 for a surface the host had never implemented.
//
// `dew_host::datamodel::members::implements` is the predicate `__index` itself
// consults before it will hand a guest a function. Asking it means this document
// cannot claim a method the dispatch will not answer, and cannot miss one it
// will.
//
// WHAT IS BEING MEASURED, because getting this wrong makes the number
// meaningless: the standard's two implementations are the ROBLOX ENGINE and the
// DEW HOST. Both run Luau applications; an application must mount and render the
// same on either, WITH OR WITHOUT Aether. Aether is a headless framework that
// runs on top of a host, the way Ark UI runs on top of a DOM -- it is a consumer
// of this surface and never an implementation of it, so nothing Aether provides
// counts here. `createPressable` taking `OnClick` says something about Aether's
// API and nothing about whether a Dew guest can write
// `button.Activated:Connect(fn)`.

/// Animation belongs to another seam, on both hosts.
///
/// `TweenPosition` bakes an easing curve into the instance and cannot be
/// interrupted and re-targeted without restarting. Excluding it is a claim that
/// a conformant host may offer motion through a separate mechanism rather than
/// through methods on the object, and it is the one exclusion here most likely
/// to be argued with -- an application calling `frame:TweenPosition(...)` on the
/// engine has no equivalent line to write on a conformant host that omits it.
const MOTION: &[&str] = &["TweenPosition", "TweenSize", "TweenSizeAndPosition"];

/// Engine bookkeeping, the method-and-event twin of `NOT_UI`.
///
/// Tags, attributes, styling, wiring, actors and reflection-on-self. None of it
/// affects what is drawn or where, and a standard that demanded it would be
/// describing Roblox's object model rather than describing a UI.
const NOT_UI_MEMBERS: &[&str] = &[
    "AddTag",
    "GetTags",
    "HasTag",
    "RemoveTag",
    "GetAttribute",
    "GetAttributes",
    "SetAttribute",
    "GetAttributeChangedSignal",
    "AttributeChanged",
    "GetStyled",
    "GetStyledPropertyChangedSignal",
    "StyledPropertiesChanged",
    "GetConnectedWires",
    "GetInputPins",
    "GetOutputPins",
    "WiringChanged",
    "GetActor",
    "GetFullName",
    "IsPropertyModified",
    "ResetPropertyToDefault",
    "QueryDescendants",
    "Clone",
    "AncestryChanged",
];

/// Input devices this host does not have -- the twin of `INPUT_DEVICE`.
///
/// Touch gestures, gamepad selection traversal, and the on-screen keyboard's
/// return key. Same reasoning and the same caveat: if Dew grows gamepad or touch
/// support these stop being out of scope and become backlog, which is why they
/// are listed apart rather than lumped in above.
const INPUT_DEVICE_MEMBERS: &[&str] = &[
    "TouchTap",
    "TouchPan",
    "TouchPinch",
    "TouchRotate",
    "TouchSwipe",
    "TouchLongPress",
    "SelectionGained",
    "SelectionLost",
    "SelectionChanged",
    "ReturnPressedFromOnScreenKeyboard",
];

// What DEW'S HOST accepts is asked of the HOST, not listed here.
//
// `dew_host::datamodel::accepts` is the predicate the guest's own assignment
// path uses: the property must exist on the class or an ancestor, be writable,
// and carry a type `coerce` can store. Calling it means this document cannot
// claim a coverage the code does not have, which a hand-written list can and did
// -- it said 35 of 138 while the host implemented none of it, because the names
// in it were Aether's.

/// What `Aether/src/host/Layout.luau` declares it reads, verbatim from
/// `Layout.Inputs`. Kept here rather than parsed: a hand-copied list that drifts
/// is visible in a diff, and a parser that silently matches nothing is not.
///
/// NOT A CONFORMANCE FIGURE. Aether is a headless framework that runs on top of
/// a host, the way Ark UI runs on top of a DOM; it is not an implementation of
/// this standard and is not required to conform. What this list is good for is
/// splitting the backlog: a property the pipeline already honours needs host
/// work only, and one it does not needs host work AND rendering work. That is a
/// genuinely different cost, and it is the most useful thing this list can say.
const AETHER_PIPELINE: &[&str] = &[
    "AnchorPoint",
    "AutomaticSize",
    "CanvasPosition",
    "FillDirection",
    "LayoutOrder",
    "Padding",
    "PaddingBottom",
    "PaddingLeft",
    "PaddingRight",
    "PaddingTop",
    "Position",
    "Scale",
    "Size",
    "Text",
    "TextSize",
    "Visible",
    // Read through accessors rather than by name, so absent from Layout.Inputs'
    // Properties list but no less implemented.
    "ClipsDescendants",
    "ZIndex",
    // Carried by the display list rather than by layout.
    "BackgroundColor3",
    "BackgroundTransparency",
    "TextColor3",
    "TextTransparency",
    "TextXAlignment",
    "TextYAlignment",
    "CornerRadius",
    "Image",
    "ImageColor3",
    "ImageTransparency",
    "Color",
    "Thickness",
    "Transparency",
    "Rotation",
    "Offset",
    // Read through accessors rather than by name, so absent from Layout.Inputs'
    // Properties list but no less honoured by the pipeline.
    "Parent",
    "Name",
    "ClassName",
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
///
/// REDRAWN FOR THE CURRENT SUBJECT. "Does not affect what is drawn" was the right
/// test while this document measured a layout pipeline. The subject is now what a
/// GUEST CAN REACH, and by that test `Parent` and `Name` were never bookkeeping:
/// `Parent` is how a tree is built and an application cannot construct anything
/// without it, and `Name` is what `FindFirstChild` searches on. Both moved out of
/// this list and into the backlog, where they are the first two things a
/// DataModel needs rather than things a conformant host may ignore.
///
/// `ClassName` never reached this list either way -- it is read-only, and the
/// walk above keeps only writable properties, because a standard describes what
/// an implementation must ACCEPT.
///
/// What remains is bookkeeping under both tests: replication, localisation,
/// studio and asset plumbing, and the attribute system.
const NOT_UI: &[&str] = &[
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
const INPUT_DEVICE: &[&str] = &[
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
const OUT_OF_SCOPE: &[&str] = &[
    "VideoFrame",
    "VideoDisplay",
    "ViewportFrame",
    "TextChannelWindow",
    "RelativeGui",
];

/// The classes a UI actually uses. `GuiObject` descendants come from the
/// hierarchy; these are the modifiers, which are `UIComponent` rather than
/// `GuiObject` and so are not found by walking superclasses.
const MODIFIERS: &[&str] = &[
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

fn main() {
    // `--markdown` emits the standard's scope section instead of a report. The
    // document is generated rather than written for the reason d4 gives: the
    // datamodel surface has a machine-readable source, and a hand-maintained
    // list of 160 properties would be wrong within one Roblox release.
    let markdown = std::env::args().any(|a| a == "--markdown");
    let db = rbx_reflection_database::get().expect("bundled reflection database");
    let pipeline: BTreeSet<&str> = AETHER_PIPELINE.iter().copied().collect();

    // ASKED OF THE HOST, one property at a time, for every class in scope.
    //
    // A NAME COUNTS AS ACCEPTED ONLY IF IT IS ACCEPTED EVERYWHERE IT APPEARS, and
    // getting that backwards produced a 100% that was not true. The census is by
    // NAME, and the same name is a different type on unrelated classes:
    // `UIStroke.Color` is a `Color3` the host takes, `UIGradient.Color` is a
    // `ColorSequence` it does not. Counting a name as accepted when ANY class
    // accepted it let the first mask the second, and `Color`, `Transparency` and
    // anything else split that way vanished from the backlog while still being
    // unassignable.
    //
    // `disagreed` is kept and reported rather than silently resolved, because a
    // name the host takes on one class and refuses on another is a fact about the
    // surface worth seeing.
    let mut accepted_somewhere: BTreeSet<&str> = BTreeSet::new();
    let mut refused_somewhere: BTreeSet<&str> = BTreeSet::new();

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
                if !matches!(
                    p.scriptability,
                    Scriptability::ReadWrite | Scriptability::Write
                ) {
                    continue;
                }
                props.push(pname.as_ref());
            }
            cursor = c.superclass.as_ref().and_then(|s| db.classes.get(s));
        }
        props.sort_unstable();
        props.dedup();
        for property in &props {
            if dew_host::datamodel::accepts(name, property) {
                accepted_somewhere.insert(property);
            } else {
                refused_somewhere.insert(property);
            }
        }

        // The per-class column reports what AETHER'S PIPELINE honours, which is a
        // different question from what the host accepts and is labelled as such
        // at every print site so the two are never read as one number.
        let covered = props.iter().filter(|p| pipeline.contains(*p)).count();
        for p in &props {
            all_props.insert(p);
        }
        per_class.push((name.to_string(), covered, props.len()));
    }

    per_class.sort_by_key(|a| std::cmp::Reverse(a.2));
    let not_ui: BTreeSet<&str> = NOT_UI.iter().copied().collect();
    let input_device: BTreeSet<&str> = INPUT_DEVICE.iter().copied().collect();

    // EXCLUSION WINS OVER ACCEPTANCE, and it did not until the host became real.
    //
    // `accepts` answers a mechanical question: would the assignment path store
    // this. The generic path stores any writable primitive, so it happily accepts
    // `Archivable` -- a replication flag this standard says a conformant host may
    // ignore. Counting that as coverage inflated the denominator from 138 to 148
    // and the score with it, which is a scope decision being overturned by an
    // implementation detail.
    //
    // The guard used to run the other way, letting `implemented` win, because the
    // old Aether list carried `Name` and `Parent` while `NOT_UI` also did. Those
    // moved out of `NOT_UI` in their own commit, so nothing needs that guard now.
    let excluded: Vec<&str> = all_props
        .iter()
        .copied()
        .filter(|p| not_ui.contains(p) || input_device.contains(p))
        .collect();
    let disagreed: Vec<&str> = accepted_somewhere
        .intersection(&refused_somewhere)
        .copied()
        .filter(|p| !not_ui.contains(p) && !input_device.contains(p))
        .collect();
    let implemented: BTreeSet<&str> = accepted_somewhere
        .difference(&refused_somewhere)
        .copied()
        .filter(|p| !not_ui.contains(p) && !input_device.contains(p))
        .collect();

    // COUNTED AFTER THE NARROWING, not before. Computing this against the raw
    // `accepts` answer counted excluded properties as coverage and reported 63
    // of 148 where the honest figure is against 138.
    let covered_total = all_props
        .iter()
        .filter(|p| implemented.contains(*p))
        .count();
    let backlog: Vec<&str> = all_props
        .iter()
        .copied()
        .filter(|p| !implemented.contains(p) && !not_ui.contains(p) && !input_device.contains(p))
        .collect();

    // TWO COSTS, NOT ONE. A property Aether's pipeline already honours needs the
    // host to accept and store it and nothing else; one it does not needs host
    // work AND rendering work. Splitting the backlog this way is the whole reason
    // to keep the Aether list in a document that does not measure Aether.
    let (backlog_host_only, backlog_host_and_render): (Vec<&str>, Vec<&str>) =
        backlog.iter().partition(|p| pipeline.contains(*p));
    let pipeline_covered = all_props.iter().filter(|p| pipeline.contains(*p)).count();

    // ── The method and event surface, measured the same way ──────────────────
    //
    // Members are declared on the class that INTRODUCES them and are not
    // repeated on descendants, so this walks the superclass chain exactly as the
    // property pass does. `TextButton` declares one function of its own; the
    // forty-odd a script can call on it come from six ancestors.
    let api: ApiSurface =
        serde_json::from_str(API_SURFACE).expect("host/datamodel/api_surface.json is malformed");

    let motion: BTreeSet<&str> = MOTION.iter().copied().collect();
    let not_ui_members: BTreeSet<&str> = NOT_UI_MEMBERS.iter().copied().collect();
    let input_device_members: BTreeSet<&str> = INPUT_DEVICE_MEMBERS.iter().copied().collect();

    let mut all_members: BTreeMap<&str, &str> = BTreeMap::new();
    // Member name to a CONCRETE in-scope class that reaches it.
    //
    // The census is by name -- a member counts once however many classes inherit
    // it -- but the host's predicate takes a class, because `CaptureFocus` is a
    // `GuiObject`'s and `GetScrollVelocity` is a `ScrollingFrame`'s. Asking about
    // the class the walk found the member on is the only question that has a
    // right answer: asking about `Instance` would report a `ScrollingFrame`
    // method as unimplemented even once it is, and asking about the class that
    // DECLARES it would name `Object`, which `rbx_reflection_database` does not
    // carry at all.
    let mut reached_from: BTreeMap<&str, &str> = BTreeMap::new();
    let mut missing_from_dump: Vec<&str> = Vec::new();
    for name in ui_classes.keys() {
        if OUT_OF_SCOPE.contains(name) {
            continue;
        }
        let mut cursor = Some((*name).to_string());
        let mut found = false;
        while let Some(current) = cursor {
            let Some(class) = api.classes.get(&current) else {
                break;
            };
            found = true;
            for (member, kind) in &class.members {
                all_members.insert(member, kind);
                reached_from.entry(member).or_insert(name);
            }
            cursor = class.superclass.clone();
        }
        // A class the reflection database has and the pinned dump does not means
        // the two describe different Roblox builds, or that MODIFIERS drifted
        // from the fetch script's copy of it. The surface reported would be short
        // either way, so it is said out loud rather than shrugged off.
        if !found {
            missing_from_dump.push(name);
        }
    }

    let classify = |m: &str| -> &'static str {
        let concrete = reached_from.get(m).copied().unwrap_or("Instance");
        if dew_host::datamodel::members::implements(concrete, m) {
            "implemented"
        } else if motion.contains(m)
            || not_ui_members.contains(m)
            || input_device_members.contains(m)
        {
            "excluded"
        } else {
            "backlog"
        }
    };

    let member_methods = all_members.values().filter(|k| **k == "Function").count();
    let member_events = all_members.len() - member_methods;
    let m_implemented = all_members
        .keys()
        .filter(|m| classify(m) == "implemented")
        .count();
    let m_excluded = all_members
        .keys()
        .filter(|m| classify(m) == "excluded")
        .count();
    let member_backlog: Vec<&str> = all_members
        .keys()
        .copied()
        .filter(|m| classify(m) == "backlog")
        .collect();
    let implemented_members: Vec<&str> = all_members
        .keys()
        .copied()
        .filter(|m| classify(m) == "implemented")
        .collect();
    let implemented_events = all_members
        .iter()
        .filter(|(m, kind)| **kind == "Event" && classify(m) == "implemented")
        .count();
    let member_in_scope = m_implemented + member_backlog.len();

    // THE TWO HALVES CITE DIFFERENT ROBLOX BUILDS, and until now nothing said so.
    // The property surface tracks whatever `rbx_reflection_database` ships; the
    // member surface is pinned by scripts/fetch_api_surface.luau; the conformance
    // cases record `verifiedAgainst` by hand. A standard measured against two
    // builds at once is one no implementation can actually satisfy, so the skew
    // is reported rather than left to be found by reading two files side by side.
    let db_version = db
        .version
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".");
    let build_skew = if db_version == api.version {
        None
    } else {
        Some((db_version.clone(), api.version.clone()))
    };

    // THE DENOMINATOR IS THE POINT. Against every writable property the figure
    // is meaningless, because it counts things nobody intends to implement.
    // Against what is in scope it is a completion percentage someone can act on.
    let in_scope_total = covered_total + backlog.len();

    if markdown {
        emit_markdown(
            &db.version,
            &ui_classes,
            &per_class,
            covered_total,
            in_scope_total,
            &excluded,
            &backlog,
            &disagreed,
            pipeline_covered,
            &backlog_host_only,
            &backlog_host_and_render,
            &api.version,
            member_methods,
            member_events,
            m_implemented,
            member_in_scope,
            m_excluded,
            &member_backlog,
            &implemented_members,
            implemented_events,
            build_skew.as_ref(),
        );
        return;
    }

    let in_scope = ui_classes.len()
        - OUT_OF_SCOPE
            .iter()
            .filter(|c| ui_classes.contains_key(*c))
            .count();

    println!(
        "DataModel surface, from Roblox {}
",
        db.version
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".")
    );
    println!(
        "{in_scope} classes in scope of {} under GuiObject, {} writable properties
",
        ui_classes.len(),
        all_props.len()
    );

    println!("{:<28} {:>10}  OF", "CLASS", "RENDERABLE");
    for (name, covered, total) in per_class.iter().take(18) {
        println!("{name:<28} {covered:>9}  {total}");
    }

    println!();
    println!(
        "IN SCOPE:  {covered_total} of {in_scope_total} accepted by the Dew host ({:.0}%)",
        100.0 * covered_total as f64 / in_scope_total as f64
    );
    println!("PIPELINE:  {pipeline_covered} of those are honoured by Aether's renderer already");
    if !disagreed.is_empty() {
        println!(
            "SPLIT:     {} name(s) the host takes on one class and refuses on another: {}",
            disagreed.len(),
            disagreed.join(", ")
        );
    }
    println!(
        "EXCLUDED:  {} ({} engine bookkeeping, {} input devices this host lacks)",
        excluded.len(),
        excluded.iter().filter(|p| not_ui.contains(*p)).count(),
        excluded
            .iter()
            .filter(|p| input_device.contains(*p))
            .count()
    );

    println!();
    println!(
        "BACKLOG ({}) -- what conformance actually requires, split by cost:",
        backlog.len()
    );
    println!();
    println!(
        "  HOST ONLY ({}) -- Aether's pipeline already honours these:",
        backlog_host_only.len()
    );
    for chunk in backlog_host_only.chunks(6) {
        println!("    {}", chunk.join(", "));
    }
    println!();
    println!("  HOST AND RENDERING ({}):", backlog_host_and_render.len());
    for chunk in backlog_host_and_render.chunks(6) {
        println!("    {}", chunk.join(", "));
    }

    println!(
        "
METHODS AND EVENTS, from the pinned dump at Roblox {}
",
        api.version
    );
    println!(
        "{} in scope of {} reachable ({member_methods} methods, {member_events} events)",
        member_in_scope,
        all_members.len()
    );
    println!(
        "IN SCOPE:  {m_implemented} of {member_in_scope} ({:.0}%)",
        100.0 * m_implemented as f64 / member_in_scope as f64
    );
    println!("EXCLUDED:  {m_excluded} (engine bookkeeping, input devices, motion)");
    println!(
        "
API BACKLOG ({}) -- no Dew guest can reach any of these:",
        member_backlog.len()
    );
    for chunk in member_backlog.chunks(6) {
        println!("  {}", chunk.join(", "));
    }

    for name in &missing_from_dump {
        println!();
        println!("WARNING  {name} is in scope and absent from host/datamodel/api_surface.json.");
        println!("         Regenerate it: lune run scripts/fetch_api_surface.luau");
    }
    if let Some((property_build, member_build)) = &build_skew {
        println!();
        println!("WARNING  this standard is measured against TWO Roblox builds.");
        println!("         properties  {property_build}  (whatever rbx_reflection_database ships)");
        println!("         methods     {member_build}  (pinned by scripts/fetch_api_surface.luau)");
        println!("         Conformance against two builds at once is not a thing an");
        println!("         implementation can satisfy.");
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_markdown(
    version: &[u32],
    ui_classes: &BTreeMap<&str, &rbx_reflection::ClassDescriptor>,
    per_class: &[(String, usize, usize)],
    covered: usize,
    in_scope: usize,
    excluded: &[&str],
    backlog: &[&str],
    disagreed: &[&str],
    pipeline_covered: usize,
    backlog_host_only: &[&str],
    backlog_host_and_render: &[&str],
    api_version: &str,
    member_methods: usize,
    member_events: usize,
    m_implemented: usize,
    member_in_scope: usize,
    m_excluded: usize,
    member_backlog: &[&str],
    implemented_members: &[&str],
    implemented_events: usize,
    build_skew: Option<&(String, String)>,
) {
    let v = version
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".");
    println!(
        "# DataModel Standard: scope
"
    );
    println!("<!-- GENERATED. Regenerate with:");
    println!("       lune run scripts/fetch_api_surface.luau   # only to move the Roblox pin");
    print!("       cargo run --manifest-path host/Cargo.toml --bin datamodel-surface");
    println!(" -- --markdown > docs/datamodel_scope.md");
    println!("     Do not edit by hand; edit the classification lists in the tool.");
    println!(
        "     The bin is datamodel-surface, hyphenated. This line said datamodel_surface and
     the command it gave had never run. -->
"
    );
    println!("Measured against Roblox **{v}**, from the reflection database that ships");
    println!("with `rbx_reflection_database`. It tracks Roblox releases, so re-running this");
    println!(
        "after an update is how the standard notices the platform moved.
"
    );
    println!("**THE SUBJECT IS THE DEW HOST**, measured against the Roblox engine. Aether is a");
    println!("headless framework that runs on top of a host, the way Ark UI runs on top of a");
    println!("DOM; it is a consumer of this surface, never an implementation of it, and is not");
    println!("required to conform. What must match is what a Luau application sees, **with or");
    println!("without Aether**.");
    println!();
    println!(
        "**{covered} of {in_scope} in-scope properties accepted by the host.** {} more are",
        excluded.len()
    );
    println!(
        "excluded by decision, and {} classes under `GuiObject` are in scope.",
        ui_classes.len() - OUT_OF_SCOPE.len()
    );
    println!();
    println!("Of those {in_scope}, **{pipeline_covered} are already honoured by Aether's");
    println!("renderer**. That is not a conformance figure; it splits the backlog by cost.");
    println!();

    println!(
        "## Coverage by class
"
    );
    println!("The middle column is what AETHER'S RENDERER honours, not what the host accepts.");
    println!("The two are different questions and the gap between them is the backlog: a");
    println!("property the host stores but the pipeline ignores is stored and not drawn.");
    println!();
    println!("| Class | Renderable | In the class |");
    println!("| :--- | ---: | ---: |");
    for (name, c, t) in per_class.iter().take(20) {
        println!("| `{name}` | {c} | {t} |");
    }

    println!(
        "
## Out of scope
"
    );
    println!("Excluded by decision rather than by oversight. Each is a claim that a");
    println!(
        "conformant implementation may ignore it.
"
    );
    println!("**Classes.** Video, viewports and chat windows are engine features rather than");
    println!(
        "layout: {}.
",
        OUT_OF_SCOPE
            .iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "**Properties.** {}
",
        excluded
            .iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    if !disagreed.is_empty() {
        println!("## Names the host is split on");
        println!();
        println!("Accepted on one class and refused on another, because the same property name");
        println!("is a different type on unrelated classes. `UIStroke.Color` is a `Color3` the");
        println!("host takes; `UIGradient.Color` is a `ColorSequence` it does not.");
        println!();
        println!("These count as NOT accepted. Counting them the other way let one class mask");
        println!("another, and reported the surface as 100% complete while two properties were");
        println!("still unassignable.");
        println!();
        for name in disagreed {
            println!("- `{name}`");
        }
        println!();
    }

    println!("## Property backlog");
    println!();
    println!(
        "What conformance actually requires, split by what it costs. {} properties.",
        backlog.len()
    );
    println!();
    println!("### Host work only ({})", backlog_host_only.len());
    println!();
    println!("Aether's renderer already honours these, so the host has to accept, validate and");
    println!("store them and nothing else has to change.");
    println!();
    for chunk in backlog_host_only.chunks(8) {
        println!(
            "- {}",
            chunk
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!();
    println!("### Host and rendering ({})", backlog_host_and_render.len());
    println!();
    for chunk in backlog_host_and_render.chunks(8) {
        println!(
            "- {}",
            chunk
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    println!();
    println!("## Methods and events");
    println!();
    println!("The other half of what an application can reach, and the half that had never");
    println!("been measured. `rbx_reflection_database` carries no methods and no events, so");
    println!("this comes from Roblox's own API dump, pinned at **{api_version}** by");
    println!("`scripts/fetch_api_surface.luau`.");
    println!();
    println!("**THE TWO IMPLEMENTATIONS ARE THE ROBLOX ENGINE AND THE DEW HOST.** Aether is a");
    println!("headless framework that runs on top of a host, the way Ark UI runs on top of a");
    println!("DOM. It is a consumer of this surface and never an implementation of it, so it");
    println!("is not measured here and is not required to conform. What must match is what a");
    println!("Luau application sees, **with or without Aether**.");
    println!();
    // ONE LITERAL PER LINE, no `\`-continuations. rustfmt joins a continued
    // string literal and keeps the indentation it was wrapped with, so the
    // generated markdown came out with runs of spaces inside a sentence.
    println!("**{m_implemented} of {member_in_scope} in-scope members implemented.**",);
    println!(
        "{m_excluded} more are excluded by decision, out of {} reachable",
        member_in_scope + m_excluded
    );
    println!("({member_methods} methods, {member_events} events).");
    println!();
    println!("Asked of `dew_host::datamodel::members::implements`, the predicate `__index`");
    println!("consults before it hands a guest a function -- so nothing below is a claim this");
    println!("document makes on the host's behalf.");
    println!();
    println!("### Implemented");
    println!();
    if implemented_members.is_empty() {
        println!("Nothing yet.");
    } else {
        for chunk in implemented_members.chunks(8) {
            println!(
                "- {}",
                chunk
                    .iter()
                    .map(|m| format!("`{m}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    // GENERATED, NOT ASSERTED. "No event is reachable" is a sentence that goes
    // stale the moment one is, so it is printed from the count rather than
    // written down -- the same reason the numbers above it are.
    if implemented_events == 0 {
        println!();
        println!("**No event is reachable.** There is no signal type yet, so there is nothing");
        println!("for a guest to connect to and nothing for the host to fire. That is the half");
        println!("of the surface a mod needs before it can respond to anything at all.");
    }
    println!();
    println!("### API backlog");
    println!();
    println!("What parity actually requires. No Dew guest can reach any of these.");
    println!();
    for chunk in member_backlog.chunks(8) {
        println!(
            "- {}",
            chunk
                .iter()
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if let Some((property_build, member_build)) = build_skew {
        println!();
        println!("### This document measures two Roblox builds at once");
        println!();
        println!("| half | build | pinned by |");
        println!("| :--- | :--- | :--- |");
        println!("| properties | {property_build} | whatever `rbx_reflection_database` ships |");
        println!("| methods and events | {member_build} | `scripts/fetch_api_surface.luau` |");
        println!();
        println!("Conformance against two builds at once is not a thing an implementation can");
        println!("satisfy. Closing this means moving the property half onto the pinned dump too,");
        println!("or pinning the crate to the build the dump names.");
    }
}
