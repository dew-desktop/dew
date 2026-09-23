//! Disk-backed record of which applets are INSTALLED, as opposed to which
//! are currently RUNNING (`coordinator`'s in-memory `registry`, tracked by a
//! transient `u32` for as long as the process lives). An applet can be
//! installed and not running -- the coordinator has not started yet, or it
//! is disabled -- or running without being installed at all, a bare
//! `dew run` against a directory nobody installed. This module only ever
//! answers the first question.
//!
//! ONE DIRECTORY PER APPLET, under `Applets/`, sibling to `positions.json`
//! and named by the applet's OWN manifest id rather than by whatever its
//! source directory was called -- a folder can be renamed independently of
//! the manifest inside it, and the two disagreeing is a refusal here, not a
//! silent pick of one. A sibling `installed.json` persists ONLY the
//! enabled/disabled bit per id; everything else about an installed applet is
//! read by scanning its actual directory, so the store and the manifest
//! inside it can never disagree about WHAT is installed, only about whether
//! it is enabled.

#![cfg(windows)]

use crate::manifest::Manifest;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Directory names a copied applet does not carry with it: pesde's install
/// output and cache. Named the same way `examples/*/.gitignore` already
/// excludes them, rather than a second list that can drift from it -- see
/// the root `.gitignore`'s `roblox_packages/`, `luau_packages/` and
/// `.pesde/` entries.
pub(crate) const SKIP_DIRS: &[&str] = &["roblox_packages", "luau_packages", ".pesde"];

pub(crate) fn dew_dir() -> Option<PathBuf> {
    let dir = dirs::data_local_dir()?.join("Dew");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn applets_dir() -> Option<PathBuf> {
    let dir = dew_dir()?.join("Applets");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn state_path() -> Option<PathBuf> {
    Some(dew_dir()?.join("installed.json"))
}

fn read_state() -> HashMap<String, bool> {
    let Some(path) = state_path() else {
        return HashMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn write_state(state: &HashMap<String, bool>) {
    let Some(path) = state_path() else { return };
    if let Ok(text) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&path, text);
    }
}

/// A directory name that is safe to use as-is: no path separator, and not a
/// way of naming a different directory (`.`, `..`) or nothing at all.
///
/// An applet's id comes from its OWN manifest, an author-controlled string,
/// and it becomes a directory name directly -- so it is checked here rather
/// than trusted, the same way a permission or a runtime is a closed set
/// rather than whatever text showed up in `dew.toml`.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains(':')
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", src.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        let name = entry.file_name();

        if file_type.is_dir() {
            if SKIP_DIRS.contains(&name.to_string_lossy().as_ref()) {
                continue;
            }
            copy_dir(&entry.path(), &dst.join(&name))?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), dst.join(&name))
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        }
        // A symlink or anything else exotic inside an applet directory is
        // not a shape any example or doc here produces; skipped rather than
        // followed, since following one could copy something well outside
        // the applet's own directory.
    }
    Ok(())
}

/// One applet the store currently knows about.
pub struct Entry {
    pub id: String,
    pub dir: PathBuf,
    pub enabled: bool,
}

/// Every installed applet, enabled or not. `id` and `dir` come from scanning
/// `Applets/` itself; `enabled` comes from `installed.json`, defaulting to
/// `true` for a directory the state file has no opinion about -- a store
/// that only ever writes an entry at install time still has to make sense of
/// a directory placed there by hand.
pub fn list() -> Vec<Entry> {
    let Some(dir) = applets_dir() else {
        return Vec::new();
    };
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let state = read_state();
    let mut out = Vec::new();
    for entry in read.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let id = entry.file_name().to_string_lossy().into_owned();
            let enabled = state.get(&id).copied().unwrap_or(true);
            out.push(Entry {
                dir: entry.path(),
                id,
                enabled,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// The id and directory of every installed applet whose enabled bit is set.
/// What `coordinator::run` auto-loads at startup.
pub fn enabled() -> Vec<(String, PathBuf)> {
    list()
        .into_iter()
        .filter(|e| e.enabled)
        .map(|e| (e.id, e.dir))
        .collect()
}

/// Copy `source` into the store under its own manifest id, and mark it
/// enabled. Refuses if that id is already installed unless `force` is set,
/// in which case the existing copy is replaced.
pub fn install(source: &Path, force: bool) -> Result<String, String> {
    let manifest = Manifest::load(source)?;
    let id = manifest.id;
    if !valid_id(&id) {
        return Err(format!(
            "{}: manifest id {id:?} cannot be used as a directory name",
            source.display()
        ));
    }

    let dest = applets_dir()
        .ok_or("could not find a per-user data directory to install into")?
        .join(&id);

    if dest.exists() {
        if force {
            std::fs::remove_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        } else {
            return Err(format!(
                "'{id}' is already installed; run `dew uninstall {id}` first, or pass --force"
            ));
        }
    }

    copy_dir(source, &dest)?;

    let mut state = read_state();
    state.insert(id.clone(), true);
    write_state(&state);

    Ok(id)
}

/// Set `id`'s enabled bit on disk. Whether `id` is currently running is not
/// this function's question, the same way `uninstall` leaves it to the
/// caller -- the management window decides separately whether the change
/// means live-loading or live-unloading it.
pub fn set_enabled(id: &str, enabled: bool) -> Result<(), String> {
    let dest = applets_dir()
        .ok_or("could not find a per-user data directory")?
        .join(id);
    if !dest.is_dir() {
        return Err(format!("no applet installed with id '{id}'"));
    }

    let mut state = read_state();
    state.insert(id.to_string(), enabled);
    write_state(&state);
    Ok(())
}

/// Remove `id` from the store and forget its enabled bit. Whether `id` is
/// currently running in an active coordinator is not this function's
/// question -- callers that care check `coordinator::query_running` first.
pub fn uninstall(id: &str) -> Result<(), String> {
    let dest = applets_dir()
        .ok_or("could not find a per-user data directory to uninstall from")?
        .join(id);

    if !dest.is_dir() {
        return Err(format!("no applet installed with id '{id}'"));
    }

    std::fs::remove_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;

    let mut state = read_state();
    state.remove(id);
    write_state(&state);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_with_a_path_separator_are_refused() {
        assert!(!valid_id("a/b"));
        assert!(!valid_id("a\\b"));
        assert!(!valid_id(".."));
        assert!(!valid_id("."));
        assert!(!valid_id(""));
        assert!(valid_id("basic-widget"));
    }
}
