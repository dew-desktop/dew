//! `dew compat`: load every example the way `dew snapshot` does and write what
//! `dew_host::compat` finds into `examples/README.md`.
//!
//! WHY THIS IS A SUBCOMMAND AND NOT A BIN BESIDE `datamodel-surface`. The loader
//! (`applets::load`), the capability table and the renderer `snapshot` drives
//! all live in this binary, and a second binary loading examples some other way
//! would report on a load the host never performs.
//!
//! `--check` regenerates in memory and fails when the committed table differs.

use super::{applets, capabilities, create_renderer, find_dir, load_script, Renderer};
use dew_host::compat::{self, Host, Row, Walk};
use dew_host::datamodel::render::honours::honours;
use dew_host::manifest::{Manifest, Permission};
use dew_host::services;
use mlua::{Lua, Value as LuaValue};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// One thing under `examples/` that runs.
#[derive(Debug)]
enum Target {
    /// A directory holding a `dew.toml`.
    Applet(PathBuf),
    /// A directory with no manifest and Luau directly in it: a `--script`.
    Script(PathBuf),
}

impl Target {
    fn dir(&self) -> &Path {
        match self {
            Target::Applet(dir) => dir,
            Target::Script(entry) => entry.parent().expect("a script has a directory"),
        }
    }
}

fn is_packages(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == "roblox_packages")
}

fn is_test(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".test.luau"))
}

fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    entries.sort();
    entries
}

/// Every applet and script under `dir`, walked rather than listed, so an
/// example added in a new folder is in the table without anyone adding it.
fn discover(dir: &Path, out: &mut Vec<Target>) {
    for child in sorted_entries(dir) {
        if !child.is_dir() || is_packages(&child) {
            continue;
        }
        if child.join("dew.toml").is_file() {
            out.push(Target::Applet(child));
            continue;
        }
        let scripts: Vec<PathBuf> = sorted_entries(&child)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "luau") && !is_test(p))
            .collect();
        match scripts.as_slice() {
            [] => discover(&child, out),
            [entry, ..] => out.push(Target::Script(entry.clone())),
        }
    }
}

/// The example's own Luau: every `.luau` under its directory, its installed
/// packages and its tests left out.
fn own_luau(dir: &Path, out: &mut Vec<PathBuf>) {
    for child in sorted_entries(dir) {
        if child.is_dir() {
            if !is_packages(&child) {
                own_luau(&child, out);
            }
        } else if child.extension().is_some_and(|e| e == "luau") && !is_test(&child) {
            out.push(child);
        }
    }
}

/// `path` under `base`, with forward slashes, so the table reads the same on
/// every platform.
fn relative(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// An error message with the checkout's own location taken out, so it reads
/// the same on every machine.
fn portable_message(message: &str, repo: &Path) -> String {
    let mut out = message.to_string();
    let mut prefixes = vec![repo.to_path_buf()];
    if let Ok(canonical) = repo.canonicalize() {
        prefixes.push(canonical);
    }
    for prefix in prefixes {
        let shown = prefix.display().to_string();
        for form in [shown.clone(), shown.replace('\\', "/")] {
            for sep in ["\\", "/"] {
                out = out.replace(&format!("{form}{sep}"), "");
            }
        }
    }
    out
}

/// Globals in this VM whose value is an `Instance`: the root a script is
/// handed to parent into.
fn instance_globals(lua: &Lua) -> BTreeSet<String> {
    let Ok(typeof_fn) = lua.globals().get::<mlua::Function>("typeof") else {
        return BTreeSet::new();
    };
    lua.globals()
        .pairs::<LuaValue, LuaValue>()
        .flatten()
        .filter_map(|(key, value)| {
            let LuaValue::String(name) = key else {
                return None;
            };
            let kind: String = typeof_fn.call(value).ok()?;
            (kind == "Instance").then(|| name.to_str().ok().map(|s| s.to_string()))?
        })
        .collect()
}

/// The capability table's global name and keys, built for `granted` on a
/// fresh VM the way the loader builds it.
fn capability_members(granted: &[Permission]) -> Result<(String, BTreeSet<String>), String> {
    let lua = Lua::new();
    let clock: services::SharedClock = Arc::new(Mutex::new(Default::default()));
    services::install(&lua, &clock).map_err(|e| e.to_string())?;
    let state = Arc::new(Mutex::new(capabilities::HostState::default()));
    let grant = capabilities::SurfaceGrant {
        requested: Arc::new(Mutex::new(None)),
        root: None,
        title: String::new(),
    };
    let table = capabilities::build(&lua, granted, &state, &grant).map_err(|e| e.to_string())?;
    let global = lua
        .globals()
        .pairs::<String, LuaValue>()
        .flatten()
        .find(|(_, value)| matches!(value, LuaValue::Table(t) if *t == table))
        .map(|(name, _)| name)
        .ok_or("the capability table is not reachable from a global")?;
    let keys = table
        .pairs::<String, LuaValue>()
        .flatten()
        .map(|(key, _)| key)
        .collect();
    Ok((global, keys))
}

/// `desktop.Member` for each member a surface permission with an engine
/// equivalent adds to the capability table.
fn mount_members(permissions: &BTreeSet<Permission>) -> Result<BTreeSet<String>, String> {
    let (_, ungated) = capability_members(&[])?;
    let mut out = BTreeSet::new();
    for permission in permissions {
        if !permission.is_surface() || permission.roblox_equivalent().is_none() {
            continue;
        }
        let (global, keys) = capability_members(&[*permission])?;
        for key in keys.difference(&ungated) {
            out.insert(format!("{global}.{key}"));
        }
    }
    Ok(out)
}

/// What one load produced.
struct Loaded {
    walk: Result<Walk, String>,
    permissions: Vec<Permission>,
}

fn load_applet(
    dir: &Path,
    installed: &mut BTreeSet<String>,
    roots: &mut BTreeSet<String>,
) -> Loaded {
    let permissions = Manifest::load(dir)
        .map(|m| m.permissions)
        .unwrap_or_default();
    let state = Arc::new(Mutex::new(capabilities::HostState::default()));
    let walk = applets::load_observed(dir, &HashMap::new(), &state, &mut |lua| {
        installed.extend(compat::global_names(lua));
        roots.extend(instance_globals(lua));
    })
    .and_then(|applet| {
        let applets::Applet {
            width,
            height,
            surface,
            mounted,
            clock,
            vm,
            ..
        } = applet;
        // ONE FRAME, AS `snapshot` PAINTS IT, so modifiers, style queries and
        // layout have run before the tree is read.
        let mut renderer = create_renderer(mounted, &vm, &clock, &surface, width, height)?;
        renderer.frame(1.0 / 60.0)?;
        let Renderer::DataModel { dom, root, .. } = &renderer;
        let guard = dom.lock().expect("dom");
        Ok(compat::walk(&guard, *root, honours))
    });
    Loaded { walk, permissions }
}

fn load_standalone(
    entry: &Path,
    installed: &mut BTreeSet<String>,
    roots: &mut BTreeSet<String>,
) -> Loaded {
    let walk = load_script(&entry.display().to_string(), &mut |lua| {
        installed.extend(compat::global_names(lua));
        roots.extend(instance_globals(lua));
    })
    .map(|(_vm, dom, root)| {
        let guard = dom.lock().expect("dom");
        compat::walk(&guard, root, honours)
    });
    Loaded {
        walk,
        permissions: Vec::new(),
    }
}

/// Every example's row, in path order, and what the host said about its own globals.
pub fn rows(examples: &Path) -> Result<(Vec<Row>, Host), String> {
    let repo = examples.parent().unwrap_or(examples).to_path_buf();
    let mut targets = Vec::new();
    discover(examples, &mut targets);
    if targets.is_empty() {
        return Err(format!("no examples found under {}", examples.display()));
    }

    let mut installed = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut loaded = Vec::new();
    for target in &targets {
        let result = match target {
            Target::Applet(dir) => load_applet(dir, &mut installed, &mut roots),
            Target::Script(entry) => load_standalone(entry, &mut installed, &mut roots),
        };
        loaded.push(result);
    }

    let dew_only_globals = compat::dew_only_globals(&installed);
    let permissions: BTreeSet<Permission> = loaded
        .iter()
        .flat_map(|l| l.permissions.iter().copied())
        .collect();
    let host = Host {
        root_globals: roots.intersection(&dew_only_globals).cloned().collect(),
        mount_members: mount_members(&permissions)?,
        dew_only_globals,
    };

    let mut rows = Vec::new();
    for (target, result) in targets.iter().zip(loaded) {
        let mut files = Vec::new();
        own_luau(target.dir(), &mut files);
        let mut reach = compat::Reach::default();
        for file in &files {
            let source =
                std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
            let found = compat::reach(&source, &host.dew_only_globals);
            reach.names.extend(found.names);
            reach.returns_mount |= found.returns_mount;
            reach.aether |= found.aether;
        }
        let walk_dew_only = result
            .walk
            .as_ref()
            .map(|w| w.dew_only.clone())
            .unwrap_or_default();
        rows.push(Row {
            example: relative(target.dir(), examples),
            reached: compat::reached(&reach, &result.permissions, &walk_dew_only, &host),
            walk: result.walk.map_err(|e| portable_message(&e, &repo)),
        });
    }
    Ok((rows, host))
}

pub fn execute(check: bool) -> Result<(), String> {
    let examples = find_dir("examples")
        .ok_or("no examples/ directory here or above; run from the repository")?;
    let readme_path = examples.join("README.md");
    let on_disk = std::fs::read_to_string(&readme_path)
        .map_err(|e| format!("{}: {e}", readme_path.display()))?;
    let crlf = on_disk.contains("\r\n");
    let current = on_disk.replace("\r\n", "\n");

    let (rows, host) = rows(&examples)?;
    let updated = compat::splice(&current, &compat::table(&rows, &host))?;

    if check {
        if updated == current {
            println!("examples/README.md: the compatibility table is current.");
            return Ok(());
        }
        return Err("examples/README.md: the compatibility table is STALE. Run \
             `cargo run --manifest-path host/Cargo.toml -- compat` and commit the result."
            .to_string());
    }

    let written = if crlf {
        updated.replace('\n', "\r\n")
    } else {
        updated
    };
    std::fs::write(&readme_path, written).map_err(|e| format!("{}: {e}", readme_path.display()))?;
    println!(
        "wrote the compatibility table for {} example(s) into examples/README.md",
        rows.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE WALK VISITS SOMETHING, EVERY TIME, FOR EVERY EXAMPLE THAT LOADS.
    ///
    /// A generator whose walk came back empty would print "none" under the
    /// properties column for everything and call it portable. This runs the
    /// real generator over the real examples and refuses that.
    #[test]
    fn every_loaded_example_walks_a_nonempty_tree() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo")
            .join("examples");
        let (rows, _) = rows(&examples).expect("rows");
        assert!(rows.len() >= 20, "found only {} example(s)", rows.len());
        let loaded: Vec<_> = rows.iter().filter(|r| r.walk.is_ok()).collect();
        assert!(
            loaded.len() >= rows.len() / 2,
            "only {} of {} examples loaded",
            loaded.len(),
            rows.len()
        );
        for row in &loaded {
            let walk = row.walk.as_ref().expect("loaded");
            assert!(walk.instances > 0, "{} walked no instances", row.example);
        }
        assert!(
            rows.iter().any(|r| r.example == "host/standalone"),
            "the standalone script is not in the table"
        );
    }

    #[test]
    fn an_error_message_loses_the_checkout_path() {
        let repo = Path::new("C:\\work\\Dew");
        assert_eq!(
            portable_message("C:\\work\\Dew\\examples\\a.luau:3: boom", repo),
            "examples\\a.luau:3: boom"
        );
    }
}
