//! The gallery: scenes that show the property surface doing something.
//!
//! `datamodel-surface` reports what the host ACCEPTS. That number says a
//! property is not rejected; it does not say it draws anything, and until this
//! module nothing said so. Measured when milestone 5 was written, everything in
//! either repository that builds a tree touched 27 of the 139 in-scope
//! properties.
//!
//! NOT A CONFORMANCE RUNNER, and the separation is
//! [ADR-005](../../.artifacts/project/milestones/5_the_surface_demonstrates_itself/decisions/adr-005-the-gallery-is-not-a-conformance-runner.md).
//! A scene states no belief about Roblox: it has no `provenance`, no
//! `verifiedAgainst` and no `expect`, and [`decode_scene`] REFUSES a file
//! carrying any of them rather than ignoring them. The two artifacts share the
//! scene format and the renderer ([`crate::conformance::TREE_BUILDER`]) and
//! nothing else -- no directory, no tally, no provenance.
//!
//! WHAT THIS SPRINT DOES NOT CLAIM. A property counts as demonstrated when a
//! scene SETS it and the scene renders. That is a weaker claim than it sounds:
//! a property the renderer ignores entirely is demonstrated under this
//! definition, because setting it changes nothing and nobody looks. Closing
//! that is the differential check, and it is the next sprint. Until then the
//! number is honest about being a floor rather than a proof.

use crate::conformance::{SurfaceSize, TREE_BUILDER};
use crate::datamodel::{handle, install, install_vocabulary, render::frame_of, SharedDom};
use dew_raster::Backend;
use dew_runtime::{Painter, RasterPainter};
use mlua::{Lua, RegistryKey, Table, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Fields that belong to a conformance case and never to a gallery scene.
///
/// Refused rather than ignored. A scene that quietly carried `provenance` would
/// be a conformance case living in the wrong directory, and the next person to
/// read it would reasonably believe someone had checked it in Studio.
const CONFORMANCE_ONLY: &[&str] = &[
    "provenance",
    "verifiedAgainst",
    "expect",
    "expectAbsent",
    "ratios",
    "requires",
    "order",
    "golden",
    "pixels",
];

/// One scene: a tree, a surface to draw it on, and what it demonstrates.
pub struct Scene {
    pub file_stem: String,
    pub name: String,
    pub surface: SurfaceSize,
    pub tree_val: RegistryKey,
    /// Property names set anywhere in the tree.
    pub demonstrates: BTreeSet<String>,
    /// The finer question: which class each property was set on.
    pub demonstrates_by_class: BTreeSet<(String, String)>,
    /// Classes the scene instantiates.
    pub classes: BTreeSet<String>,
}

/// Where the scenes live.
///
/// Walks upward so the tool runs from the repository root or from `host/`,
/// which is the difference between a tool people use and one they invoke wrong
/// and believe.
pub fn find_scenes_dir(custom: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(dir) = custom {
        return if dir.is_dir() {
            Ok(dir.to_path_buf())
        } else {
            Err(format!("{} is not a directory", dir.display()))
        };
    }
    let mut cursor = std::env::current_dir().map_err(|e| e.to_string())?;
    loop {
        let candidate = cursor.join("gallery/scenes");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !cursor.pop() {
            return Err("could not find gallery/scenes; run from the repository".to_string());
        }
    }
}

/// Every `.luau` scene in the directory, sorted, so a run is reproducible.
pub fn scene_paths(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("luau"))
        .collect();
    out.sort();
    Ok(out)
}

/// Collect `class` and `props` keys from a tree table, recursively.
fn walk(
    node: &Table,
    props: &mut BTreeSet<String>,
    by_class: &mut BTreeSet<(String, String)>,
    classes: &mut BTreeSet<String>,
) -> Result<(), String> {
    let class: String = node
        .get::<Option<String>>("class")
        .map_err(|e| e.to_string())?
        .ok_or("a node has no class")?;
    classes.insert(class.clone());

    if let Ok(Some(p)) = node.get::<Option<Table>>("props") {
        for pair in p.pairs::<String, Value>() {
            let (k, _) = pair.map_err(|e| e.to_string())?;
            props.insert(k.clone());
            by_class.insert((class.clone(), k));
        }
    }
    if let Ok(Some(children)) = node.get::<Option<Table>>("children") {
        for child in children.sequence_values::<Table>() {
            walk(&child.map_err(|e| e.to_string())?, props, by_class, classes)?;
        }
    }
    Ok(())
}

/// Load one scene, refusing anything that belongs to a conformance case.
pub fn decode_scene(lua: &Lua, path: &Path) -> Result<Scene, String> {
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("scene")
        .to_string();
    let source = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let table: Table = lua
        .load(&source)
        .set_name(&file_stem)
        .eval()
        .map_err(|e| format!("{file_stem}: {e}"))?;

    for banned in CONFORMANCE_ONLY {
        if table.contains_key(*banned).unwrap_or(false) {
            return Err(format!(
                "{file_stem}: gallery scenes carry no '{banned}'. A scene shows the host \
                 rendering something; it states no belief about Roblox. If this file wants \
                 to assert engine behaviour it is a conformance case and belongs in \
                 aether/conformance/cases. See ADR-005."
            ));
        }
    }

    let name: String = table
        .get::<Option<String>>("name")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| file_stem.clone());

    let surface = match table.get::<Option<Table>>("surface") {
        Ok(Some(s)) => SurfaceSize {
            width: s.get::<f32>("width").unwrap_or(200.0),
            height: s.get::<f32>("height").unwrap_or(120.0),
        },
        _ => SurfaceSize {
            width: 200.0,
            height: 120.0,
        },
    };

    let tree: Table = table
        .get::<Option<Table>>("tree")
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{file_stem}: no tree"))?;

    let mut demonstrates = BTreeSet::new();
    let mut demonstrates_by_class = BTreeSet::new();
    let mut classes = BTreeSet::new();
    walk(
        &tree,
        &mut demonstrates,
        &mut demonstrates_by_class,
        &mut classes,
    )
    .map_err(|e| format!("{file_stem}: {e}"))?;

    let tree_val = lua.create_registry_value(tree).map_err(|e| e.to_string())?;

    Ok(Scene {
        file_stem,
        name,
        surface,
        tree_val,
        demonstrates,
        demonstrates_by_class,
        classes,
    })
}

/// Build the scene through Dew's own `Instance.new` and property setters, then
/// paint it and write a PNG.
///
/// THROUGH THE REAL PATH, not a fixture. The tree is built by the same setters a
/// guest calls, laid out by `frame_of`, and painted by `dew_raster`. A scene
/// that renders is a scene whose properties the host actually took.
pub fn render_scene(lua: &Lua, scene: &Scene, out: &Path) -> Result<(), String> {
    let dom = SharedDom::default();
    install(lua, &dom).map_err(|e| format!("failed to install DataModel: {e}"))?;
    install_vocabulary(lua).map_err(|e| format!("failed to install vocabulary: {e}"))?;

    let root_id = dom
        .lock()
        .map_err(|_| "dom lock")?
        .insert("ScreenGui".into(), "DewRoot".into());
    let root_handle = handle(lua, &dom, root_id).map_err(|e| format!("root handle: {e}"))?;

    let build_fn: mlua::Function = lua
        .load(TREE_BUILDER)
        .eval()
        .map_err(|e| format!("tree builder: {e}"))?;
    let tree_table: Table = lua
        .registry_value(&scene.tree_val)
        .map_err(|e| format!("tree lookup: {e}"))?;
    build_fn
        .call::<Value>((tree_table, root_handle))
        .map_err(|e| format!("the tree failed to build: {e}"))?;

    let frame = frame_of(&dom, root_id, scene.surface.width, scene.surface.height);

    let mut painter = RasterPainter::new(
        scene.surface.width as u32,
        scene.surface.height as u32,
        Backend::VelloCpu,
    )
    .ok_or("failed to create raster painter")?;
    if let Some(font) = crate::services::face() {
        painter = painter.with_font(font);
    }
    painter.paint_frame(&frame, None);

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let path_str = out.to_string_lossy().to_string();
    painter
        .write_png(&path_str)
        .map_err(|code| format!("write_png failed with code {code}"))?;
    Ok(())
}

/// What the gallery demonstrates, against the standard's own denominator.
pub struct Coverage {
    pub in_scope: BTreeSet<String>,
    pub demonstrated: BTreeSet<String>,
    pub missing: Vec<String>,
    /// (class, property) pairs the scenes set that are in scope for that class.
    pub pairs_demonstrated: usize,
    pub pairs_in_scope: usize,
}

impl Coverage {
    pub fn total(&self) -> usize {
        self.in_scope.len()
    }
    pub fn count(&self) -> usize {
        self.demonstrated.len()
    }
}

/// Measure the scenes against [`crate::scope`].
///
/// THE DENOMINATOR IS NOT COMPUTED HERE. It comes from `scope`, the same module
/// `datamodel-surface` reads, so the two tools cannot drift into disagreeing
/// about what the standard covers.
pub fn coverage(scenes: &[Scene]) -> Coverage {
    let in_scope = crate::scope::in_scope_properties();
    let by_class = crate::scope::in_scope_by_class();

    let mut demonstrated: BTreeSet<String> = BTreeSet::new();
    let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for scene in scenes {
        for p in &scene.demonstrates {
            if in_scope.contains(p) {
                demonstrated.insert(p.clone());
            }
        }
        for (c, p) in &scene.demonstrates_by_class {
            if by_class.get(c).map(|s| s.contains(p)).unwrap_or(false) {
                pairs.insert((c.clone(), p.clone()));
            }
        }
    }

    let pairs_in_scope: usize = by_class.values().map(|s| s.len()).sum();
    let missing: Vec<String> = in_scope.difference(&demonstrated).cloned().collect();

    Coverage {
        in_scope,
        demonstrated,
        missing,
        pairs_demonstrated: pairs.len(),
        pairs_in_scope,
    }
}

/// Load every scene in a directory.
pub fn load_all(lua: &Lua, dir: &Path) -> Result<Vec<Scene>, String> {
    let mut scenes = Vec::new();
    for path in scene_paths(dir)? {
        scenes.push(decode_scene(lua, &path)?);
    }
    Ok(scenes)
}

/// Names of the classes the gallery instantiates, for the report.
pub fn classes_used(scenes: &[Scene]) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for s in scenes {
        for c in &s.classes {
            *out.entry(c.clone()).or_insert(0) += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A UNIQUE FILE PER CALL. Both tests below used one path named by process
    /// id, and cargo runs tests in parallel threads: they raced on the same
    /// file, and the pair passed individually while failing together. A shared
    /// temp path is a flake waiting for CI.
    static PROBE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn scene_from(src: &str) -> Result<Scene, String> {
        let n = PROBE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("dew_gallery_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("probe_{n}.luau"));
        std::fs::write(&path, src).unwrap();
        let lua = Lua::new();
        let r = decode_scene(&lua, &path);
        let _ = std::fs::remove_file(&path);
        r
    }

    /// ADR-005, ENFORCED RATHER THAN WRITTEN DOWN.
    ///
    /// A scene carrying `provenance` is a conformance case in the wrong
    /// directory, and the next person to read it would reasonably believe
    /// someone had checked it in Studio. Refused, not ignored.
    #[test]
    fn a_scene_carrying_conformance_fields_is_refused() {
        // `match` rather than `expect_err`, which would require `Scene: Debug`
        // and a Debug on a `RegistryKey` for no benefit to the assertion.
        let err = match scene_from(
            r#"return { name = "x", provenance = "roblox",
                       tree = { class = "Frame", name = "R", props = {} } }"#,
        ) {
            Ok(_) => panic!("a scene with provenance must be refused"),
            Err(e) => e,
        };
        assert!(err.contains("provenance"), "{err}");
        assert!(err.contains("ADR-005"), "{err}");
    }

    #[test]
    fn a_plain_scene_reports_what_it_sets() {
        let scene = scene_from(
            r#"return {
                name = "x",
                surface = { width = 10, height = 10 },
                tree = {
                    class = "Frame", name = "R",
                    props = { BackgroundTransparency = 0.5 },
                    children = {
                        { class = "TextLabel", name = "T", props = { Text = "hi", TextSize = 12 } },
                    },
                },
            }"#,
        )
        .unwrap_or_else(|e| panic!("a plain scene loads: {e}"));
        assert!(scene.demonstrates.contains("BackgroundTransparency"));
        assert!(scene.demonstrates.contains("TextSize"));
        assert!(scene
            .demonstrates_by_class
            .contains(&("TextLabel".to_string(), "Text".to_string())));
        // And NOT attributed to the wrong class: counting by name across classes
        // is how one class's answer once masked another's.
        assert!(!scene
            .demonstrates_by_class
            .contains(&("Frame".to_string(), "Text".to_string())));
    }

    /// The gallery counts against `scope`, not against a list of its own.
    #[test]
    fn coverage_measures_against_the_shared_denominator() {
        let cov = coverage(&[]);
        assert_eq!(cov.total(), crate::scope::in_scope_properties().len());
        assert_eq!(cov.count(), 0, "no scenes demonstrates nothing");
        assert_eq!(cov.missing.len(), cov.total());
    }
}
