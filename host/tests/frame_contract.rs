//! The display-list contract, checked against a real display list.
//!
//! `frame.rs` has said since it was written that this file asserts what
//! `Frame::CONTRACT_FIELDS` claims. IT DID NOT EXIST. The list was referenced by
//! nothing at all -- a constant whose whole purpose is to keep two things in
//! step, and nothing read it -- so the property it describes was never once
//! checked, and a field added in `Live.luau` and forgotten in `frame.rs` would
//! have been dropped in silence on the way to the painter, which is the exact
//! failure the list was written against.
//!
//! Milestone 2's sprint 4 added `image` to `Node`, which is the first change that
//! had to move that list. Wiring it up in the commit that makes it necessary is
//! cheaper than the alternative, which is a second comment about a test.
//!
//! WHAT IS CHECKED HERE, and it is the direction a unit test cannot reach:
//! Live.luau's real output, from a real application, driven for a real frame. The
//! other direction -- a `Node` field named in neither contract list -- is a
//! compile error in `frame.rs`'s own
//! `every_node_field_is_named_in_one_contract_or_the_other`, because a
//! destructuring pattern fails to build rather than failing to pass.

use dew_runtime::{Application, Capabilities, Frame};
use std::collections::BTreeSet;
use std::path::PathBuf;

mod common;

// THE FIXTURES STAYED IN `crates/runtime/tests`, with `render.rs`, which has not
// moved. See this suite's header: `render.rs` asserts on PIXELS and installing a
// DataModel changes which Aether host `detect()` selects, so moving it is not a
// relocation but a behaviour change. It is filed rather than forced.
fn aether_root() -> PathBuf {
    dew_runtime::installed_package("aether")
        .expect("no installed aether -- run `pesde install` at the repository root")
}

fn caps() -> Capabilities {
    let root = aether_root();
    let mut caps = Capabilities::cli(root.clone());
    caps.aliases.insert("aether".to_string(), root.join("src"));
    match dew_runtime::installed_package("vide") {
        Some(vide) => {
            caps.aliases.insert("vide".to_string(), vide.join("src"));
        }
        None => panic!("no installed vide -- run `pesde install` at the repository root"),
    }
    caps
}

/// Every key any node of a real snapshot carries.
///
/// THE UNION ACROSS NODES, not the keys of one. Live.luau emits nil rather than a
/// default for `fill`, `stroke`, `gradient` and `text`, so no single node carries
/// the whole vocabulary and asserting against the first one would check a handful
/// of fields and call it a contract.
fn keys_of_a_real_snapshot() -> BTreeSet<String> {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/app.luau");
    let app =
        Application::load_with(caps(), &fixture, common::install_host).expect("fixture loads");
    let session = app.session().expect("session");
    session.step(1.0 / 120.0).expect("step");
    let snapshot = session.snapshot_table().expect("snapshot");

    let nodes: mlua::Table = snapshot.get("Nodes").expect("Nodes");
    let mut keys = BTreeSet::new();
    for node in nodes.sequence_values::<mlua::Table>() {
        let node = node.expect("a node");
        for pair in node.pairs::<mlua::Value, mlua::Value>() {
            let (key, _) = pair.expect("a field");
            if let Some(name) = key.as_string() {
                keys.insert(name.to_string_lossy().to_string());
            }
        }
    }
    assert!(!keys.is_empty(), "the fixture drew no nodes to check");
    keys
}

#[test]
fn a_real_display_list_carries_no_key_the_contract_does_not_name() {
    // THE DIRECTION THAT MATTERS FOR AETHER. A field added to `buildNode` and not
    // to `frame.rs` is decoded by nobody and reaches no painter, and every test in
    // this crate would still pass -- the display simply paints less than the
    // engine does, which is what this whole module exists to prevent.
    let keys = keys_of_a_real_snapshot();
    let unknown: Vec<&String> = keys
        .iter()
        .filter(|k| !Frame::CONTRACT_FIELDS.contains(&k.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "Live.luau emits {unknown:?}, which `Frame::CONTRACT_FIELDS` does not name \
         — add the field to `frame.rs` and to that list, or the painter never sees it"
    );
}

#[test]
fn the_contract_names_what_a_real_display_list_actually_carries() {
    // AND THE OTHER WAY: a name in the list that no real snapshot has is a
    // contract with something that does not exist, which is how a list stops
    // meaning anything. `focused`, `dirty` and the rest live on the FRAME rather
    // than on a node and are correctly absent here; what is checked is that the
    // geometry and paint fields every node must carry are all present.
    let keys = keys_of_a_real_snapshot();
    for required in ["id", "name", "x", "y", "w", "h", "alpha", "radius"] {
        assert!(
            keys.contains(required),
            "no node in a real snapshot carried `{required}`, which the contract names"
        );
    }
}

#[test]
fn the_host_half_of_the_contract_is_not_something_lua_can_emit() {
    // `image` CARRIES DECODED PIXELS. There is no Lua form of it, which is why it
    // is in `HOST_FIELDS` rather than `CONTRACT_FIELDS`, and why the assertion
    // above is "no key outside the contract" rather than "exactly the contract".
    // Stated as a test so that a future reader who tries to reconcile the two
    // lists is told why they are two.
    let keys = keys_of_a_real_snapshot();
    for host_only in Frame::HOST_FIELDS {
        assert!(
            !keys.contains(*host_only),
            "`{host_only}` arrived from Live.luau — if a display list can carry it, \
             it belongs in CONTRACT_FIELDS and `Node::from_lua` must decode it"
        );
        assert!(
            !Frame::CONTRACT_FIELDS.contains(host_only),
            "`{host_only}` is named in both contract lists"
        );
    }
}
