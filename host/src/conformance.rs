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
use mlua::prelude::*;
use mlua::{Table, Value};
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

/// Execute a single case against Dew's native DataModel.
pub fn run_case(lua: &Lua, case: &Case) -> CaseResult {
    // 1. Check feature support declared in `requires`
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

    // 2. A case that asserts nothing cannot pass
    if case.expect.is_empty() && case.ratios.is_empty() && case.order.is_none() {
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

    // All assertions satisfied
    let status = if case.provenance == "divergent" {
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

#[derive(Debug, Default)]
pub struct SuiteSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub open_questions: usize,
    pub divergent: usize,
    pub unsupported: usize,
    pub undecodable: usize,
    pub verified_against_roblox: usize,
}

/// Run all conformance cases in `cases_dir` with optional filter.
pub fn run_suite(cases_dir: &Path, filter: Option<&str>) -> (Vec<CaseResult>, SuiteSummary) {
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

        let res = run_case(&lua, &case);
        results.push(res);
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));

    let mut summary = SuiteSummary {
        total: results.len(),
        ..Default::default()
    };

    for r in &results {
        match &r.status {
            CaseStatus::Pass => {
                summary.passed += 1;
                if r.provenance == "roblox" {
                    summary.verified_against_roblox += 1;
                }
            }
            CaseStatus::Divergent => {
                summary.passed += 1;
                summary.divergent += 1;
                if r.provenance == "roblox" {
                    summary.verified_against_roblox += 1;
                }
            }
            CaseStatus::Fail { .. } => {
                summary.failed += 1;
            }
            CaseStatus::OpenQuestion { .. } => {
                summary.open_questions += 1;
            }
            CaseStatus::Unsupported { .. } => {
                summary.unsupported += 1;
            }
            CaseStatus::Undecodable { .. } => {
                summary.undecodable += 1;
            }
        }
    }

    (results, summary)
}

/// Print formatted report matching `conformance/run.luau` output.
pub fn print_report(results: &[CaseResult], summary: &SuiteSummary) {
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

    if summary.failed > 0 {
        println!();
        println!("FAILURES REQUIRING NATIVE LAYOUT (Sprint 2 backlog):");
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
        assert_eq!(summary.unsupported, 4, "expected 4 unsupported cases");
        assert_eq!(
            summary.passed, 6,
            "expected exactly 6 passing cases on demo layout"
        );
        assert_eq!(
            summary.failed, 14,
            "expected 14 failing cases on demo layout"
        );

        // Verify the 6 specific cases that pass on demo layout
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
    }

    #[test]
    fn single_case_filter_works() {
        let dir = find_cases_dir(None).expect("cases dir");
        let (results, summary) = run_suite(&dir, Some("scale resolves against the parent"));
        assert_eq!(results.len(), 1);
        assert_eq!(summary.passed, 1);
        assert_eq!(results[0].status, CaseStatus::Pass);
    }
}
