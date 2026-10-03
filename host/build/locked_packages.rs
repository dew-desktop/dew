//! The packages an applet's `pesde.lock` pins, and whether its
//! `roblox_packages/.pesde` holds those versions. `host/build.rs` includes
//! this file to refuse a stale install, and `host/src/lib.rs` includes it
//! under `cfg(test)` so the tests at the bottom run with the host's.
//!
//! A hand parser rather than a TOML crate: the only thing read is the quoted
//! key of each `[graph."..."]` table, `scope/name@version target`, and
//! pesde installs that package at `.pesde/scope+name/version/`.

use std::collections::BTreeMap;

/// One package the lockfile pins.
#[derive(Debug, PartialEq, Eq)]
pub struct Locked<'a> {
    /// `scope/name`.
    pub name: &'a str,
    pub version: &'a str,
}

impl Locked<'_> {
    /// The package's folder under `roblox_packages/.pesde`.
    pub fn folder(&self) -> String {
        self.name.replace('/', "+")
    }
}

/// A locked package whose pinned version is not installed.
#[derive(Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub name: String,
    pub expected: String,
    /// The versions installed instead, empty when the package has no folder.
    pub found: Vec<String>,
}

/// Every package in the lockfile's graph, once each, in the order they
/// first appear. A package can appear only in a subtable such as
/// `[graph."...".pkg_ref]`, so every graph header counts, not just bare ones.
pub fn locked_packages(lock: &str) -> Vec<Locked<'_>> {
    let mut packages: Vec<Locked> = Vec::new();
    for line in lock.lines() {
        let Some(rest) = line.trim().strip_prefix("[graph.\"") else {
            continue;
        };
        let Some((key, _)) = rest.split_once('"') else {
            continue;
        };
        let id = key.split_once(' ').map_or(key, |(id, _target)| id);
        let Some((name, version)) = id.split_once('@') else {
            continue;
        };
        if !packages
            .iter()
            .any(|p| p.name == name && p.version == version)
        {
            packages.push(Locked { name, version });
        }
    }
    packages
}

/// The locked packages whose pinned version is not among the versions
/// `installed` holds for them. `installed` maps a folder under `.pesde` to
/// the version folders inside it. A version the lockfile does not name is
/// not itself a mismatch: `pesde install` removes the version it replaces,
/// so an extra one beside the right one is not what a pin bump leaves, and
/// the applet's `require`s resolve to the pinned folder regardless.
pub fn mismatches(locked: &[Locked], installed: &BTreeMap<String, Vec<String>>) -> Vec<Mismatch> {
    locked
        .iter()
        .filter_map(|package| {
            let found = installed
                .get(&package.folder())
                .cloned()
                .unwrap_or_default();
            (!found.iter().any(|v| v == package.version)).then(|| Mismatch {
                name: package.name.to_string(),
                expected: package.version.to_string(),
                found,
            })
        })
        .collect()
}

/// The build failure for `applet`, whose folder is `dir`.
pub fn report(applet: &str, dir: &str, mismatches: &[Mismatch]) -> String {
    let mut text =
        format!("\nThe {applet} applet's installed packages do not match its pesde.lock:\n");
    for m in mismatches {
        let found = if m.found.is_empty() {
            "not installed".to_string()
        } else {
            m.found.join(", ")
        };
        text.push_str(&format!(
            "  {}: expected {}, found {found}\n",
            m.name, m.expected
        ));
    }
    text.push_str(&format!(
        "The bundled applets are built into dew with their packages, \
         so run `pesde install` in {dir} and build again.\n"
    ));
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = r#"format = 2
name = "dew/dashboard"

[graph."centau/vide@0.0.0-df9f roblox"]
direct = ["vide", { repo = "https://github.com/es9-dev/vide", rev = "7dca" }, "standard"]

[graph."spektr/aether@0.0.0-d59e roblox"]
direct = ["aether", { repo = "https://github.com/project-aether-ui/aether", rev = "d08a" }, "standard"]

[graph."spektr/aether@0.0.0-d59e roblox".dependencies]
vide = ["centau/vide@0.0.0-df9f roblox", "peer"]
Virtual = ["spektr/virtual@0.0.0-b956 roblox", "standard"]

[graph."spektr/virtual@0.0.0-b956 roblox".pkg_ref]
ref_ty = "git"
"#;

    fn installed(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<String>> {
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (folder, version) in entries {
            map.entry(folder.to_string())
                .or_default()
                .push(version.to_string());
        }
        map
    }

    #[test]
    fn reads_every_graph_package_once() {
        assert_eq!(
            locked_packages(LOCK),
            vec![
                Locked {
                    name: "centau/vide",
                    version: "0.0.0-df9f"
                },
                Locked {
                    name: "spektr/aether",
                    version: "0.0.0-d59e"
                },
                Locked {
                    name: "spektr/virtual",
                    version: "0.0.0-b956"
                },
            ]
        );
    }

    #[test]
    fn matching_install_passes() {
        let have = installed(&[
            ("centau+vide", "0.0.0-df9f"),
            ("spektr+aether", "0.0.0-d59e"),
            ("spektr+virtual", "0.0.0-b956"),
        ]);
        assert_eq!(mismatches(&locked_packages(LOCK), &have), vec![]);
    }

    #[test]
    fn stale_install_fails() {
        let have = installed(&[
            ("centau+vide", "0.0.0-df9f"),
            ("spektr+aether", "0.0.0-193d"),
            ("spektr+virtual", "0.0.0-b956"),
        ]);
        assert_eq!(
            mismatches(&locked_packages(LOCK), &have),
            vec![Mismatch {
                name: "spektr/aether".into(),
                expected: "0.0.0-d59e".into(),
                found: vec!["0.0.0-193d".into()],
            }]
        );
    }

    #[test]
    fn missing_install_fails() {
        let have = installed(&[
            ("centau+vide", "0.0.0-df9f"),
            ("spektr+aether", "0.0.0-d59e"),
        ]);
        assert_eq!(
            mismatches(&locked_packages(LOCK), &have),
            vec![Mismatch {
                name: "spektr/virtual".into(),
                expected: "0.0.0-b956".into(),
                found: vec![],
            }]
        );
    }

    #[test]
    fn extra_version_beside_the_pinned_one_passes() {
        let have = installed(&[
            ("centau+vide", "0.0.0-df9f"),
            ("spektr+aether", "0.0.0-193d"),
            ("spektr+aether", "0.0.0-d59e"),
            ("spektr+virtual", "0.0.0-b956"),
        ]);
        assert_eq!(mismatches(&locked_packages(LOCK), &have), vec![]);
    }

    #[test]
    fn report_names_applet_package_versions_and_the_fix() {
        let text = report(
            "dashboard",
            "C:/dew/dashboard",
            &[Mismatch {
                name: "spektr/aether".into(),
                expected: "0.0.0-d59e".into(),
                found: vec!["0.0.0-193d".into()],
            }],
        );
        assert!(text.contains("dashboard applet"));
        assert!(text.contains("spektr/aether: expected 0.0.0-d59e, found 0.0.0-193d"));
        assert!(text.contains("run `pesde install` in C:/dew/dashboard"));
    }
}
