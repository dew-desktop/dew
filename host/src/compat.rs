//! Where each example runs, derived rather than labelled.
//!
//! Two findings per example. [`reach`] reads the example's own Luau for names
//! only Dew supplies, and [`walk`] goes over the tree the example built and asks
//! the renderer's own predicate about every property set away from its
//! default. [`Row::verdict`] turns both into a verdict and [`table`] into the Markdown
//! that `examples/README.md` carries between [`BEGIN`] and [`END`].
//!
//! NOTHING HERE LOADS AN EXAMPLE. `dew compat` in `main.rs` does that the way
//! `dew snapshot` does and hands the results in, so every function below can be
//! fed a hand-built tree or a string of Luau by a test.
//!
//! NOTHING HERE NAMES AN EXAMPLE OR A GLOBAL. The globals Dew installs are read
//! from the VMs the loader prepared, the defaults from the reflection database,
//! and what is drawn from [`crate::datamodel::render::honours`].

use crate::datamodel::members::class_is_a;
use crate::datamodel::render::honours::Honour;
use crate::datamodel::{self, Dom};
use crate::gallery;
use crate::manifest::Permission;
use crate::scope;
use mlua::Lua;
use rbx_types::Variant;
use std::collections::{BTreeMap, BTreeSet};

/// The line before the generated block in `examples/README.md`.
pub const BEGIN: &str = "<!-- BEGIN GENERATED: examples compatibility. Regenerate with \
`cargo run --manifest-path host/Cargo.toml -- compat`; CI runs it with `--check`. \
Do not edit by hand. -->";

/// The line after it.
pub const END: &str = "<!-- END GENERATED: examples compatibility -->";

// ── Reading Luau ─────────────────────────────────────────────────────────────

/// One lexical token of Luau, as much as [`reach`] needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Ident(String),
    /// A string literal's contents. Interpolated strings keep only their
    /// literal text here; the code inside their braces is tokenised as code.
    Str(String),
    Punct(String),
}

/// `[`, any number of `=`, `[`: the level of a long bracket starting at `i`.
fn long_bracket(chars: &[char], i: usize) -> Option<usize> {
    if chars.get(i) != Some(&'[') {
        return None;
    }
    let mut j = i + 1;
    while chars.get(j) == Some(&'=') {
        j += 1;
    }
    (chars.get(j) == Some(&'[')).then_some(j - i - 1)
}

/// The index just past the `]=*]` closing a long bracket of `level` opened
/// before `from`, or the end of the input.
fn close_long_bracket(chars: &[char], from: usize, level: usize) -> (usize, usize) {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == ']' {
            let mut j = i + 1;
            while chars.get(j) == Some(&'=') {
                j += 1;
            }
            if j - i - 1 == level && chars.get(j) == Some(&']') {
                return (i, j + 1);
            }
        }
        i += 1;
    }
    (chars.len(), chars.len())
}

/// Tokenise Luau, dropping comments and whitespace.
///
/// A COMMENT IS NOT A REFERENCE, AND NEITHER IS A STRING. `-- desktop.Widget`
/// and `"dew.toml must grant"` name nothing a guest reaches, so a scan over
/// raw text would report both.
pub fn tokens(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '-' && chars.get(i + 1) == Some(&'-') {
            if let Some(level) = long_bracket(&chars, i + 2) {
                i = close_long_bracket(&chars, i + 2 + level + 2, level).1;
            } else {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            continue;
        }
        if let Some(level) = long_bracket(&chars, i) {
            let start = i + level + 2;
            let (end, next) = close_long_bracket(&chars, start, level);
            out.push(Token::Str(chars[start..end].iter().collect()));
            i = next;
            continue;
        }
        if c == '"' || c == '\'' {
            let mut text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != c && chars[i] != '\n' {
                if chars[i] == '\\' {
                    i += 1;
                }
                if let Some(ch) = chars.get(i) {
                    text.push(*ch);
                }
                i += 1;
            }
            out.push(Token::Str(text));
            i += 1;
            continue;
        }
        if c == '`' {
            let mut text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '`' {
                match chars[i] {
                    '\\' => {
                        if let Some(ch) = chars.get(i + 1) {
                            text.push(*ch);
                        }
                        i += 2;
                    }
                    '{' => {
                        let mut depth = 1;
                        let start = i + 1;
                        i += 1;
                        while i < chars.len() && depth > 0 {
                            match chars[i] {
                                '{' => depth += 1,
                                '}' => depth -= 1,
                                _ => {}
                            }
                            i += 1;
                        }
                        let inner: String = chars[start..i.saturating_sub(1).max(start)]
                            .iter()
                            .collect();
                        out.extend(tokens(&inner));
                    }
                    ch => {
                        text.push(ch);
                        i += 1;
                    }
                }
            }
            out.push(Token::Str(text));
            i += 1;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token::Ident(chars[start..i].iter().collect()));
            continue;
        }
        if c.is_ascii_digit() {
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                i += 1;
            }
            continue;
        }
        let three: String = chars[i..(i + 3).min(chars.len())].iter().collect();
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        let op = if three == "..." || three == "..=" {
            three
        } else if matches!(
            two.as_str(),
            "::" | "=="
                | "~="
                | "<="
                | ">="
                | ".."
                | "->"
                | "+="
                | "-="
                | "*="
                | "/="
                | "%="
                | "^="
        ) {
            two
        } else {
            c.to_string()
        };
        i += op.chars().count();
        out.push(Token::Punct(op));
    }
    out
}

/// What an example's own code reaches for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reach {
    /// `Global` or `Global.Member`, for each name in the set [`reach`] was given.
    pub names: BTreeSet<String>,
    /// It returns a declaration table with a `mount` field, the applet contract
    /// that has the host call it with a root.
    pub returns_mount: bool,
    /// It requires Aether.
    pub aether: bool,
}

fn is_punct(token: Option<&Token>, text: &str) -> bool {
    matches!(token, Some(Token::Punct(p)) if p == text)
}

fn is_word(token: Option<&Token>, words: &[&str]) -> bool {
    matches!(token, Some(Token::Ident(w)) if words.contains(&w.as_str()))
}

/// Scan one file of Luau for the globals in `globals`, the `mount` contract,
/// and a `require` of Aether.
pub fn reach(source: &str, globals: &BTreeSet<String>) -> Reach {
    let toks = tokens(source);
    let mut out = Reach::default();
    for (i, token) in toks.iter().enumerate() {
        let Token::Ident(name) = token else {
            continue;
        };
        let before = i.checked_sub(1).and_then(|j| toks.get(j));
        let after = toks.get(i + 1);
        // A member of something else, `x.desktop`, is not the global.
        if is_punct(before, ".") || is_punct(before, ":") {
            continue;
        }
        if globals.contains(name) {
            let member = match (after, toks.get(i + 2)) {
                (Some(Token::Punct(p)), Some(Token::Ident(m))) if p == "." || p == ":" => {
                    Some(m.clone())
                }
                _ => None,
            };
            // ASKING WHETHER A GLOBAL IS THERE IS NOT USING IT. `if desktop then`
            // is how one file mounts on a host that has `desktop` and on one
            // that does not; whatever the branch then reaches is reported on its
            // own. Only the whole condition counts: `if desktop and x then`
            // hands the value on, and stays reported.
            if member.is_none() && is_word(before, &["if", "elseif"]) && is_word(after, &["then"]) {
                continue;
            }
            out.names.insert(match member {
                Some(m) => format!("{name}.{m}"),
                None => name.clone(),
            });
        }
        if name == "mount"
            && (is_punct(before, "{") || is_punct(before, ",") || is_punct(before, ";"))
            && is_punct(after, "=")
        {
            out.returns_mount = true;
        }
        if name == "require" {
            let argument = match after {
                Some(Token::Punct(p)) if p == "(" => toks.get(i + 2),
                other => other,
            };
            if let Some(Token::Str(path)) = argument {
                if path == "aether" || path.ends_with("/aether") {
                    out.aether = true;
                }
            }
        }
    }
    out
}

// ── Which globals are Dew's ──────────────────────────────────────────────────

/// Every string key on a VM's globals table.
pub fn global_names(lua: &Lua) -> BTreeSet<String> {
    lua.globals()
        .pairs::<mlua::Value, mlua::Value>()
        .flatten()
        .filter_map(|(key, _)| match key {
            mlua::Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
            _ => None,
        })
        .collect()
}

/// Of the globals a host VM had before a guest ran, the ones the engine has no
/// counterpart for.
///
/// What a plain Luau VM already has is Luau's. What
/// [`datamodel::install_vocabulary`] puts there is the engine's value
/// vocabulary by that function's own contract. A name the reflection database
/// carries as a class, `Instance`, is the engine's. Everything left is Dew's.
pub fn dew_only_globals(installed: &BTreeSet<String>) -> BTreeSet<String> {
    let luau = global_names(&Lua::new());
    let vocabulary = {
        let lua = Lua::new();
        let before = global_names(&lua);
        datamodel::install_vocabulary(&lua).expect("the vocabulary installs on a fresh VM");
        &global_names(&lua) - &before
    };
    let db = rbx_reflection_database::get().ok();
    installed
        .iter()
        .filter(|name| !luau.contains(*name) && !vocabulary.contains(*name))
        .filter(|name| !db.is_some_and(|db| db.classes.contains_key(name.as_str())))
        .cloned()
        .collect()
}

// ── Walking the tree ─────────────────────────────────────────────────────────

/// What the walk over one example's tree found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Walk {
    /// Instances under the root, the root itself not counted.
    pub instances: usize,
    /// Pairs set away from their default that the renderer does not read, or
    /// reads in part, with its answer.
    pub gaps: BTreeMap<(String, String), Honour>,
    /// Classes and properties the reflection database does not carry.
    pub dew_only: BTreeSet<String>,
}

fn reflected_class(class: &str) -> bool {
    rbx_reflection_database::get()
        .map(|db| db.classes.contains_key(class))
        .unwrap_or(false)
}

fn reflected_property(class: &str, property: &str) -> bool {
    let Ok(db) = rbx_reflection_database::get() else {
        return false;
    };
    let mut cursor = Some(class);
    while let Some(c) = cursor {
        let Some(descriptor) = db.classes.get(c) else {
            return false;
        };
        if descriptor.properties.contains_key(property) {
            return true;
        }
        cursor = descriptor.superclass;
    }
    false
}

/// A class whose instances the engine draws or lays out with: a `GuiBase2d`
/// or a `UIComponent`. Anything else, a `StyleSheet` or a `Folder`, draws
/// nothing on either host and is counted but not asked about.
fn visual(class: &str) -> bool {
    class_is_a(class, "GuiBase2d") || class_is_a(class, "UIComponent")
}

/// Walk everything under `root`, asking `honours` about each instance's class
/// and each property set away from the reflection database's default.
///
/// `Parent` IS ASKED OF EVERY VISUAL INSTANCE, because an instance in the tree
/// differs from its default by being there, and the renderer's answer for
/// `Parent` is whether it looks for the class at all.
///
/// SKIPPED, EACH FOR A STATED REASON ELSEWHERE: a name the standard leaves out
/// of scope ([`scope::excluded`]), a property no guest could have assigned
/// ([`datamodel::accepts`], which is how layout's own outputs stay out), and a
/// property with no paint of its own on any host ([`gallery::excused`]).
pub fn walk(dom: &Dom, root: usize, honours: impl Fn(&str, &str) -> Honour) -> Walk {
    let mut out = Walk::default();
    let mut stack = dom.children(root);
    stack.reverse();
    while let Some(id) = stack.pop() {
        let mut children = dom.children(id);
        children.reverse();
        stack.extend(children);

        let Some(class) = dom.class_of(id) else {
            continue;
        };
        out.instances += 1;
        if !reflected_class(&class) {
            out.dew_only.insert(format!("class {class}"));
            continue;
        }
        if !visual(&class) {
            continue;
        }

        let mut ask = |property: &str| {
            let answer = honours(&class, property);
            let excused = property != "Parent" && gallery::excused(property).is_some();
            if answer != Honour::Implemented && !excused {
                out.gaps
                    .insert((class.clone(), property.to_string()), answer);
            }
        };

        ask("Parent");
        // `Name` paints only as a layout's sort key, and a layout sorts the
        // `GuiObject`s beside it, never a modifier.
        let default_name = match datamodel::default_value(&class, "Name") {
            Some(Variant::String(name)) => name,
            _ => class.clone(),
        };
        if class_is_a(&class, "GuiObject") && dom.name_of(id).is_some_and(|n| n != default_name) {
            ask("Name");
        }
        for (property, value) in dom.set_properties(id) {
            if scope::excluded(&property) || !datamodel::accepts(&class, &property) {
                continue;
            }
            if datamodel::default_value(&class, &property).as_ref() == Some(&value) {
                continue;
            }
            if !reflected_property(&class, &property) {
                out.dew_only.insert(format!("{class}.{property}"));
                continue;
            }
            ask(&property);
        }
    }
    out
}

// ── One row ──────────────────────────────────────────────────────────────────

/// What a reached name means for where the example runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// The root the tree is parented into. A mount helper replaces it, so it
    /// does not decide the verdict.
    Mount,
    /// Aether, which asks which host it is on and runs on both.
    Aether,
    /// API no Roblox place has.
    DewOnly,
}

/// One entry in the Dew-only column.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reached {
    pub kind: Kind,
    pub text: String,
}

/// What the host told the generator about its own globals.
#[derive(Clone, Debug, Default)]
pub struct Host {
    /// From [`dew_only_globals`].
    pub dew_only_globals: BTreeSet<String>,
    /// Dew-only globals holding the root instance a script parents into.
    pub root_globals: BTreeSet<String>,
    /// `desktop.Member` for each member a surface permission with an engine
    /// equivalent puts on the capability table.
    pub mount_members: BTreeSet<String>,
}

/// The Dew-only column for one example.
pub fn reached(
    reach: &Reach,
    permissions: &[Permission],
    walk_dew_only: &BTreeSet<String>,
    host: &Host,
) -> Vec<Reached> {
    let mut out = BTreeSet::new();
    for name in &reach.names {
        let global = name.split('.').next().unwrap_or(name);
        let kind = if host.mount_members.contains(name) || host.root_globals.contains(global) {
            Kind::Mount
        } else {
            Kind::DewOnly
        };
        out.insert(Reached {
            kind,
            text: format!("`{name}`"),
        });
    }
    if reach.returns_mount {
        out.insert(Reached {
            kind: Kind::Mount,
            text: "a returned `mount`".to_string(),
        });
    }
    if reach.aether {
        out.insert(Reached {
            kind: Kind::Aether,
            text: "Aether, which detects its host".to_string(),
        });
    }
    for permission in permissions {
        if permission.roblox_equivalent().is_none() {
            out.insert(Reached {
                kind: Kind::DewOnly,
                text: format!("permission `{}`", permission.name()),
            });
        }
    }
    for name in walk_dew_only {
        out.insert(Reached {
            kind: Kind::DewOnly,
            text: format!("`{name}`"),
        });
    }
    out.into_iter().collect()
}

/// Where an example runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Reaches nothing Dew-only beyond its mount, and every property it sets
    /// is one Dew's renderer reads.
    Portable,
    /// Reaches API no Roblox place has.
    DewOnly,
    /// Mounts portably, and sets a property Dew's renderer does not read, or
    /// reads in part.
    RobloxAhead,
    /// Did not load, or loaded and the walk found nothing.
    Error,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Verdict::Portable => "portable",
            Verdict::DewOnly => "Dew-only",
            Verdict::RobloxAhead => "Roblox-ahead",
            Verdict::Error => "error",
        }
    }
}

/// One example's row.
#[derive(Clone, Debug)]
pub struct Row {
    /// The example's path under `examples/`.
    pub example: String,
    pub reached: Vec<Reached>,
    /// The walk, or why there is none.
    pub walk: Result<Walk, String>,
}

impl Row {
    /// A loaded example whose walk visited nothing is an error, not a pass.
    ///
    /// WHAT THE TABLE WOULD SAY WITH THE WALK SWITCHED OFF is "nothing undrawn"
    /// for every example, which reads exactly like portable. Refusing an empty
    /// walk here is what keeps a broken loader from producing a green table.
    pub fn verdict(&self) -> Verdict {
        let Ok(walk) = &self.walk else {
            return Verdict::Error;
        };
        if walk.instances == 0 {
            return Verdict::Error;
        }
        if self.reached.iter().any(|r| r.kind == Kind::DewOnly) {
            return Verdict::DewOnly;
        }
        if !walk.gaps.is_empty() {
            return Verdict::RobloxAhead;
        }
        Verdict::Portable
    }
}

// ── The Markdown ─────────────────────────────────────────────────────────────

/// One line of plain ASCII from an error message, safe inside a table cell.
pub fn one_line(message: &str) -> String {
    let first = message.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let mut out = String::new();
    for c in first.trim().chars() {
        match c {
            '|' => out.push('/'),
            '\\' => out.push('/'),
            '\u{2026}' => out.push_str("..."),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            _ => out.push('?'),
        }
    }
    const LIMIT: usize = 400;
    if out.len() > LIMIT {
        out.truncate(LIMIT);
        out.push_str("...");
    }
    out
}

fn reached_cell(row: &Row) -> String {
    if row.reached.is_empty() {
        return "none".to_string();
    }
    row.reached
        .iter()
        .map(|r| match r.kind {
            Kind::Mount => format!("{} (mount)", r.text),
            Kind::Aether => format!("{} (runs on both)", r.text),
            Kind::DewOnly => r.text.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn gaps_cell(row: &Row) -> String {
    match &row.walk {
        Err(message) => format!("did not load: {}", one_line(message)),
        Ok(walk) if walk.instances == 0 => "the walk visited no instances".to_string(),
        Ok(walk) if walk.gaps.is_empty() => "none".to_string(),
        Ok(walk) => walk
            .gaps
            .iter()
            .map(|((class, property), answer)| match answer {
                Honour::Partial(_) => format!("`{class}.{property}` (in part)"),
                _ => format!("`{class}.{property}`"),
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn code_list(names: &BTreeSet<String>) -> String {
    if names.is_empty() {
        return "none".to_string();
    }
    names
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The generated block, markers included.
pub fn table(rows: &[Row], host: &Host) -> String {
    let mut out = String::new();
    out.push_str(BEGIN);
    out.push_str("\n\n");
    out.push_str(&format!(
        "Globals the host installs that the engine has no counterpart for: {}. \
         Of those, the ones holding a root to parent into: {}. Capability members \
         that are a surface with an engine equivalent: {}.\n\n",
        code_list(&host.dew_only_globals),
        code_list(&host.root_globals),
        code_list(&host.mount_members),
    ));
    out.push_str(
        "| Example | Verdict | Dew-only API it reaches | Set, and not read by Dew's renderer | Instances walked |\n",
    );
    out.push_str("| :--- | :--- | :--- | :--- | ---: |\n");
    let mut partial: BTreeMap<String, &'static str> = BTreeMap::new();
    for row in rows {
        let walked = match &row.walk {
            Ok(walk) => walk.instances.to_string(),
            Err(_) => "0".to_string(),
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            row.example,
            row.verdict().label(),
            reached_cell(row),
            gaps_cell(row),
            walked
        ));
        if let Ok(walk) = &row.walk {
            for ((class, property), answer) in &walk.gaps {
                if let Honour::Partial(reason) = answer {
                    partial.insert(format!("{class}.{property}"), reason);
                }
            }
        }
    }
    if !partial.is_empty() {
        out.push_str("\nRead in part, in the renderer's own words:\n\n");
        for (pair, reason) in partial {
            out.push_str(&format!("- `{pair}`: {reason}\n"));
        }
    }
    out.push('\n');
    out.push_str(END);
    out
}

/// `readme` with the generated block replaced by `generated`.
pub fn splice(readme: &str, generated: &str) -> Result<String, String> {
    let start = readme
        .find(BEGIN)
        .ok_or("examples/README.md has no BEGIN GENERATED marker for the table")?;
    let end_at = readme[start..]
        .find(END)
        .ok_or("examples/README.md has no END GENERATED marker after the BEGIN one")?;
    let end = start + end_at + END.len();
    Ok(format!(
        "{}{}{}",
        &readme[..start],
        generated,
        &readme[end..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamodel::render::honours::honours;
    use crate::datamodel::SharedDom;

    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// A COMMENT OR A STRING NAMING `desktop` REACHES NOTHING.
    #[test]
    fn a_comment_or_a_string_is_not_a_reference() {
        let globals = names(&["desktop", "DewRoot"]);
        let source = r#"
            -- desktop.Widget is what a widget calls
            --[[ and DewRoot
                 is what a script parents into ]]
            --[==[ desktop.Overlay ]==]
            local message = "dew.toml must grant `widget`, see desktop.Widget"
            local other = [[desktop.Storage]]
            local label = `desktop {1 + 1}`
            print(message, other, label)
        "#;
        let found = reach(source, &globals);
        assert!(found.names.is_empty(), "found {:?}", found.names);
    }

    #[test]
    fn a_member_is_reported_by_name_and_a_bare_use_alone() {
        let globals = names(&["desktop", "services", "DewRoot"]);
        let source = r#"
            local widget = assert(desktop.Widget, "x")
            local host = desktop
            local cs = (services :: any):GetService("CollectionService")
            card.Parent = DewRoot
            local t = x.desktop
            print(`{desktop.Time.now()}`)
        "#;
        let found = reach(source, &globals);
        assert_eq!(
            found.names,
            names(&[
                "DewRoot",
                "desktop",
                "desktop.Time",
                "desktop.Widget",
                "services"
            ])
        );
    }

    #[test]
    fn a_presence_test_is_not_a_reference_but_its_branch_is() {
        let globals = names(&["desktop"]);
        let guarded = r#"
            local root
            if desktop then
                root = desktop.Widget({ width = 1, height = 1 })
            elseif desktop then
            else
                root = game:GetService("Players")
            end
            local x = if desktop then 1 else 2
        "#;
        assert_eq!(reach(guarded, &globals).names, names(&["desktop.Widget"]));

        for handed_on in [
            "if desktop and x then end",
            "if not desktop then end",
            "local d = desktop",
        ] {
            assert_eq!(
                reach(handed_on, &globals).names,
                names(&["desktop"]),
                "{handed_on}"
            );
        }
    }

    #[test]
    fn the_mount_contract_and_an_aether_require_are_seen() {
        let source = r#"
            local Aether = require("./roblox_packages/aether")
            vide.mount(function() end)
            return { id = "x", mount = function(_desktop, root) end }
        "#;
        let found = reach(source, &BTreeSet::new());
        assert!(found.returns_mount);
        assert!(found.aether);

        let neither = reach(
            "vide.mount(build, root)\nlocal x = require('./vide')",
            &BTreeSet::new(),
        );
        assert!(!neither.returns_mount);
        assert!(!neither.aether);
    }

    /// The globals a host installs on top of Luau and the vocabulary are the
    /// ones reported. `Instance` is a reflection class and `UDim2` is
    /// vocabulary, so neither is Dew's.
    #[test]
    fn only_globals_the_engine_lacks_are_dew_only() {
        let lua = Lua::new();
        let dom = SharedDom::default();
        datamodel::install(&lua, &dom).expect("install");
        datamodel::install_vocabulary(&lua).expect("vocabulary");
        lua.globals()
            .set("desktop", lua.create_table().expect("table"))
            .expect("desktop");
        let found = dew_only_globals(&global_names(&lua));
        assert!(found.contains("desktop"), "{found:?}");
        assert!(found.contains("services"), "{found:?}");
        for engine in ["Instance", "UDim2", "Color3", "Enum", "print", "math"] {
            assert!(!found.contains(engine), "{engine} reported as Dew-only");
        }
    }

    fn tree(source: &str) -> (Lua, SharedDom, usize) {
        let lua = Lua::new();
        let dom = SharedDom::default();
        datamodel::install(&lua, &dom).expect("install");
        datamodel::install_vocabulary(&lua).expect("vocabulary");
        let root = dom
            .lock()
            .expect("dom")
            .insert("ScreenGui".into(), "DewRoot".into());
        let handle = datamodel::handle(&lua, &dom, root).expect("handle");
        lua.globals().set("Root", handle).expect("root");
        lua.load(source).exec().expect("tree builds");
        (lua, dom, root)
    }

    const CARD: &str = r#"
        local card = Instance.new("Frame")
        card.Size = UDim2.fromOffset(100, 40)
        card.BackgroundColor3 = Color3.new(1, 0, 0)
        card.Rotation = 10
        local corner = Instance.new("UICorner")
        corner.CornerRadius = UDim.new(0, 8)
        corner.Parent = card
        card.Parent = Root
    "#;

    #[test]
    fn the_walk_reports_what_the_renderer_does_not_read() {
        let (_lua, dom, root) = tree(CARD);
        let walk = walk(&dom.lock().expect("dom"), root, honours);
        assert_eq!(walk.instances, 2);
        assert_eq!(
            walk.gaps.get(&("Frame".into(), "Rotation".into())),
            Some(&Honour::Absent)
        );
        assert!(matches!(
            walk.gaps.get(&("UICorner".into(), "CornerRadius".into())),
            Some(Honour::Partial(_))
        ));
        // Read, so not reported.
        assert!(!walk.gaps.contains_key(&("Frame".into(), "Size".into())));
        assert!(!walk.gaps.contains_key(&("Frame".into(), "Parent".into())));
    }

    /// TURNING ONE PROPERTY OFF CHANGES THE ANSWER. The walk asks the predicate
    /// it is handed, so a predicate that stops reading `Size` has to surface
    /// `Frame.Size`, and nothing else may move.
    #[test]
    fn a_property_turned_off_appears() {
        let (_lua, dom, root) = tree(CARD);
        let guard = dom.lock().expect("dom");
        let before = walk(&guard, root, honours);
        let after = walk(&guard, root, |class, property| {
            if class == "Frame" && property == "Size" {
                Honour::Absent
            } else {
                honours(class, property)
            }
        });
        let added: Vec<_> = after
            .gaps
            .keys()
            .filter(|k| !before.gaps.contains_key(*k))
            .collect();
        assert_eq!(added, vec![&("Frame".to_string(), "Size".to_string())]);
    }

    /// A DEFAULT IS NOT A SETTING. A frame whose colour is assigned its own
    /// default is not reported for it, which only holds if the default came
    /// from the reflection database.
    #[test]
    fn a_property_assigned_its_default_is_not_asked() {
        let (_lua, dom, root) = tree(
            r#"
            local f = Instance.new("Frame")
            f.Rotation = 0
            f.Parent = Root
        "#,
        );
        let walk = walk(&dom.lock().expect("dom"), root, honours);
        assert_eq!(walk.instances, 1);
        assert!(walk.gaps.is_empty(), "{:?}", walk.gaps);
    }

    /// WITH THE WALK SWITCHED OFF THE TABLE MUST NOT READ PORTABLE. An empty
    /// tree is what a loader that built nothing hands over.
    #[test]
    fn an_empty_walk_is_an_error_and_never_portable() {
        let (_lua, dom, root) = tree("");
        let walk = walk(&dom.lock().expect("dom"), root, honours);
        assert_eq!(walk.instances, 0);
        let row = Row {
            example: "x".into(),
            reached: Vec::new(),
            walk: Ok(walk),
        };
        assert_eq!(row.verdict(), Verdict::Error);
        assert!(table(&[row], &Host::default()).contains("the walk visited no instances"));
    }

    #[test]
    fn verdicts_follow_the_findings() {
        let (_lua, dom, root) = tree(CARD);
        let walked = walk(&dom.lock().expect("dom"), root, honours);
        let mount = Reached {
            kind: Kind::Mount,
            text: "`desktop.Widget`".into(),
        };
        let dew = Reached {
            kind: Kind::DewOnly,
            text: "`desktop.Storage`".into(),
        };
        let clean = Walk {
            instances: 1,
            ..Walk::default()
        };
        let row = |reached: Vec<Reached>, walk: Result<Walk, String>| Row {
            example: "x".into(),
            reached,
            walk,
        };
        assert_eq!(
            row(vec![mount.clone()], Ok(clean.clone())).verdict(),
            Verdict::Portable
        );
        assert_eq!(
            row(vec![mount.clone()], Ok(walked.clone())).verdict(),
            Verdict::RobloxAhead
        );
        assert_eq!(
            row(vec![mount, dew], Ok(walked)).verdict(),
            Verdict::DewOnly
        );
        assert_eq!(row(vec![], Err("boom".into())).verdict(), Verdict::Error);
    }

    #[test]
    fn the_block_splices_between_its_markers_and_nowhere_else() {
        let readme = format!("before\n{BEGIN}\nstale\n{END}\nafter\n");
        let spliced = splice(&readme, &format!("{BEGIN}\nfresh\n{END}")).expect("splices");
        assert_eq!(spliced, format!("before\n{BEGIN}\nfresh\n{END}\nafter\n"));
        assert!(splice("no markers", "x").is_err());
    }

    #[test]
    fn an_error_becomes_one_ascii_line() {
        assert_eq!(
            one_line("C:\\x\\a.luau:3: bad | thing \u{2026}\nstack traceback"),
            "C:/x/a.luau:3: bad / thing ..."
        );
    }
}
