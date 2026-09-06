//! The Rust Conformance Runner (DataModel Standard: layout).
//!
//! Evaluates cases in `conformance/cases/*.luau` against Dew's native DataModel
//! and layout renderer (`dew_host::datamodel::render::frame_of`).
//!
//! WHY THIS EXISTS (ADR-001, Milestone 3 Vision):
//! There are at least three implementations of the DataModel standard:
//! Roblox engine, Dew native DataModel, and Aether's Luau implementation.
//! The cases are declared as pure data tables so that any runner can execute them.
//! This runner exercises Dew's real DataModel path (Instance.new, property setters,
//! arena storage, and render::frame_of) rather than a mock fixture.

use crate::datamodel::{install, install_vocabulary, render::frame_of, SharedDom};
use dew_raster::Backend;
use dew_runtime::{Painter, RasterPainter};
use mlua::prelude::*;
use mlua::{Function, Table, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const TOLERANCE: f32 = 0.01;

/// Supported layout features declared by Dew's native layout engine.
///
/// Features not in this list that are requested via a case's `requires` field
/// are reported as UNSUPPORTED rather than failed.
pub static SUPPORTS: &[&str] = &[
    "AnchorPoint",
    "ClipsDescendants",
    "UICorner",
    "UIGradient.Radial",
    "UIStroke",
    "Visible",
    "ZIndex",
];

#[derive(Debug, Clone)]
pub struct SurfaceSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct ExpectedNode {
    pub name: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub w: Option<f32>,
    pub h: Option<f32>,
    pub other_numbers: HashMap<String, f32>,
    pub other_strings: HashMap<String, String>,
    pub other_bools: HashMap<String, bool>,
}

#[derive(Debug, Clone)]
pub struct Ratio {
    pub of: String,
    pub to: String,
    pub field: String,
    pub expect: Option<f32>,
    pub tolerance: Option<f32>,
    pub min: Option<f32>,
    pub integral: Option<bool>,
}

/// A pixel probe at integer surface coordinates (x, y).
#[derive(Debug, Clone)]
pub struct PixelProbe {
    pub x: u32,
    pub y: u32,
    pub expected: [u8; 4], // RGBA
    pub divergent: Option<[u8; 4]>,
    pub tolerance: u8,
    pub note: Option<String>,
}

/// Options controlling conformance suite execution.
#[derive(Debug, Clone)]
pub struct PixelOptions {
    pub enabled: bool,
    pub generate_goldens: bool,
    pub run_unsupported: bool,
    pub tolerance: u8,
}

impl Default for PixelOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            generate_goldens: false,
            run_unsupported: false,
            tolerance: 8,
        }
    }
}

#[derive(Debug)]
pub struct Case {
    pub file_stem: String,
    pub name: String,
    pub provenance: String,
    pub verified_against: Option<String>,
    pub requires: Vec<String>,
    pub note: Option<String>,
    pub surface: SurfaceSize,
    pub expect: Vec<ExpectedNode>,
    pub ratios: Vec<Ratio>,
    pub order: Option<Vec<String>>,
    pub expect_absent: Option<Vec<String>>,
    pub pixels: Vec<PixelProbe>,
    pub golden: Option<String>,
    pub tree_val: mlua::RegistryKey,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaseStatus {
    Pass,
    Divergent,
    OpenQuestion {
        detail: String,
        note: Option<String>,
    },
    Fail {
        detail: String,
        remediation: Option<String>,
    },
    Unsupported {
        feature: String,
    },
    Undecodable {
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct CaseResult {
    pub file_stem: String,
    pub name: String,
    pub provenance: String,
    pub status: CaseStatus,
    pub note: Option<String>,
}

/// Find the `conformance/goldens` directory located alongside `conformance/cases`.
pub fn find_goldens_dir(cases_dir: &Path) -> PathBuf {
    cases_dir
        .parent()
        .map(|p| p.join("goldens"))
        .unwrap_or_else(|| PathBuf::from("conformance/goldens"))
}

/// Decode an RGBA color value from a Lua value (hex string or table).
fn decode_color(val: &Value) -> Result<[u8; 4], String> {
    match val {
        Value::String(s) => {
            let hex = s.to_str().map_err(|e| e.to_string())?;
            let clean = hex.trim_start_matches('#');
            match clean.len() {
                6 => {
                    let r = u8::from_str_radix(&clean[0..2], 16).map_err(|e| e.to_string())?;
                    let g = u8::from_str_radix(&clean[2..4], 16).map_err(|e| e.to_string())?;
                    let b = u8::from_str_radix(&clean[4..6], 16).map_err(|e| e.to_string())?;
                    Ok([r, g, b, 255])
                }
                8 => {
                    let r = u8::from_str_radix(&clean[0..2], 16).map_err(|e| e.to_string())?;
                    let g = u8::from_str_radix(&clean[2..4], 16).map_err(|e| e.to_string())?;
                    let b = u8::from_str_radix(&clean[4..6], 16).map_err(|e| e.to_string())?;
                    let a = u8::from_str_radix(&clean[6..8], 16).map_err(|e| e.to_string())?;
                    Ok([r, g, b, a])
                }
                _ => Err(format!("invalid hex color: '{hex}'")),
            }
        }
        Value::Table(t) => {
            if let Ok(first) = t.get::<String>(1) {
                if first == "Color3" {
                    let r: f32 = t.get(2).unwrap_or(0.0);
                    let g: f32 = t.get(3).unwrap_or(0.0);
                    let b: f32 = t.get(4).unwrap_or(0.0);
                    return Ok([
                        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                        255,
                    ]);
                }
            }
            if let (Ok(r), Ok(g), Ok(b)) = (t.get::<f32>(1), t.get::<f32>(2), t.get::<f32>(3)) {
                let a = t.get::<f32>(4).unwrap_or(255.0);
                let to_u8 = |v: f32| -> u8 {
                    if v <= 1.0 && v > 0.0 {
                        (v * 255.0).round() as u8
                    } else {
                        v.clamp(0.0, 255.0).round() as u8
                    }
                };
                return Ok([to_u8(r), to_u8(g), to_u8(b), to_u8(a)]);
            }
            if let (Ok(r), Ok(g), Ok(b)) = (t.get::<f32>("r"), t.get::<f32>("g"), t.get::<f32>("b"))
            {
                let a = t.get::<f32>("a").unwrap_or(255.0);
                let to_u8 = |v: f32| -> u8 {
                    if v <= 1.0 && v > 0.0 {
                        (v * 255.0).round() as u8
                    } else {
                        v.clamp(0.0, 255.0).round() as u8
                    }
                };
                return Ok([to_u8(r), to_u8(g), to_u8(b), to_u8(a)]);
            }
            Err("could not decode color from table".to_string())
        }
        other => Err(format!("unexpected color value: {:?}", other.type_name())),
    }
}

/// Find the `conformance/cases` directory across possible workspace layouts.
pub fn find_cases_dir(custom_dir: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(dir) = custom_dir {
        if dir.is_dir() {
            return Ok(dir.to_path_buf());
        }
        return Err(format!(
            "custom cases directory '{}' does not exist",
            dir.display()
        ));
    }

    let candidates = [
        PathBuf::from("conformance/cases"),
        PathBuf::from("../aether/conformance/cases"),
        PathBuf::from("../../aether/conformance/cases"),
    ];

    for c in &candidates {
        if c.is_dir() {
            return Ok(c.clone());
        }
    }

    // Check sibling aether directory relative to workspace root
    if let Ok(cur) = std::env::current_dir() {
        let mut p = cur.clone();
        loop {
            let candidate = p.join("aether").join("conformance").join("cases");
            if candidate.is_dir() {
                return Ok(candidate);
            }
            let candidate2 = p.join("conformance").join("cases");
            if candidate2.is_dir() {
                return Ok(candidate2);
            }
            if !p.pop() {
                break;
            }
        }
    }

    // Check installed pesde package for aether
    if let Some(pkg) = dew_runtime::installed_package("aether") {
        let candidate = pkg.join("conformance").join("cases");
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(
        "could not find 'conformance/cases' directory (checked local, sibling, and pesde paths)"
            .to_string(),
    )
}

/// Decode a case file using Lua table evaluation.
pub fn decode_case(lua: &Lua, path: &Path) -> Result<Case, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;

    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "case".to_string());

    let val: Value = lua
        .load(&source)
        .set_name(path.to_string_lossy().as_ref())
        .eval()
        .map_err(|e| format!("syntax/eval error in {}: {e}", path.display()))?;

    let table = match val {
        Value::Table(t) => t,
        other => {
            return Err(format!(
                "case must return a table, got {:?}",
                other.type_name()
            ))
        }
    };

    let name: String = table
        .get("name")
        .map_err(|e| format!("missing/invalid 'name': {e}"))?;
    let provenance: String = table
        .get("provenance")
        .unwrap_or_else(|_| "asserted".to_string());
    let verified_against: Option<String> = table.get("verifiedAgainst").ok();
    let note: Option<String> = table.get("note").ok();

    let requires: Vec<String> = table
        .get::<Table>("requires")
        .map(|t| t.sequence_values::<String>().flatten().collect())
        .unwrap_or_default();

    let surface_table = table.get::<Table>("surface").ok();
    let surface = if let Some(st) = surface_table {
        SurfaceSize {
            width: st.get("width").unwrap_or(200.0),
            height: st.get("height").unwrap_or(100.0),
        }
    } else {
        SurfaceSize {
            width: 200.0,
            height: 100.0,
        }
    };

    let tree_table: Table = table
        .get("tree")
        .map_err(|e| format!("missing/invalid 'tree': {e}"))?;
    let tree_val = lua
        .create_registry_value(tree_table)
        .map_err(|e| format!("registry error: {e}"))?;

    let mut expect = Vec::new();
    if let Ok(exp_table) = table.get::<Table>("expect") {
        for v in exp_table.sequence_values::<Table>() {
            let item = v.map_err(|e| format!("invalid expect entry: {e}"))?;
            let node_name: String = item
                .get("name")
                .map_err(|e| format!("expect entry missing 'name': {e}"))?;
            let mut other_numbers = HashMap::new();
            let mut other_strings = HashMap::new();
            let mut other_bools = HashMap::new();
            let mut x = None;
            let mut y = None;
            let mut w = None;
            let mut h = None;

            for (k, val) in item.pairs::<String, Value>().flatten() {
                match k.as_str() {
                    "name" => {}
                    "x" => {
                        if let Value::Number(n) = val {
                            x = Some(n as f32);
                        } else if let Value::Integer(n) = val {
                            x = Some(n as f32);
                        }
                    }
                    "y" => {
                        if let Value::Number(n) = val {
                            y = Some(n as f32);
                        } else if let Value::Integer(n) = val {
                            y = Some(n as f32);
                        }
                    }
                    "w" => {
                        if let Value::Number(n) = val {
                            w = Some(n as f32);
                        } else if let Value::Integer(n) = val {
                            w = Some(n as f32);
                        }
                    }
                    "h" => {
                        if let Value::Number(n) = val {
                            h = Some(n as f32);
                        } else if let Value::Integer(n) = val {
                            h = Some(n as f32);
                        }
                    }
                    _ => match val {
                        Value::Number(n) => {
                            other_numbers.insert(k, n as f32);
                        }
                        Value::Integer(n) => {
                            other_numbers.insert(k, n as f32);
                        }
                        Value::String(s) => {
                            if let Ok(s_str) = s.to_str() {
                                other_strings.insert(k, s_str.to_string());
                            }
                        }
                        Value::Boolean(b) => {
                            other_bools.insert(k, b);
                        }
                        _ => {}
                    },
                }
            }

            expect.push(ExpectedNode {
                name: node_name,
                x,
                y,
                w,
                h,
                other_numbers,
                other_strings,
                other_bools,
            });
        }
    }

    let mut ratios = Vec::new();
    if let Ok(rat_table) = table.get::<Table>("ratios") {
        for v in rat_table.sequence_values::<Table>() {
            let item = v.map_err(|e| format!("invalid ratio entry: {e}"))?;
            let of: String = item
                .get("of")
                .map_err(|e| format!("ratio entry missing 'of': {e}"))?;
            let to: String = item
                .get("to")
                .map_err(|e| format!("ratio entry missing 'to': {e}"))?;
            let field: String = item
                .get("field")
                .map_err(|e| format!("ratio entry missing 'field': {e}"))?;
            let exp_val: Option<f32> = item.get::<f32>("expect").ok();
            let tolerance: Option<f32> = item.get::<f32>("tolerance").ok();
            let min: Option<f32> = item.get::<f32>("min").ok();
            let integral: Option<bool> = item.get::<bool>("integral").ok();

            ratios.push(Ratio {
                of,
                to,
                field,
                expect: exp_val,
                tolerance,
                min,
                integral,
            });
        }
    }

    let order = table
        .get::<Table>("order")
        .ok()
        .map(|t| t.sequence_values::<String>().flatten().collect());

    let expect_absent = table
        .get::<Table>("expectAbsent")
        .ok()
        .map(|t| t.sequence_values::<String>().flatten().collect());

    let mut pixels = Vec::new();
    if let Ok(pix_table) = table.get::<Table>("pixels") {
        for v in pix_table.sequence_values::<Table>() {
            let item = v.map_err(|e| format!("invalid pixel entry: {e}"))?;
            let x: u32 = item
                .get("x")
                .map_err(|e| format!("pixel probe missing 'x': {e}"))?;
            let y: u32 = item
                .get("y")
                .map_err(|e| format!("pixel probe missing 'y': {e}"))?;
            let color_val: Value = item
                .get("color")
                .map_err(|e| format!("pixel probe missing 'color': {e}"))?;
            let expected = decode_color(&color_val)?;
            let divergent = item
                .get::<Value>("divergentColor")
                .ok()
                .and_then(|v| decode_color(&v).ok());
            let tolerance: u8 = item.get("tolerance").unwrap_or(5);
            let note: Option<String> = item.get("note").ok();
            pixels.push(PixelProbe {
                x,
                y,
                expected,
                divergent,
                tolerance,
                note,
            });
        }
    }

    let golden = match table.get::<Value>("golden") {
        Ok(Value::String(s)) => s.to_str().ok().map(|s| s.to_string()),
        Ok(Value::Boolean(true)) => Some(format!("{file_stem}.png")),
        _ => None,
    };

    Ok(Case {
        file_stem,
        name,
        provenance,
        verified_against,
        requires,
        note,
        surface,
        expect,
        ratios,
        order,
        expect_absent,
        pixels,
        golden,
        tree_val,
    })
}

/// Diagnoses the likely missing rule/feature based on the case contents and mismatch.
fn diagnose_remediation(case: &Case, detail: &str) -> String {
    let mut needs = Vec::new();
    let name_lower = case.name.to_lowercase();
    let stem_lower = case.file_stem.to_lowercase();

    if name_lower.contains("automaticsize") || stem_lower.contains("automatic_size") {
        needs.push("AutomaticSize layout resolution on container");
    }
    if name_lower.contains("uilistlayout") || stem_lower.contains("layout") {
        needs.push("UIListLayout sequential position calculation");
    }
    if name_lower.contains("padding") || stem_lower.contains("padding") {
        needs.push("UIPadding content box insets");
    }
    if name_lower.contains("text measurement")
        || stem_lower.contains("text_measurement")
        || stem_lower.contains("line_height")
    {
        needs.push("text bounds and glyph metric integration with layout");
    }
    if detail.contains("radial") || name_lower.contains("radial") || stem_lower.contains("radial") {
        needs.push("radial UIGradient rendering");
    }
    if detail.contains("paint order")
        || name_lower.contains("zindex")
        || stem_lower.contains("zindex")
    {
        needs.push("ZIndex paint sequence sorting");
    }
    if detail.contains("radius") || detail.contains("clip") {
        needs.push("clip radius in display list");
    }

    if needs.is_empty() {
        if detail.contains("AutomaticSize") {
            needs.push("AutomaticSize solver");
        } else if detail.contains("Layout") {
            needs.push("layout container solver");
        } else {
            needs.push("native layout rule implementation");
        }
    }

    needs.join(", ")
}

/// Execute a single case against Dew's native DataModel using default options.
pub fn run_case(lua: &Lua, case: &Case) -> CaseResult {
    run_case_with_options(lua, case, &PixelOptions::default(), None)
}

/// Execute a single case against Dew's native DataModel with configurable pixel/golden options.
pub fn run_case_with_options(
    lua: &Lua,
    case: &Case,
    options: &PixelOptions,
    cases_dir: Option<&Path>,
) -> CaseResult {
    // 1. Check feature support declared in `requires`
    if !options.run_unsupported {
        for req in &case.requires {
            if !SUPPORTS.contains(&req.as_str()) {
                return CaseResult {
                    file_stem: case.file_stem.clone(),
                    name: case.name.clone(),
                    provenance: case.provenance.clone(),
                    status: CaseStatus::Unsupported {
                        feature: req.clone(),
                    },
                    note: Some("unsupported".to_string()),
                };
            }
        }
    }

    // 2. A case that asserts nothing cannot pass
    if case.expect.is_empty()
        && case.ratios.is_empty()
        && case.order.is_none()
        && case.pixels.is_empty()
        && case.golden.is_none()
    {
        return CaseResult {
            file_stem: case.file_stem.clone(),
            name: case.name.clone(),
            provenance: case.provenance.clone(),
            status: CaseStatus::Fail {
                detail: "the case asserts nothing".to_string(),
                remediation: None,
            },
            note: case.note.clone(),
        };
    }

    // 3. Set up Dew's DataModel environment
    let dom = SharedDom::default();
    if let Err(e) = install(lua, &dom) {
        return CaseResult {
            file_stem: case.file_stem.clone(),
            name: case.name.clone(),
            provenance: case.provenance.clone(),
            status: CaseStatus::Fail {
                detail: format!("failed to install DataModel: {e}"),
                remediation: None,
            },
            note: case.note.clone(),
        };
    }
    if let Err(e) = install_vocabulary(lua) {
        return CaseResult {
            file_stem: case.file_stem.clone(),
            name: case.name.clone(),
            provenance: case.provenance.clone(),
            status: CaseStatus::Fail {
                detail: format!("failed to install vocabulary: {e}"),
                remediation: None,
            },
            note: case.note.clone(),
        };
    }

    // Root instance in DOM
    let root_id = dom
        .lock()
        .expect("dom")
        .insert("ScreenGui".into(), "DewRoot".into());
    let root_handle = match crate::datamodel::handle(lua, &dom, root_id) {
        Ok(h) => h,
        Err(e) => {
            return CaseResult {
                file_stem: case.file_stem.clone(),
                name: case.name.clone(),
                provenance: case.provenance.clone(),
                status: CaseStatus::Fail {
                    detail: format!("failed to create root handle: {e}"),
                    remediation: None,
                },
                note: case.note.clone(),
            };
        }
    };

    // 4. Build the tree through Dew's Instance.new and property setters
    let builder_chunk = r#"
        local decode_val
        function decode_val(v)
            if type(v) ~= "table" then return v end
            local k = v[1]
            if k == "UDim2" then return UDim2.new(v[2], v[3], v[4], v[5])
            elseif k == "UDim" then return UDim.new(v[2], v[3])
            elseif k == "Color3" then return Color3.new(v[2], v[3], v[4])
            elseif k == "Vector2" then return Vector2.new(v[2], v[3])
            elseif k == "Rect" then return Rect.new(v[2], v[3], v[4], v[5])
            elseif k == "Enum" then
                local cat = Enum[v[2]]
                if not cat then error("no enum category " .. tostring(v[2])) end
                local mem = cat[v[3]]
                if not mem then error("no enum member " .. tostring(v[3])) end
                return mem
            end
            error("unknown encoded type " .. tostring(k))
        end

        local function build_node(node, parent)
            local inst = Instance.new(node.class)
            if node.name then inst.Name = node.name end
            for k, v in pairs(node.props or {}) do
                inst[k] = decode_val(v)
            end
            inst.Parent = parent
            for _, child in ipairs(node.children or {}) do
                build_node(child, inst)
            end
            return inst
        end

        return function(tree, root)
            return build_node(tree, root)
        end
    "#;

    let build_fn: mlua::Function = match lua.load(builder_chunk).eval() {
        Ok(f) => f,
        Err(e) => {
            return CaseResult {
                file_stem: case.file_stem.clone(),
                name: case.name.clone(),
                provenance: case.provenance.clone(),
                status: CaseStatus::Fail {
                    detail: format!("failed to create tree builder: {e}"),
                    remediation: None,
                },
                note: case.note.clone(),
            };
        }
    };

    let tree_table: Table = match lua.registry_value(&case.tree_val) {
        Ok(t) => t,
        Err(e) => {
            return CaseResult {
                file_stem: case.file_stem.clone(),
                name: case.name.clone(),
                provenance: case.provenance.clone(),
                status: CaseStatus::Fail {
                    detail: format!("tree registry lookup failed: {e}"),
                    remediation: None,
                },
                note: case.note.clone(),
            };
        }
    };

    if let Err(e) = build_fn.call::<Value>((tree_table, root_handle)) {
        return CaseResult {
            file_stem: case.file_stem.clone(),
            name: case.name.clone(),
            provenance: case.provenance.clone(),
            status: CaseStatus::Fail {
                detail: format!("the tree failed to build: {e}"),
                remediation: Some("DataModel property or class support".to_string()),
            },
            note: case.note.clone(),
        };
    }

    // 5. Render the frame through Dew's layout and display list builder
    let frame = frame_of(&dom, root_id, case.surface.width, case.surface.height);

    // Index generated nodes by name
    let mut by_name = HashMap::new();
    for node in &frame.nodes {
        by_name.insert(node.name.clone(), node);
    }

    // 6. Evaluate expectations
    for exp in &case.expect {
        let actual = match by_name.get(&exp.name) {
            Some(n) => *n,
            None => {
                let detail = format!(
                    "no node named '{}' in the display list (a node solved to zero width or height is dropped, so it may have been built and measured to nothing)",
                    exp.name
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        };

        if let Some(expected_x) = exp.x {
            if (actual.rect.x - expected_x).abs() > TOLERANCE {
                let detail = format!(
                    "{}: x: expected {expected_x}, got {}",
                    exp.name, actual.rect.x
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        }
        if let Some(expected_y) = exp.y {
            if (actual.rect.y - expected_y).abs() > TOLERANCE {
                let detail = format!(
                    "{}: y: expected {expected_y}, got {}",
                    exp.name, actual.rect.y
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        }
        if let Some(expected_w) = exp.w {
            if (actual.rect.w - expected_w).abs() > TOLERANCE {
                let detail = format!(
                    "{}: w: expected {expected_w}, got {}",
                    exp.name, actual.rect.w
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        }
        if let Some(expected_h) = exp.h {
            if (actual.rect.h - expected_h).abs() > TOLERANCE {
                let detail = format!(
                    "{}: h: expected {expected_h}, got {}",
                    exp.name, actual.rect.h
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        }
    }

    // 7. Evaluate ratios
    for r in &case.ratios {
        let of_node = match by_name.get(&r.of) {
            Some(n) => *n,
            None => {
                let detail = format!(
                    "ratio needs '{}' and '{}'; one is missing from the display list",
                    r.of, r.to
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        };
        let to_node = match by_name.get(&r.to) {
            Some(n) => *n,
            None => {
                let detail = format!(
                    "ratio needs '{}' and '{}'; one is missing from the display list",
                    r.of, r.to
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        };

        let read_field = |node: &dew_runtime::Node, field: &str| -> Option<f32> {
            match field {
                "textHeight" => Some(node.text_size),
                "x" => Some(node.rect.x),
                "y" => Some(node.rect.y),
                "w" => Some(node.rect.w),
                "h" => Some(node.rect.h),
                _ => None,
            }
        };

        let num = match read_field(of_node, &r.field) {
            Some(v) => v,
            None => {
                let detail = format!("node '{}' has no measurable field '{}'", r.of, r.field);
                return finalize_result(case, detail, None);
            }
        };
        let den = match read_field(to_node, &r.field) {
            Some(v) => v,
            None => {
                let detail = format!("node '{}' has no measurable field '{}'", r.to, r.field);
                return finalize_result(case, detail, None);
            }
        };

        if den == 0.0 {
            let detail = format!(
                "ratio {}.{} / {}.{}: the denominator is zero",
                r.of, r.field, r.to, r.field
            );
            let remediation = diagnose_remediation(case, &detail);
            return finalize_result(case, detail, Some(remediation));
        }

        let actual = num / den;

        if let Some(true) = r.integral {
            let nearest = (actual + 0.5).floor();
            let slack = r.tolerance.unwrap_or(0.02);
            if nearest < 1.0 || (actual - nearest).abs() > slack {
                let detail = format!(
                    "ratio {}.{} / {}.{}: expected a whole multiple, got {:.4}",
                    r.of, r.field, r.to, r.field, actual
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        } else if let Some(min_val) = r.min {
            if actual < min_val {
                let detail = format!(
                    "ratio {}.{} / {}.{}: expected at least {:.3}, got {:.3}",
                    r.of, r.field, r.to, r.field, min_val, actual
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        } else if let Some(exp_val) = r.expect {
            let slack = r.tolerance.unwrap_or(0.02);
            if (actual - exp_val).abs() > slack {
                let detail = format!(
                    "ratio {}.{} / {}.{}: expected {:.3} +/- {:.3}, got {:.3}",
                    r.of, r.field, r.to, r.field, exp_val, slack, actual
                );
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        }
    }

    // 8. Evaluate paint order
    if let Some(ref wanted_order) = case.order {
        let mut seen = Vec::new();
        for node in &frame.nodes {
            if wanted_order.contains(&node.name) {
                seen.push(node.name.clone());
            }
        }
        if seen != *wanted_order {
            let detail = format!(
                "paint order: expected {}, got {}",
                wanted_order.join(" < "),
                seen.join(" < ")
            );
            let remediation = Some("ZIndex paint sequence sorting".to_string());
            return finalize_result(case, detail, remediation);
        }
    }

    // 9. Evaluate expectAbsent
    if let Some(ref absent_list) = case.expect_absent {
        for absent in absent_list {
            if by_name.contains_key(absent) {
                let detail =
                    format!("'{absent}' is in the display list and the case expects it absent");
                let remediation = diagnose_remediation(case, &detail);
                return finalize_result(case, detail, Some(remediation));
            }
        }
    }

    // 10. Evaluate pixel probes and goldens if pixel execution is enabled
    let mut matched_divergent = false;
    if options.enabled {
        let width = case.surface.width as usize;
        let height = case.surface.height as usize;
        let mut painter = match RasterPainter::new(
            case.surface.width as u32,
            case.surface.height as u32,
            Backend::VelloCpu,
        ) {
            Some(p) => p,
            None => {
                return finalize_result(case, "failed to create raster painter".to_string(), None);
            }
        };
        if let Some(font) = crate::services::face() {
            painter = painter.with_font(font);
        }
        painter.paint_frame(&frame, None);

        let bgra = match painter.canvas_mut().bgra() {
            Some(b) => b,
            None => {
                return finalize_result(case, "rasteriser produced no pixels".to_string(), None);
            }
        };

        // Convert BGRA to row-major RGBA
        let mut rgba = vec![0u8; width * height * 4];
        for i in 0..(width * height) {
            rgba[i * 4] = bgra[i * 4 + 2];
            rgba[i * 4 + 1] = bgra[i * 4 + 1];
            rgba[i * 4 + 2] = bgra[i * 4];
            rgba[i * 4 + 3] = bgra[i * 4 + 3];
        }

        // 10a. Probe pixels
        for probe in &case.pixels {
            let x = probe.x as usize;
            let y = probe.y as usize;
            if x >= width || y >= height {
                let detail =
                    format!("probe at ({x}, {y}) is out of surface bounds ({width}x{height})");
                return finalize_result(case, detail, None);
            }
            let idx = (y * width + x) * 4;
            let actual = [rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3]];
            let delta = (0..4)
                .map(|c| actual[c].abs_diff(probe.expected[c]))
                .max()
                .unwrap_or(0);
            if delta <= probe.tolerance {
                continue;
            }
            if let Some(div) = probe.divergent {
                let div_delta = (0..4)
                    .map(|c| actual[c].abs_diff(div[c]))
                    .max()
                    .unwrap_or(0);
                if div_delta <= probe.tolerance {
                    matched_divergent = true;
                    continue;
                }
            }
            let detail = format!(
                "pixel at ({x}, {y}): expected #{:02x}{:02x}{:02x}{:02x}, got #{:02x}{:02x}{:02x}{:02x} (delta {delta}){}",
                probe.expected[0],
                probe.expected[1],
                probe.expected[2],
                probe.expected[3],
                actual[0],
                actual[1],
                actual[2],
                actual[3],
                probe
                    .note
                    .as_deref()
                    .map(|n| format!(" ({n})"))
                    .unwrap_or_default()
            );
            let remediation = diagnose_remediation(case, &detail);
            return finalize_result(case, detail, Some(remediation));
        }

        // 10b. Golden comparison / generation
        if let Some(dir) = cases_dir {
            let goldens_dir = find_goldens_dir(dir);
            let golden_name = case
                .golden
                .clone()
                .unwrap_or_else(|| format!("{}.png", case.file_stem));
            let golden_path = goldens_dir.join(&golden_name);

            if options.generate_goldens {
                if !case.pixels.is_empty() || case.golden.is_some() {
                    let _ = std::fs::create_dir_all(&goldens_dir);
                    let path_str = golden_path.to_string_lossy();
                    if let Err(code) = painter.write_png(&path_str) {
                        eprintln!(
                            "failed to write golden {}: code {code}",
                            golden_path.display()
                        );
                    } else {
                        println!(
                            "[golden] generated reference image {}",
                            golden_path.display()
                        );
                    }
                }
            } else if golden_path.is_file() {
                match image::open(&golden_path) {
                    Ok(img) => {
                        let golden_rgba = img.to_rgba8();
                        if golden_rgba.width() != case.surface.width as u32
                            || golden_rgba.height() != case.surface.height as u32
                        {
                            let detail = format!(
                                "golden dimensions mismatch: expected {}x{}, got {}x{}",
                                case.surface.width,
                                case.surface.height,
                                golden_rgba.width(),
                                golden_rgba.height()
                            );
                            return finalize_result(case, detail, None);
                        }
                        let golden_raw = golden_rgba.as_raw();
                        let mut mismatched_pixels = 0usize;
                        let mut max_delta = 0u8;
                        let mut diff_pixels = Vec::with_capacity(rgba.len());

                        for i in 0..(width * height) {
                            let p_act = &rgba[i * 4..i * 4 + 4];
                            let p_gold = &golden_raw[i * 4..i * 4 + 4];
                            let d = (0..4)
                                .map(|c| p_act[c].abs_diff(p_gold[c]))
                                .max()
                                .unwrap_or(0);
                            if d > options.tolerance {
                                mismatched_pixels += 1;
                                max_delta = max_delta.max(d);
                                diff_pixels.extend_from_slice(&[255, 0, 255, 255]);
                            } else {
                                let gray = ((p_act[0] as u16 + p_act[1] as u16 + p_act[2] as u16)
                                    / 6) as u8;
                                diff_pixels.extend_from_slice(&[gray, gray, gray, p_act[3] / 2]);
                            }
                        }

                        let mismatch_ratio = mismatched_pixels as f32 / (width * height) as f32;
                        // 1.0% tolerance for cross-platform rasteriser antialiasing contour differences
                        if mismatch_ratio > 0.01 {
                            let diffs_dir = if Path::new("target").is_dir() {
                                PathBuf::from("target/conformance_diffs")
                            } else {
                                goldens_dir.join(".diffs")
                            };
                            let _ = std::fs::create_dir_all(&diffs_dir);
                            let diff_path = diffs_dir.join(format!("{}_diff.png", case.file_stem));
                            if let Some(diff_img) =
                                image::RgbaImage::from_raw(width as u32, height as u32, diff_pixels)
                            {
                                let _ = diff_img.save(&diff_path);
                            }
                            let detail = format!(
                                "golden mismatch: {:.2}% pixels differ (max delta: {}) [diff written to {}]",
                                mismatch_ratio * 100.0,
                                max_delta,
                                diff_path.display()
                            );
                            let remediation = diagnose_remediation(case, &detail);
                            return finalize_result(case, detail, Some(remediation));
                        }
                    }
                    Err(e) => {
                        let detail =
                            format!("could not open golden {}: {e}", golden_path.display());
                        return finalize_result(case, detail, None);
                    }
                }
            }
        }
    }

    // All assertions satisfied
    let status = if matched_divergent || case.provenance == "divergent" {
        CaseStatus::Divergent
    } else {
        CaseStatus::Pass
    };

    CaseResult {
        file_stem: case.file_stem.clone(),
        name: case.name.clone(),
        provenance: case.provenance.clone(),
        status,
        note: case.note.clone(),
    }
}

fn finalize_result(case: &Case, detail: String, remediation: Option<String>) -> CaseResult {
    let status = if case.provenance == "asserted" {
        CaseStatus::OpenQuestion {
            detail,
            note: case.note.clone(),
        }
    } else {
        CaseStatus::Fail {
            detail,
            remediation,
        }
    };

    CaseResult {
        file_stem: case.file_stem.clone(),
        name: case.name.clone(),
        provenance: case.provenance.clone(),
        status,
        note: case.note.clone(),
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SummarySource {
    #[default]
    SharedModule,
    NativeFallback,
    Native,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SuiteSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub open_questions: usize,
    pub divergent: usize,
    pub unsupported: usize,
    pub undecodable: usize,
    pub verified_against_roblox: usize,
    pub disagreed_open: usize,
    pub source: SummarySource,
}

/// Pure-Rust tally calculation matching `conformance/tally.luau` semantics.
pub fn compute_summary_native(results: &[CaseResult]) -> SuiteSummary {
    let total = results.len();
    let mut passed = 0;
    let mut divergent = 0;
    let mut failed = 0;
    let mut disagreed_open = 0;
    let mut unsupported = 0;
    let mut undecodable = 0;

    for r in results {
        match &r.status {
            CaseStatus::Pass => {
                passed += 1;
                if r.provenance == "divergent" {
                    divergent += 1;
                }
            }
            CaseStatus::Divergent => {
                passed += 1;
                divergent += 1;
            }
            CaseStatus::Fail { .. } => {
                if r.provenance == "asserted" {
                    disagreed_open += 1;
                } else {
                    failed += 1;
                }
            }
            CaseStatus::OpenQuestion { .. } => {
                disagreed_open += 1;
            }
            CaseStatus::Unsupported { .. } => {
                unsupported += 1;
            }
            CaseStatus::Undecodable { .. } => {
                undecodable += 1;
            }
        }
    }

    let mut verified_against_roblox = 0;
    let mut open_questions = 0;

    for r in results {
        let is_unsupported = matches!(r.status, CaseStatus::Unsupported { .. });
        let is_undecodable = matches!(r.status, CaseStatus::Undecodable { .. });
        if !is_unsupported && !is_undecodable {
            if r.provenance == "roblox" {
                verified_against_roblox += 1;
            } else if r.provenance == "asserted" {
                open_questions += 1;
            }
        }
    }

    SuiteSummary {
        total,
        passed,
        failed,
        open_questions,
        divergent,
        unsupported,
        undecodable,
        verified_against_roblox,
        disagreed_open,
        source: SummarySource::Native,
    }
}

/// Evaluates `tally.luau` source via Lua and returns the parsed `SuiteSummary`.
pub fn compute_summary_via_lua(
    lua: &Lua,
    tally_source: &str,
    results: &[CaseResult],
) -> Result<SuiteSummary, String> {
    let tally_module = lua
        .load(tally_source)
        .set_name("tally.luau")
        .eval::<Table>()
        .map_err(|e| format!("failed to load tally.luau: {e}"))?;

    let summarize_fn: Function = tally_module
        .get("summarize")
        .map_err(|e| format!("missing 'summarize' in tally.luau: {e}"))?;

    let lua_results = lua
        .create_table()
        .map_err(|e| format!("failed to create lua table: {e}"))?;

    for (i, r) in results.iter().enumerate() {
        let item = lua
            .create_table()
            .map_err(|e| format!("failed to create result item: {e}"))?;
        let _ = item.set("name", r.name.as_str());
        let _ = item.set("provenance", r.provenance.as_str());
        let _ = item.set(
            "ok",
            matches!(r.status, CaseStatus::Pass | CaseStatus::Divergent),
        );
        let _ = item.set(
            "unsupported",
            matches!(r.status, CaseStatus::Unsupported { .. }),
        );
        let _ = item.set(
            "divergent",
            matches!(r.status, CaseStatus::Divergent) || r.provenance == "divergent",
        );
        let _ = item.set(
            "undecodable",
            matches!(r.status, CaseStatus::Undecodable { .. }),
        );
        let _ = lua_results.set(i + 1, item);
    }

    let summary_table: Table = summarize_fn
        .call(lua_results)
        .map_err(|e| format!("summarize() failed: {e}"))?;

    Ok(SuiteSummary {
        total: summary_table.get("total").unwrap_or(results.len()),
        passed: summary_table.get("passed").unwrap_or(0),
        failed: summary_table.get("failed").unwrap_or(0),
        open_questions: summary_table.get("open_questions").unwrap_or(0),
        divergent: summary_table.get("divergent").unwrap_or(0),
        unsupported: summary_table.get("unsupported").unwrap_or(0),
        undecodable: summary_table.get("undecodable").unwrap_or(0),
        verified_against_roblox: summary_table.get("verified_against_roblox").unwrap_or(0),
        disagreed_open: summary_table.get("disagreed_open").unwrap_or(0),
        source: SummarySource::SharedModule,
    })
}

/// The shared module `conformance/tally.luau` is the source both runners read to
/// produce their suite summaries, with native Rust retained in Dew as an explicit
/// fallback when running detached from Aether.
pub fn compute_summary(cases_dir: Option<&Path>, results: &[CaseResult]) -> SuiteSummary {
    if let Some(dir) = cases_dir {
        let tally_path = dir.parent().map(|p| p.join("tally.luau"));
        if let Some(ref path) = tally_path {
            match std::fs::read_to_string(path) {
                Ok(source) => {
                    let lua = Lua::new();
                    match compute_summary_via_lua(&lua, &source, results) {
                        Ok(mut summary) => {
                            summary.source = SummarySource::SharedModule;
                            return summary;
                        }
                        Err(e) => {
                            eprintln!(
                                "  warning: failed to evaluate shared tally.luau: {e}; falling back to native Rust"
                            );
                        }
                    }
                }
                Err(_) => {
                    let mut summary = compute_summary_native(results);
                    summary.source = SummarySource::NativeFallback;
                    return summary;
                }
            }
        }
    }

    let mut summary = compute_summary_native(results);
    summary.source = SummarySource::Native;
    summary
}

/// Run all conformance cases in `cases_dir` with optional filter.
pub fn run_suite(cases_dir: &Path, filter: Option<&str>) -> (Vec<CaseResult>, SuiteSummary) {
    run_suite_with_options(cases_dir, filter, &PixelOptions::default())
}

/// Run all conformance cases in `cases_dir` with pixel-level verification.
pub fn run_suite_pixel(
    cases_dir: &Path,
    filter: Option<&str>,
    generate_goldens: bool,
    run_unsupported: bool,
) -> (Vec<CaseResult>, SuiteSummary) {
    let options = PixelOptions {
        enabled: true,
        generate_goldens,
        run_unsupported,
        tolerance: 8,
    };
    run_suite_with_options(cases_dir, filter, &options)
}

/// Run conformance cases with specified pixel options.
pub fn run_suite_with_options(
    cases_dir: &Path,
    filter: Option<&str>,
    options: &PixelOptions,
) -> (Vec<CaseResult>, SuiteSummary) {
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(cases_dir) {
        Ok(read_dir) => read_dir
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|ext| ext == "luau").unwrap_or(false))
            .collect(),
        Err(_) => Vec::new(),
    };

    entries.sort();
    let mut results = Vec::new();

    for path in entries {
        let file_stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let lua = Lua::new();

        let case = match decode_case(&lua, &path) {
            Ok(c) => c,
            Err(e) => {
                results.push(CaseResult {
                    file_stem: file_stem.clone(),
                    name: file_stem.clone(),
                    provenance: "unknown".to_string(),
                    status: CaseStatus::Undecodable { error: e },
                    note: None,
                });
                continue;
            }
        };

        if let Some(f) = filter {
            let f_lower = f.to_lowercase();
            if !case.name.to_lowercase().contains(&f_lower)
                && !case.file_stem.to_lowercase().contains(&f_lower)
            {
                continue;
            }
        }

        let res = run_case_with_options(&lua, &case, options, Some(cases_dir));
        results.push(res);
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));

    let summary = compute_summary(Some(cases_dir), &results);

    (results, summary)
}

/// Print formatted report matching `conformance/run.luau` output.
pub fn print_report(results: &[CaseResult], summary: &SuiteSummary) {
    if summary.source == SummarySource::NativeFallback {
        println!("  note: shared tally.luau not found alongside cases; using native Rust summary");
    }
    println!();
    for r in results {
        match &r.status {
            CaseStatus::Unsupported { feature } => {
                println!("  -  {}  (unsupported: {})", r.name, feature);
            }
            CaseStatus::Pass => {
                println!("  ok {}  [{}]", r.name, r.provenance);
            }
            CaseStatus::Divergent => {
                println!(
                    "  ~  {}  (divergent, still matching the documented gap)",
                    r.name
                );
            }
            CaseStatus::OpenQuestion { detail, note } => {
                println!("  ?  {}  (unverified — implementation disagrees)", r.name);
                println!("       {detail}");
                if let Some(n) = note {
                    println!("       {n}");
                }
            }
            CaseStatus::Fail {
                detail,
                remediation,
            } => {
                println!("  FAIL {}  [{}]", r.name, r.provenance);
                println!("       {detail}");
                if let Some(rem) = remediation {
                    println!("       [needs: {rem}]");
                }
            }
            CaseStatus::Undecodable { error } => {
                println!("  !  {}  (undecodable: {error})", r.name);
            }
        }
    }

    let executable = summary.total - summary.unsupported;
    println!();
    println!(
        "CONFORMANCE: {} of {} passing ({} verified against Roblox, {} documented gaps, {} open questions, {} unsupported{})",
        summary.passed,
        executable,
        summary.verified_against_roblox,
        summary.divergent,
        summary.open_questions,
        summary.unsupported,
        if summary.undecodable > 0 {
            format!(", {} undecodable", summary.undecodable)
        } else {
            String::new()
        }
    );

    if summary.open_questions > 0 {
        let disagree_note = if summary.disagreed_open > 0 {
            format!(
                ", {} of which this implementation disagrees with",
                summary.disagreed_open
            )
        } else {
            String::new()
        };
        println!(
            "  {} case(s) state a belief nobody has checked in Studio{}. See conformance/README.md for how to verify one.",
            summary.open_questions, disagree_note
        );
    }

    match summary.source {
        SummarySource::SharedModule => {
            println!("  (summary: shared tally.luau)");
        }
        SummarySource::NativeFallback => {
            println!("  (summary: native Rust fallback; shared tally.luau not found)");
        }
        SummarySource::Native => {
            println!("  (summary: native Rust)");
        }
    }

    if summary.failed > 0 {
        println!();
        println!("FAILURES REQUIRING TEXT METRICS (Sprint 3 backlog):");
        for r in results {
            if let CaseStatus::Fail {
                detail,
                remediation,
            } = &r.status
            {
                println!(
                    "  - {}: {}\n      -> Requirement: {}",
                    r.name,
                    detail,
                    remediation.as_deref().unwrap_or("layout implementation")
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conformance_suite_loads_and_runs() {
        let dir = find_cases_dir(None).expect("cases dir");
        let (results, summary) = run_suite(&dir, None);
        assert_eq!(
            summary.total, 24,
            "expected 24 cases, got {}",
            summary.total
        );
        assert_eq!(summary.undecodable, 0, "expected 0 undecodable cases");
        assert_eq!(summary.unsupported, 3, "expected 3 unsupported cases");
        assert_eq!(
            summary.passed, 21,
            "expected all 21 executable conformance cases to pass"
        );
        assert_eq!(summary.failed, 0, "expected 0 failing conformance cases");
        assert_eq!(summary.open_questions, 1, "expected 1 open question");

        // Verify the 21 passing cases
        let passing_names: Vec<&str> = results
            .iter()
            .filter(|r| matches!(r.status, CaseStatus::Pass | CaseStatus::Divergent))
            .map(|r| r.name.as_str())
            .collect();
        assert!(passing_names.contains(&"scale resolves against the parent, not the surface"));
        assert!(passing_names.contains(&"scale and offset combine additively"));
        assert!(
            passing_names.contains(&"anchor point shifts an element by a fraction of its own size")
        );
        assert!(passing_names.contains(&"a node with zero area is absent from the display list"));
        assert!(passing_names.contains(&"ZIndex orders the paint, then depth, then declaration"));
        assert!(passing_names
            .contains(&"a clipped child keeps its rectangle (the radius gap is invisible here)"));
        assert!(passing_names.contains(&"a radial UIGradient reaches the display list as radial"));

        // 11 native geometry cases
        assert!(passing_names.contains(&"AnchorPoint offsets by the size AutomaticSize produced"));
        assert!(passing_names.contains(&"AutomaticSize Y grows a frame to fit its children"));
        assert!(passing_names.contains(&"AutomaticSize includes UIPadding in the measured size"));
        assert!(passing_names
            .contains(&"AutomaticSize resolves a Scale-sized child against the available space"));
        assert!(passing_names.contains(&"AutomaticSize with a FULL scale-sized child"));
        assert!(passing_names.contains(&"AutomaticSize with a Scale child and NO layout"));
        assert!(passing_names
            .contains(&"AutomaticSize with a Scale child that has content, under a layout"));
        assert!(
            passing_names.contains(&"AutomaticSize with a Scale child under a smaller grandparent")
        );
        assert!(passing_names.contains(&"AutomaticSize with a half-Scale child that HAS content"));
        assert!(passing_names.contains(&"AutomaticSize with a quarter-Scale child under a layout"));
        assert!(passing_names.contains(&"UIListLayout stacks children and ignores their Position"));

        // 3 native text metrics cases
        assert!(passing_names.contains(&"one line of text is a fixed multiple of TextSize"));
        assert!(passing_names.contains(&"text measurement is linear in TextSize"));
        assert!(passing_names.contains(&"text measurement is per glyph, not per character"));
    }

    #[test]
    fn single_case_filter_works() {
        let dir = find_cases_dir(None).expect("cases dir");
        let (results, summary) = run_suite(&dir, Some("scale resolves against the parent"));
        assert_eq!(results.len(), 1);
        assert_eq!(summary.passed, 1);
        assert_eq!(results[0].status, CaseStatus::Pass);
    }

    #[test]
    fn pixel_runner_passes_paint_order_and_golden() {
        let dir = find_cases_dir(None).expect("cases dir");
        let options = PixelOptions {
            enabled: true,
            generate_goldens: false,
            run_unsupported: false,
            tolerance: 8,
        };
        let (results, summary) =
            run_suite_with_options(&dir, Some("zindex_orders_the_paint"), &options);
        assert_eq!(results.len(), 1);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(results[0].status, CaseStatus::Pass);
    }

    #[test]
    fn pixel_runner_observes_clip_radius_divergence() {
        let dir = find_cases_dir(None).expect("cases dir");
        let options = PixelOptions {
            enabled: true,
            generate_goldens: false,
            run_unsupported: false,
            tolerance: 8,
        };
        let (results, summary) =
            run_suite_with_options(&dir, Some("clips_descendants_has_no_radius"), &options);
        assert_eq!(results.len(), 1);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.divergent, 1);
        assert_eq!(results[0].status, CaseStatus::Divergent);
    }

    #[test]
    fn radial_gradient_case_reaches_display_list_and_renders() {
        let dir = find_cases_dir(None).expect("cases dir");
        let (results, summary) = run_suite(&dir, Some("uigradient_radial"));
        assert_eq!(results.len(), 1);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.unsupported, 0);
        assert_eq!(results[0].status, CaseStatus::Pass);
    }

    #[test]
    fn pixel_runner_full_suite_passes() {
        let dir = find_cases_dir(None).expect("cases dir");
        let (_results, summary) = run_suite_pixel(&dir, None, false, false);
        assert_eq!(summary.total, 24);
        assert_eq!(summary.undecodable, 0);
        assert_eq!(summary.unsupported, 3);
        assert_eq!(summary.passed, 21);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.divergent, 1);
        assert_eq!(summary.verified_against_roblox, 18);
        assert_eq!(summary.open_questions, 1);
    }

    #[test]
    fn shared_and_native_tally_produce_identical_summaries() {
        let dir = find_cases_dir(None).expect("cases dir");
        let tally_path = dir.parent().expect("conformance dir").join("tally.luau");
        let tally_source = std::fs::read_to_string(&tally_path).expect("read tally.luau");
        let lua = Lua::new();

        // Provenance is a closed set: roblox, asserted, reference, divergent.
        // Outcome is a closed set: pass, disagree, unsupported, undecodable.
        let provenances = ["roblox", "asserted", "reference", "divergent"];

        #[derive(Debug, Clone, Copy)]
        enum Outcome {
            Pass,
            Disagree,
            Unsupported,
            Undecodable,
        }

        let outcomes = [
            Outcome::Pass,
            Outcome::Disagree,
            Outcome::Unsupported,
            Outcome::Undecodable,
        ];

        let make_case_result = |prov: &'static str, outcome: Outcome, id: usize| -> CaseResult {
            let name = format!("case_{prov}_{outcome:?}_{id}");
            let status = match (prov, outcome) {
                ("divergent", Outcome::Pass) => CaseStatus::Divergent,
                (_, Outcome::Pass) => CaseStatus::Pass,
                ("asserted", Outcome::Disagree) => CaseStatus::OpenQuestion {
                    detail: "implementation disagrees".into(),
                    note: None,
                },
                (_, Outcome::Disagree) => CaseStatus::Fail {
                    detail: "assertion failed".into(),
                    remediation: None,
                },
                (_, Outcome::Unsupported) => CaseStatus::Unsupported {
                    feature: "HypotheticalFeature".into(),
                },
                (_, Outcome::Undecodable) => CaseStatus::Undecodable {
                    error: "syntax error".into(),
                },
            };
            CaseResult {
                file_stem: name.clone(),
                name,
                provenance: prov.to_string(),
                status,
                note: None,
            }
        };

        let assert_same_tallies = |shared: &SuiteSummary, native: &SuiteSummary, ctx: &str| {
            assert_eq!(
                (
                    shared.total,
                    shared.passed,
                    shared.failed,
                    shared.open_questions,
                    shared.divergent,
                    shared.unsupported,
                    shared.undecodable,
                    shared.verified_against_roblox,
                    shared.disagreed_open,
                ),
                (
                    native.total,
                    native.passed,
                    native.failed,
                    native.open_questions,
                    native.divergent,
                    native.unsupported,
                    native.undecodable,
                    native.verified_against_roblox,
                    native.disagreed_open,
                ),
                "mismatch in tallies for {ctx}: shared={shared:?}, native={native:?}"
            );
        };

        // 1. Enumerate and test every individual combination as a single-case suite (16 suites)
        for &prov in &provenances {
            for &outcome in &outcomes {
                let single_case = vec![make_case_result(prov, outcome, 1)];
                let shared = compute_summary_via_lua(&lua, &tally_source, &single_case)
                    .expect("lua summary");
                let native = compute_summary_native(&single_case);
                assert_same_tallies(&shared, &native, &format!("single {prov} x {outcome:?}"));
            }
        }

        // 2. Test an exhaustive suite containing all 16 combinations simultaneously
        let mut all_combinations = Vec::new();
        for (i, &prov) in provenances.iter().enumerate() {
            for (j, &outcome) in outcomes.iter().enumerate() {
                all_combinations.push(make_case_result(prov, outcome, i * 4 + j));
            }
        }
        assert_eq!(all_combinations.len(), 16);
        let shared_all =
            compute_summary_via_lua(&lua, &tally_source, &all_combinations).expect("lua summary");
        let native_all = compute_summary_native(&all_combinations);
        assert_same_tallies(&shared_all, &native_all, "all 16 combinations");

        // Verify specific semantic tallies on the complete 16-combination suite:
        assert_eq!(native_all.total, 16);
        assert_eq!(native_all.unsupported, 4);
        assert_eq!(native_all.undecodable, 4);
        assert_eq!(native_all.passed, 4);
        assert_eq!(native_all.failed, 3);
        assert_eq!(native_all.disagreed_open, 1);
        assert_eq!(native_all.open_questions, 2);
        assert_eq!(native_all.verified_against_roblox, 2);
        assert_eq!(native_all.divergent, 1);

        // 3. Test multi-instance combinations (3 of each combination = 48 cases)
        let mut multi_combinations = Vec::new();
        for rep in 0..3 {
            for &prov in &provenances {
                for &outcome in &outcomes {
                    multi_combinations.push(make_case_result(prov, outcome, rep));
                }
            }
        }
        let shared_multi =
            compute_summary_via_lua(&lua, &tally_source, &multi_combinations).expect("lua summary");
        let native_multi = compute_summary_native(&multi_combinations);
        assert_same_tallies(&shared_multi, &native_multi, "48-case multi combination");

        // 4. Test empty results set
        let empty: Vec<CaseResult> = Vec::new();
        let shared_empty =
            compute_summary_via_lua(&lua, &tally_source, &empty).expect("lua summary");
        let native_empty = compute_summary_native(&empty);
        assert_same_tallies(&shared_empty, &native_empty, "empty set");

        // 5. Test today's live case directory
        let (results, summary_shared) = run_suite(&dir, None);
        let summary_native = compute_summary_native(&results);
        assert_same_tallies(&summary_shared, &summary_native, "live case directory");
    }
}
