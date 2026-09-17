//! `dew.toml` — what a mod says it is, before any of it runs.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Storage,
    Audio,
    Notifications,
    Clipboard,
    #[serde(rename = "rbxassetid")]
    RbxAssetId,

    // SURFACES ARE CAPABILITIES (ADR-012), and they are separate ones because
    // they differ in weight. A float draws in a corner. An overlay that is
    // topmost and click-through can draw over everything on screen while the
    // user does not know it is there. Granting those with one word would be
    // saying they are the same request.
    //
    // Every applet gets a surface today without asking. These make the asking
    // explicit, which is the honest version of what was always happening.
    Float,
    Window,
    Overlay,
    Popover,
}

impl Permission {
    pub fn name(self) -> &'static str {
        match self {
            Permission::Storage => "storage",
            Permission::Audio => "audio",
            Permission::Notifications => "notifications",
            Permission::Clipboard => "clipboard",
            Permission::RbxAssetId => "rbxassetid",
            Permission::Float => "float",
            Permission::Window => "window",
            Permission::Overlay => "overlay",
            Permission::Popover => "popover",
        }
    }

    /// The surface permission named by a word, for `dew init --surface`.
    ///
    /// SURFACES ONLY, not every permission. The scaffolder's job is to grant the
    /// applet somewhere to draw; handing it `--surface storage` should be refused
    /// rather than written into the manifest as though it meant something.
    pub fn surface_from_name(word: &str) -> Option<Permission> {
        let found = match word {
            "float" => Permission::Float,
            "window" => Permission::Window,
            "overlay" => Permission::Overlay,
            "popover" => Permission::Popover,
            _ => return None,
        };
        Some(found)
    }

    /// Is this permission a surface, meaning something Dew renders a tree into?
    ///
    /// THE RULE FROM ADR-012, in code. `notifications` and a future `tray` are
    /// capabilities the operating system draws, so they are not surfaces however
    /// visible they are.
    pub fn is_surface(self) -> bool {
        matches!(
            self,
            Permission::Float | Permission::Window | Permission::Overlay | Permission::Popover
        )
    }
}

/// The runtime a mod is written against.
///
/// DECLARED BY THE AUTHOR, NEVER GUESSED BY THE HOST. There is one runtime, but
/// the key stays explicit rather than inferred, so that adding a second one
/// later does not have to guess at what today's mods meant.
///
/// A CLOSED SET, like `Permission`, and for the same reason. `"runtime": "solid"`
/// is refused at parse rather than silently falling back to a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    /// No framework. `mount(dew, root)` parents instances into the root the host
    /// made, and the host renders that tree directly.
    ///
    /// THE DEFAULT, and the only runtime there is. A manifest written before this
    /// key existed, or one that never sets it, keeps working, and the absent key
    /// means this.
    #[default]
    #[serde(rename = "datamodel")]
    DataModel,
}

impl Runtime {
    pub fn name(self) -> &'static str {
        match self {
            Runtime::DataModel => "datamodel",
        }
    }
}

/// What a mod declares about itself.
///
/// EVERY FIELD HERE IS READ BY SOMETHING. `id` names the mod and picks its entry
/// module, `runtime` picks the branch `applets::load` takes, `permissions` builds
/// the capability table, `name` titles the window and the tray, `description`
/// fills the tray tooltip. `hotkeys` is the one exception, and it is a LOUD one:
/// nothing in the host registers a global hotkey yet, so `unhonoured()` names
/// every binding a mod declared, at load.
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

    /// Which runtime this mod's `mount` is written against.
    ///
    /// There is one runtime today, `datamodel`, so this field currently has one
    /// valid value. It stays a field rather than being dropped because a manifest
    /// may still say so explicitly, and because a second runtime is the kind of
    /// thing this format needs to be able to add without a breaking change.
    #[serde(default)]
    pub runtime: Runtime,

    /// What this mod may reach. Absent means NOTHING, which is the correct
    /// default and the one a mod author is least likely to have intended by
    /// accident — an empty list produces a mod that renders and cannot touch the
    /// machine, and that failure is loud and local.
    #[serde(default)]
    pub permissions: Vec<Permission>,

    /// Named bindings the mod wants, e.g. `"togglePomodoro": "Alt+Shift+P"`.
    ///
    /// DECLARED, NOT YET BOUND. Nothing in the host registers a global hotkey,
    /// so every one of these is inert -- `examples/aether/timetracker` ships three that
    /// have never once fired. The field stays because it is the contract Dew
    /// intends to honour, and `unhonoured()` says so at load rather than leaving
    /// an author to conclude from silence that the binding took.
    #[serde(default)]
    pub hotkeys: BTreeMap<String, String>,

    /// Keys in `dew.toml` that this struct does not model.
    ///
    /// Serde drops an unknown key without a word, which is how `timetracker`'s
    /// `version`, `author` and its entire `settings` block have gone nowhere for
    /// as long as they have existed. Collected before the typed parse and
    /// reported at load; NOT rejected, because a manifest is a forward-compatible
    /// format and a host that refuses tomorrow's field cannot read tomorrow's mod.
    #[serde(skip)]
    pub unknown: Vec<String>,
}

/// The keys `Manifest` models. Anything else in a `dew.toml` lands in `unknown`.
///
/// Written out rather than derived. Serde offers no way to ask a struct for its
/// field names, and the alternative -- a list of keys seen in the wild -- goes
/// stale in the direction that stays quiet.
const KNOWN: &[&str] = &[
    "id",
    "name",
    "description",
    "runtime",
    "permissions",
    "hotkeys",
];

impl Manifest {
    pub fn load(dir: &Path) -> Result<Manifest, String> {
        let path = dir.join("dew.toml");
        let raw = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Manifest::parse(&raw, &path.display().to_string())
    }

    /// The whole of `load` except the disk, so it can be asserted on.
    ///
    /// Split out for the tests rather than for the callers: what is worth
    /// checking is which keys survive and which get reported, and routing that
    /// through a temporary directory would test the filesystem instead.
    pub fn parse(raw: &str, label: &str) -> Result<Manifest, String> {
        // A BOM survives `read_to_string` and a parser rejects it as a value.
        let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);

        // TWO PASSES OVER THE SAME TEXT, and the first is the only chance to see
        // what the second throws away: `toml::Value` keeps every key, and
        // `Manifest` keeps six.
        let mut unknown: Vec<String> = match raw.parse::<toml::Value>() {
            Ok(toml::Value::Table(map)) => map
                .into_iter()
                .map(|(key, _)| key)
                .filter(|key| !KNOWN.contains(&key.as_str()))
                .collect(),
            // Not a table, or not TOML at all. The typed parse below reports
            // that properly, and there is nothing useful to say about its keys.
            _ => Vec::new(),
        };
        unknown.sort();

        let mut manifest: Manifest = toml::from_str(raw).map_err(|e| format!("{label}: {e}"))?;
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
        // The three keys `examples/aether/timetracker/dew.toml` really carries.
        let m = parse("id = \"t\"\nversion = \"1.0.0\"\nauthor = \"someone\"\n[settings]\na = 1\n");
        assert_eq!(m.unknown, vec!["author", "settings", "version"]);
    }

    #[test]
    fn a_manifest_the_host_fully_reads_reports_nothing() {
        let m =
            parse("id = \"t\"\nname = \"T\"\ndescription = \"d\"\npermissions = [\"storage\"]\n");
        assert!(m.unknown.is_empty());
        assert!(m.unhonoured().is_empty());
    }

    #[test]
    fn every_declared_hotkey_is_named_as_unbound() {
        let m = parse("id = \"t\"\n[hotkeys]\ngo = \"Alt+G\"\nstop = \"Alt+S\"\n");
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
        let m = parse("id = \"t\"\nauthor = \"someone\"\n[hotkeys]\ngo = \"Alt+G\"\n");
        assert_eq!(m.unhonoured().len(), 2);
    }

    #[test]
    fn display_name_falls_back_to_the_id() {
        assert_eq!(parse("id = \"t\"\n").display_name(), "t");
        assert_eq!(parse("id = \"t\"\nname = \"  \"\n").display_name(), "t");
        assert_eq!(parse("id = \"t\"\nname = \"T\"\n").display_name(), "T");
    }

    #[test]
    fn runtime_defaults_to_datamodel_and_is_not_reported_as_unknown() {
        let m = parse("id = \"t\"\n");
        assert_eq!(m.runtime, Runtime::DataModel);
        assert!(m.unhonoured().is_empty());
    }

    #[test]
    fn a_datamodel_mod_declares_its_runtime_and_the_key_is_read() {
        let m = parse("id = \"t\"\nruntime = \"datamodel\"\n");
        assert_eq!(m.runtime, Runtime::DataModel);
        // The whole reason this is a modelled field rather than an extra key:
        // arriving in `unknown` would mean the host ignored it.
        assert!(m.unknown.is_empty());
    }

    #[test]
    fn an_unknown_runtime_is_refused_rather_than_falling_back() {
        assert!(Manifest::parse("id = \"t\"\nruntime = \"solid\"\n", "test").is_err());
    }

    #[test]
    fn an_unknown_permission_is_refused_rather_than_ignored() {
        assert!(Manifest::parse("id = \"t\"\npermissions = [\"filesystem\"]\n", "test").is_err());
    }

    #[test]
    fn a_leading_bom_still_parses() {
        assert_eq!(parse("\u{feff}id = \"t\"\n").id, "t");
    }
}
