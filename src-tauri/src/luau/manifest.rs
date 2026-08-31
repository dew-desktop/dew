use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub hotkeys: HashMap<String, String>,
    #[serde(default)]
    pub entrypoint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredMod {
    pub manifest: ModManifest,
    pub script_path: PathBuf,
    pub root_dir: PathBuf,
}

impl DiscoveredMod {
    pub fn from_directory(dir: &Path) -> Option<Self> {
        let manifest_path = dir.join("mod.json");
        if !manifest_path.exists() {
            return None;
        }

        let manifest_content = fs::read_to_string(&manifest_path).ok()?;
        let clean_content = manifest_content.strip_prefix('\u{feff}').unwrap_or(&manifest_content);
        let manifest: ModManifest = serde_json::from_str(clean_content).ok()?;

        // Locate script entrypoint: explicit entrypoint or <id>.luau or main.luau
        let script_name = manifest
            .entrypoint
            .clone()
            .unwrap_or_else(|| format!("{}.luau", manifest.id));

        let mut script_path = dir.join(&script_name);
        if !script_path.exists() {
            script_path = dir.join("main.luau");
        }

        if !script_path.exists() {
            return None;
        }

        Some(Self {
            manifest,
            script_path,
            root_dir: dir.to_path_buf(),
        })
    }
}

pub fn discover_mods(base_dirs: &[PathBuf]) -> Vec<DiscoveredMod> {
    let mut mods = Vec::new();

    for base in base_dirs {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(discovered) = DiscoveredMod::from_directory(&path) {
                        println!(
                            "[Dew Discovery] Found mod '{}' (v{}) at {}",
                            discovered.manifest.name,
                            discovered.manifest.version,
                            discovered.script_path.display()
                        );
                        mods.push(discovered);
                    }
                }
            }
        }
    }

    mods
}
