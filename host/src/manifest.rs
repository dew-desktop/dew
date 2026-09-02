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

/// What a mod declares about itself.
///
/// EVERY FIELD HERE IS READ BY SOMETHING. `id` names the mod and picks its entry
/// module, `permissions` builds the capability table, `name` titles the window
/// and the tray, `description` fills the tray tooltip. `hotkeys` is the one
/// exception, and it is a LOUD one: nothing in the host registers a global
/// hotkey yet, so `unhonoured()` names every binding a mod declared, at load.
///
/// The rule this struct is built around is that a mod author is never met with
/// silence. A field Dew accepts and ignores, and a key Dew has never heard of,
/// are both indistinguishable from one that works -- so both are said out loud.
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

    /// Named bindings the mod wants, e.g. `"togglePomodoro": "Alt+Shift+P"`.
    ///
    /// DECLARED, NOT YET BOUND. Nothing in the host registers a global hotkey,
    /// so every one of these is inert -- `mods/timetracker` ships three that
    /// have never once fired. The field stays because it is the contract Dew
    /// intends to honour, and `unhonoured()` says so at load rather than leaving
    /// an author to conclude from silence that the binding took.
    #[serde(default)]
    pub hotkeys: BTreeMap<String, String>,

    /// Keys in `mod.json` that this struct does not model.
    ///
    /// Serde drops an unknown key without a word, which is how `timetracker`'s
    /// `version`, `author` and its entire `settings` block have gone nowhere for
    /// as long as they have existed. Collected before the typed parse and
    /// reported at load; NOT rejected, because a manifest is a forward-compatible
    /// format and a host that refuses tomorrow's field cannot read tomorrow's mod.
    #[serde(skip)]
    pub unknown: Vec<String>,
}

/// The keys `Manifest` models. Anything else in a `mod.json` lands in `unknown`.
///
/// Written out rather than derived. Serde offers no way to ask a struct for its
/// field names, and the alternative -- a list of keys seen in the wild -- goes
/// stale in the direction that stays quiet.
const KNOWN: &[&str] = &["id", "name", "description", "permissions", "hotkeys"];

impl Manifest {
    pub fn load(dir: &Path) -> Result<Manifest, String> {
        let path = dir.join("mod.json");
        let raw = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Manifest::parse(&raw, &path.display().to_string())
    }

    /// The whole of `load` except the disk, so it can be asserted on.
    ///
    /// Split out for the tests rather than for the callers: what is worth
    /// checking is which keys survive and which get reported, and routing that
    /// through a temporary directory would test the filesystem instead.
    fn parse(raw: &str, label: &str) -> Result<Manifest, String> {
        // A BOM survives `read_to_string` and serde_json rejects it as a value.
        let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);

        // TWO PASSES OVER THE SAME TEXT, and the first is the only chance to see
        // what the second throws away: `serde_json::Value` keeps every key, and
        // `Manifest` keeps five.
        let mut unknown: Vec<String> = match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(serde_json::Value::Object(map)) => map
                .into_iter()
                .map(|(key, _)| key)
                .filter(|key| !KNOWN.contains(&key.as_str()))
                .collect(),
            // Not an object, or not JSON at all. The typed parse below reports
            // that properly, and there is nothing useful to say about its keys.
            _ => Vec::new(),
        };
        unknown.sort();

        let mut manifest: Manifest =
            serde_json::from_str(raw).map_err(|e| format!("{label}: {e}"))?;
        manifest.unknown = unknown;
        Ok(manifest)
    }

    /// What this manifest asked for that the host will not do, one plain line each.
    ///
    /// EMPTY IS THE GOAL. Every entry is a promise the format makes and the host
    /// does not keep, so this list shrinking is what "wire the hotkeys up" looks
    /// like from outside -- and a newly unread field cannot be added without
    /// showing up here.
    pub fn unhonoured(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (name, binding) in &self.hotkeys {
            out.push(format!(
                "hotkey {name:?} ({binding}) is declared and not bound: \
                 Dew registers no global hotkeys yet"
            ));
        }
        for key in &self.unknown {
            out.push(format!("{key:?} is not a field Dew reads, and was ignored"));
        }
        out
    }

    /// The name to show a person: the declared one, or the id when there is none.
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            &self.id
        } else {
            &self.name
        }
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

/// WHAT THESE COVER is the promise the manifest makes to a mod author: a key the
/// host does not read is reported rather than dropped, and a field the host does
/// read is actually reached. Before this, `cargo test` in the host compiled zero
/// tests and printed `ok. 0 passed`, which is the same sentence a passing suite
/// prints.
#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> Manifest {
        Manifest::parse(raw, "test").expect("valid manifest")
    }

    #[test]
    fn unknown_keys_are_captured_rather_than_dropped() {
        // The three keys `mods/timetracker/mod.json` really carries.
        let m = parse(
            r#"{ "id": "t", "version": "1.0.0", "author": "someone",
                 "settings": { "a": 1 } }"#,
        );
        assert_eq!(m.unknown, vec!["author", "settings", "version"]);
    }

    #[test]
    fn a_manifest_the_host_fully_reads_reports_nothing() {
        let m =
            parse(r#"{ "id": "t", "name": "T", "description": "d", "permissions": ["storage"] }"#);
        assert!(m.unknown.is_empty());
        assert!(m.unhonoured().is_empty());
    }

    #[test]
    fn every_declared_hotkey_is_named_as_unbound() {
        let m = parse(r#"{ "id": "t", "hotkeys": { "go": "Alt+G", "stop": "Alt+S" } }"#);
        let said = m.unhonoured();
        assert_eq!(said.len(), 2);
        assert!(said
            .iter()
            .any(|s| s.contains("\"go\"") && s.contains("Alt+G")));
        assert!(said
            .iter()
            .any(|s| s.contains("\"stop\"") && s.contains("Alt+S")));
    }

    #[test]
    fn unhonoured_covers_hotkeys_and_unknown_keys_together() {
        let m = parse(r#"{ "id": "t", "hotkeys": { "go": "Alt+G" }, "author": "someone" }"#);
        assert_eq!(m.unhonoured().len(), 2);
    }

    #[test]
    fn display_name_falls_back_to_the_id() {
        assert_eq!(parse(r#"{ "id": "t" }"#).display_name(), "t");
        assert_eq!(parse(r#"{ "id": "t", "name": "  " }"#).display_name(), "t");
        assert_eq!(parse(r#"{ "id": "t", "name": "T" }"#).display_name(), "T");
    }

    #[test]
    fn an_unknown_permission_is_refused_rather_than_ignored() {
        assert!(
            Manifest::parse(r#"{ "id": "t", "permissions": ["filesystem"] }"#, "test").is_err()
        );
    }

    #[test]
    fn a_leading_bom_still_parses() {
        assert_eq!(parse("\u{feff}{ \"id\": \"t\" }").id, "t");
    }
}
