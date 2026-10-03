//! Cascade resolution: given an instance, which `StyleRule`s reach it and
//! which one wins each contested property.
//!
//! WHAT REACHES AN INSTANCE. Every `StyleSheet` parented on the instance
//! itself or on an ancestor -- "applies down whatever subtree it is parented
//! under" means the subtree includes its own root. A `StyleLink` found the
//! same way redirects to whatever `StyleSheet` its own `StyleSheet` property
//! names, rather than contributing rules of its own; a `StyleLink` has none.
//! More than one `StyleSheet` can reach one instance this way -- a theme
//! near the root and a one-off override lower down are both legitimate at
//! once -- so every reachable rule is gathered before anything is resolved.
//!
//! HOW A CONTEST IS DECIDED. `StyleRule.Priority` first, higher wins, exactly
//! as the vision for this styling effort states it: an explicit tiebreak
//! rather than CSS's implicit specificity math. Equal `Priority` is not
//! specified by that document, so this file makes and states the same kind
//! of call sprint 1 and sprint 2 made for the cases their own specs left
//! open. Two rules tied on `Priority`:
//!
//! 1. The rule whose `StyleSheet` is CLOSER to the target instance wins --
//!    the same proximity CSS itself falls back on once specificity ties,
//!    read here as "how far up the tree from the instance being styled".
//! 2. Still tied (both rules belong to the same `StyleSheet`): the rule
//!    declared LATER among that sheet's own `StyleRule` children wins,
//!    which is source order, the oldest and least surprising tiebreak
//!    there is.
//!
//! WHERE PAINT MEETS THIS. [`Dom::styled_property`] is the render path's own
//! read: an instance's own explicit value first, this file's resolution
//! filling the gap, the engine default last. It is deliberately NOT what
//! `Index` reads -- a script reading a property back still sees only what it,
//! or nothing, set. Cascading into every property read a guest can make,
//! rather than only the ones a paint pass takes, is a bigger and separate
//! decision than wiring the resolved cascade into paint is, and this sprint
//! is scoped to the latter.
//!
//! WHAT [`resolve`] ITSELF DOES NOT DO. Its map is not filtered against the
//! target's own declared properties -- an instance receiving a name a paint
//! pass has no use for is that pass's decision, not this one's. And no
//! re-resolution scoping: this is one full resolve of one instance, called
//! however often a live-update mechanism needs it; making that call cheap
//! when only a handful of properties actually changed is that mechanism's
//! problem to solve, not a shortcut to bake in before it exists.
//!
//! `$TOKEN` VALUES (milestone 28 sprint 1). Roblox's own real cascade lets a
//! `StyleRule` property name an Attribute instead of a literal value, with a
//! `$` prefix -- confirmed against Roblox's own documentation (`ui/styling
//! /css-comparisons`), not assumed from the property system looking
//! CSS-shaped. `$FrameColor` on a rule whose own `StyleSheet` (its parent)
//! carries `FrameColor` as an Attribute resolves to that Attribute's value.
//! An unresolved token (no Attribute by that name anywhere in the sheet's
//! own derive chain) is left as the literal string rather than silently
//! dropped -- loud in the painted output, the same reason an invalid
//! selector keeps its own `SelectorError` rather than resolving to nothing
//! quietly.
//!
//! `StyleDerive` (milestone 28 sprint 2). Parented INSIDE a `StyleSheet`
//! (unlike `StyleLink`, which reaches OUT from an instance's own ancestry
//! toward a sheet elsewhere), a `StyleDerive` names a second `StyleSheet`
//! its own parent composes rules and tokens from -- Roblox's own real
//! theming primitive, confirmed against the same documentation. Composition
//! is TRANSPARENT to distance: a derived sheet's own rules and tokens are
//! treated as though they belonged to the deriving sheet itself, at that
//! sheet's own tree position, not as a separate, farther contribution.
//! MULTIPLE `StyleDerive`s under one sheet rank by `StyleDerive.Priority`,
//! the same explicit-tiebreak philosophy `StyleRule.Priority` already uses
//! here -- higher wins a token contest, applied last over weaker sources.
//! CYCLES ARE BROKEN, NOT ERRORED: a chain that would revisit a sheet
//! already being composed for the current resolution stops at that edge
//! rather than failing the whole resolve, the same "a fact about the rule,
//! not a thrown error" spirit `style::parse`'s own `SelectorError` already
//! has for a different kind of authoring mistake. Real Roblox's own
//! behavior for a derive cycle is not documented anywhere this project
//! checked; this is this file's own stated decision, not a mirror of a
//! confirmed spec.

use super::{style, transition, Dom};
use rbx_types::Variant;
use std::collections::{BTreeMap, HashSet};

/// One `StyleRule` that matched the target, with what it takes to rank it
/// against every other one that also matched.
struct Candidate {
    style_rule: usize,
    priority: f64,
    /// Steps from the target instance up to the ancestor whose child
    /// introduced this rule's `StyleSheet` -- 0 is the target's own
    /// children, 1 its parent's, and so on. Smaller is closer.
    distance: usize,
    /// This rule's child index within its own `StyleSheet`, for the
    /// same-sheet, same-`Priority` tiebreak.
    order: usize,
}

/// Every property value the cascade resolves for `id`, `StyleRule.Priority`
/// breaking every contest -- the map [`Dom::styled_property`] consults for
/// whatever `id`'s own explicit properties leave unset.
pub fn resolve(dom: &Dom, id: usize) -> BTreeMap<String, Variant> {
    resolve_full(dom, id)
        .into_iter()
        .map(|(name, (value, _rule))| (name, value))
        .collect()
}

// How many times this thread has resolved an instance's cascade, for a test
// to count what one render pass costs.
#[cfg(test)]
thread_local! {
    pub(crate) static RESOLVES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// [`resolve`], with the winning `StyleRule`'s own id kept beside each
/// value -- what [`transition::advance`](super::transition::advance) needs
/// to look up whether the rule that JUST won a property also declared a
/// transition for it (milestone 29 part C4). `resolve` itself is this
/// function with the rule id dropped, not a separate walk -- one cascade
/// pass answers both questions.
pub fn resolve_full(dom: &Dom, id: usize) -> BTreeMap<String, (Variant, usize)> {
    #[cfg(test)]
    RESOLVES.with(|count| count.set(count.get() + 1));
    let mut candidates = Vec::new();
    for (distance, sheet) in applicable_style_sheets(dom, id) {
        let mut order = 0usize;
        for rule in dom.children(sheet) {
            gather_rule_candidates(dom, id, distance, rule, None, &mut order, &mut candidates);
        }
    }

    // ASCENDING: the weakest candidate first, so writing each one's
    // properties into the result in order leaves the strongest last,
    // overwriting whatever a weaker rule set for the same name.
    candidates.sort_by(|a, b| {
        a.priority
            .partial_cmp(&b.priority)
            .expect("Priority is never NaN: coerce refuses a value fraction check already needs")
            // FARTHER FIRST: a smaller distance is closer, and closer must
            // win a `Priority` tie, so it has to be applied last.
            .then(b.distance.cmp(&a.distance))
            .then(a.order.cmp(&b.order))
    });

    let mut resolved = BTreeMap::new();
    for candidate in candidates {
        // A `StyleRule`'s own `Selector` and `Priority` never carry a `$`
        // token themselves -- `dom.parent_of` is the rule's own `StyleSheet`
        // by construction (a rule is always a `StyleSheet` child), which is
        // the only place sprint 1 looks for the Attribute a token names.
        let sheet = dom.parent_of(candidate.style_rule);
        let Some(props) = dom.style_properties.get(&candidate.style_rule) else {
            continue;
        };
        // `Font` WRITES `FontFace` TOO, as assigning it does, so the two are
        // one contest: whichever this loop writes last wins. That is the
        // stronger rule, or within one rule the one set last, so `Font` is
        // held back until after `FontFace` when it was set after it.
        let font_last = dom.font_set_last.contains(&candidate.style_rule);
        let held = font_last.then(|| props.get_key_value("Font")).flatten();
        let ordered = props
            .iter()
            .filter(|(name, _)| !(font_last && name.as_str() == "Font"))
            .chain(held);
        for (name, value) in ordered {
            let Some(value) = applicable(dom, id, name, resolve_token(dom, sheet, value.clone()))
            else {
                continue;
            };
            if name == "Font" {
                if let Some(face) = face_of_font(dom, id, &value) {
                    resolved.insert("FontFace".to_string(), (face, candidate.style_rule));
                }
            }
            resolved.insert(name.clone(), (value, candidate.style_rule));
        }
    }
    resolved
}

/// A rule's value as `id` would hold it, or nothing when `id` cannot take
/// it. Where `id`'s class declares `name` as an enum, only an item of that
/// enum applies, turned into the bare `Variant::Enum` an assignment stores;
/// a string, a number or another enum's item is skipped, as measured in
/// Studio. An enum item under a name declared as anything else is skipped
/// too. Every other value, and any value under a name `id` does not
/// declare, passes unchanged.
fn applicable(dom: &Dom, id: usize, name: &str, value: Variant) -> Option<Variant> {
    let class = dom.node(id).map(|node| node.class.as_str())?;
    match (super::describe(class, name).map(|d| &d.data_type), value) {
        (Some(rbx_reflection::DataType::Enum(ty)), Variant::EnumItem(item)) if *ty == item.ty => {
            Some(Variant::Enum(rbx_types::Enum::from_u32(item.value)))
        }
        (Some(rbx_reflection::DataType::Enum(_)), _) => None,
        (Some(_), Variant::EnumItem(_)) => None,
        (_, value) => Some(value),
    }
}

/// The `FontFace` a resolved `Font` stands for on `id`, when `id`'s class
/// has a `FontFace` for it to set.
fn face_of_font(dom: &Dom, id: usize, value: &Variant) -> Option<Variant> {
    let Variant::Enum(raw) = value else {
        return None;
    };
    let class = dom.node(id).map(|node| node.class.as_str())?;
    super::describe(class, "FontFace")?;
    let item = super::enums::item_by_value("Font", raw.to_u32())?;
    crate::fonts::from_enum(item.name).map(Variant::Font)
}

/// `value`, unless it is a `$Name` token string, in which case the value of
/// `Name` found on `sheet` or anywhere in the `StyleDerive` chain it
/// composes from -- or the literal string back, unresolved, if nothing in
/// that chain carries such an Attribute. See this module's own doc comment
/// for why an unresolved token stays loud rather than disappearing.
fn resolve_token(dom: &Dom, sheet: Option<usize>, value: Variant) -> Variant {
    let Variant::String(s) = &value else {
        return value;
    };
    let Some(token) = s.strip_prefix('$') else {
        return value;
    };
    let Some(sheet) = sheet else {
        return value;
    };
    lookup_token(dom, sheet, token, &mut HashSet::new()).unwrap_or(value)
}

/// `token`, read as an Attribute on `sheet` itself, or on the strongest
/// `StyleDerive` source that has it -- weakest source first, so a stronger
/// one's own answer overwrites a weaker one's, the same rule
/// [`resolve`]'s own candidate loop already applies to `StyleRule`
/// properties. `visited` is fresh per top-level lookup ([`resolve_token`]
/// starts it empty); a sheet already being composed for this same lookup is
/// skipped rather than revisited, breaking a cycle at the edge that would
/// close it rather than failing the whole resolve.
fn lookup_token(
    dom: &Dom,
    sheet: usize,
    token: &str,
    visited: &mut HashSet<usize>,
) -> Option<Variant> {
    if !visited.insert(sheet) {
        return None;
    }
    if let Some(v) = dom.get_attribute(sheet, token) {
        return Some(v);
    }
    let mut found = None;
    for source in derive_sources(dom, sheet) {
        if let Some(v) = lookup_token(dom, source, token, visited) {
            found = Some(v);
        }
    }
    found
}

/// Every `StyleSheet` a `StyleDerive` child of `sheet` names, weakest
/// `StyleDerive.Priority` first (ties by child order) -- the order a caller
/// should apply them in so a stronger source's own answer wins last, the
/// same convention [`resolve`]'s own `Candidate` sort already uses for
/// `StyleRule.Priority`. A `StyleDerive` naming a destroyed or non-sheet
/// target contributes nothing, the same tolerance [`style_link_target`]
/// already has for a dangling `StyleLink`.
fn derive_sources(dom: &Dom, sheet: usize) -> Vec<usize> {
    let mut derives: Vec<(f64, usize, usize)> = Vec::new();
    for (order, child) in dom.children(sheet).into_iter().enumerate() {
        if dom.class_of(child).as_deref() != Some("StyleDerive") {
            continue;
        }
        let Some(target) = style_derive_target(dom, child) else {
            continue;
        };
        derives.push((priority_of(dom, child), order, target));
    }
    derives.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("Priority is never NaN: coerce refuses a value fraction check already needs")
            .then(a.1.cmp(&b.1))
    });
    derives.into_iter().map(|(_, _, target)| target).collect()
}

fn style_derive_target(dom: &Dom, derive: usize) -> Option<usize> {
    let target = dom.get_style_derive(derive)?;
    (dom.class_of(target).as_deref() == Some("StyleSheet")).then_some(target)
}

/// `(distance, StyleSheet id)` for every `StyleSheet` that reaches `id`,
/// through direct parenting, through a `StyleLink`, or composed into either
/// one through a `StyleDerive` chain -- nearest first. A derived sheet's own
/// entry carries the SAME distance as whatever sheet composes from it: see
/// this module's own doc comment for why composition is transparent to tree
/// position rather than a farther, separate contribution.
fn applicable_style_sheets(dom: &Dom, id: usize) -> Vec<(usize, usize)> {
    let mut sheets = Vec::new();
    let mut cursor = Some(id);
    let mut distance = 0;
    while let Some(current) = cursor {
        for child in dom.children(current) {
            match dom.class_of(child).as_deref() {
                Some("StyleSheet") => {
                    sheets.push((distance, child));
                    collect_derived_sheets(dom, child, distance, &mut sheets, &mut HashSet::new());
                }
                // A `StyleLink` NAMES A SHEET, IT IS NOT ONE. Its own
                // children (it has none that matter here) are never walked
                // for `StyleRule`s; only the sheet it points at is.
                Some("StyleLink") => {
                    if let Some(target) = style_link_target(dom, child) {
                        sheets.push((distance, target));
                        collect_derived_sheets(
                            dom,
                            target,
                            distance,
                            &mut sheets,
                            &mut HashSet::new(),
                        );
                    }
                }
                _ => {}
            }
        }
        cursor = dom.parent_of(current);
        distance += 1;
    }
    sheets
}

/// Every `StyleSheet` `sheet`'s own `StyleDerive` children compose from,
/// pushed at `distance` alongside `sheet` itself, then recursed into --
/// a derive chain can be more than one link long. `visited` guards one
/// call's own chain against a cycle; see this module's own doc comment.
fn collect_derived_sheets(
    dom: &Dom,
    sheet: usize,
    distance: usize,
    out: &mut Vec<(usize, usize)>,
    visited: &mut HashSet<usize>,
) {
    if !visited.insert(sheet) {
        return;
    }
    for source in derive_sources(dom, sheet) {
        out.push((distance, source));
        collect_derived_sheets(dom, source, distance, out, visited);
    }
}

fn style_link_target(dom: &Dom, link: usize) -> Option<usize> {
    let target = dom.get_style_link(link)?;
    (dom.class_of(target).as_deref() == Some("StyleSheet")).then_some(target)
}

/// Every directive `rule`'s own `Selector` string carries, `parse_full`'s
/// own three-part shape read off this rule: the match selector itself, an
/// optional `@Name` query gate (part C3), and an optional `::Modifier`
/// auto-spawn class (part C2). `None` for a rule whose selector does not
/// parse at all, the same as `None` for one that simply does not match: an
/// invalid selector already carries its own `SelectorError`, and repeating
/// that complaint here for every instance it is tested against would be
/// noise beyond what one property read already says.
fn directives_of(
    dom: &Dom,
    rule: usize,
) -> Option<(style::Selector, Option<String>, Option<String>)> {
    let Variant::String(selector) = dom.property(rule, "Selector")? else {
        return None;
    };
    style::parse_full(&selector).ok()
}

/// `directives_of`, with the query gate already applied and the modifier
/// folded into the selector rather than dropped -- `None` for a rule an
/// `@Name` query has shut off right now, the same as `None` for one whose
/// selector does not parse at all.
///
/// A `::Modifier` RULE'S OWN PROPERTIES STYLE THE SPAWNED CHILD, NOT THE
/// INSTANCE THE BASE SELECTOR NAMES -- confirmed by what the feature would
/// be without this: `apply_modifiers` already guarantees a `UICorner`
/// exists under every matching `Frame`, but an UNSTYLED one has
/// `CornerRadius = 0`, indistinguishable from square corners. Real CSS's
/// own `::before`/`::after` style the pseudo-element they introduce, not
/// the element hosting it, and `"Frame.RoundedCorner20::UICorner"`'s own
/// real behavior is the same shape: the rule's properties belong to the
/// `UICorner`. `"BASE::Class"` becomes `Selector::Child(BASE,
/// ClassName(Class))` here -- exactly what `apply_modifiers` already
/// ensures exists, so the cascade reaches it the same way any ordinary `>`
/// rule would reach a real child.
fn selector_of(dom: &Dom, rule: usize) -> Option<style::Selector> {
    let (selector, query, modifier) = directives_of(dom, rule)?;
    if let Some(name) = query {
        let sheet = dom.parent_of(rule)?;
        if !query_holds(dom, sheet, &name) {
            return None;
        }
    }
    match modifier {
        Some(class) => Some(style::Selector::Child(
            Box::new(selector),
            Box::new(style::Selector::ClassName(class)),
        )),
        None => Some(selector),
    }
}

/// A NESTED `StyleRule`'s own effective selector, `parent`'s already-
/// resolved selector merged in through [`style::parse_nested`]. Milestone
/// 29 part C5. Query/modifier parsing is NOT extended to nested rules this
/// sprint -- the one confirmed real example nests a bare combinator only,
/// and a nested rule's own `Selector` goes through `parse_nested` rather
/// than `directives_of`.
fn nested_selector_of(dom: &Dom, rule: usize, parent: &style::Selector) -> Option<style::Selector> {
    let Variant::String(raw) = dom.property(rule, "Selector")? else {
        return None;
    };
    style::parse_nested(&raw, parent).ok()
}

/// Adds `rule` as a candidate for `id` if its own (possibly nested-merged)
/// selector matches, then recurses into `rule`'s own `StyleRule` children
/// with ITS effective selector as their parent -- `"> TextButton"` nested
/// inside `"#MenuFrame"` resolves as `"#MenuFrame > TextButton"` (milestone
/// 29 part C5), any depth deep, not just one level.
///
/// RECURSES WHETHER `rule` ITSELF MATCHED `id` OR NOT: the merge is
/// textual, not conditional on the parent rule ALSO having a live match
/// somewhere -- `#MenuFrame > TextButton` matches a `TextButton` under
/// `#MenuFrame` on its own terms, the same as writing that one string
/// directly would.
///
/// `order` IS SHARED ACROSS THE WHOLE RECURSIVE WALK, not reset per
/// nesting level -- source order across a sheet's own top-level AND nested
/// rules together, which is what the existing same-`Priority` tiebreak
/// (`resolve`'s own `Candidate` sort) already expects a monotonic counter
/// to mean.
fn gather_rule_candidates(
    dom: &Dom,
    id: usize,
    distance: usize,
    rule: usize,
    parent_selector: Option<&style::Selector>,
    order: &mut usize,
    candidates: &mut Vec<Candidate>,
) {
    if dom.class_of(rule).as_deref() != Some("StyleRule") {
        return;
    }
    let this_order = *order;
    *order += 1;

    let selector = match parent_selector {
        None => selector_of(dom, rule),
        Some(parent) => nested_selector_of(dom, rule, parent),
    };

    if let Some(selector) = &selector {
        if style::matches(dom, id, selector) {
            candidates.push(Candidate {
                style_rule: rule,
                priority: priority_of(dom, rule),
                distance,
                order: this_order,
            });
        }
    }

    if let Some(effective) = &selector {
        for child in dom.children(rule) {
            gather_rule_candidates(dom, id, distance, child, Some(effective), order, candidates);
        }
    }
}

/// Walks every instance at or under `root` and, for every `StyleRule` that
/// matches it and carries a `::Modifier` suffix, ensures a child of the
/// named class exists -- creating one if absent. Milestone 29 part C2.
///
/// IDEMPOTENT, ON PURPOSE: a modifier whose child already exists does
/// nothing, which is what lets a caller run this every dirty frame (the
/// same frequency [`resolve`] already runs at, through `styled_property`)
/// rather than tracking which frame first satisfied which modifier.
///
/// NO SIGNAL IS FIRED for the child it creates -- unlike a guest's own
/// `Parent` assignment (`mod.rs`'s own `NewIndex` handler), which announces
/// `ChildAdded`/`DescendantAdded` to anything listening. Wiring that in is
/// a real gap if a script ever needs to react to an auto-spawned modifier
/// child specifically, not something this sprint's own examples needed.
pub fn apply_modifiers(dom: &mut Dom, root: usize) {
    // EVERY `::Modifier` RULE IN THE ARENA, parsed once. The walk below can
    // only act on one of these, so an arena without any (most of them) skips
    // the walk, and one with some does not parse every rule per instance.
    let modifier_rules: std::collections::HashMap<
        usize,
        (style::Selector, Option<String>, String),
    > = dom
        .slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.as_ref().is_some_and(|node| node.class == "StyleRule"))
        .filter_map(|(rule, _)| match directives_of(dom, rule)? {
            (selector, query, Some(modifier)) => Some((rule, (selector, query, modifier))),
            _ => None,
        })
        .collect();
    if modifier_rules.is_empty() {
        return;
    }
    let mut to_create: Vec<(usize, String)> = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        for (_, sheet) in applicable_style_sheets(dom, id) {
            for rule in dom.children(sheet) {
                let Some((selector, query, modifier_class)) = modifier_rules.get(&rule) else {
                    continue;
                };
                let modifier_class = modifier_class.clone();
                if let Some(name) = query {
                    if !query_holds(dom, sheet, name) {
                        continue;
                    }
                }
                if !style::matches(dom, id, selector) {
                    continue;
                }
                let already = dom
                    .children(id)
                    .into_iter()
                    .any(|c| dom.class_of(c).as_deref() == Some(modifier_class.as_str()));
                if !already
                    && !to_create
                        .iter()
                        .any(|(p, c)| *p == id && c == &modifier_class)
                {
                    to_create.push((id, modifier_class));
                }
            }
        }
        stack.extend(dom.children(id));
    }
    for (parent, class) in to_create {
        let child = dom.insert(class.clone(), class);
        dom.adopt(parent, child);
    }
}

// PART C3: `StyleQuery`, real conditions gating whether a rule reaches
// anything at all. `StyleQuery` (properties `MinSize`, `MaxSize`,
// `AspectRatioRange`, `ViewportDisplaySize`, `PreferredInput`,
// `PreferredTextSize`, `ReducedMotionEnabled`, and a read-only `IsActive`)
// is a real Roblox class, confirmed directly against
// `rbx_reflection_database` rather than assumed from the milestone plan's
// own informal description -- `class_exists("StyleQuery")` is already
// `true` with no extension registration needed, the same as `StyleDerive`
// needed none once milestone 28 sprint 2 went looking.

/// `DisplaySize.Small/Medium/Large`'s own confirmed numeric values (0/1/2),
/// read directly off `rbx_reflection_database`'s own `DisplaySize` enum --
/// but the pixel WIDTH at which one bucket ends and the next begins is NOT
/// documented anywhere this project checked, only that the buckets exist.
/// These two cutoffs are Dew's own stated, reasonable call, the same kind
/// this file's own module doc already makes for an undocumented `Priority`
/// tie -- replace them if a real source ever turns up.
const VIEWPORT_SMALL_MAX: u32 = 600;
const VIEWPORT_MEDIUM_MAX: u32 = 1200;

fn viewport_display_size(dom: &Dom) -> u32 {
    let (width, _) = dom.viewport();
    if width < VIEWPORT_SMALL_MAX {
        0 // DisplaySize.Small
    } else if width < VIEWPORT_MEDIUM_MAX {
        1 // DisplaySize.Medium
    } else {
        2 // DisplaySize.Large
    }
}

/// `PreferredInput.KeyboardAndMouse`'s own confirmed value (0). Dew is a
/// desktop host and delivers no other input modality (`input.rs`'s own
/// event types have no touch or gamepad case) -- a real, stated fact about
/// THIS host, not a guess standing in for a signal that does not exist.
fn preferred_input(_dom: &Dom) -> u32 {
    0 // PreferredInput.KeyboardAndMouse
}

/// NOT YET WIRED TO ANY REAL HOST SIGNAL: Dew reads no OS-level "reduced
/// motion" preference anywhere today. Always `false` rather than a coin
/// flip -- reduced-motion styling should be opt-in, not opt-out, until a
/// real signal exists to opt it in correctly.
fn reduced_motion_enabled(_dom: &Dom) -> bool {
    false
}

/// Does a BUILT-IN or CUSTOM query named `name` hold right now? `sheet` is
/// where a custom `::StyleQuery #Name` declaration is searched for, if
/// `name` is not one of the built-in names below -- the same `StyleSheet`
/// a rule referencing `@Name` already belongs to, exactly as `$TOKEN`
/// resolution already searches a rule's own sheet rather than the whole
/// tree.
fn query_holds(dom: &Dom, sheet: usize, name: &str) -> bool {
    match name {
        "ViewportDisplaySizeSmall" => viewport_display_size(dom) == 0,
        "ViewportDisplaySizeMedium" => viewport_display_size(dom) == 1,
        "ViewportDisplaySizeLarge" => viewport_display_size(dom) == 2,
        "PreferredInputKeyboardAndMouse" => preferred_input(dom) == 0,
        "PreferredInputTouch" => preferred_input(dom) == 2,
        "ReducedMotionEnabledTrue" => reduced_motion_enabled(dom),
        "ReducedMotionEnabledFalse" => !reduced_motion_enabled(dom),
        _ => custom_query_holds(dom, sheet, name),
    }
}

/// A custom `::StyleQuery #Name` declaration: a real `StyleQuery` instance,
/// its own `Name` matching, searched among `sheet`'s own children the same
/// way a `StyleDerive` is. Unknown (no such instance found) fails closed,
/// the same "absent means no" reading an unresolved `$TOKEN` or an unset
/// `GuiState` both already get in this file.
fn custom_query_holds(dom: &Dom, sheet: usize, name: &str) -> bool {
    for child in dom.children(sheet) {
        if dom.class_of(child).as_deref() != Some("StyleQuery") {
            continue;
        }
        if dom.name_of(child).as_deref() == Some(name) {
            return style_query_holds(dom, child);
        }
    }
    false
}

/// Do ALL of `query`'s own SET condition properties hold right now? A
/// condition the instance never set is not evaluated at all -- the same
/// "absent means unconstrained" reading CSS's own media-feature list gives
/// a feature it omits, so a `StyleQuery` that only sets `MinSize` is a
/// pure minimum-size gate, not one silently also demanding some default
/// `ViewportDisplaySize`.
///
/// `AspectRatioRange` AND `PreferredTextSize` ARE RECOGNIZED BUT NOT
/// EVALUATED -- this sprint's own confirmed scope stops at the conditions
/// the milestone plan actually named (`MinSize`, plus the built-in
/// equivalents of `ViewportDisplaySize`/`PreferredInput`/
/// `ReducedMotionEnabled`). Reporting a query active when one of its own
/// conditions was never actually checked would be a worse lie than
/// reporting it inactive, so a `StyleQuery` that sets either property
/// never activates, loudly rather than silently ignoring the property it
/// cannot yet honor.
fn style_query_holds(dom: &Dom, query: usize) -> bool {
    let Some(node) = dom.node(query) else {
        return false;
    };
    let (width, height) = dom.viewport();
    let (width, height) = (width as f32, height as f32);

    if let Some(Variant::Vector2(min)) = node.props.get("MinSize") {
        if width < min.x || height < min.y {
            return false;
        }
    }
    if let Some(Variant::Vector2(max)) = node.props.get("MaxSize") {
        if width > max.x || height > max.y {
            return false;
        }
    }
    if let Some(Variant::Enum(display_size)) = node.props.get("ViewportDisplaySize") {
        if display_size.to_u32() != viewport_display_size(dom) {
            return false;
        }
    }
    if let Some(Variant::Enum(input)) = node.props.get("PreferredInput") {
        if input.to_u32() != preferred_input(dom) {
            return false;
        }
    }
    if let Some(Variant::Bool(wants)) = node.props.get("ReducedMotionEnabled") {
        if *wants != reduced_motion_enabled(dom) {
            return false;
        }
    }
    if node.props.contains_key("AspectRatioRange") || node.props.contains_key("PreferredTextSize") {
        return false;
    }

    true
}

/// Writes every `StyleQuery` instance's own `IsActive` up to date, for a
/// guest script to read back (`StyleQuery.IsActive` is real and read-only
/// on the engine, confirmed via `rbx_reflection_database`). Through
/// [`Dom::set_internal`], the same path `AbsolutePosition` already uses for
/// a computed, guest-read-only property -- this does NOT mark the tree
/// dirty on its own (an unchanged answer costs nothing, and a changed one
/// already means SOMETHING else made it dirty first, the same reasoning
/// `AbsolutePosition`'s own writes already state).
pub fn refresh_style_queries(dom: &mut Dom, root: usize) {
    let mut stack = vec![root];
    let mut updates = Vec::new();
    while let Some(id) = stack.pop() {
        if dom.class_of(id).as_deref() == Some("StyleQuery") {
            updates.push((id, style_query_holds(dom, id)));
        }
        stack.extend(dom.children(id));
    }
    for (id, active) in updates {
        if dom.property(id, "IsActive") != Some(Variant::Bool(active)) {
            dom.set_internal(id, "IsActive", Variant::Bool(active));
        }
    }
}

fn priority_of(dom: &Dom, rule: usize) -> f64 {
    match dom.property(rule, "Priority") {
        Some(Variant::Float32(v)) => v as f64,
        Some(Variant::Float64(v)) => v,
        Some(Variant::Int32(v)) => v as f64,
        Some(Variant::Int64(v)) => v as f64,
        _ => 0.0,
    }
}

/// [`resolve`]'s answer for each instance, kept for the length of one render
/// pass.
///
/// A PASS ASKS THE SAME QUESTION MANY TIMES. Layout reads a dozen properties
/// of every element, measures an AutomaticSize child before placing it, and
/// solves the tree more than once, and each read an element did not set
/// itself used to resolve the whole cascade again against every rule in
/// reach. Open, the memo answers each instance's cascade once.
///
/// CLOSED OUTSIDE A PASS, AND EMPTIED BY ANY WRITE. [`Dom::open_style_memo`]
/// opens it and the same caller closes it; nothing resolved in one pass is
/// seen by the next. Inside a pass, every write that could change a match,
/// a winner or a token (a node through `node_mut`, an insert or destroy, a
/// tag, a rule's property table, a link, a derive, the viewport) calls
/// [`StyleMemo::forget`], so a write made while it is open is never answered
/// from before it.
///
/// A `RefCell` because a pass reads the tree through `&Dom`. The tree is
/// only ever reached through its `Mutex`, so one pass at a time touches it.
#[derive(Default)]
pub struct StyleMemo(
    std::cell::RefCell<Option<std::collections::HashMap<usize, BTreeMap<String, Variant>>>>,
);

impl StyleMemo {
    /// Drop everything resolved so far, leaving the memo open if it was.
    pub fn forget(&mut self) {
        if let Some(resolved) = self.0.get_mut() {
            resolved.clear();
        }
    }
}

impl Dom {
    /// Starts memoising the cascade for one render pass. Answers whether
    /// this call opened it, so a pass nested in another (a hit test inside
    /// a frame) leaves closing to the outer one: hand the answer back to
    /// [`Dom::close_style_memo`].
    pub fn open_style_memo(&self) -> bool {
        let mut memo = self.style_memo.0.borrow_mut();
        if memo.is_some() {
            return false;
        }
        *memo = Some(std::collections::HashMap::new());
        true
    }

    /// Ends the pass [`Dom::open_style_memo`] started, if `opened` says
    /// that call was the one that started it.
    pub fn close_style_memo(&self, opened: bool) {
        if opened {
            *self.style_memo.0.borrow_mut() = None;
        }
    }

    /// What the cascade resolves `key` to on `id`, from the memo when one is
    /// open, resolving `id` into it on a miss.
    fn cascaded(&self, id: usize, key: &str) -> Option<Variant> {
        {
            let memo = self.style_memo.0.borrow();
            match memo.as_ref() {
                None => return resolve(self, id).remove(key),
                Some(resolved) => {
                    if let Some(map) = resolved.get(&id) {
                        return map.get(key).cloned();
                    }
                }
            }
        }
        let map = resolve(self, id);
        let value = map.get(key).cloned();
        if let Some(resolved) = self.style_memo.0.borrow_mut().as_mut() {
            resolved.insert(id, map);
        }
        value
    }

    /// `property`, with a matching `StyleRule` filling the gap between `id`'s
    /// own explicit value and its engine default. The render path's own
    /// read -- see this module's own doc comment for why `Index` does not
    /// call this instead.
    pub fn styled_property(&self, id: usize, key: &str) -> Option<Variant> {
        if let Some(explicit) = self.node(id)?.props.get(key).cloned() {
            return Some(explicit);
        }
        // AN ACTIVE TRANSITION (part C4) TAKES PRIORITY OVER A FRESH
        // RESOLVE -- it answers the same question `resolve` would (what is
        // this cascade-driven property right now), just mid-animation
        // rather than already at its target.
        if let Some(animating) = transition::current(self, id, key, self.now()) {
            return Some(animating);
        }
        if let Some(styled) = self.cascaded(id, key) {
            return Some(styled);
        }
        self.property(id, key)
    }

    /// Every property `id` carries by assignment or by a matching `StyleRule`,
    /// with an assignment winning over a rule as it does in
    /// [`Dom::styled_property`]. What nothing set is absent.
    pub fn set_properties(&self, id: usize) -> BTreeMap<String, Variant> {
        let mut out = resolve(self, id);
        if let Some(node) = self.node(id) {
            for (key, value) in &node.props {
                out.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// Wires [`apply_modifiers`] onto `Dom` itself, the same way
    /// `styled_property` sits beside `resolve` above -- `main.rs`'s own
    /// render loop lives outside `datamodel`'s own private module tree and
    /// cannot see the free function directly.
    pub fn apply_modifiers(&mut self, root: usize) {
        apply_modifiers(self, root);
    }

    /// Wires [`refresh_style_queries`] onto `Dom` itself, for the same
    /// reason `apply_modifiers` needed its own wrapper just above.
    pub fn refresh_style_queries(&mut self, root: usize) {
        refresh_style_queries(self, root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamodel::SharedDom;

    fn parent(dom: &SharedDom, id: usize, on: usize) {
        let mut guard = dom.lock().expect("dom");
        guard.node_mut(id).expect("id").parent = Some(on);
        guard.node_mut(on).expect("on").children.push(id);
    }

    fn rule(
        dom: &SharedDom,
        sheet: usize,
        selector: &str,
        priority: f64,
        props: &[(&str, Variant)],
    ) -> usize {
        let mut guard = dom.lock().expect("dom");
        let rule = guard.insert("StyleRule".to_string(), "StyleRule".to_string());
        guard.node_mut(rule).expect("rule").props.insert(
            "Selector".to_string(),
            Variant::String(selector.to_string()),
        );
        guard
            .node_mut(rule)
            .expect("rule")
            .props
            .insert("Priority".to_string(), Variant::Float64(priority));
        for (name, value) in props {
            guard.set_style_property(rule, name, Some(value.clone()));
        }
        drop(guard);
        parent(dom, rule, sheet);
        rule
    }

    /// THE FIRST HALF OF THIS SPRINT'S OWN COMPLETION TEST: two rules that
    /// both match, disagreeing on one property, resolved by `Priority` alone.
    #[test]
    fn a_higher_priority_rule_wins_a_conflict() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Frame".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        rule(
            &dom,
            sheet,
            "Frame",
            1.0,
            &[("BackgroundTransparency", Variant::Float64(0.0))],
        );
        rule(
            &dom,
            sheet,
            "Frame",
            5.0,
            &[("BackgroundTransparency", Variant::Float64(0.75))],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundTransparency"),
            Some(&Variant::Float64(0.75))
        );
    }

    /// THE SECOND HALF: a `StyleLink` elsewhere in the tree reaches the same
    /// target the same way an inline `StyleSheet` would, without a second
    /// copy of the rule.
    #[test]
    fn a_style_link_resolves_the_same_as_an_inline_style_sheet() {
        let dom = SharedDom::default();
        let (root, target, theme_root, sheet, link) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            // The real StyleSheet lives entirely outside the target's own
            // ancestry, reached only through the link below.
            let theme_root = guard.insert("Folder".to_string(), "Theme".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            let link = guard.insert("StyleLink".to_string(), "StyleLink".to_string());
            (root, target, theme_root, sheet, link)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, theme_root);
        parent(&dom, link, root);
        rule(
            &dom,
            sheet,
            "Frame",
            1.0,
            &[("BackgroundTransparency", Variant::Float64(0.5))],
        );
        dom.lock().expect("dom").set_style_link(link, Some(sheet));

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundTransparency"),
            Some(&Variant::Float64(0.5))
        );
    }

    #[test]
    fn a_closer_style_sheet_wins_a_priority_tie() {
        let dom = SharedDom::default();
        let (root, mid, target, far_sheet, near_sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let mid = guard.insert("Frame".to_string(), "Mid".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let far_sheet = guard.insert("StyleSheet".to_string(), "Far".to_string());
            let near_sheet = guard.insert("StyleSheet".to_string(), "Near".to_string());
            (root, mid, target, far_sheet, near_sheet)
        };
        parent(&dom, mid, root);
        parent(&dom, target, mid);
        parent(&dom, far_sheet, root);
        parent(&dom, near_sheet, mid);
        rule(
            &dom,
            far_sheet,
            "Frame",
            1.0,
            &[("BackgroundTransparency", Variant::Float64(0.1))],
        );
        rule(
            &dom,
            near_sheet,
            "Frame",
            1.0,
            &[("BackgroundTransparency", Variant::Float64(0.9))],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundTransparency"),
            Some(&Variant::Float64(0.9))
        );
    }

    #[test]
    fn a_non_matching_rule_does_not_contribute() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        rule(
            &dom,
            sheet,
            "TextLabel",
            10.0,
            &[("BackgroundTransparency", Variant::Float64(0.9))],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert!(!resolved.contains_key("BackgroundTransparency"));
    }

    /// THE FIRST HALF OF SPRINT 1'S OWN COMPLETION TEST: a `$Name` value
    /// resolves against its own rule's `StyleSheet`, the same shape
    /// Roblox's own `ui/styling/css-comparisons` doc states directly.
    #[test]
    fn a_dollar_token_resolves_against_its_own_sheet() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        dom.lock().expect("dom").set_attribute(
            sheet,
            "FrameColor",
            Some(Variant::Color3(rbx_types::Color3::new(1.0, 0.0, 0.0))),
        );
        rule(
            &dom,
            sheet,
            "Frame",
            1.0,
            &[(
                "BackgroundColor3",
                Variant::String("$FrameColor".to_string()),
            )],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundColor3"),
            Some(&Variant::Color3(rbx_types::Color3::new(1.0, 0.0, 0.0)))
        );
    }

    /// AN UNRESOLVED TOKEN STAYS LOUD, per this module's own stated decision
    /// -- the literal string survives rather than silently vanishing.
    #[test]
    fn an_unresolved_token_stays_as_the_literal_string() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        // NO Attribute named "Missing" is ever set on `sheet`.
        rule(
            &dom,
            sheet,
            "Frame",
            1.0,
            &[("BackgroundColor3", Variant::String("$Missing".to_string()))],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundColor3"),
            Some(&Variant::String("$Missing".to_string()))
        );
    }

    /// A STRING VALUE WITH NO `$` PREFIX IS ORDINARY DATA, not a token this
    /// file has any business intercepting -- `Text = "Hello"` must reach
    /// paint unchanged, and must not be looked up as an Attribute named
    /// "Hello" either.
    #[test]
    fn a_plain_string_without_a_dollar_prefix_is_untouched() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("TextLabel".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        // An Attribute happens to share the literal value's own text, so a
        // test that only checked "is it unresolved" could pass by accident
        // if the `$` check were missing entirely.
        dom.lock().expect("dom").set_attribute(
            sheet,
            "Hello",
            Some(Variant::String("wrong".to_string())),
        );
        rule(
            &dom,
            sheet,
            "TextLabel",
            1.0,
            &[("Text", Variant::String("Hello".to_string()))],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("Text"),
            Some(&Variant::String("Hello".to_string()))
        );
    }

    fn derive(dom: &SharedDom, on_sheet: usize, target_sheet: usize, priority: f64) -> usize {
        let mut guard = dom.lock().expect("dom");
        let d = guard.insert("StyleDerive".to_string(), "StyleDerive".to_string());
        guard
            .node_mut(d)
            .expect("derive")
            .props
            .insert("Priority".to_string(), Variant::Float64(priority));
        drop(guard);
        parent(dom, d, on_sheet);
        dom.lock()
            .expect("dom")
            .set_style_derive(d, Some(target_sheet));
        d
    }

    /// THE FIRST HALF OF SPRINT 2'S OWN COMPLETION TEST: a token declared
    /// only on a `StyleSheet` a `StyleDerive` composes from resolves through
    /// the deriving sheet, the same way real Roblox's own theming is
    /// documented to work.
    #[test]
    fn a_token_resolves_through_a_style_derive() {
        let dom = SharedDom::default();
        let (root, target, design, tokens) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let design = guard.insert("StyleSheet".to_string(), "Design".to_string());
            let tokens = guard.insert("StyleSheet".to_string(), "Tokens".to_string());
            (root, target, design, tokens)
        };
        parent(&dom, target, root);
        parent(&dom, design, root);
        dom.lock().expect("dom").set_attribute(
            tokens,
            "FrameColor",
            Some(Variant::Color3(rbx_types::Color3::new(0.0, 1.0, 0.0))),
        );
        derive(&dom, design, tokens, 1.0);
        rule(
            &dom,
            design,
            "Frame",
            1.0,
            &[(
                "BackgroundColor3",
                Variant::String("$FrameColor".to_string()),
            )],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundColor3"),
            Some(&Variant::Color3(rbx_types::Color3::new(0.0, 1.0, 0.0)))
        );
    }

    /// THE SECOND HALF: re-pointing the `StyleDerive` at a DIFFERENT source
    /// sheet changes the resolved token in one write -- the real engine's
    /// own equivalent of a `data-theme` swap.
    #[test]
    fn re_pointing_a_style_derive_changes_the_resolved_token() {
        let dom = SharedDom::default();
        let (root, target, design, theme_a, theme_b) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let design = guard.insert("StyleSheet".to_string(), "Design".to_string());
            let theme_a = guard.insert("StyleSheet".to_string(), "ThemeA".to_string());
            let theme_b = guard.insert("StyleSheet".to_string(), "ThemeB".to_string());
            (root, target, design, theme_a, theme_b)
        };
        parent(&dom, target, root);
        parent(&dom, design, root);
        dom.lock().expect("dom").set_attribute(
            theme_a,
            "FrameColor",
            Some(Variant::Color3(rbx_types::Color3::new(1.0, 0.0, 0.0))),
        );
        dom.lock().expect("dom").set_attribute(
            theme_b,
            "FrameColor",
            Some(Variant::Color3(rbx_types::Color3::new(0.0, 0.0, 1.0))),
        );
        let d = derive(&dom, design, theme_a, 1.0);
        rule(
            &dom,
            design,
            "Frame",
            1.0,
            &[(
                "BackgroundColor3",
                Variant::String("$FrameColor".to_string()),
            )],
        );

        {
            let guard = dom.lock().expect("dom");
            let resolved = resolve(&guard, target);
            assert_eq!(
                resolved.get("BackgroundColor3"),
                Some(&Variant::Color3(rbx_types::Color3::new(1.0, 0.0, 0.0))),
                "should start on ThemeA"
            );
        }

        dom.lock().expect("dom").set_style_derive(d, Some(theme_b));

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundColor3"),
            Some(&Variant::Color3(rbx_types::Color3::new(0.0, 0.0, 1.0))),
            "one StyleDerive.StyleSheet write should re-resolve to ThemeB"
        );
    }

    /// A `StyleRule` on a DERIVED sheet applies to the deriving sheet's own
    /// reach, competing directly on its own `Priority` alongside the
    /// deriving sheet's own rules -- this file's own stated decision for
    /// how `StyleDerive` composition and `StyleRule.Priority` interact.
    #[test]
    fn a_rule_on_a_derived_sheet_applies_through_the_deriving_sheet() {
        let dom = SharedDom::default();
        let (root, target, design, base) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let design = guard.insert("StyleSheet".to_string(), "Design".to_string());
            let base = guard.insert("StyleSheet".to_string(), "Base".to_string());
            (root, target, design, base)
        };
        parent(&dom, target, root);
        parent(&dom, design, root);
        derive(&dom, design, base, 1.0);
        rule(
            &dom,
            base,
            "Frame",
            1.0,
            &[("BackgroundTransparency", Variant::Float64(0.25))],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundTransparency"),
            Some(&Variant::Float64(0.25))
        );
    }

    /// HIGHER `StyleDerive.Priority` WINS a token contest between two
    /// sources composed into the same sheet, applied last over the weaker
    /// one -- the same convention `StyleRule.Priority` already has here.
    #[test]
    fn a_higher_priority_style_derive_wins_a_token_contest() {
        let dom = SharedDom::default();
        let (root, target, design, weak, strong) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let design = guard.insert("StyleSheet".to_string(), "Design".to_string());
            let weak = guard.insert("StyleSheet".to_string(), "Weak".to_string());
            let strong = guard.insert("StyleSheet".to_string(), "Strong".to_string());
            (root, target, design, weak, strong)
        };
        parent(&dom, target, root);
        parent(&dom, design, root);
        dom.lock().expect("dom").set_attribute(
            weak,
            "FrameColor",
            Some(Variant::Color3(rbx_types::Color3::new(1.0, 0.0, 0.0))),
        );
        dom.lock().expect("dom").set_attribute(
            strong,
            "FrameColor",
            Some(Variant::Color3(rbx_types::Color3::new(0.0, 1.0, 0.0))),
        );
        // DECLARED WEAKEST FIRST, on purpose: this proves Priority decides
        // the winner, not insertion order.
        derive(&dom, design, weak, 1.0);
        derive(&dom, design, strong, 5.0);
        rule(
            &dom,
            design,
            "Frame",
            1.0,
            &[(
                "BackgroundColor3",
                Variant::String("$FrameColor".to_string()),
            )],
        );

        let guard = dom.lock().expect("dom");
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundColor3"),
            Some(&Variant::Color3(rbx_types::Color3::new(0.0, 1.0, 0.0)))
        );
    }

    /// A CYCLE STOPS AT THE EDGE THAT WOULD CLOSE IT, rather than recursing
    /// forever or failing the whole resolve -- this file's own stated
    /// decision where real Roblox's behavior is undocumented.
    #[test]
    fn a_style_derive_cycle_does_not_hang_or_panic() {
        let dom = SharedDom::default();
        let (root, target, a, b) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let a = guard.insert("StyleSheet".to_string(), "A".to_string());
            let b = guard.insert("StyleSheet".to_string(), "B".to_string());
            (root, target, a, b)
        };
        parent(&dom, target, root);
        parent(&dom, a, root);
        derive(&dom, a, b, 1.0);
        derive(&dom, b, a, 1.0);
        rule(
            &dom,
            a,
            "Frame",
            1.0,
            &[("BackgroundTransparency", Variant::Float64(0.5))],
        );

        let guard = dom.lock().expect("dom");
        // Must return at all (a hang would time out the test binary) and
        // must still resolve the one real rule reachable in the cycle.
        let resolved = resolve(&guard, target);
        assert_eq!(
            resolved.get("BackgroundTransparency"),
            Some(&Variant::Float64(0.5))
        );
    }

    // PART C2: `::Modifier`.

    /// THE FIRST HALF OF THIS SPRINT'S OWN COMPLETION TEST: a matching
    /// `::Modifier` rule creates the child it names, real and parented,
    /// with no Luau anywhere doing it by hand.
    #[test]
    fn a_modifier_rule_spawns_a_real_child_on_a_match() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        rule(&dom, sheet, "Frame::UICorner", 0.0, &[]);

        {
            let mut guard = dom.lock().expect("dom");
            apply_modifiers(&mut guard, root);
        }

        let guard = dom.lock().expect("dom");
        let children = guard.children(target);
        assert_eq!(children.len(), 1, "{children:?}");
        assert_eq!(guard.class_of(children[0]).as_deref(), Some("UICorner"));
    }

    /// THE SECOND HALF OF THIS SPRINT'S OWN COMPLETION TEST: the modifier
    /// rule's own declared properties reach the CHILD it spawns, not the
    /// `Frame` the base selector matched -- an unstyled `UICorner` has
    /// `CornerRadius = 0`, indistinguishable from no corner at all, so this
    /// is what makes the feature visible rather than just present.
    #[test]
    fn a_modifier_rules_own_properties_style_the_spawned_child() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        rule(
            &dom,
            sheet,
            "Frame::UICorner",
            0.0,
            &[("CornerRadius", Variant::UDim(rbx_types::UDim::new(0.0, 12)))],
        );

        {
            let mut guard = dom.lock().expect("dom");
            apply_modifiers(&mut guard, root);
        }

        let guard = dom.lock().expect("dom");
        let corner = guard.children(target)[0];
        assert_eq!(
            resolve(&guard, corner).get("CornerRadius"),
            Some(&Variant::UDim(rbx_types::UDim::new(0.0, 12)))
        );
        // AND NOT ON THE FRAME ITSELF -- the rule's own base selector
        // (`Frame`) still would have matched `target` directly had the
        // modifier not redirected it.
        assert_eq!(resolve(&guard, target).get("CornerRadius"), None);
    }

    /// AN ALREADY-PRESENT CHILD OF THE SAME CLASS IS LEFT ALONE, both in
    /// the sense of not being duplicated and in the sense that running the
    /// pass twice does not create a second one either -- idempotence is
    /// this sprint's own stated reason it is safe to call every dirty
    /// frame.
    #[test]
    fn a_modifier_does_not_duplicate_an_existing_child() {
        let dom = SharedDom::default();
        let (root, target, sheet, corner) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            let corner = guard.insert("UICorner".to_string(), "UICorner".to_string());
            (root, target, sheet, corner)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        parent(&dom, corner, target);
        rule(&dom, sheet, "Frame::UICorner", 0.0, &[]);

        {
            let mut guard = dom.lock().expect("dom");
            apply_modifiers(&mut guard, root);
            apply_modifiers(&mut guard, root);
        }

        let guard = dom.lock().expect("dom");
        assert_eq!(guard.children(target), vec![corner]);
    }

    #[test]
    fn a_modifier_never_touches_a_non_matching_instance() {
        let dom = SharedDom::default();
        let (root, other, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let other = guard.insert("TextLabel".to_string(), "Label".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, other, sheet)
        };
        parent(&dom, other, root);
        parent(&dom, sheet, root);
        rule(&dom, sheet, ".Themed::UICorner", 0.0, &[]);

        {
            let mut guard = dom.lock().expect("dom");
            apply_modifiers(&mut guard, root);
        }

        let guard = dom.lock().expect("dom");
        assert_eq!(guard.children(other), Vec::<usize>::new());
    }

    // PART C3: `StyleQuery`.

    /// THE FIRST HALF OF THIS SPRINT'S OWN COMPLETION TEST: a built-in
    /// query wired to a real, checkable host signal (window size).
    #[test]
    fn a_viewport_query_only_contributes_at_the_right_size() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        rule(
            &dom,
            sheet,
            "@ViewportDisplaySizeSmall Frame",
            0.0,
            &[("BackgroundTransparency", Variant::Float64(0.5))],
        );

        {
            let mut guard = dom.lock().expect("dom");
            guard.set_viewport(2000, 2000);
        }
        let guard = dom.lock().expect("dom");
        assert_eq!(resolve(&guard, target).get("BackgroundTransparency"), None);
        drop(guard);

        {
            let mut guard = dom.lock().expect("dom");
            guard.set_viewport(300, 300);
        }
        let guard = dom.lock().expect("dom");
        assert_eq!(
            resolve(&guard, target).get("BackgroundTransparency"),
            Some(&Variant::Float64(0.5))
        );
    }

    /// A custom `::StyleQuery #Name` declaration's own `MinSize` gates a
    /// rule referencing it by `@Name`, the same shape the built-in
    /// viewport queries above use for their own condition.
    #[test]
    fn a_custom_style_query_gates_a_rule_by_min_size() {
        let dom = SharedDom::default();
        let (root, target, sheet, query) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            let query = guard.insert("StyleQuery".to_string(), "Roomy".to_string());
            guard.node_mut(query).expect("query").props.insert(
                "MinSize".to_string(),
                Variant::Vector2(rbx_types::Vector2::new(800.0, 600.0)),
            );
            (root, target, sheet, query)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        parent(&dom, query, sheet);
        rule(
            &dom,
            sheet,
            "@Roomy Frame",
            0.0,
            &[("BackgroundTransparency", Variant::Float64(0.5))],
        );

        {
            let mut guard = dom.lock().expect("dom");
            guard.set_viewport(400, 300);
        }
        let guard = dom.lock().expect("dom");
        assert_eq!(resolve(&guard, target).get("BackgroundTransparency"), None);
        drop(guard);

        {
            let mut guard = dom.lock().expect("dom");
            guard.set_viewport(1000, 800);
        }
        let guard = dom.lock().expect("dom");
        assert_eq!(
            resolve(&guard, target).get("BackgroundTransparency"),
            Some(&Variant::Float64(0.5))
        );
    }

    /// `IsActive` IS THE READ-ONLY PROPERTY A GUEST SCRIPT ACTUALLY SEES --
    /// `refresh_style_queries` is what keeps it in sync with the same
    /// condition `custom_query_holds` already checks internally.
    #[test]
    fn refresh_style_queries_keeps_is_active_in_sync() {
        let dom = SharedDom::default();
        let (root, sheet, query) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            let query = guard.insert("StyleQuery".to_string(), "Roomy".to_string());
            guard.node_mut(query).expect("query").props.insert(
                "MinSize".to_string(),
                Variant::Vector2(rbx_types::Vector2::new(800.0, 600.0)),
            );
            (root, sheet, query)
        };
        parent(&dom, sheet, root);
        parent(&dom, query, sheet);

        {
            let mut guard = dom.lock().expect("dom");
            guard.set_viewport(400, 300);
            guard.refresh_style_queries(root);
        }
        assert_eq!(
            dom.lock().expect("dom").property(query, "IsActive"),
            Some(Variant::Bool(false))
        );

        {
            let mut guard = dom.lock().expect("dom");
            guard.set_viewport(1000, 800);
            guard.refresh_style_queries(root);
        }
        assert_eq!(
            dom.lock().expect("dom").property(query, "IsActive"),
            Some(Variant::Bool(true))
        );
    }

    /// AN UNSET CONDITION DOES NOT CONSTRAIN. A `StyleQuery` naming only
    /// `MinSize` is a pure minimum-size gate, not one that also silently
    /// demands some default `ViewportDisplaySize`/`PreferredInput` too.
    #[test]
    fn a_style_query_with_one_condition_ignores_every_other_one() {
        let dom = SharedDom::default();
        let (root, query) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let query = guard.insert("StyleQuery".to_string(), "JustSize".to_string());
            guard.node_mut(query).expect("query").props.insert(
                "MinSize".to_string(),
                Variant::Vector2(rbx_types::Vector2::new(100.0, 100.0)),
            );
            (root, query)
        };
        let _ = root;
        dom.lock().expect("dom").set_viewport(5000, 5000);
        assert!(style_query_holds(&dom.lock().expect("dom"), query));
    }

    // PART C5: `StyleRule` NESTING.

    /// THIS SPRINT'S OWN COMPLETION TEST: a nested rule's own merged
    /// selector matches exactly what typing the combined selector directly
    /// would -- proven by resolving the SAME target through both a nested
    /// `"> TextButton"` under `"#MenuFrame"` and a single rule written
    /// `"#MenuFrame > TextButton"` directly, and getting the same answer.
    #[test]
    fn a_nested_rule_merges_with_its_parents_selector() {
        let dom = SharedDom::default();
        let (root, menu, button, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let menu = guard.insert("Frame".to_string(), "MenuFrame".to_string());
            let button = guard.insert("TextButton".to_string(), "Button".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, menu, button, sheet)
        };
        parent(&dom, menu, root);
        parent(&dom, button, menu);
        parent(&dom, sheet, root);

        let outer = rule(&dom, sheet, "#MenuFrame", 0.0, &[]);
        let nested = {
            let mut guard = dom.lock().expect("dom");
            let nested = guard.insert("StyleRule".to_string(), "StyleRule".to_string());
            guard.node_mut(nested).expect("nested").props.insert(
                "Selector".to_string(),
                Variant::String("> TextButton".to_string()),
            );
            guard.set_style_property(
                nested,
                "BackgroundTransparency",
                Some(Variant::Float64(0.25)),
            );
            nested
        };
        parent(&dom, nested, outer);

        let guard = dom.lock().expect("dom");
        assert_eq!(
            resolve(&guard, button).get("BackgroundTransparency"),
            Some(&Variant::Float64(0.25))
        );
    }

    /// THE MERGE IS `>` (CHILD), NOT `>>` (DESCENDANT) -- a `TextButton`
    /// two levels under `MenuFrame` is out of a nested `"> TextButton"`'s
    /// own reach, the same distinction C1's own combinator tests already
    /// draw for a top-level rule.
    #[test]
    fn a_nested_rules_merge_respects_child_not_descendant() {
        let dom = SharedDom::default();
        let (root, menu, wrapper, button, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let menu = guard.insert("Frame".to_string(), "MenuFrame".to_string());
            let wrapper = guard.insert("Frame".to_string(), "Wrapper".to_string());
            let button = guard.insert("TextButton".to_string(), "Button".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, menu, wrapper, button, sheet)
        };
        parent(&dom, menu, root);
        parent(&dom, wrapper, menu);
        parent(&dom, button, wrapper);
        parent(&dom, sheet, root);

        let outer = rule(&dom, sheet, "#MenuFrame", 0.0, &[]);
        let nested = {
            let mut guard = dom.lock().expect("dom");
            let nested = guard.insert("StyleRule".to_string(), "StyleRule".to_string());
            guard.node_mut(nested).expect("nested").props.insert(
                "Selector".to_string(),
                Variant::String("> TextButton".to_string()),
            );
            guard.set_style_property(
                nested,
                "BackgroundTransparency",
                Some(Variant::Float64(0.25)),
            );
            nested
        };
        parent(&dom, nested, outer);

        let guard = dom.lock().expect("dom");
        assert_eq!(resolve(&guard, button).get("BackgroundTransparency"), None);
    }

    /// NESTING GOES MORE THAN ONE LEVEL DEEP: a rule nested inside a rule
    /// nested inside a rule still merges all three selectors in order.
    #[test]
    fn nesting_composes_more_than_one_level_deep() {
        let dom = SharedDom::default();
        let (root, menu, item, label, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let menu = guard.insert("Frame".to_string(), "MenuFrame".to_string());
            let item = guard.insert("Frame".to_string(), "Item".to_string());
            let label = guard.insert("TextLabel".to_string(), "Label".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, menu, item, label, sheet)
        };
        parent(&dom, menu, root);
        parent(&dom, item, menu);
        parent(&dom, label, item);
        parent(&dom, sheet, root);

        let outer = rule(&dom, sheet, "#MenuFrame", 0.0, &[]);
        let middle = {
            let mut guard = dom.lock().expect("dom");
            let middle = guard.insert("StyleRule".to_string(), "StyleRule".to_string());
            guard.node_mut(middle).expect("middle").props.insert(
                "Selector".to_string(),
                Variant::String("> Frame".to_string()),
            );
            middle
        };
        parent(&dom, middle, outer);
        let inner = {
            let mut guard = dom.lock().expect("dom");
            let inner = guard.insert("StyleRule".to_string(), "StyleRule".to_string());
            guard.node_mut(inner).expect("inner").props.insert(
                "Selector".to_string(),
                Variant::String("> TextLabel".to_string()),
            );
            guard.set_style_property(inner, "TextTransparency", Some(Variant::Float64(0.1)));
            inner
        };
        parent(&dom, inner, middle);

        let guard = dom.lock().expect("dom");
        assert_eq!(
            resolve(&guard, label).get("TextTransparency"),
            Some(&Variant::Float64(0.1))
        );
    }

    /// A NESTED RULE WHOSE OWN SELECTOR IS NOT A LEADING COMBINATOR IS
    /// PARSED INDEPENDENTLY, not merged -- this sprint's own stated scope
    /// cut, since the one confirmed real example only nests a leading
    /// combinator.
    #[test]
    fn a_nested_rule_without_a_leading_combinator_is_independent() {
        let dom = SharedDom::default();
        let (root, menu, button, other, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let menu = guard.insert("Frame".to_string(), "MenuFrame".to_string());
            let button = guard.insert("TextButton".to_string(), "Button".to_string());
            let other = guard.insert("TextButton".to_string(), "Elsewhere".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, menu, button, other, sheet)
        };
        parent(&dom, menu, root);
        parent(&dom, button, menu);
        parent(&dom, other, root);
        parent(&dom, sheet, root);

        let outer = rule(&dom, sheet, "#MenuFrame", 0.0, &[]);
        let nested = {
            let mut guard = dom.lock().expect("dom");
            let nested = guard.insert("StyleRule".to_string(), "StyleRule".to_string());
            guard.node_mut(nested).expect("nested").props.insert(
                "Selector".to_string(),
                Variant::String("TextButton".to_string()),
            );
            guard.set_style_property(
                nested,
                "BackgroundTransparency",
                Some(Variant::Float64(0.9)),
            );
            nested
        };
        parent(&dom, nested, outer);

        let guard = dom.lock().expect("dom");
        // Reaches `other`, ANYWHERE, since the nested selector is bare
        // `TextButton` on its own -- not merged into `#MenuFrame`'s own
        // reach, so `button` being under `MenuFrame` is not what matched
        // it here either.
        assert_eq!(
            resolve(&guard, other).get("BackgroundTransparency"),
            Some(&Variant::Float64(0.9))
        );
    }

    // PART C4: `SetPropertyTransition`.

    /// THIS SPRINT'S OWN COMPLETION TEST: a `StyleDerive` swap (the same
    /// shape a `:Hover` rule taking effect has, a cascade re-resolution
    /// rather than a script's own direct write) eases over the declared
    /// `TweenInfo` instead of snapping -- observed mid-flight, not just at
    /// the two endpoints.
    #[test]
    fn a_cascade_driven_change_eases_instead_of_snapping() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        let rule_id = rule(
            &dom,
            sheet,
            "Frame",
            0.0,
            &[("BackgroundTransparency", Variant::Float64(0.0))],
        );
        {
            let mut guard = dom.lock().expect("dom");
            guard.set_style_transition(
                rule_id,
                "BackgroundTransparency",
                Some(transition::TweenInfoValue {
                    time: 10.0,
                    ..transition::TweenInfoValue::default()
                }),
            );
        }

        // FIRST RESOLUTION: nothing to ease FROM yet, so this settles
        // immediately at its own starting value.
        dom.lock().expect("dom").advance_transitions(root, 0.0);
        assert_eq!(
            dom.lock()
                .expect("dom")
                .styled_property(target, "BackgroundTransparency"),
            Some(Variant::Float64(0.0))
        );

        // THE CASCADE CHANGES: swap the rule's own declared value, the
        // same shape a `StyleDerive` swap or a `:Hover` match starting
        // would produce.
        {
            let mut guard = dom.lock().expect("dom");
            guard.set_style_property(
                rule_id,
                "BackgroundTransparency",
                Some(Variant::Float64(1.0)),
            );
            guard.advance_transitions(root, 0.0);
        }
        assert!(dom.lock().expect("dom").transitions_active());

        // MID-FLIGHT: neither endpoint, part way between them.
        dom.lock().expect("dom").advance_transitions(root, 5.0);
        assert_eq!(
            dom.lock()
                .expect("dom")
                .styled_property(target, "BackgroundTransparency"),
            Some(Variant::Float64(0.5))
        );

        // SETTLED: past the declared duration, at the real target, and no
        // longer reported as animating.
        dom.lock().expect("dom").advance_transitions(root, 11.0);
        assert_eq!(
            dom.lock()
                .expect("dom")
                .styled_property(target, "BackgroundTransparency"),
            Some(Variant::Float64(1.0))
        );
        assert!(!dom.lock().expect("dom").transitions_active());
    }

    /// A GUEST'S OWN DIRECT WRITE STILL SNAPS -- `styled_property`'s own
    /// "instance's own explicit value first" rule already refuses the
    /// cascade for an instance that set the property itself, and a
    /// transition sits on the SAME side of that rule the cascade does, not
    /// ahead of it.
    #[test]
    fn an_explicit_write_is_not_animated() {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        let rule_id = rule(
            &dom,
            sheet,
            "Frame",
            0.0,
            &[("BackgroundTransparency", Variant::Float64(0.0))],
        );
        {
            let mut guard = dom.lock().expect("dom");
            guard.set_style_transition(
                rule_id,
                "BackgroundTransparency",
                Some(transition::TweenInfoValue::default()),
            );
            guard
                .node_mut(target)
                .expect("target")
                .props
                .insert("BackgroundTransparency".to_string(), Variant::Float64(0.7));
        }

        let guard = dom.lock().expect("dom");
        assert_eq!(
            guard.styled_property(target, "BackgroundTransparency"),
            Some(Variant::Float64(0.7))
        );
    }

    // THE MEMO, OPEN WHILE THE TREE CHANGES. A render pass does not change
    // the cascade's inputs, but nothing stops a caller doing so while one is
    // open, and an answer from before the change must not survive it.

    /// A `Frame` named `Card` under `root`, reached by one `#Card` rule
    /// setting `BackgroundTransparency` to 0.25 from a `StyleSheet` on `root`.
    fn memo_scene() -> (SharedDom, usize, usize, usize, usize) {
        let dom = SharedDom::default();
        let (root, target, sheet) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Root".to_string());
            let target = guard.insert("Frame".to_string(), "Card".to_string());
            let sheet = guard.insert("StyleSheet".to_string(), "StyleSheet".to_string());
            (root, target, sheet)
        };
        parent(&dom, target, root);
        parent(&dom, sheet, root);
        let card_rule = rule(
            &dom,
            sheet,
            "#Card",
            0.0,
            &[("BackgroundTransparency", Variant::Float64(0.25))],
        );
        (dom, root, target, sheet, card_rule)
    }

    const KEY: &str = "BackgroundTransparency";

    /// What a `Frame` reads for [`KEY`] with no rule reaching it.
    fn unstyled() -> Option<Variant> {
        crate::datamodel::default_for("Frame", KEY)
    }

    /// Opens the memo, reads `target` once so its answer is memoised, applies
    /// `change`, and answers what `target` reads with the memo still open.
    fn read_across(
        dom: &SharedDom,
        target: usize,
        change: impl FnOnce(&mut Dom),
    ) -> Option<Variant> {
        let mut guard = dom.lock().expect("dom");
        let opened = guard.open_style_memo();
        assert!(opened);
        assert_eq!(
            guard.styled_property(target, KEY),
            Some(Variant::Float64(0.25))
        );
        change(&mut guard);
        let after = guard.styled_property(target, KEY);
        guard.close_style_memo(opened);
        after
    }

    #[test]
    fn the_memo_answers_what_a_fresh_resolve_answers() {
        let (dom, _, target, _, _) = memo_scene();
        let guard = dom.lock().expect("dom");
        let opened = guard.open_style_memo();
        let first = guard.styled_property(target, KEY);
        let second = guard.styled_property(target, KEY);
        guard.close_style_memo(opened);
        assert_eq!(first, Some(Variant::Float64(0.25)));
        assert_eq!(second, first);
        assert_eq!(guard.styled_property(target, KEY), first);
    }

    #[test]
    fn a_rename_inside_an_open_memo_is_seen() {
        let (dom, _, target, _, _) = memo_scene();
        let after = read_across(&dom, target, |dom| {
            dom.node_mut(target).expect("target").name = "Other".to_string();
        });
        assert_eq!(after, unstyled());
    }

    #[test]
    fn a_rule_property_change_inside_an_open_memo_is_seen() {
        let (dom, _, target, _, card_rule) = memo_scene();
        let after = read_across(&dom, target, |dom| {
            dom.set_style_property(card_rule, KEY, Some(Variant::Float64(0.5)));
        });
        assert_eq!(after, Some(Variant::Float64(0.5)));
    }

    #[test]
    fn a_rule_selector_change_inside_an_open_memo_is_seen() {
        let (dom, _, target, _, card_rule) = memo_scene();
        let after = read_across(&dom, target, |dom| {
            dom.node_mut(card_rule).expect("rule").props.insert(
                "Selector".to_string(),
                Variant::String(".Accent".to_string()),
            );
        });
        assert_eq!(after, unstyled());
    }

    #[test]
    fn a_tag_inside_an_open_memo_is_seen() {
        let (dom, _, target, sheet, _) = memo_scene();
        rule(
            &dom,
            sheet,
            ".Accent",
            5.0,
            &[(KEY, Variant::Float64(0.75))],
        );
        let after = read_across(&dom, target, |dom| {
            assert!(dom.add_tag(target, "Accent"));
        });
        assert_eq!(after, Some(Variant::Float64(0.75)));
    }

    #[test]
    fn a_reparent_inside_an_open_memo_is_seen() {
        let (dom, _, target, _, _) = memo_scene();
        let elsewhere = dom
            .lock()
            .expect("dom")
            .insert("Frame".to_string(), "Elsewhere".to_string());
        let after = read_across(&dom, target, |dom| {
            dom.unparent(target);
            dom.node_mut(target).expect("target").parent = Some(elsewhere);
            dom.node_mut(elsewhere)
                .expect("elsewhere")
                .children
                .push(target);
        });
        assert_eq!(after, unstyled());
    }

    #[test]
    fn a_token_change_inside_an_open_memo_is_seen() {
        let (dom, _, target, sheet, card_rule) = memo_scene();
        {
            let mut guard = dom.lock().expect("dom");
            guard.set_attribute(sheet, "Fade", Some(Variant::Float64(0.25)));
            guard.set_style_property(card_rule, KEY, Some(Variant::String("$Fade".to_string())));
        }
        let after = read_across(&dom, target, |dom| {
            dom.set_attribute(sheet, "Fade", Some(Variant::Float64(0.9)));
        });
        assert_eq!(after, Some(Variant::Float64(0.9)));
    }

    #[test]
    fn a_viewport_change_inside_an_open_memo_reaches_a_query() {
        let (dom, _, target, _, card_rule) = memo_scene();
        {
            let mut guard = dom.lock().expect("dom");
            guard.set_viewport(400, 300);
            guard.node_mut(card_rule).expect("rule").props.insert(
                "Selector".to_string(),
                Variant::String("@ViewportDisplaySizeSmall #Card".to_string()),
            );
        }
        let after = read_across(&dom, target, |dom| {
            dom.set_viewport(1600, 900);
        });
        assert_eq!(after, unstyled());
    }

    #[test]
    fn a_destroyed_sheet_inside_an_open_memo_stops_styling() {
        let (dom, _, target, sheet, _) = memo_scene();
        let after = read_across(&dom, target, |dom| dom.destroy(sheet));
        assert_eq!(after, unstyled());
    }

    #[test]
    fn a_nested_pass_leaves_closing_to_the_outer_one() {
        let (dom, _, target, _, card_rule) = memo_scene();
        let mut guard = dom.lock().expect("dom");
        let outer = guard.open_style_memo();
        let inner = guard.open_style_memo();
        assert!(outer);
        assert!(!inner);
        assert_eq!(
            guard.styled_property(target, KEY),
            Some(Variant::Float64(0.25))
        );
        guard.close_style_memo(inner);
        assert!(guard.style_memo.0.borrow().is_some(), "still open");
        guard.close_style_memo(outer);
        assert!(guard.style_memo.0.borrow().is_none(), "closed");
        guard.set_style_property(card_rule, KEY, Some(Variant::Float64(0.5)));
        assert_eq!(
            guard.styled_property(target, KEY),
            Some(Variant::Float64(0.5))
        );
    }
}
