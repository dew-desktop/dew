//! What "the framework surface" means, in one place.
//!
//! THE DENOMINATOR IS ASKED FOR, NOT WRITTEN DOWN. `datamodel_scope` learned this
//! the hard way: a hand-listed surface drifts from the real one silently, and the
//! drift always runs in the flattering direction. The symbols below come from
//! requiring Aether's `api` and reading the table's keys, so a symbol added
//! upstream appears here without anybody editing this file.
//!
//! WHAT THIS FILE DOES OWN is the classification. Not every export is a feature,
//! and counting the ones that are not would inflate a number this milestone
//! exists to move honestly.

/// Exports that are namespaces rather than features.
///
/// Each is a table of other exports -- `Aether.Primitives.Combobox` is the same
/// function as `Aether.Combobox` -- so demonstrating one demonstrates nothing on
/// its own, and requiring a demo to "use" it would mean touching a table to
/// satisfy a counter.
///
/// A REASON PER ENTRY, because an exclusion without one is indistinguishable
/// from a number somebody found inconvenient.
pub const NAMESPACES: &[(&str, &str)] = &[
    (
        "Controllers",
        "a table of the controllers, each exported at the top level too",
    ),
    (
        "Core",
        "a table of the Layer 1 modules, each exported at the top level too",
    ),
    (
        "Engine",
        "a grouping of the behaviour modules, no member of its own",
    ),
    (
        "Primitives",
        "a table of the primitives, each exported at the top level too",
    ),
    (
        "Assemblies",
        "a grouping of composed primitives, each exported at the top level too",
    ),
    (
        "Runtime",
        "a grouping of the reactive primitives, each exported at the top level too",
    ),
    (
        "Host",
        "the resolved host record, which the host layer owns rather than a demo",
    ),
    (
        "Desktop",
        "the ceremony a host performs, invoked by the host and never by an application",
    ),
];

/// Is this export a namespace rather than a feature?
pub fn is_namespace(symbol: &str) -> bool {
    NAMESPACES.iter().any(|(name, _)| *name == symbol)
}

/// Why a symbol is excluded, for the report.
pub fn exclusion_reason(symbol: &str) -> Option<&'static str> {
    NAMESPACES
        .iter()
        .find(|(name, _)| *name == symbol)
        .map(|(_, why)| *why)
}

/// The in-scope surface: everything exported that is not a namespace.
pub fn in_scope(exported: &[String]) -> Vec<String> {
    let mut out: Vec<String> = exported
        .iter()
        .filter(|s| !is_namespace(s))
        .cloned()
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PINNED, so the classification cannot drift without somebody saying so.
    ///
    /// Not a claim that eight is the right number forever -- a claim that
    /// changing it is a decision rather than a side effect. `datamodel_scope`
    /// pins its denominator the same way and for the same reason.
    #[test]
    fn eight_exports_are_namespaces_and_each_says_why() {
        assert_eq!(
            NAMESPACES.len(),
            8,
            "the namespace list changed; update the count deliberately"
        );
        for (name, why) in NAMESPACES {
            assert!(
                why.len() > 20,
                "`{name}` is excluded without a reason worth reading"
            );
        }
    }

    #[test]
    fn a_namespace_is_out_of_scope_and_a_feature_is_in() {
        let exported = vec![
            "Combobox".to_string(),
            "Primitives".to_string(),
            "source".to_string(),
        ];
        let scope = in_scope(&exported);
        assert_eq!(scope, vec!["Combobox".to_string(), "source".to_string()]);
        assert!(exclusion_reason("Primitives").is_some());
        assert!(exclusion_reason("Combobox").is_none());
    }
}
