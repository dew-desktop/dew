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
//! WHAT THIS FILE DOES NOT DO. No paint: nothing here reaches `render.rs`,
//! and the map [`resolve`] returns is not filtered against the target's own
//! declared properties -- an instance receiving a name a paint pass has no
//! use for is that pass's decision, not this one's. No re-resolution
//! scoping: this is one full resolve of one instance, called however often a
//! future live-update needs it; making that call cheap when only a handful
//! of properties actually changed is that update mechanism's problem to
//! solve, not a shortcut to bake in before it exists.

use super::{style, Dom};
use rbx_types::Variant;
use std::collections::BTreeMap;

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
/// breaking every contest -- the map a paint pass would apply on top of
/// `id`'s own explicit properties.
///
/// NO CALLER OUTSIDE THIS FILE'S OWN TESTS YET. Wiring this into the paint
/// path, and re-resolving it live as rules change, are both later work; this
/// is the resolution itself, proven against hand-built trees first.
#[allow(dead_code)]
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
        resolved.extend(dom.get_style_properties(candidate.style_rule));
    }
    resolved
}

/// `(distance, StyleSheet id)` for every `StyleSheet` that reaches `id`,
/// through direct parenting or through a `StyleLink`, nearest first.
fn applicable_style_sheets(dom: &Dom, id: usize) -> Vec<(usize, usize)> {
    let mut sheets = Vec::new();
    let mut cursor = Some(id);
    let mut distance = 0;
    while let Some(current) = cursor {
        for child in dom.children(current) {
            match dom.class_of(child).as_deref() {
                Some("StyleSheet") => sheets.push((distance, child)),
                // A `StyleLink` NAMES A SHEET, IT IS NOT ONE. Its own
                // children (it has none that matter here) are never walked
                // for `StyleRule`s; only the sheet it points at is.
                Some("StyleLink") => {
                    if let Some(target) = style_link_target(dom, child) {
                        sheets.push((distance, target));
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
}
