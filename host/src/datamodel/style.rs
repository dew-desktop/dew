//! `StyleRule.Selector` parsing and matching -- the model half of `StyleSheet`.
//!
//! WHAT IS HERE, AND WHAT IS NOT. Parsing a selector string into a
//! [`Selector`], and testing one instance against one. No cascade: nothing
//! here resolves conflicting rules, follows a `StyleLink`, or reads
//! `Priority`. No paint: nothing here reaches `render.rs`. Those build on
//! this file's model rather than living in it.
//!
//! FOUR ATOMIC FORMS, PLUS COMBINATORS AND LISTS. [`parse`] covers
//! `ClassName`, `.Tag`, `#Name`, `:State`, each alone, plus two ways to
//! combine them: `>` (child) and `>>` (descendant) join two selectors,
//! space-bounded on both sides (`".MenuContainer > TextButton"`), and `,`
//! joins a list any of which may match (`"ImageLabel, TextLabel"`) --
//! confirmed exact syntax against Roblox's own CSS-comparison
//! documentation, not assumed from the shape looking like CSS (`>>` stands
//! in for CSS's whitespace-descendant combinator, since a Lua string
//! selector has no significant-whitespace convention to borrow).
//!
//! COMPOUND SELECTORS ARE STILL REFUSED. `Frame.Enemy` or `Frame:Hover` --
//! two atomic forms glued together with NO space -- is real on the engine
//! and still out of scope here; refusing it beats matching the class name
//! `Frame.Enemy`, which is nothing. The combinator forms above are
//! distinguished from this by the space: `>`/`>>`/`,` always sit between
//! whitespace, a compound glue never does.
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

use super::{class_exists, enums, members, Dom};
use rbx_types::Variant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    /// A bare word: `Frame`. Matches through `IsA`, not exact `ClassName`.
    ClassName(String),
    /// `.foo`: a `CollectionService` tag.
    Tag(String),
    /// `#foo`: `Instance.Name`.
    Name(String),
    /// `:Hover`, `:Press`, `:Idle`, `:NonInteractable` -- `GuiState`,
    /// milestone 29 part B. BARE ONLY, not the real engine's own compound
    /// form (`"ImageLabel:Hover"`, class name AND state together) -- that
    /// needs the compound-selector support part C of the same milestone
    /// scopes separately, not smuggled in here. A bare state selector is
    /// not the narrower half of a feature on this host specifically: unlike
    /// real Roblox, where every `GuiObject` carries a `GuiState` the engine
    /// computes automatically (making a bare `:Hover` match everything
    /// interactive on screen), this host's own `GuiState` is set only where
    /// a caller explicitly calls `SetGuiState` -- already exactly as scoped
    /// as a class filter would make it.
    State(u32),
    /// `A > B`: `B` matches `id`, and `A` matches `id`'s own parent.
    /// Milestone 29 part C1.
    Child(Box<Selector>, Box<Selector>),
    /// `A >> B`: `B` matches `id`, and `A` matches some ancestor of `id`
    /// (not necessarily the immediate parent). Milestone 29 part C1.
    Descendant(Box<Selector>, Box<Selector>),
    /// `A, B`: matches whatever either side alone would. Milestone 29
    /// part C1.
    List(Vec<Selector>),
}

/// Parse one `StyleRule.Selector` string, or say why it does not parse.
///
/// THE MESSAGE IS WHAT `StyleRule.SelectorError` READS ON THE ENGINE -- an
/// invalid selector is a fact about the rule, not a thrown error, which is
/// why this returns `Result` rather than `LuaResult`: wiring it to that
/// property is the caller's job, once something writes `Selector` live.
pub fn parse(selector: &str) -> Result<Selector, String> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return Err("a selector cannot be empty".to_string());
    }

    // SELECTOR LISTS, THE WIDEST SPLIT. `"ImageLabel, TextLabel"` matches
    // either side; each side is parsed on its own (and may itself use a
    // combinator), so this has to run before combinator-splitting does.
    if trimmed.contains(',') {
        let mut list = Vec::new();
        for part in trimmed.split(',') {
            let part = part.trim();
            if part.is_empty() {
                return Err(format!("`{selector}` has an empty selector in its list"));
            }
            list.push(parse(part)?);
        }
        return Ok(Selector::List(list));
    }

    // COMBINATORS. Confirmed exact syntax against Roblox's own
    // CSS-comparison documentation: `>` (child) and `>>` (descendant),
    // both always space-bounded on both sides -- which is also what tells
    // a combinator apart from a compound selector's own bare glue
    // (`Frame.Enemy`, no space) a few lines down still refuses. Only a
    // single combinator is handled: `"A > B > C"` falls through to the
    // atomic parse below and is refused same as any other shape this
    // grammar does not cover yet.
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() == 3 && (tokens[1] == ">" || tokens[1] == ">>") {
        let left = parse(tokens[0])?;
        let right = parse(tokens[2])?;
        return Ok(if tokens[1] == ">" {
            Selector::Child(Box::new(left), Box::new(right))
        } else {
            Selector::Descendant(Box::new(left), Box::new(right))
        });
    }

    parse_atomic(trimmed)
}

fn parse_atomic(selector: &str) -> Result<Selector, String> {
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

    if let Some(state) = selector.strip_prefix(':') {
        if state.is_empty() {
            return Err("a `:` selector needs a state name after the colon".to_string());
        }
        let Some(item) = enums::item_by_name("GuiState", state) else {
            return Err(format!("`{state}` is not a GuiState this host knows"));
        };
        return Ok(Selector::State(item.value));
    }

    // A COMPOUND OR COMBINATOR SELECTOR IS REFUSED, NOT MISREAD. Roblox's own
    // grammar allows one, and treating `Frame.Enemy` as the class name
    // `Frame.Enemy` (which matches nothing) would fail silently instead of
    // saying why.
    if selector
        .chars()
        .any(|c| c.is_whitespace() || c == '.' || c == '#' || c == ':')
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

/// Splits a `::Modifier` suffix off `selector`, if there is one, and parses
/// the rest normally. `"Frame.RoundedCorner20::UICorner"` doesn't just
/// select an existing `UICorner` -- confirmed against Roblox's own
/// documentation, matching a rule that carries one also ensures a child of
/// the named class exists wherever it matches (milestone 29 part C2, no CSS
/// analogue: `::before`/`::after` are paint-only, never real DOM nodes).
/// This is NOT a fourth [`Selector`] variant: it is not a match condition at
/// all, so it is returned alongside the parsed selector rather than folded
/// into it, the same way `Priority` sits beside a selector rather than
/// inside one.
pub fn parse_with_modifier(selector: &str) -> Result<(Selector, Option<String>), String> {
    let trimmed = selector.trim();
    let Some(idx) = trimmed.find("::") else {
        return Ok((parse(trimmed)?, None));
    };

    let (base, rest) = trimmed.split_at(idx);
    let modifier = rest[2..].trim();
    if base.trim().is_empty() {
        return Err(format!(
            "`{selector}` has a `::` modifier but no selector before it"
        ));
    }
    if modifier.is_empty() {
        return Err(format!("`{selector}` has an empty `::` modifier"));
    }
    if !class_exists(modifier) {
        return Err(format!("`{modifier}` is not a class name this host knows"));
    }

    Ok((parse(base)?, Some(modifier.to_string())))
}

/// A NESTED `StyleRule`'s own `Selector`, merged with its own parent
/// `StyleRule`'s already-resolved selector. `"> TextButton"` nested inside
/// a rule whose own selector is `"#MenuFrame"` resolves as though it had
/// been written `"#MenuFrame > TextButton"` in one string (milestone 29
/// part C5).
///
/// ONLY THE LEADING-COMBINATOR FORM MERGES. A nested rule whose own
/// selector does not start with `>`/`>>` is parsed independently instead,
/// `parent` unused -- the one confirmed real example nests a leading
/// combinator, and inventing a wider "any nested rule is implicitly a
/// descendant of its parent" rule this project has not confirmed would be
/// a guess dressed as a feature.
pub fn parse_nested(selector: &str, parent: &Selector) -> Result<Selector, String> {
    let trimmed = selector.trim();
    if let Some(rest) = trimmed.strip_prefix(">>") {
        let right = parse(rest.trim())?;
        return Ok(Selector::Descendant(
            Box::new(parent.clone()),
            Box::new(right),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix('>') {
        let right = parse(rest.trim())?;
        return Ok(Selector::Child(Box::new(parent.clone()), Box::new(right)));
    }
    parse(trimmed)
}

/// Splits a leading `@Name ` query reference off `selector`, if there is
/// one, then hands the rest to [`parse_with_modifier`]. `"@ViewportDisplay
/// SizeSmall Frame"` -- milestone 29 part C3. `Name` is not resolved here:
/// a built-in name (`@ViewportDisplaySizeSmall/Medium/Large`,
/// `@PreferredInputKeyboardAndMouse`/`@PreferredInputTouch`,
/// `@ReducedMotionEnabledTrue/False`) or a custom `::StyleQuery #Name`
/// declaration are both just a string this far down -- `cascade.rs` is
/// where either kind is actually evaluated, the same separation
/// `parse_with_modifier` already draws between parsing a directive and
/// carrying it out.
pub fn parse_full(selector: &str) -> Result<(Selector, Option<String>, Option<String>), String> {
    let trimmed = selector.trim();
    let (query, rest) = match trimmed.strip_prefix('@') {
        Some(after_at) => match after_at.find(char::is_whitespace) {
            Some(idx) if !after_at[..idx].is_empty() => (
                Some(after_at[..idx].to_string()),
                after_at[idx..].trim_start(),
            ),
            _ => {
                return Err(format!(
                    "`{selector}` has an `@` query with no selector after it"
                ));
            }
        },
        None => (None, trimmed),
    };
    let (base, modifier) = parse_with_modifier(rest)?;
    Ok((base, query, modifier))
}

/// Does `id` match `selector`? Cascade resolution calls this once per
/// candidate `StyleRule` to decide whether it reaches a given instance at
/// all, before `Priority` ever enters into it.
pub fn matches(dom: &Dom, id: usize, selector: &Selector) -> bool {
    match selector {
        Selector::ClassName(name) => dom
            .class_of(id)
            .is_some_and(|class| members::class_is_a(&class, name)),
        Selector::Tag(tag) => dom.has_tag(id, tag),
        Selector::Name(name) => dom.name_of(id).as_deref() == Some(name.as_str()),
        // NO `GuiState` SET AT ALL DOES NOT MATCH `:Idle`. Real Roblox's own
        // default is `Idle` for every `GuiObject`, but this host never
        // writes one unless `SetGuiState` is called -- an instance nobody
        // has arbitrated yet is simply not in any state, not silently
        // `Idle`, the same reason an unset Attribute is absent rather than
        // a default token value.
        Selector::State(value) => {
            dom.property(id, "GuiState") == Some(Variant::Enum(rbx_types::Enum::from_u32(*value)))
        }
        // `id` ITSELF STILL HAS TO MATCH THE RIGHT SIDE. A `Child`/
        // `Descendant` selector narrows WHERE the right side is allowed to
        // match, it does not replace matching it in the first place --
        // `"A > B"` selects a `B` with an `A` parent, not an `A` with some
        // `B` descendant.
        Selector::Child(parent, child) => {
            matches(dom, id, child) && dom.parent_of(id).is_some_and(|p| matches(dom, p, parent))
        }
        Selector::Descendant(ancestor, descendant) => {
            matches(dom, id, descendant) && {
                let mut current = dom.parent_of(id);
                loop {
                    match current {
                        None => break false,
                        Some(p) => {
                            if matches(dom, p, ancestor) {
                                break true;
                            }
                            current = dom.parent_of(p);
                        }
                    }
                }
            }
        }
        Selector::List(list) => list.iter().any(|s| matches(dom, id, s)),
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
/// Unlike `matches`, nothing outside this file's own tests calls this yet:
/// cascade resolution walks outward from one instance rather than needing
/// every match under a root at once, so this stays a test-only convenience
/// until something asks the other direction of the same question.
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

    fn set_gui_state(dom: &SharedDom, id: usize, state: &str) {
        let item = enums::item_by_name("GuiState", state).expect("a real GuiState name");
        dom.lock()
            .expect("dom")
            .node_mut(id)
            .expect("id")
            .props
            .insert(
                "GuiState".to_string(),
                Variant::Enum(rbx_types::Enum::from_u32(item.value)),
            );
    }

    /// THE FIRST HALF OF PART B'S OWN COMPLETION TEST: a `:StateName`
    /// selector matches only the instance whose `GuiState` is that state.
    #[test]
    fn a_state_selector_matches_only_that_state() {
        let (dom, root, card, _label, other) = tree();
        set_gui_state(&dom, card, "Hover");
        set_gui_state(&dom, other, "Press");

        let guard = dom.lock().expect("dom");
        let hover = parse(":Hover").expect("parse");
        let press = parse(":Press").expect("parse");

        assert_eq!(matching_in(&guard, root, &hover), vec![card]);
        assert_eq!(matching_in(&guard, root, &press), vec![other]);
    }

    /// AN INSTANCE NOBODY HAS ARBITRATED YET MATCHES NOTHING, not `:Idle` by
    /// a real-engine-shaped default -- confirmed directly (`f.GuiState`
    /// reads `nil` on a fresh instance, not `Enum.GuiState.Idle`), not
    /// assumed from how the reflection database treats other properties.
    #[test]
    fn an_unset_gui_state_does_not_match_idle() {
        let (dom, root, _card, _label, _other) = tree();
        let guard = dom.lock().expect("dom");
        let idle = parse(":Idle").expect("parse");
        assert_eq!(matching_in(&guard, root, &idle), Vec::<usize>::new());
    }

    #[test]
    fn an_unknown_state_name_does_not_parse() {
        let err = parse(":NotARealState").unwrap_err();
        assert!(err.contains("NotARealState"), "{err}");
    }

    #[test]
    fn a_colon_with_nothing_after_it_does_not_parse() {
        assert!(parse(":").is_err());
    }

    #[test]
    fn a_compound_state_selector_is_refused_rather_than_misread() {
        // `"Frame:Hover"` is the real engine's own compound form and out of
        // this milestone's own part B scope (part C's own combinator work).
        let err = parse("Frame:Hover").unwrap_err();
        assert!(err.contains("compound"), "{err}");
    }

    // PART C1: COMBINATORS AND SELECTOR LISTS.

    /// `wrapper(Frame, tagged "Enemy") -> card(Frame) -> label(TextLabel)`,
    /// deep enough that a child combinator and a descendant combinator
    /// disagree about `label`: `card` sits between `wrapper` and `label`,
    /// so only the descendant form reaches through it.
    fn nested_tree() -> (SharedDom, usize, usize, usize, usize) {
        let dom = SharedDom::default();
        let (root, wrapper, card, label) = {
            let mut guard = dom.lock().expect("dom");
            let root = guard.insert("Frame".to_string(), "Frame".to_string());
            let wrapper = guard.insert("Frame".to_string(), "Frame".to_string());
            let card = guard.insert("Frame".to_string(), "Frame".to_string());
            let label = guard.insert("TextLabel".to_string(), "Label".to_string());
            guard.add_tag(wrapper, "Enemy");
            (root, wrapper, card, label)
        };
        parent(&dom, wrapper, root);
        parent(&dom, card, wrapper);
        parent(&dom, label, card);
        (dom, root, wrapper, card, label)
    }

    #[test]
    fn a_child_combinator_matches_only_the_immediate_parent() {
        let (dom, root, wrapper, _card, _label) = nested_tree();
        let guard = dom.lock().expect("dom");

        // `label`'s own parent is `card`, not `wrapper` -- the tagged
        // instance is one level too far up for `>` to reach.
        let too_far = parse(".Enemy > TextLabel").expect("parse");
        assert_eq!(matching_in(&guard, root, &too_far), Vec::<usize>::new());

        // `wrapper`'s own parent is `root`, a plain `Frame` -- `>` reaches
        // exactly that.
        let immediate = parse("Frame > Frame").expect("parse");
        let got = matching_in(&guard, root, &immediate);
        assert!(got.contains(&wrapper), "{got:?}");
    }

    #[test]
    fn a_descendant_combinator_reaches_through_an_intermediate_instance() {
        let (dom, root, _wrapper, _card, label) = nested_tree();
        let guard = dom.lock().expect("dom");

        // `>>` is not stopped by `card` sitting between `wrapper` and
        // `label`, unlike `>` in the sibling test above.
        let selector = parse(".Enemy >> TextLabel").expect("parse");
        assert_eq!(matching_in(&guard, root, &selector), vec![label]);
    }

    #[test]
    fn a_selector_list_matches_whatever_any_side_would() {
        let (dom, root, card, label, other) = tree();
        let guard = dom.lock().expect("dom");
        let selector = parse("TextLabel, TextButton").expect("parse");
        let got = matching_in(&guard, root, &selector);
        assert!(got.contains(&label), "{got:?}");
        assert!(got.contains(&other), "{got:?}");
        assert!(!got.contains(&card), "{got:?}");
    }

    #[test]
    fn a_selector_list_side_may_itself_be_a_combinator() {
        let (dom, root, wrapper, _card, label) = nested_tree();
        let guard = dom.lock().expect("dom");
        let selector = parse("GuiButton, .Enemy >> TextLabel").expect("parse");
        let got = matching_in(&guard, root, &selector);
        assert!(got.contains(&label), "{got:?}");
        assert!(!got.contains(&wrapper), "{got:?}");
    }

    #[test]
    fn a_trailing_comma_leaves_an_empty_selector_and_does_not_parse() {
        let err = parse("Frame,").unwrap_err();
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn a_chain_of_more_than_one_combinator_does_not_parse_yet() {
        // Only a single `>`/`>>` per selector is handled; a real chain is
        // wider scope than this sprint's own confirmed examples covered.
        let err = parse("Frame > Frame > TextLabel").unwrap_err();
        assert!(err.contains("compound"), "{err}");
    }

    #[test]
    fn whitespace_around_a_combinator_is_tolerated() {
        let (dom, root, _wrapper, _card, label) = nested_tree();
        let guard = dom.lock().expect("dom");
        let selector = parse("  .Enemy   >>   TextLabel  ").expect("parse");
        assert_eq!(matching_in(&guard, root, &selector), vec![label]);
    }

    // PART C2: `::Modifier`.

    #[test]
    fn a_modifier_suffix_splits_off_and_the_base_still_parses() {
        let (selector, modifier) = parse_with_modifier("Frame::UICorner").expect("parse");
        assert_eq!(selector, Selector::ClassName("Frame".to_string()));
        assert_eq!(modifier.as_deref(), Some("UICorner"));
    }

    #[test]
    fn a_selector_with_no_modifier_returns_none_for_it() {
        let (selector, modifier) = parse_with_modifier(".Enemy").expect("parse");
        assert_eq!(selector, Selector::Tag("Enemy".to_string()));
        assert_eq!(modifier, None);
    }

    #[test]
    fn a_modifier_naming_an_unknown_class_does_not_parse() {
        let err = parse_with_modifier("Frame::NotARealClass").unwrap_err();
        assert!(err.contains("NotARealClass"), "{err}");
    }

    #[test]
    fn a_modifier_with_nothing_before_it_does_not_parse() {
        let err = parse_with_modifier("::UICorner").unwrap_err();
        assert!(err.contains("no selector before"), "{err}");
    }

    #[test]
    fn a_modifier_with_nothing_after_it_does_not_parse() {
        let err = parse_with_modifier("Frame::").unwrap_err();
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn a_modifier_works_alongside_a_combinator_base() {
        let (selector, modifier) =
            parse_with_modifier(".Enemy >> TextLabel::UIStroke").expect("parse");
        assert!(matches!(selector, Selector::Descendant(_, _)));
        assert_eq!(modifier.as_deref(), Some("UIStroke"));
    }

    // PART C3: `StyleQuery`.

    #[test]
    fn a_query_prefix_splits_off_and_the_rest_still_parses() {
        let (selector, query, modifier) =
            parse_full("@ViewportDisplaySizeSmall Frame").expect("parse");
        assert_eq!(selector, Selector::ClassName("Frame".to_string()));
        assert_eq!(query.as_deref(), Some("ViewportDisplaySizeSmall"));
        assert_eq!(modifier, None);
    }

    #[test]
    fn a_selector_with_no_query_returns_none_for_it() {
        let (_, query, _) = parse_full("Frame").expect("parse");
        assert_eq!(query, None);
    }

    #[test]
    fn a_query_composes_with_a_modifier_on_the_same_rule() {
        let (selector, query, modifier) =
            parse_full("@ViewportDisplaySizeSmall Frame::UICorner").expect("parse");
        assert_eq!(selector, Selector::ClassName("Frame".to_string()));
        assert_eq!(query.as_deref(), Some("ViewportDisplaySizeSmall"));
        assert_eq!(modifier.as_deref(), Some("UICorner"));
    }

    #[test]
    fn a_query_with_nothing_after_it_does_not_parse() {
        let err = parse_full("@ViewportDisplaySizeSmall").unwrap_err();
        assert!(err.contains("no selector after"), "{err}");
    }

    #[test]
    fn a_bare_at_sign_does_not_parse() {
        assert!(parse_full("@").is_err());
        assert!(parse_full("@ Frame").is_err());
    }

    // PART C5: `StyleRule` NESTING.

    #[test]
    fn a_leading_child_combinator_merges_with_the_parent() {
        let parent = parse("#MenuFrame").expect("parse");
        let nested = parse_nested("> TextButton", &parent).expect("parse");
        let direct = parse("#MenuFrame > TextButton").expect("parse");
        assert_eq!(nested, direct);
    }

    #[test]
    fn a_leading_descendant_combinator_merges_with_the_parent() {
        let parent = parse("#MenuFrame").expect("parse");
        let nested = parse_nested(">> TextButton", &parent).expect("parse");
        let direct = parse("#MenuFrame >> TextButton").expect("parse");
        assert_eq!(nested, direct);
    }

    #[test]
    fn a_nested_selector_without_a_leading_combinator_ignores_the_parent() {
        let parent = parse("#MenuFrame").expect("parse");
        let nested = parse_nested("TextButton", &parent).expect("parse");
        assert_eq!(nested, Selector::ClassName("TextButton".to_string()));
    }
}
