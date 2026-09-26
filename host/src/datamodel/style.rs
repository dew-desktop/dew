//! `StyleRule.Selector` parsing and matching -- the model half of `StyleSheet`.
//!
//! WHAT IS HERE, AND WHAT IS NOT. Parsing a selector string into a
//! [`Selector`], and testing one instance against one. No cascade: nothing
//! here resolves conflicting rules, follows a `StyleLink`, or reads
//! `Priority`. No paint: nothing here reaches `render.rs`. Those build on
//! this file's model rather than living in it.
//!
//! THE THREE FORMS, AND ONLY THOSE THREE. Roblox's own selector grammar
//! supports compound and combinator selectors (`Frame.Enemy`, descendant
//! combinators, and the rest) -- confirmed against the real engine's own
//! documentation, not assumed from the shape looking like CSS. [`parse`]
//! covers only the three atomic forms below, and refuses anything else with
//! a message rather than silently matching the wrong half of a compound
//! selector it cannot actually parse. Widening this to the real grammar is a
//! deliberate later decision, not a bug in this file.
//!
//! WHERE ROBLOX'S OWN NAMES DIVERGE FROM CSS'S. The bare-word form is a
//! "class name selector" in Roblox's own styling documentation, matched
//! through `IsA` (a `Frame` selector matches a `TextButton` too, since both
//! descend from `GuiObject` -- exact-`ClassName` matching would report a
//! screen of buttons as containing no `GuiObject` at all). The `.foo` form is
//! Roblox's own "tag selector", reading `CollectionService`'s own tags
//! directly -- so it is named [`Selector::Tag`] here, matching that
//! service's own vocabulary, rather than `Selector::Class`, which would
//! collide with a completely different Lua/OOP idea of "class" this codebase
//! does not have.

use super::{class_exists, members, Dom};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    /// A bare word: `Frame`. Matches through `IsA`, not exact `ClassName`.
    ClassName(String),
    /// `.foo`: a `CollectionService` tag.
    Tag(String),
    /// `#foo`: `Instance.Name`.
    Name(String),
}

/// Parse one `StyleRule.Selector` string, or say why it does not parse.
///
/// THE MESSAGE IS WHAT `StyleRule.SelectorError` READS ON THE ENGINE -- an
/// invalid selector is a fact about the rule, not a thrown error, which is
/// why this returns `Result` rather than `LuaResult`: wiring it to that
/// property is the caller's job, once something writes `Selector` live.
pub fn parse(selector: &str) -> Result<Selector, String> {
    if selector.is_empty() {
        return Err("a selector cannot be empty".to_string());
    }

    if let Some(tag) = selector.strip_prefix('.') {
        if tag.is_empty() {
            return Err("a `.` selector needs a tag name after the dot".to_string());
        }
        return Ok(Selector::Tag(tag.to_string()));
    }

    if let Some(name) = selector.strip_prefix('#') {
        if name.is_empty() {
            return Err("a `#` selector needs a name after the hash".to_string());
        }
        return Ok(Selector::Name(name.to_string()));
    }

    // A COMPOUND OR COMBINATOR SELECTOR IS REFUSED, NOT MISREAD. Roblox's own
    // grammar allows one, and treating `Frame.Enemy` as the class name
    // `Frame.Enemy` (which matches nothing) would fail silently instead of
    // saying why.
    if selector
        .chars()
        .any(|c| c.is_whitespace() || c == '.' || c == '#')
    {
        return Err(format!(
            "`{selector}` looks like a compound or combinator selector, \
             which this host does not parse yet"
        ));
    }

    if !class_exists(selector) {
        return Err(format!("`{selector}` is not a class name this host knows"));
    }

    Ok(Selector::ClassName(selector.to_string()))
}

/// Does `id` match `selector`?
///
/// NO CALLER OUTSIDE THIS FILE'S OWN TESTS YET, AND THAT IS STAGING, NOT AN
/// OVERSIGHT. Cascade resolution, resolving a `StyleSheet`'s rules against an
/// instance with `Priority` as the tiebreak, is what calls this; proving the
/// predicate against a hand-built tree first is what this file is for.
#[allow(dead_code)]
pub fn matches(dom: &Dom, id: usize, selector: &Selector) -> bool {
    match selector {
        Selector::ClassName(name) => dom
            .class_of(id)
            .is_some_and(|class| members::class_is_a(&class, name)),
        Selector::Tag(tag) => dom.has_tag(id, tag),
        Selector::Name(name) => dom.name_of(id).as_deref() == Some(name.as_str()),
    }
}

/// Every instance at or under `root` that `selector` matches, depth first,
/// parent before child, in child order.
///
/// MULTIPLE MATCHES FOR A `#name` SELECTOR ARE NOT AN ERROR, AND THIS IS THE
/// DECISION THE MILESTONE PLAN ASKED FOR IN WRITING RATHER THAN GUESSED AT.
/// Roblox does not enforce sibling name uniqueness -- two children of the
/// same parent can share a `Name` -- so a `#Header` selector styles every
/// instance it finds named `Header`, the same as a duplicate `id` in
/// HTML/CSS styles every element that carries it rather than refusing to
/// render. Turning a naming collision the tree already tolerates into an
/// ambiguity error would fail a paint pass over a mistake nothing else in
/// this host treats as one.
///
/// Also unused outside this file's own tests until sprint 3 -- see
/// `matches`'s own doc comment just above.
#[allow(dead_code)]
pub fn matching_in(dom: &Dom, root: usize, selector: &Selector) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if matches(dom, id, selector) {
            out.push(id);
        }
        let mut children = dom.children(id);
        children.reverse();
        stack.extend(children);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamodel::SharedDom;

    fn tree() -> (SharedDom, usize, usize, usize, usize) {
        // root(Frame)
        //   card(Frame, tagged "Enemy")
        //     label(TextLabel, Name = "Header")
        //   other(TextButton, Name = "Header")
        let dom = SharedDom::default();
        let (root, card, label, other) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Frame".to_string());
            let card = guard.insert("Frame".to_string(), "Frame".to_string());
            let label = guard.insert("TextLabel".to_string(), "Header".to_string());
            let other = guard.insert("TextButton".to_string(), "Header".to_string());
            guard.add_tag(card, "Enemy");
            (root, card, label, other)
        };
        parent(&dom, card, root);
        parent(&dom, label, card);
        parent(&dom, other, root);
        (dom, root, card, label, other)
    }

    fn parent(dom: &SharedDom, id: usize, on: usize) {
        let mut guard = dom.lock().expect("dom");
        guard.node_mut(id).expect("id").parent = Some(on);
        guard.node_mut(on).expect("on").children.push(id);
    }

    #[test]
    fn a_bare_word_matches_by_is_a_not_exact_class_name() {
        let (dom, root, card, label, other) = tree();
        let guard = dom.lock().expect("dom");

        // `GuiButton` selects only `other` (a `TextButton`): `card` (a
        // `Frame`) and `label` (a `TextLabel`) do not descend from it.
        let button_selector = parse("GuiButton").expect("parse");
        assert_eq!(matching_in(&guard, root, &button_selector), vec![other]);

        // `GuiObject` selects `card`, `label` AND `other`: a `Frame`, a
        // `TextLabel` and a `TextButton` all descend from it, which is the
        // difference `IsA` matching makes over an exact `ClassName` compare
        // -- the latter would answer none of them for a selector naming
        // their common ancestor rather than their own class.
        let object_selector = parse("GuiObject").expect("parse");
        let got = matching_in(&guard, root, &object_selector);
        assert!(got.contains(&card), "{got:?}");
        assert!(got.contains(&label), "{got:?}");
        assert!(got.contains(&other), "{got:?}");
    }

    #[test]
    fn a_dot_selector_reads_the_collection_service_tag() {
        let (dom, root, card, _label, other) = tree();
        let guard = dom.lock().expect("dom");
        let selector = parse(".Enemy").expect("parse");
        let got = matching_in(&guard, root, &selector);
        assert_eq!(got, vec![card]);
        assert!(!got.contains(&other));
    }

    #[test]
    fn a_hash_selector_matches_every_instance_sharing_the_name() {
        let (dom, root, _card, label, other) = tree();
        let guard = dom.lock().expect("dom");
        let selector = parse("#Header").expect("parse");
        let got = matching_in(&guard, root, &selector);
        // BOTH, per this file's own decision: duplicate names are not an
        // ambiguity error.
        assert!(got.contains(&label), "{got:?}");
        assert!(got.contains(&other), "{got:?}");
    }

    #[test]
    fn an_empty_selector_does_not_parse() {
        assert!(parse("").is_err());
    }

    #[test]
    fn a_dot_or_hash_with_nothing_after_it_does_not_parse() {
        assert!(parse(".").is_err());
        assert!(parse("#").is_err());
    }

    #[test]
    fn an_unknown_class_name_does_not_parse() {
        let err = parse("NotARealClass").unwrap_err();
        assert!(err.contains("NotARealClass"), "{err}");
    }

    #[test]
    fn a_compound_selector_is_refused_rather_than_misread() {
        // `Frame.Enemy` is valid on the real engine and out of this
        // milestone's scope; refusing it beats matching the class name
        // `Frame.Enemy`, which is nothing.
        let err = parse("Frame.Enemy").unwrap_err();
        assert!(err.contains("compound"), "{err}");
    }
}
