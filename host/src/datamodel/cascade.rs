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

use super::{style, Dom};
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
    let mut candidates = Vec::new();
    for (distance, sheet) in applicable_style_sheets(dom, id) {
        for (order, rule) in dom.children(sheet).into_iter().enumerate() {
            if dom.class_of(rule).as_deref() != Some("StyleRule") {
                continue;
            }
            let Some(selector) = selector_of(dom, rule) else {
                continue;
            };
            if !style::matches(dom, id, &selector) {
                continue;
            }
            candidates.push(Candidate {
                style_rule: rule,
                priority: priority_of(dom, rule),
                distance,
                order,
            });
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
        for (name, value) in dom.get_style_properties(candidate.style_rule) {
            resolved.insert(name, resolve_token(dom, sheet, value));
        }
    }
    resolved
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

/// `StyleRule.Selector`, parsed -- `None` for a rule whose selector does not
/// parse, the same as `None` for one that simply does not match: an invalid
/// selector already carries its own `SelectorError`, and repeating that
/// complaint here for every instance it is tested against would be noise
/// beyond what one property read already says.
fn selector_of(dom: &Dom, rule: usize) -> Option<style::Selector> {
    let Variant::String(selector) = dom.property(rule, "Selector")? else {
        return None;
    };
    style::parse(&selector).ok()
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

impl Dom {
    /// `property`, with a matching `StyleRule` filling the gap between `id`'s
    /// own explicit value and its engine default. The render path's own
    /// read -- see this module's own doc comment for why `Index` does not
    /// call this instead.
    pub fn styled_property(&self, id: usize, key: &str) -> Option<Variant> {
        if let Some(explicit) = self.node(id)?.props.get(key).cloned() {
            return Some(explicit);
        }
        if let Some(styled) = resolve(self, id).remove(key) {
            return Some(styled);
        }
        self.property(id, key)
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
}
