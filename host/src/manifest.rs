//! `dew.toml` — what a mod says it is, before any of it runs.
//!
//! Read from disk and checked BEFORE the mod's own code is loaded, which is the
//! whole reason the mod contract is a returned declaration rather than a
//! registration call. A manifest that had to be discovered by running the mod
//! would be a manifest that arrives after the mod has already acted.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The host-fixed class a `Permission` belongs to (ADR-017).
///
/// NEVER SOMETHING A MANIFEST DECLARES. `dew.toml` states permissions, not
/// capabilities -- the same way it states `overlay` without also stating
/// that an overlay asks for a great deal. Which capability a permission
/// falls under is this file's own fixed table, not the mod's to assert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Anything any installed mod may ask for. Granted on the manifest's
    /// say-so alone, the same as every permission worked before this
    /// distinction existed.
    User,
    /// Auth, Discover, Install. Granted only to a mod the coordinator
    /// itself loaded from its own bundled directory -- never to anything
    /// under `installed::list()`'s user-writable one, regardless of what
    /// that mod's own `dew.toml` claims.
    Host,
}

/// A permission a mod may ask for.
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
    // they differ in weight. A widget draws in a corner. An overlay that is
    // topmost and click-through can draw over everything on screen while the
    // user does not know it is there. Granting those with one word would be
    // saying they are the same request.
    //
    // Every applet gets a surface today without asking. These make the asking
    // explicit, which is the honest version of what was always happening.
    Widget,
    Window,
    Overlay,
    Popover,

    // `Capability::Host` ONLY (ADR-017). Named after the CLI verbs they
    // mirror -- `dew login`/`dew logout`, `dew discover`, `dew install` --
    // rather than inventing new words for old ones. `applets::load` refuses
    // to load a mod declaring any of these unless it came from the
    // coordinator's own bundled directory; see `Permission::capability`.
    Auth,
    Discover,
    Install,
    // `dew.Library` (milestone 25): list, launch, uninstall and
    // enable/disable an installed applet. Host-only for the same reason as
    // the three above -- it reaches every applet on the machine, not just
    // the one that asked.
    Library,
}

impl Permission {
    pub fn name(self) -> &'static str {
        match self {
            Permission::Storage => "storage",
            Permission::Audio => "audio",
            Permission::Notifications => "notifications",
            Permission::Clipboard => "clipboard",
            Permission::RbxAssetId => "rbxassetid",
            Permission::Widget => "widget",
            Permission::Window => "window",
            Permission::Overlay => "overlay",
            Permission::Popover => "popover",
            Permission::Auth => "auth",
            Permission::Discover => "discover",
            Permission::Install => "install",
            Permission::Library => "library",
        }
    }

    /// Which `Capability` this permission falls under (ADR-017). Fixed by
    /// the host, not by anything a manifest says.
    pub fn capability(self) -> Capability {
        match self {
            Permission::Auth | Permission::Discover | Permission::Install | Permission::Library => {
                Capability::Host
            }
            Permission::Storage
            | Permission::Audio
            | Permission::Notifications
            | Permission::Clipboard
            | Permission::RbxAssetId
            | Permission::Widget
            | Permission::Window
            | Permission::Overlay
            | Permission::Popover => Capability::User,
        }
    }

    /// The surface permission named by a word, for `dew init --surface`.
    ///
    /// SURFACES ONLY, not every permission. The scaffolder's job is to grant the
    /// applet somewhere to draw; handing it `--surface storage` should be refused
    /// rather than written into the manifest as though it meant something.
    pub fn surface_from_name(word: &str) -> Option<Permission> {
        let found = match word {
            "widget" => Permission::Widget,
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
            Permission::Widget | Permission::Window | Permission::Overlay | Permission::Popover
        )
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
    /// so every one of these is inert -- `examples/aether/timetracker` ships three that
    /// have never once fired. The field stays because it is the contract Dew
    /// intends to honour, and `unhonoured()` says so at load rather than leaving
    /// an author to conclude from silence that the binding took.
    #[serde(default)]
    pub hotkeys: BTreeMap<String, String>,

    /// `"Class.Property@N"` entries this mod wants unlocked, e.g.
    /// `["SomeClass.SomeProperty@1"]` -- `N` pins the revision of
    /// `Tier::Experimental` the mod was written against, and is required,
    /// not optional (milestone 12 sprint 2): a declaration that never pins a
    /// revision can never be told it fell behind one. Each key must name a
    /// real `Tier::Experimental` property in
    /// `datamodel::extensions::PROPERTIES` -- see `experimental_flags`,
    /// which is what actually resolves and validates this list.
    #[serde(default, rename = "experimentalDatamodel")]
    pub experimental_datamodel: Vec<String>,

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
    "permissions",
    "hotkeys",
    "experimentalDatamodel",
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

    /// Resolve `experimentalDatamodel` against the registry, deduplicated.
    ///
    /// REFUSES TO LOAD ON AN UNKNOWN ENTRY, A MISSING/INVALID REVISION, OR A
    /// STALE ONE, rather than ignoring any of them -- the same choice
    /// `Permission` already makes for an unknown value. Each is a distinct
    /// failure with its own message (`extensions::DeclarationError`'s
    /// `Display`): "not a known property" calls for fixing a typo, "no
    /// @revision" calls for adding one, and a stale revision calls for
    /// reading what changed and updating the mod -- three different fixes,
    /// so a generic "unknown flag" error would send an author down the
    /// wrong path for two of the three.
    pub fn experimental_flags(&self) -> Result<Vec<(&'static str, u32)>, String> {
        let mut flags: Vec<(&'static str, u32)> = Vec::new();
        for declaration in &self.experimental_datamodel {
            let flag =
                crate::datamodel::extensions::resolve_declaration(declaration).map_err(|e| {
                    format!(
                        "{}: experimentalDatamodel entry {declaration:?} {e}",
                        self.id
                    )
                })?;
            if !flags.contains(&flag) {
                flags.push(flag);
            }
        }
        Ok(flags)
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

    /// `runtime` NAMED NO FRAMEWORK EVEN WHEN IT EXISTED -- `mount(dew, root)`
    /// was the only signature there ever was -- and milestone 9 retired the
    /// field along with the ceremony that branched on it. A `dew.toml` written
    /// before that still parses: the key falls out of `KNOWN` and into
    /// `unknown`, reported rather than silently dropped, the same as any other
    /// field the host has stopped reading.
    #[test]
    fn a_leftover_runtime_key_is_reported_as_unknown_rather_than_read() {
        let m = parse("id = \"t\"\nruntime = \"datamodel\"\n");
        assert_eq!(m.unknown, vec!["runtime"]);
    }

    #[test]
    fn an_unknown_permission_is_refused_rather_than_ignored() {
        assert!(Manifest::parse("id = \"t\"\npermissions = [\"filesystem\"]\n", "test").is_err());
    }

    #[test]
    fn a_leading_bom_still_parses() {
        assert_eq!(parse("\u{feff}id = \"t\"\n").id, "t");
    }

    #[test]
    fn experimental_datamodel_is_read_rather_than_reported_unknown() {
        let m = parse("id = \"t\"\nexperimentalDatamodel = [\"InputActionLabel.InputAction\"]\n");
        assert_eq!(
            m.experimental_datamodel,
            vec!["InputActionLabel.InputAction"]
        );
        assert!(m.unknown.is_empty());
    }

    /// A REAL PROPERTY, BUT NOT AN EXPERIMENTAL ONE. `InputActionLabel.InputAction`
    /// is a genuine row in the registry, and is still refused here: it is
    /// `Tier::DewOnly`, always on, with no flag to resolve to -- declaring it
    /// experimental would ask for a flag that does not exist. `@1` is there
    /// so this test exercises that refusal specifically, not the (also true,
    /// but different) missing-revision one below.
    #[test]
    fn a_dew_only_entry_cannot_be_declared_experimental() {
        let m = parse("id = \"t\"\nexperimentalDatamodel = [\"InputActionLabel.InputAction@1\"]\n");
        let err = m.experimental_flags().unwrap_err();
        assert!(err.contains("InputActionLabel.InputAction"), "{err}");
        assert!(err.contains("not a known experimental property"), "{err}");
    }

    #[test]
    fn an_unknown_experimental_entry_is_refused_rather_than_ignored() {
        let m = parse("id = \"t\"\nexperimentalDatamodel = [\"NotAThing.Nope@1\"]\n");
        let err = m.experimental_flags().unwrap_err();
        assert!(err.contains("NotAThing.Nope"), "{err}");
    }

    #[test]
    fn no_experimental_declaration_resolves_to_no_flags() {
        let m = parse("id = \"t\"\n");
        assert!(m.experimental_flags().unwrap().is_empty());
    }

    /// A BARE `"Class.Property"`, WITH NO `@revision`, REFUSES TO LOAD
    /// (milestone 12 sprint 2's decision: mandatory, not permissive -- see
    /// the sprint record for why a default-to-current revision would defeat
    /// the whole mechanism).
    #[test]
    fn a_declaration_without_a_revision_refuses_to_load() {
        let m = parse("id = \"t\"\nexperimentalDatamodel = [\"GuiObject.BlendingMode\"]\n");
        let err = m.experimental_flags().unwrap_err();
        assert!(err.contains("GuiObject.BlendingMode"), "{err}");
        assert!(err.contains("no @revision"), "{err}");
    }

    #[test]
    fn a_matching_revision_loads() {
        let m = parse("id = \"t\"\nexperimentalDatamodel = [\"GuiObject.BlendingMode@1\"]\n");
        assert_eq!(
            m.experimental_flags().unwrap(),
            vec![("FFlagDewGuiObjectBlendingMode", 1)]
        );
    }

    /// THE MISMATCH, THROUGH THE WHOLE MANIFEST PATH -- TOML parse, key
    /// split, registry lookup, revision compare -- not a unit test of
    /// `resolve_declaration` alone. `DewSprintTestOnly.RevisionProbe` is
    /// frozen at revision 2 for exactly this (see `extensions.rs`).
    #[test]
    fn a_mismatched_revision_refuses_with_a_specific_message() {
        let m =
            parse("id = \"t\"\nexperimentalDatamodel = [\"DewSprintTestOnly.RevisionProbe@1\"]\n");
        let err = m.experimental_flags().unwrap_err();
        assert_eq!(
            err,
            "t: experimentalDatamodel entry \"DewSprintTestOnly.RevisionProbe@1\" was written \
             against revision 1, but the registry is now at revision 2 -- the property's shape \
             changed; update the mod for the new behavior and declare @2"
        );
    }
}
