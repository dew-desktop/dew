//! Tests for `bundled.rs`. Every write goes to a scratch directory under the
//! system temp folder, never to the user's own `Bundled` folder.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// This checkout's folder for `id`, a sibling of `host/`.
fn checkout_dir(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("host/ has a parent")
        .join(id)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dew-bundled-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A folder's directories and files, by path relative to its root with `/`
/// between components.
#[derive(Default)]
struct Tree {
    dirs: BTreeSet<String>,
    files: BTreeMap<String, Vec<u8>>,
}

/// Read `root` as a copy of it is meant to hold it: directories recursed
/// into, regular files read, links and anything else skipped. Written apart
/// from `host/build.rs`'s walk so the two can disagree.
fn read_tree(root: &Path) -> Tree {
    let mut tree = Tree::default();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read_dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name().into_string().expect("UTF-8 name");
            let child = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            let file_type = entry.file_type().expect("file type");
            if file_type.is_dir() {
                tree.dirs.insert(child.clone());
                stack.push((entry.path(), child));
            } else if file_type.is_file() {
                tree.files
                    .insert(child, std::fs::read(entry.path()).expect("read"));
            }
        }
    }
    tree
}

fn embedded_tree(id: &str) -> Tree {
    let applet = EMBEDDED
        .iter()
        .find(|a| a.id == id)
        .unwrap_or_else(|| panic!("{id} is not embedded"));
    Tree {
        dirs: applet.dirs.iter().map(|d| d.to_string()).collect(),
        files: applet
            .files
            .iter()
            .map(|(rel, bytes)| (rel.to_string(), bytes.to_vec()))
            .collect(),
    }
}

/// Fail naming only the paths that differ, rather than every path.
fn assert_same_tree(label: &str, actual: &Tree, expected: &Tree) {
    let extra_dirs: Vec<_> = actual.dirs.difference(&expected.dirs).collect();
    let missing_dirs: Vec<_> = expected.dirs.difference(&actual.dirs).collect();
    assert!(
        extra_dirs.is_empty() && missing_dirs.is_empty(),
        "{label}: directories only in the copy {extra_dirs:?}, only in the source {missing_dirs:?}"
    );
    let extra: Vec<_> = actual
        .files
        .keys()
        .filter(|k| !expected.files.contains_key(*k))
        .collect();
    let missing: Vec<_> = expected
        .files
        .keys()
        .filter(|k| !actual.files.contains_key(*k))
        .collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "{label}: files only in the copy {extra:?}, only in the source {missing:?}"
    );
    for (rel, bytes) in &expected.files {
        assert!(actual.files[rel] == *bytes, "{label}: {rel} differs");
    }
}

/// The copy `bundled.rs` made from the checkout before the applets were
/// embedded: regular files copied, directories recursed into, links and
/// anything else skipped. Kept to prove the embedded copy writes the same tree.
fn copy_from_checkout(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create dst");
    for entry in std::fs::read_dir(src).expect("read_dir") {
        let entry = entry.expect("entry");
        let file_type = entry.file_type().expect("file type");
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_from_checkout(&entry.path(), &target);
        } else if file_type.is_file() {
            std::fs::write(&target, std::fs::read(entry.path()).expect("read")).expect("write");
        }
    }
}

#[test]
fn every_bundled_applet_is_embedded_and_nothing_else() {
    let embedded: Vec<_> = EMBEDDED.iter().map(|a| a.id).collect();
    let bundled: Vec<_> = APPLETS.iter().map(|a| a.id).collect();
    assert_eq!(embedded, bundled);
}

/// THE BINARY CARRIES THE CHECKOUT'S APPLETS, file for file and byte for
/// byte, `roblox_packages/` included.
#[test]
fn the_embedded_applets_match_the_checkout() {
    for applet in APPLETS {
        let source = checkout_dir(applet.id);
        let expected = read_tree(&source);
        assert!(expected.files.contains_key("dew.toml"), "{}", applet.id);
        assert_same_tree(applet.id, &embedded_tree(applet.id), &expected);
    }
    let dashboard = embedded_tree("dashboard");
    for package in ["roblox_packages/aether.luau", "roblox_packages/vide.luau"] {
        assert!(
            dashboard.files.contains_key(package),
            "the dashboard must carry {package}"
        );
    }
}

/// A FIRST SYNC WRITES THE TREE THE CHECKOUT COPY WROTE, so a build that
/// embeds its applets leaves the same `Bundled` folder as one that copied
/// them.
#[test]
fn ensure_into_an_empty_folder_writes_what_the_checkout_copy_wrote() {
    let root = scratch("parity");
    for applet in APPLETS {
        let embedded = root.join("embedded").join(applet.id);
        let copied = root.join("copied").join(applet.id);
        ensure_into(applet.id, &embedded).expect("ensure_into");
        copy_from_checkout(&checkout_dir(applet.id), &copied);

        let written = read_tree(&embedded);
        assert_same_tree(applet.id, &written, &read_tree(&copied));
        assert_same_tree(applet.id, &written, &embedded_tree(applet.id));
        let bytes: usize = written.files.values().map(Vec::len).sum();
        eprintln!(
            "{}: {} files, {} directories, {bytes} bytes, identical to the checkout copy",
            applet.id,
            written.files.len(),
            written.dirs.len()
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A stale copy is brought back: a modified file is rewritten, a file and a
/// folder the build does not ship are removed, and a file whose bytes match
/// is left untouched, which its unchanged modification time proves.
#[test]
fn ensure_into_restores_a_stale_copy_and_leaves_a_current_file_alone() {
    let root = scratch("stale");
    let dest = root.join("quickpanel");
    ensure_into("quickpanel", &dest).expect("first sync");

    let entry = dest.join("quickpanel.luau");
    let shipped = std::fs::read(&entry).expect("entry");
    // Same length, different bytes: a size check alone would keep it.
    let mut edited = shipped.clone();
    edited[0] = if edited[0] == b'x' { b'y' } else { b'x' };
    std::fs::write(&entry, &edited).expect("edit");
    std::fs::write(dest.join("stale.luau"), "return 1").expect("stale file");
    std::fs::create_dir_all(dest.join("old_folder")).expect("stale folder");
    std::fs::write(dest.join("old_folder").join("x.luau"), "").expect("stale nested");

    let manifest = std::fs::File::options()
        .write(true)
        .open(dest.join("dew.toml"))
        .expect("dew.toml");
    let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    manifest.set_modified(old).expect("set mtime");
    drop(manifest);

    ensure_into("quickpanel", &dest).expect("second sync");

    assert_eq!(std::fs::read(&entry).expect("entry"), shipped);
    assert!(!dest.join("stale.luau").exists(), "a stray file must go");
    assert!(!dest.join("old_folder").exists(), "a stray folder must go");
    assert_eq!(
        std::fs::metadata(dest.join("dew.toml"))
            .expect("dew.toml")
            .modified()
            .expect("mtime"),
        old,
        "a file whose bytes already match must not be rewritten"
    );
    assert_same_tree(
        "quickpanel",
        &read_tree(&dest),
        &embedded_tree("quickpanel"),
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn ensure_refuses_an_id_dew_does_not_ship() {
    for id in ["some-installed-applet", "..", ""] {
        let err = ensure(id).expect_err("only Dew's own ids may be bundled");
        assert!(err.contains("not an applet Dew ships"), "got: {err}");
    }

    let root = scratch("refuse");
    let dest = root.join("target");
    let err = ensure_into("..", &dest).expect_err("ensure_into refuses too");
    assert!(err.contains("not an applet Dew ships"), "got: {err}");
    assert!(!dest.exists(), "a refused id writes nothing");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_dashboard_is_bundled_but_not_listed_and_the_quick_panel_is_listed() {
    assert!(is_bundled_id("dashboard") && is_bundled_id("quickpanel"));
    let listed: Vec<_> = listed_ids().collect();
    assert_eq!(listed, vec!["quickpanel"]);
}
