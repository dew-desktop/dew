//! The applets Dew ships itself, and the one writer of the bundled
//! directory `applets::load` trusts with a `Capability::Host` permission.
//!
//! THE SET IS FIXED HERE, NOT READ FROM ANY MANIFEST. Every destination this
//! module writes is `Bundled/<id>` for an id in [`APPLETS`], and every source
//! is that id's folder in this checkout. Nothing a user, an installed applet
//! or the marketplace supplies can choose a destination, which is what keeps
//! "loaded from Bundled" meaning "shipped with this build of Dew".
//!
//! COPIED FROM THE CHECKOUT, found through `CARGO_MANIFEST_DIR` at compile
//! time. There is no installer yet to place these folders next to `dew.exe`,
//! so the checkout stands in for one. The day an installer exists,
//! [`source_dir`] is the function that changes.
//!
//! TWO KINDS OF BUNDLED APPLET. The dashboard is opened from the tray and is
//! never listed: it is the surface the list is shown in. The quick panel is
//! listed in `dew.Library` like an installed applet, enabled and disabled
//! the same way, and has no tray entry of its own. It cannot be uninstalled,
//! because the next sync would put it straight back.

#![cfg(windows)]

use std::path::{Path, PathBuf};

pub struct BundledApplet {
    pub id: &'static str,
    /// Shown in `dew.Library.List` and toggled there like an installed applet.
    pub listed: bool,
}

pub const APPLETS: &[BundledApplet] = &[
    BundledApplet {
        id: "dashboard",
        listed: false,
    },
    BundledApplet {
        id: "quickpanel",
        listed: true,
    },
];

/// Is `id` one of Dew's own applets? Such an id cannot be installed over,
/// uninstalled, or shadowed by a folder under `Applets/`.
pub fn is_bundled_id(id: &str) -> bool {
    APPLETS.iter().any(|a| a.id == id)
}

/// The ids `dew.Library` lists alongside the installed applets.
pub fn listed_ids() -> impl Iterator<Item = &'static str> {
    APPLETS.iter().filter(|a| a.listed).map(|a| a.id)
}

/// The refusal `Uninstall` gives for a bundled applet.
pub fn uninstall_refusal(id: &str) -> String {
    format!("'{id}' ships with Dew and cannot be uninstalled; disable it instead")
}

/// This checkout's own folder for `id`, a sibling of `host/`.
fn source_dir(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("host/ has a parent")
        .join(id)
}

/// Where `id`'s bundled copy lives, whether or not it has been written yet.
pub fn installed_dir(id: &str) -> Option<PathBuf> {
    Some(crate::installed::bundled_applets_dir()?.join(id))
}

/// Bring `id`'s bundled copy up to date with the files this build ships, and
/// return its directory.
///
/// Refuses an id not in [`APPLETS`], so no caller can make this write a
/// folder of its choosing into the bundled directory.
pub fn ensure(id: &str) -> Result<PathBuf, String> {
    if !is_bundled_id(id) {
        return Err(format!("'{id}' is not an applet Dew ships"));
    }
    let dest = installed_dir(id)
        .ok_or("could not find a per-user data directory to bundle applets into")?;
    let source = source_dir(id);
    if !source.join("dew.toml").is_file() {
        return Err(format!(
            "{}: no bundled applet source here",
            source.display()
        ));
    }
    sync_dir(&source, &dest)?;
    Ok(dest)
}

/// Sync every listed bundled applet. Called once at service start, before
/// the enabled applets load, so the list and the copies it points at belong
/// to the build that is running.
pub fn ensure_listed() {
    for id in listed_ids() {
        if let Err(e) = ensure(id) {
            eprintln!("[dew] bundled {id}: {e}");
        }
    }
}

/// Make `dst` hold exactly the files under `src`.
///
/// THE BYTES, NOT THE TIMESTAMP OR THE SIZE. A file whose bytes already
/// match is left alone; any other is rewritten. A file or folder in `dst`
/// that `src` no longer has is removed, since a leftover module would still
/// be `require`-able from the copy.
///
/// Symlinks and other exotic entries in `src` are skipped rather than
/// followed, as `installed::copy_dir` does, so a link cannot pull a file
/// from outside the applet's folder into the trusted directory.
pub fn sync_dir(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("{}: {e}", dst.display()))?;

    let mut wanted = std::collections::HashSet::new();
    for entry in std::fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", src.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        let name = entry.file_name();
        let target = dst.join(&name);

        if file_type.is_dir() {
            if target.is_file() {
                std::fs::remove_file(&target).map_err(|e| format!("{}: {e}", target.display()))?;
            }
            sync_dir(&entry.path(), &target)?;
        } else if file_type.is_file() {
            let bytes = std::fs::read(entry.path())
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
            if target.is_dir() {
                std::fs::remove_dir_all(&target)
                    .map_err(|e| format!("{}: {e}", target.display()))?;
            }
            let same = std::fs::metadata(&target).map(|m| m.len()).ok() == Some(bytes.len() as u64)
                && std::fs::read(&target).is_ok_and(|current| current == bytes);
            if !same {
                std::fs::write(&target, &bytes)
                    .map_err(|e| format!("{}: {e}", target.display()))?;
            }
        } else {
            continue;
        }
        wanted.insert(name);
    }

    for entry in std::fs::read_dir(dst).map_err(|e| format!("{}: {e}", dst.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", dst.display()))?;
        if wanted.contains(&entry.file_name()) {
            continue;
        }
        let path = entry.path();
        let removed = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        removed.map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dew-bundled-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A first sync copies every file and folder.
    #[test]
    fn sync_copies_a_tree_into_an_empty_destination() {
        let root = scratch("copy");
        let src = root.join("src");
        let dst = root.join("dst");
        std::fs::create_dir_all(src.join("roblox_packages")).unwrap();
        std::fs::write(src.join("dew.toml"), "id = \"x\"\n").unwrap();
        std::fs::write(src.join("roblox_packages").join("vide.luau"), "return {}").unwrap();

        sync_dir(&src, &dst).expect("sync");

        assert_eq!(
            std::fs::read_to_string(dst.join("dew.toml")).unwrap(),
            "id = \"x\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(dst.join("roblox_packages").join("vide.luau")).unwrap(),
            "return {}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A stale copy is replaced: a file with different bytes is rewritten,
    /// a file the source dropped is removed, and a file whose bytes match is
    /// left untouched, which its unchanged modification time proves.
    #[test]
    fn sync_replaces_a_stale_copy_and_leaves_a_current_file_alone() {
        let root = scratch("stale");
        let src = root.join("src");
        let dst = root.join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(dst.join("old_folder")).unwrap();
        std::fs::write(src.join("main.luau"), "return 2").unwrap();
        std::fs::write(src.join("same.luau"), "unchanged").unwrap();
        // Same length as the source, different bytes: a size check alone
        // would keep it.
        std::fs::write(dst.join("main.luau"), "return 1").unwrap();
        std::fs::write(dst.join("same.luau"), "unchanged").unwrap();
        std::fs::write(dst.join("removed.luau"), "gone from the source").unwrap();
        std::fs::write(dst.join("old_folder").join("x.luau"), "").unwrap();

        let same = std::fs::File::options()
            .write(true)
            .open(dst.join("same.luau"))
            .unwrap();
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        same.set_modified(old).unwrap();
        drop(same);

        sync_dir(&src, &dst).expect("sync");

        assert_eq!(
            std::fs::read_to_string(dst.join("main.luau")).unwrap(),
            "return 2"
        );
        assert!(
            !dst.join("removed.luau").exists(),
            "a file the source dropped must go"
        );
        assert!(
            !dst.join("old_folder").exists(),
            "a folder the source dropped must go"
        );
        assert_eq!(
            std::fs::metadata(dst.join("same.luau"))
                .unwrap()
                .modified()
                .unwrap(),
            old,
            "a file whose bytes already match must not be rewritten"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ensure_refuses_an_id_dew_does_not_ship() {
        let err = ensure("some-installed-applet").expect_err("only Dew's own ids may be bundled");
        assert!(err.contains("not an applet Dew ships"), "got: {err}");
        let err = ensure("..").expect_err("a path-shaped id must be refused");
        assert!(err.contains("not an applet Dew ships"), "got: {err}");
    }

    #[test]
    fn the_dashboard_is_bundled_but_not_listed_and_the_quick_panel_is_listed() {
        assert!(is_bundled_id("dashboard") && is_bundled_id("quickpanel"));
        let listed: Vec<_> = listed_ids().collect();
        assert_eq!(listed, vec!["quickpanel"]);
    }
}
