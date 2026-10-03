//! What the DataModel Standard has to cover, measured rather than guessed.
//!
//! Walks the engine's pinned API dump for everything under `GuiObject` plus the UI
//! modifiers, and reports it against what this stack actually implements. The
//! answer is a number rather than an impression, and it is regenerated when
//! the pinned dump tracks a new upstream build.
//!
//! Run with `cargo run --bin datamodel-surface`.

use dew_host::datamodel::render::honours::{honours, Honour};
use dew_host::scope::{
    ApiClass, ApiSurface, API_SURFACE_MISSING, INPUT_DEVICE, MODIFIERS, NOT_UI, OUT_OF_SCOPE,
};
use std::collections::{BTreeMap, BTreeSet};

/// One class's row: its in-scope properties, and three different questions
/// asked of each.
struct ClassRow {
    name: String,
    in_scope: usize,
    /// `datamodel::accepts`: would an assignment be stored.
    accepted: usize,
    /// `render::honours`: does Dew's renderer read it, whole.
    implemented: Vec<String>,
    /// `render::honours`: read, with what is missing.
    partial: Vec<(String, &'static str)>,
    /// `render::honours`: read, and drawn differently from the engine on
    /// purpose, with how.
    divergent: Vec<(String, &'static str)>,
    /// `render::honours`: never read.
    absent: Vec<String>,
    /// By name, in `AETHER_PIPELINE`.
    pipeline: usize,
}

impl ClassRow {
    fn rendered(&self) -> usize {
        self.implemented.len() + self.partial.len() + self.divergent.len()
    }
}

// ── The method and event surface ─────────────────────────────────────────────

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

/// Tags, styling, wiring, actors and reflection-on-self.
///
/// Attributes (GetAttribute, GetAttributes, SetAttribute) were originally
/// excluded under the claim that none of it "affects what is drawn or where".
/// However, `aether/src/host/DataModel.luau` and `HitTest.luau` depend on attributes
/// for hit testing corner radii and bounds overrides. Like `Parent` and `Name`,
/// they are host infrastructure that UI frameworks require and are now implemented.
const NOT_UI_MEMBERS: &[&str] = &[
    "AddTag",
    "GetTags",
    "HasTag",
    "RemoveTag",
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

/// Which members are POINTER INPUT, as opposed to something an instance says
/// about itself.
///
/// A CLASSIFICATION OF ROBLOX'S API, WHICH IS THIS TOOL'S JOB, and it is the one
/// thing here that is a list rather than a question asked of the host. Whether
/// Dew implements any of them is still `implements`'s answer -- this only says
/// which names would count if it did, the same way `NOT_UI_MEMBERS` says which
/// names are out of scope without claiming anything about the host.
///
/// IT EXISTS TO KEEP ONE PARAGRAPH HONEST. The prose under "Implemented" used to
/// assert that no implemented event was input, which was true for exactly one
/// sprint. Printing that claim from a count means it corrects itself instead of
/// having to be noticed.
///
/// FOCUS IS NOT HERE. `Focused` and `FocusLost` are input in the ordinary sense
/// and they are a different mechanism -- an owner and keyboard routing -- so
/// counting them here would make this paragraph claim a pointer story the host
/// does not have.
const INPUT_MEMBERS: &[&str] = &[
    "Activated",
    "SecondaryActivated",
    "MouseButton1Click",
    "MouseButton1Down",
    "MouseButton1Up",
    "MouseButton2Click",
    "MouseButton2Down",
    "MouseButton2Up",
    "MouseEnter",
    "MouseLeave",
    "MouseMoved",
    "MouseWheelForward",
    "MouseWheelBackward",
    "InputBegan",
    "InputChanged",
    "InputEnded",
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

/// What Aether's pipeline honours: the first two groups are Aether's
/// `src/core/Layout.luau` `Layout.Inputs`, verbatim, and the third is what its
/// display list carries. Kept here rather than parsed: a hand-copied list that
/// drifts is visible in a diff, and a parser that silently matches nothing is
/// not.
///
/// NOT A DEW FIGURE EITHER. What Dew's renderer honours is asked of
/// `dew_host::datamodel::render::honours`, per class; this list is by name and
/// describes Aether.
///
/// NOT A CONFORMANCE FIGURE. Aether is a headless framework that runs on top of
/// a host, the way Ark UI runs on top of a DOM; it is not an implementation of
/// this standard and is not required to conform. What this list is good for is
/// splitting the backlog: a property the pipeline already honours needs host
/// work only, and one it does not needs host work AND rendering work. That is a
/// genuinely different cost, and it is the most useful thing this list can say.
const AETHER_PIPELINE: &[&str] = &[
    // `Layout.Inputs.Properties`.
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
    "TextScaled",
    "TextSize",
    "TextWrapped",
    "Visible",
    // `Layout.Inputs.Accessors`, as the property each one reads: `Clips`,
    // `ZIndex`, `Parent` and `ClassOf`. `Children` and `Raw` read no property
    // of their own.
    "ClipsDescendants",
    "ZIndex",
    "Parent",
    "ClassName",
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
];

fn main() {
    // `--markdown` emits the standard's scope section instead of a report. The
    // document is generated rather than written for the reason d4 gives: the
    // datamodel surface has a machine-readable source, and a hand-maintained
    // list of 160 properties would be wrong within one upstream release.
    let markdown = std::env::args().any(|a| a == "--markdown");
    // NOT GENERATED IS AN ORDINARY STATE, and this says so rather than panicking:
    // the file is gitignored because it derives from a third party's published
    // data (NOTICE), so a fresh clone has never had it.
    let Some(api): Option<ApiSurface> = dew_host::scope::api() else {
        eprintln!("{API_SURFACE_MISSING}");
        std::process::exit(2);
    };
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
    let mut ui_classes: BTreeMap<&str, &ApiClass> = BTreeMap::new();
    for (name, class) in &api.classes {
        if name == "GuiObject" {
            ui_classes.insert(name.as_str(), class);
            continue;
        }
        let mut cursor = class.superclass.as_deref();
        while let Some(c_name) = cursor {
            if c_name == "GuiObject" {
                ui_classes.insert(name.as_str(), class);
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
            ui_classes.insert(m, c);
        }
    }

    let mut all_props: BTreeSet<&str> = BTreeSet::new();
    let mut per_class: Vec<ClassRow> = Vec::new();

    for name in ui_classes.keys() {
        if OUT_OF_SCOPE.contains(name) {
            continue;
        }
        // INPUTS ONLY. A standard describes what an implementation must ACCEPT,
        // not what it reports back: `AbsolutePosition` and `AbsoluteSize` are
        // read-only results of layout, and demanding them of an implementation
        // that has not laid out yet is incoherent. Deprecated properties are
        // excluded for the same reason nobody should implement against them.
        let mut props: Vec<&str> = Vec::new();
        let mut cursor = Some(*name);
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
        for property in &props {
            if dew_host::datamodel::accepts(name, property) {
                accepted_somewhere.insert(property);
            } else {
                refused_somewhere.insert(property);
            }
        }

        for p in &props {
            all_props.insert(p);
        }

        // THREE QUESTIONS PER CLASS, over the properties in scope for it, each
        // labelled at every print site so no two are read as one number:
        // whether the host stores it, whether Dew's renderer reads it on THIS
        // class, and whether Aether's pipeline names it.
        let mut row = ClassRow {
            name: name.to_string(),
            in_scope: 0,
            accepted: 0,
            implemented: Vec::new(),
            partial: Vec::new(),
            divergent: Vec::new(),
            absent: Vec::new(),
            pipeline: 0,
        };
        for property in props.iter().filter(|p| !dew_host::scope::excluded(p)) {
            row.in_scope += 1;
            if dew_host::datamodel::accepts(name, property) {
                row.accepted += 1;
            }
            if pipeline.contains(property) {
                row.pipeline += 1;
            }
            match honours(name, property) {
                Honour::Implemented => row.implemented.push(property.to_string()),
                Honour::Partial(reason) => row.partial.push((property.to_string(), reason)),
                Honour::Divergent(reason) => row.divergent.push((property.to_string(), reason)),
                Honour::Absent => row.absent.push(property.to_string()),
            }
        }
        per_class.push(row);
    }

    // EVERY CLASS IS PRINTED. This took the first twenty, which left the
    // padding, scale and constraint classes out of the document altogether.
    per_class.sort_by(|a, b| b.in_scope.cmp(&a.in_scope).then(a.name.cmp(&b.name)));
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
        let mut cursor = Some(*name);
        let mut found = false;
        while let Some(current) = cursor {
            let Some(class) = api.classes.get(current) else {
                break;
            };
            found = true;
            for (member, kind) in &class.members {
                all_members.insert(member, kind);
                reached_from.entry(member).or_insert(name);
            }
            cursor = class.superclass.as_deref();
        }
        // A class the reflection database has and the pinned dump does not means
        // the two describe different upstream builds, or that MODIFIERS drifted
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
    // THROUGH `classify`, so this is `implements`'s answer and not a second one.
    let implemented_input = all_members
        .keys()
        .filter(|m| INPUT_MEMBERS.contains(*m) && classify(m) == "implemented")
        .count();
    let member_in_scope = m_implemented + member_backlog.len();

    // Both halves are pinned to the same upstream build from api_surface.json.
    let build_skew: Option<(&str, &str)> = None;

    // THE DENOMINATOR IS THE POINT. Against every writable property the figure
    // is meaningless, because it counts things nobody intends to implement.
    // Against what is in scope it is a completion percentage someone can act on.
    let in_scope_total = covered_total + backlog.len();

    if markdown {
        emit_markdown(
            &api.version,
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
            implemented_input,
            build_skew,
        );
        return;
    }

    let in_scope = ui_classes.len()
        - OUT_OF_SCOPE
            .iter()
            .filter(|c| ui_classes.contains_key(*c))
            .count();

    println!("DataModel surface, from the engine {}\n", api.version);
    println!(
        "{in_scope} classes in scope of {} under GuiObject, {} writable properties
",
        ui_classes.len(),
        all_props.len()
    );

    println!(
        "{:<26} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "CLASS", "IN SCOPE", "ACCEPTED", "RENDERED", "PARTIAL", "AETHER"
    );
    for row in &per_class {
        println!(
            "{:<26} {:>8} {:>8} {:>8} {:>8} {:>8}",
            row.name,
            row.in_scope,
            row.accepted,
            row.rendered(),
            row.partial.len(),
            row.pipeline
        );
    }
    println!("  ACCEPTED: the host stores an assignment (datamodel::accepts)");
    println!(
        "  RENDERED: Dew's renderer reads it on this class (render::honours), PARTIAL and \
         divergent included"
    );
    println!("  AETHER:   named in Aether's pipeline (AETHER_PIPELINE), by name");
    for row in &per_class {
        for (property, reason) in &row.partial {
            println!("  partial  {}.{property}: {reason}", row.name);
        }
        for (property, reason) in &row.divergent {
            println!("  diverges {}.{property}: {reason}", row.name);
        }
    }

    println!();
    println!(
        "IN SCOPE:  {covered_total} of {in_scope_total} accepted by the Dew host ({:.0}%)",
        100.0 * covered_total as f64 / in_scope_total as f64
    );
    println!(
        "RENDERED:  {} of {} (class, property) pairs read by Dew's renderer, {} partially, \
         {} diverging from the engine on purpose",
        per_class.iter().map(ClassRow::rendered).sum::<usize>(),
        per_class.iter().map(|row| row.in_scope).sum::<usize>(),
        per_class.iter().map(|row| row.partial.len()).sum::<usize>(),
        per_class
            .iter()
            .map(|row| row.divergent.len())
            .sum::<usize>()
    );
    println!("PIPELINE:  {pipeline_covered} of those names are honoured by Aether's pipeline");
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
METHODS AND EVENTS, from the pinned dump at the engine {}
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
        println!("WARNING  this standard is measured against TWO upstream builds.");
        println!("         properties  {property_build}  (whatever rbx_reflection_database ships)");
        println!("         methods     {member_build}  (pinned by scripts/fetch_api_surface.luau)");
        println!("         Conformance against two builds at once is not a thing an");
        println!("         implementation can satisfy.");
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_markdown(
    version: &str,
    ui_classes: &BTreeMap<&str, &ApiClass>,
    per_class: &[ClassRow],
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
    implemented_input: usize,
    build_skew: Option<(&str, &str)>,
) {
    println!("# DataModel Standard: scope\n");
    println!("<!-- GENERATED. Regenerate with:");
    println!("       lune run scripts/fetch_api_surface.luau   # only to move the engine pin");
    print!("       cargo run --manifest-path host/Cargo.toml --bin datamodel-surface");
    println!(" -- --markdown > docs/datamodel_scope.md");
    println!("     Do not edit by hand; edit the classification lists in the tool.");
    println!(
        "     The bin is datamodel-surface, hyphenated. This line said datamodel_surface and
     the command it gave had never run. -->\n"
    );
    println!("Measured against the engine **{version}**, from the engine's API dump pinned");
    println!("at that build by `scripts/fetch_api_surface.luau`. Both the property half");
    println!("and the method/event half come from that one source, which is also the build");
    println!("every verified conformance case cites.\n");
    println!("**THE SUBJECT IS THE DEW HOST**, measured against the engine. Aether is a");
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
    let pairs: usize = per_class.iter().map(|row| row.in_scope).sum();
    let rendered: usize = per_class.iter().map(ClassRow::rendered).sum();
    let partial: usize = per_class.iter().map(|row| row.partial.len()).sum();
    let divergent: usize = per_class.iter().map(|row| row.divergent.len()).sum();
    println!("Counted per class rather than by name, those properties make {pairs} (class,");
    println!(
        "property) pairs. **{rendered} of the {pairs} are rendered by Dew**, {partial} of them"
    );
    println!("partially and {divergent} differently from the engine on purpose; see the two");
    println!("sections below.");
    println!();
    println!("Of the {in_scope} names, **{pipeline_covered} are honoured by Aether's pipeline**.");
    println!("That is not a conformance figure; it splits the backlog by cost.");
    println!();

    println!("## Coverage by class");
    println!();
    println!("Three different questions, asked of every property in scope for each class:");
    println!();
    println!("- **Accepted by the host**: an assignment to it is stored. Asked of");
    println!("  `dew_host::datamodel::accepts`.");
    println!("- **Rendered by Dew**: Dew's renderer reads it on an instance of THIS class,");
    println!("  so changing it changes what is drawn or where. Asked of");
    println!("  `dew_host::datamodel::render::honours`, which answers per class. Partial");
    println!("  answers are counted here and shown in their own column, with what is");
    println!("  missing listed below. A deliberate divergence from the engine is counted");
    println!("  here too, and listed below with what Dew does and what the engine does.");
    println!("  A property with no paint of its own counts as not rendered. A test holds");
    println!("  every claim to a gallery variant that moved pixels, a passing");
    println!("  engine-verified conformance case, or a stated excuse.");
    println!("- **Honoured by Aether's pipeline**: named by Aether's `Layout.Inputs` or its");
    println!("  display list, by NAME and not by class. It describes Aether, not Dew, and");
    println!("  splits the backlog by cost.");
    println!();
    println!(
        "| Class | In scope | Accepted by the host | Rendered by Dew | of which partial | Honoured by Aether's pipeline |"
    );
    println!("| :--- | ---: | ---: | ---: | ---: | ---: |");
    for row in per_class {
        println!(
            "| `{}` | {} | {} | {} | {} | {} |",
            row.name,
            row.in_scope,
            row.accepted,
            row.rendered(),
            row.partial.len(),
            row.pipeline
        );
    }
    println!();
    println!("## Rendered by Dew, by class");
    println!();
    println!("What `render::honours` answers for each in-scope property, so every count in");
    println!("the table above can be traced to the names behind it.");
    let ticked = |names: &[String]| {
        names
            .iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    for row in per_class {
        println!();
        println!("### `{}`", row.name);
        println!();
        if !row.implemented.is_empty() {
            println!("- **Rendered:** {}", ticked(&row.implemented));
        }
        for (property, reason) in &row.partial {
            println!("- **Partial,** `{property}`: {reason}");
        }
        for (property, reason) in &row.divergent {
            println!("- **Diverges from the engine,** `{property}`: {reason}");
        }
        if !row.absent.is_empty() {
            println!("- **Not rendered:** {}", ticked(&row.absent));
        }
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
    println!("The other half of what an application can reach, from the same pinned dump at");
    println!("**{api_version}**.");
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
    } else {
        // The sentence above went stale exactly as predicted, on the sprint that
        // built the signal type. Its replacement is printed from the same count
        // rather than written down, so it can go stale in its turn without
        // anybody having to notice.
        //
        // AND IT DID, ONE SPRINT LATER. It read "Every one is something an
        // instance says about ITSELF ... None of them is input: that needs hit
        // testing, which is a different problem" -- true while the only events
        // were `Changed` and the tree notices, and false the moment the hit test
        // landed. So this paragraph splits on a COUNT too rather than being
        // reworded: whether input is reachable is asked of `implements`, the same
        // predicate every number here comes from, and the prose cannot disagree
        // with the list above it.
        println!();
        println!(
            "**{implemented_events} of them are events**, reachable through \
             `RBXScriptSignal` and"
        );
        println!("`RBXScriptConnection`.");
        println!();
        if implemented_input == 0 {
            println!("**None of them is input.** Every one is something an instance says about");
            println!("ITSELF -- its properties, its children, its own destruction. Input needs a");
            println!("hit test to say which instance is at (x, y), which is a different problem");
            println!("with a different failure mode.");
        } else {
            println!(
                "**{implemented_input} of them are input**, which is what makes a mod written"
            );
            println!("without a framework CLICKABLE. The host resolves the tree's geometry once");
            println!("and both the painter and the hit test read that one answer, so what");
            println!("responds to a click is what is on screen. The rest are what an instance");
            println!("says about ITSELF -- its properties, its children, its own destruction.");
        }
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
        println!("### This document measures two upstream builds at once");
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
