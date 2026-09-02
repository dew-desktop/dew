//! `mod.json` — what a mod says it is, before any of it runs.
//!
//! Read from disk and checked BEFORE the mod's own code is loaded, which is the
//! whole reason the mod contract is a returned declaration rather than a
//! registration call. A manifest that had to be discovered by running the mod
//! would be a manifest that arrives after the mod has already acted.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A capability a mod may ask for.
///
/// A CLOSED SET, not free-form strings. An unknown permission in a manifest is
/// refused rather than ignored: silently dropping one means a mod that asks for
/// `filesystem` on a host that has never heard of it runs anyway, with the
/// author believing it was granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Storage,
    Audio,
    Notifications,
    Clipboard,
}

impl Permission {
    pub fn name(self) -> &'static str {
        match self {
            Permission::Storage => "storage",
            Permission::Audio => "audio",
            Permission::Notifications => "notifications",
            Permission::Clipboard => "clipboard",
        }
    }
}

/// PARSED AND NOT ALL USED. Only `id` and `permissions` are read today; `name`,
/// `description` and `hotkeys` are accepted, validated as strings, and dropped.
///
/// They stay because they are the manifest CONTRACT rather than leftovers -- a mod
/// author writing one is describing something Dew intends to honour, and deleting
/// the fields would make the format quietly narrower without deciding anything.
/// `hotkeys` in particular is not decoration: `mods/timetracker` binds three of
/// them, and today the host reads none, so a mod can ship a keybinding that has
/// never once fired.
///
/// The allow is scoped to this struct on purpose. A crate-level one would also
/// hide the next field that stops being read for a worse reason.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,

    /// What this mod may reach. Absent means NOTHING, which is the correct
    /// default and the one a mod author is least likely to have intended by
    /// accident — an empty list produces a mod that renders and cannot touch the
    /// machine, and that failure is loud and local.
    #[serde(default)]
    pub permissions: Vec<Permission>,

    #[serde(default)]
    pub hotkeys: BTreeMap<String, String>,
}

impl Manifest {
    pub fn load(dir: &Path) -> Result<Manifest, String> {
        let path = dir.join("mod.json");
        let raw = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        // A BOM survives `read_to_string` and serde_json rejects it as a value.
        let raw = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
        serde_json::from_str(raw).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The module implementing this mod: `<dir>/<id>.luau`, else `<dir>/main.luau`.
    pub fn entry(&self, dir: &Path) -> Option<PathBuf> {
        let named = dir.join(format!("{}.luau", self.id));
        if named.is_file() {
            return Some(named);
        }
        let main = dir.join("main.luau");
        main.is_file().then_some(main)
    }
}
