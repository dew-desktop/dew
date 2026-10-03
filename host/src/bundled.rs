//! The applets Dew ships itself, and the one writer of the bundled
//! directory `applets::load` trusts with a `Capability::Host` permission.
//!
//! THE SET IS FIXED HERE, NOT READ FROM ANY MANIFEST. Every destination this
//! module writes is `Bundled/<id>` for an id in [`APPLETS`], and every file
//! it writes there is one built into this binary. Nothing a user, an
//! installed applet or the marketplace supplies can choose a destination or
//! a file, which is what keeps "loaded from Bundled" meaning "shipped with
//! this build of Dew".
//!
//! BUILT INTO THE BINARY. `host/build.rs` walks each applet's folder,
//! `roblox_packages/` included, and generates a table of its files as
//! `include_bytes!`, so a `dew.exe` with nothing beside it still has its
//! dashboard and quick panel. An applet loads from a directory, so the
//! table is written to `<local data>/Dew/Bundled/<id>` and loaded from
//! there, the way `fonts.rs` writes the faces it ships.
//!
//! TWO KINDS OF BUNDLED APPLET. The dashboard is opened from the tray and is
//! never listed: it is the surface the list is shown in. The quick panel is
//! listed in `dew.Library` like an installed applet, enabled and disabled
//! the same way, and has no tray entry of its own. It cannot be uninstalled,
//! because the next sync would put it straight back.

#![cfg(windows)]

use std::collections::HashSet;
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

/// One bundled applet's files as `host/build.rs` embedded them. Paths are
/// relative to the applet's folder, with `/` between components.
struct EmbeddedApplet {
    id: &'static str,
    /// Every directory, each listed after its parent.
    dirs: &'static [&'static str],
    files: &'static [(&'static str, &'static [u8])],
}

// Defines `EMBEDDED: &[EmbeddedApplet]`, one entry per applet folder.
include!(concat!(env!("OUT_DIR"), "/bundled_applets.rs"));

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

/// Where `id`'s bundled copy lives, whether or not it has been written yet.
pub fn installed_dir(id: &str) -> Option<PathBuf> {
    Some(crate::installed::bundled_applets_dir()?.join(id))
}

fn not_shipped(id: &str) -> String {
    format!("'{id}' is not an applet Dew ships")
}

/// Bring `id`'s bundled copy up to date with the files this build ships, and
/// return its directory.
///
/// Refuses an id not in [`APPLETS`], so no caller can make this write a
/// folder of its choosing into the bundled directory.
pub fn ensure(id: &str) -> Result<PathBuf, String> {
    if !is_bundled_id(id) {
        return Err(not_shipped(id));
    }
    let dest = installed_dir(id)
        .ok_or("could not find a per-user data directory to bundle applets into")?;
    ensure_into(id, &dest)?;
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

/// Make `dest` hold exactly the files this build embeds for `id`.
///
/// THE BYTES, NOT THE TIMESTAMP OR THE SIZE. A file whose bytes already
/// match is left alone; any other is written to a temporary name and renamed
/// over it, so a reader never sees half a file. A file or folder in `dest`
/// that the table does not have is removed, since a leftover module would
/// still be `require`-able from the copy.
pub(crate) fn ensure_into(id: &str, dest: &Path) -> Result<(), String> {
    let applet = EMBEDDED
        .iter()
        .find(|a| a.id == id && is_bundled_id(id))
        .ok_or_else(|| not_shipped(id))?;
    let fail = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());

    std::fs::create_dir_all(dest).map_err(|e| fail(dest, e))?;
    for rel in applet.dirs {
        let target = dest.join(rel);
        clear_unless(&target, Kind::Dir).map_err(|e| fail(&target, e))?;
        std::fs::create_dir_all(&target).map_err(|e| fail(&target, e))?;
    }
    for (rel, bytes) in applet.files {
        let target = dest.join(rel);
        clear_unless(&target, Kind::File).map_err(|e| fail(&target, e))?;
        if holds(&target, bytes) {
            continue;
        }
        let name = target.file_name().unwrap_or_default().to_string_lossy();
        let temp = target.with_file_name(format!("{name}.{}.tmp", std::process::id()));
        std::fs::write(&temp, bytes).map_err(|e| fail(&temp, e))?;
        if let Err(e) = std::fs::rename(&temp, &target) {
            let _ = std::fs::remove_file(&temp);
            // Another process syncing the same build may have won the race.
            if !holds(&target, bytes) {
                return Err(fail(&target, e));
            }
        }
    }

    let wanted: HashSet<&str> = applet
        .dirs
        .iter()
        .copied()
        .chain(applet.files.iter().map(|(rel, _)| *rel))
        .collect();
    prune(dest, "", &wanted)
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Dir,
    File,
}

/// Remove whatever is at `path` unless it is already a `keep`. A link is
/// never kept, so nothing written afterwards lands outside the copy.
fn clear_unless(path: &Path, keep: Kind) -> std::io::Result<()> {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Ok(());
    };
    if meta.is_dir() {
        return match keep {
            Kind::Dir => Ok(()),
            Kind::File => std::fs::remove_dir_all(path),
        };
    }
    if meta.is_file() && keep == Kind::File {
        return Ok(());
    }
    std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
}

/// Does the file at `path` already hold exactly `bytes`?
fn holds(path: &Path, bytes: &[u8]) -> bool {
    std::fs::metadata(path).map(|m| m.len()).ok() == Some(bytes.len() as u64)
        && std::fs::read(path).is_ok_and(|current| current == bytes)
}

/// Remove every entry under `dir` whose path relative to the copy's root is
/// not in `wanted`. `rel` is `dir`'s own relative path.
fn prune(dir: &Path, rel: &str, wanted: &HashSet<&str>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = entry.path();
        let child = entry.file_name().to_str().map(|name| {
            if rel.is_empty() {
                name.to_string()
            } else {
                format!("{rel}/{name}")
            }
        });
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if child.as_deref().is_some_and(|c| wanted.contains(c)) {
            if is_dir {
                prune(&path, child.as_deref().unwrap_or_default(), wanted)?;
            }
            continue;
        }
        let removed = if is_dir {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path).or_else(|_| std::fs::remove_dir(&path))
        };
        removed.map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

// The tests compare the embedded table with this checkout's applet folders,
// so they sit in a file of their own and this one names no source folder.
#[cfg(test)]
#[path = "bundled_tests.rs"]
mod tests;
