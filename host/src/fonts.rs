//! Which face file draws a `Font`, and at what scale.
//!
//! A `Font` names a FAMILY by URI (`rbxasset://fonts/families/SourceSansPro.json`)
//! plus a weight and a style. The family file is a small JSON document that
//! maps each weight and style to a face file. This module reads that format,
//! picks a face, and says where the face came from:
//!
//! - `Shipped`: one of the open licence faces built into Dew (`host/fonts`).
//! - `LocalStudio`: a face read from a Roblox Studio install on this machine.
//!   Reading a file the user already has is not redistributing it, which is
//!   how the Builder families, which Dew may not ship, can still be drawn.
//! - `Fallback`: the face asked for is not available here, and the answer is
//!   the nearest one that is, with the reason.
//!
//! WHY THE FACES ARE EMBEDDED. A release is one `dew.exe` and nothing beside
//! it, so a face folder next to the binary would be missing for anyone who
//! downloaded only the executable. The rasteriser loads fonts by PATH, so the
//! embedded bytes are written once to `<local data>/Dew/fonts/<version>/` and
//! the path to that copy is what `resolve` answers. The bytes cost about 2.7 MB
//! in any binary that reaches `resolve`; a binary that never calls it links
//! none of them.
//!
//! THE EM SCALE. The legacy families draw their glyphs at 1.5 times
//! `TextSize`: measured in Studio, `LegacyArial` "Hamburgefonts" at 20 is 1.50
//! times as wide as `Arimo`, its cap height is 1.5 times as tall, and its
//! one line `TextBounds.Y` is 30 where the modern families give 20. Which
//! families are legacy is read from the family file's own name, "Arimo
//! (Legacy)", rather than from a list here.
//!
//! NOTHING HERE DRAWS. This answers with a path and a number; measuring and
//! painting decide what to do with them.

use rbx_types::{Font, FontStyle, FontWeight};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The URI prefix `Font.new` and the reflection defaults use for a family.
pub const FAMILY_PREFIX: &str = "rbxasset://fonts/families/";

/// The prefix a family file uses for a face that is a file in the install.
const FACE_PREFIX: &str = "rbxasset://fonts/";

/// Overrides where Studio's `content/fonts` folder is looked for. An empty
/// value means "behave as though Studio is not installed".
pub const STUDIO_FONTS_ENV: &str = "DEW_STUDIO_FONTS";

/// The scale the legacy families draw at, relative to `TextSize`.
pub const LEGACY_EM_SCALE: f32 = 1.5;

// -- What Dew ships ------------------------------------------------------------

/// Face files built into Dew. Every one is under the SIL Open Font License 1.1
/// or the Apache License 2.0, read from the file's own `name` table; the texts
/// are in `host/fonts/LICENSES` and the attribution is in `NOTICE`.
const SHIPPED_FACES: &[(&str, &[u8])] = &[
    (
        "Arimo-Regular.ttf",
        include_bytes!("../fonts/Arimo-Regular.ttf"),
    ),
    ("Arimo-Bold.ttf", include_bytes!("../fonts/Arimo-Bold.ttf")),
    (
        "SourceSansPro-Light.ttf",
        include_bytes!("../fonts/SourceSansPro-Light.ttf"),
    ),
    (
        "SourceSansPro-Regular.ttf",
        include_bytes!("../fonts/SourceSansPro-Regular.ttf"),
    ),
    (
        "SourceSansPro-It.ttf",
        include_bytes!("../fonts/SourceSansPro-It.ttf"),
    ),
    (
        "SourceSansPro-Semibold.ttf",
        include_bytes!("../fonts/SourceSansPro-Semibold.ttf"),
    ),
    (
        "SourceSansPro-Bold.ttf",
        include_bytes!("../fonts/SourceSansPro-Bold.ttf"),
    ),
    (
        "Montserrat-Regular.ttf",
        include_bytes!("../fonts/Montserrat-Regular.ttf"),
    ),
    (
        "Montserrat-Medium.ttf",
        include_bytes!("../fonts/Montserrat-Medium.ttf"),
    ),
    (
        "Montserrat-Bold.ttf",
        include_bytes!("../fonts/Montserrat-Bold.ttf"),
    ),
    (
        "Montserrat-Black.ttf",
        include_bytes!("../fonts/Montserrat-Black.ttf"),
    ),
    (
        "RobotoMono-Regular.ttf",
        include_bytes!("../fonts/RobotoMono-Regular.ttf"),
    ),
];

/// Family files for the shipped families, in the engine's own format, so a
/// shipped family resolves the same way with or without a Studio install.
const SHIPPED_FAMILIES: &[(&str, &str)] = &[
    (
        "LegacyArial",
        include_str!("../fonts/families/LegacyArial.json"),
    ),
    (
        "LegacyArimo",
        include_str!("../fonts/families/LegacyArimo.json"),
    ),
    ("Arimo", include_str!("../fonts/families/Arimo.json")),
    (
        "SourceSansPro",
        include_str!("../fonts/families/SourceSansPro.json"),
    ),
    (
        "Montserrat",
        include_str!("../fonts/families/Montserrat.json"),
    ),
    (
        "RobotoMono",
        include_str!("../fonts/families/RobotoMono.json"),
    ),
];

/// Families the engine no longer has, and what it draws in their place.
///
/// Roblox removed Gotham and Arial on 2024-05-28 and replaced them with
/// Montserrat and Arimo ("Introducing Builder Font + Deprecating Gotham and
/// Arial", devforum.roblox.com/t/2868222). `Enum.Font.Gotham*` still maps to
/// `GothamSSm.json` and `Enum.Font.Arial*` to `Arial.json`, and neither file
/// is in a current install.
const REMOVED_FAMILIES: &[(&str, &str)] = &[("GothamSSm", "Montserrat"), ("Arial", "Arimo")];

/// What to draw a Builder family with when there is no Studio install.
///
/// Builder may not be redistributed, so it is never shipped. The stand ins are
/// chosen by measured advance of "Hamburgefonts" in em: Builder Sans 7.12
/// against Arimo 6.84 (Source Sans Pro is 6.57); Builder Extended 8.08 against
/// Montserrat 8.04 with the same 0.700 cap height; Builder Mono 7.80 against
/// Roboto Mono 7.80, both monospaced.
const BUILDER_STAND_INS: &[(&str, &str)] = &[
    ("BuilderSans", "Arimo"),
    ("BuilderExtended", "Montserrat"),
    ("BuilderMono", "RobotoMono"),
];

/// The family a request falls back to when nothing better is known: the face
/// a new `TextLabel` draws with.
const DEFAULT_STAND_IN: &str = "Arimo";

// -- The answer ----------------------------------------------------------------

/// Where a resolved face came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A face built into Dew.
    Shipped,
    /// A face read from the Studio install on this machine.
    LocalStudio,
    /// The face asked for is not available; this is the nearest one that is.
    Fallback {
        /// What was asked for, as `<family> <weight> <style>`.
        wanted: String,
        /// Why it could not be drawn as asked.
        reason: String,
    },
}

/// A face file to draw with, and the scale to draw it at.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFace {
    /// The face file on disk.
    pub path: PathBuf,
    /// Glyph size and line height as a multiple of `TextSize`: 1.5 for the
    /// legacy families, 1.0 for every other.
    pub em_scale: f32,
    /// Where the face came from.
    pub source: Source,
    /// The weight of the face chosen, which a fallback may change.
    pub weight: FontWeight,
    /// The style of the face chosen, which a fallback may change.
    pub style: FontStyle,
}

impl ResolvedFace {
    /// True when the face is exactly the one asked for.
    pub fn is_exact(&self) -> bool {
        !matches!(self.source, Source::Fallback { .. })
    }
}

// -- The family file format ----------------------------------------------------

/// One family file: `{ "name": ..., "faces": [ ... ] }`. Other keys, such as
/// `loadStrategy` on the CJK fallback family, are accepted and ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct FamilyFile {
    pub name: String,
    pub faces: Vec<FaceEntry>,
}

/// One face in a family file.
#[derive(Debug, Clone, Deserialize)]
pub struct FaceEntry {
    pub name: String,
    pub weight: u16,
    /// `"normal"` or `"italic"`.
    pub style: String,
    /// `rbxasset://fonts/<file>` for a file in the install, or
    /// `rbxassetid://<n>` for a face the engine downloads.
    #[serde(rename = "assetId")]
    pub asset_id: String,
}

impl FaceEntry {
    fn italic(&self) -> bool {
        self.style.eq_ignore_ascii_case("italic")
    }

    /// The file name, when the face is a file in the install.
    fn file(&self) -> Option<&str> {
        self.asset_id
            .strip_prefix(FACE_PREFIX)
            .filter(|f| !f.contains('/'))
    }
}

impl FamilyFile {
    /// Parse a family file. A UTF-8 byte order mark is tolerated.
    pub fn parse(text: &str) -> Result<FamilyFile, serde_json::Error> {
        serde_json::from_str(text.trim_start_matches('\u{feff}'))
    }

    /// Only LegacyArial draws at 1.5 times `TextSize` in the engine; LegacyArimo
    /// shares the "(Legacy)" name in its family file but measures at 1.0.
    pub fn is_legacy(&self) -> bool {
        false
    }
}

/// The family file stem a URI names, or `None` when it names no family file.
///
/// Accepts the full form `rbxasset://fonts/families/<Name>.json` and the
/// short form `Font.fromName` takes, `<Name>` alone (letters, digits, `_` and
/// `-`, per its documentation).
pub fn family_stem(uri: &str) -> Option<&str> {
    let stem = match uri.strip_prefix(FAMILY_PREFIX) {
        Some(rest) => rest.strip_suffix(".json")?,
        None => uri,
    };
    let valid = !stem.is_empty()
        && stem
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    valid.then_some(stem)
}

// -- The legacy `Font` enum ----------------------------------------------------

/// The family, weight and style an `Enum.Font` item stands for, by item name.
///
/// `None` only for `Unknown`, for which `Font.fromEnum` throws.
///
/// Items 0 to 45 use rbx-dom's `FontToFontFace` migration
/// (`rbx_reflection::MigrationOperation`), which gives family, weight and
/// style for each. Its families agree with the `Font.fromEnum` table in the
/// engine reference (creator-docs, `datatypes/Font.yaml`) for every item that
/// table lists; the table omits `Arial`, `ArialBold` and `Gotham*`. The items
/// added after the migration was written are named in that table with their
/// family only; their weights here come from the item name and are listed as
/// unverified until a Studio probe confirms them.
pub fn from_enum(item: &str) -> Option<Font> {
    let value = enum_value(item)?;
    if value <= 45 {
        let variant = rbx_types::Variant::Enum(rbx_types::Enum::from_u32(value));
        if let Some(Ok(rbx_types::Variant::Font(font))) =
            font_migration().map(|m| m.perform(&variant))
        {
            return Some(font);
        }
    }
    let (family, weight) = match item {
        "BuilderSans" => ("BuilderSans", FontWeight::Regular),
        "BuilderSansMedium" => ("BuilderSans", FontWeight::Medium),
        "BuilderSansBold" => ("BuilderSans", FontWeight::Bold),
        "BuilderSansExtraBold" => ("BuilderSans", FontWeight::ExtraBold),
        "Arimo" => ("Arimo", FontWeight::Regular),
        "ArimoBold" => ("Arimo", FontWeight::Bold),
        _ => return None,
    };
    Some(Font::new(
        &format!("{FAMILY_PREFIX}{family}.json"),
        weight,
        FontStyle::Normal,
    ))
}

/// The database's own migration from `TextLabel.Font` to `TextLabel.FontFace`.
fn font_migration() -> Option<&'static rbx_reflection::PropertyMigration<'static>> {
    use rbx_reflection::{PropertyKind, PropertySerialization};
    let db = rbx_reflection_database::get().ok()?;
    let property = db.classes.get("TextLabel")?.properties.get("Font")?;
    match &property.kind {
        PropertyKind::Canonical {
            serialization: PropertySerialization::Migrate(m),
        } => Some(m),
        _ => None,
    }
}

/// The value of an `Enum.Font` item in the pinned reflection database.
fn enum_value(item: &str) -> Option<u32> {
    let db = rbx_reflection_database::get().ok()?;
    db.enums.get("Font")?.items.get(item).copied()
}

/// Every `Enum.Font` item name in the pinned reflection database.
pub fn enum_items() -> Vec<String> {
    rbx_reflection_database::get()
        .ok()
        .and_then(|db| db.enums.get("Font"))
        .map(|e| e.items.keys().map(|k| k.to_string()).collect())
        .unwrap_or_default()
}

// -- Finding Studio ------------------------------------------------------------

/// The `content/fonts` folder of the newest Studio install on this machine.
///
/// `DEW_STUDIO_FONTS` overrides the search; set it empty to pretend Studio is
/// absent. Otherwise this looks for `<local data>/Roblox/Versions/*/content/fonts`
/// with a `families` folder in it, and takes the one whose `families` folder
/// was modified last, since Studio keeps old version folders around.
pub fn studio_fonts_dir() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os(STUDIO_FONTS_ENV) {
        let path = PathBuf::from(value);
        return (!path.as_os_str().is_empty() && path.join("families").is_dir()).then_some(path);
    }
    let versions = dirs::data_local_dir()?.join("Roblox").join("Versions");
    std::fs::read_dir(versions)
        .ok()?
        .flatten()
        .map(|entry| entry.path().join("content").join("fonts"))
        .filter_map(|fonts| {
            let modified = std::fs::metadata(fonts.join("families"))
                .ok()?
                .modified()
                .ok()?;
            Some((modified, fonts))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, fonts)| fonts)
}

// -- Writing the shipped faces out -----------------------------------------------

/// Write the shipped faces into `dir`, skipping any already there with the
/// same bytes, and return `dir`.
///
/// Written to a temporary name and renamed, so a second process starting at
/// the same moment never reads half a file.
pub fn write_shipped(dir: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    for (name, bytes) in SHIPPED_FACES {
        let target = dir.join(name);
        // THE BYTES, NOT THE SIZE. The folder is named by crate version, and two
        // builds of one version can embed different files; a copy left by the
        // other would be drawn while this build measured its own.
        let same_size =
            std::fs::metadata(&target).map(|m| m.len()).ok() == Some(bytes.len() as u64);
        if same_size && std::fs::read(&target).is_ok_and(|current| current == *bytes) {
            continue;
        }
        let temp = dir.join(format!("{name}.{}.tmp", std::process::id()));
        std::fs::write(&temp, bytes)?;
        if std::fs::rename(&temp, &target).is_err() {
            // Another process won the race; its copy is the same bytes.
            let _ = std::fs::remove_file(&temp);
        }
    }
    Ok(dir.to_path_buf())
}

/// Where the shipped faces are written by default.
fn default_shipped_dir() -> Option<PathBuf> {
    let version = env!("CARGO_PKG_VERSION");
    let primary = dirs::data_local_dir().map(|d| d.join("Dew").join("fonts").join(version));
    let temp = std::env::temp_dir().join("dew-fonts").join(version);
    primary
        .into_iter()
        .chain(std::iter::once(temp))
        .find_map(|dir| write_shipped(&dir).ok())
}

// -- Resolving -----------------------------------------------------------------

/// A resolver over one shipped folder and, optionally, one Studio folder.
///
/// `resolve` uses the process wide one; tests build their own to choose
/// whether Studio is present.
#[derive(Debug, Clone)]
pub struct Resolver {
    shipped_dir: PathBuf,
    studio_dir: Option<PathBuf>,
    shipped_families: BTreeMap<&'static str, FamilyFile>,
}

/// One face that can actually be opened, with where it came from.
struct Candidate<'a> {
    entry: &'a FaceEntry,
    path: PathBuf,
    shipped: bool,
}

impl Resolver {
    /// A resolver reading shipped faces from `shipped_dir` (as written by
    /// `write_shipped`) and other faces from `studio_dir`, a Studio
    /// `content/fonts` folder, when given.
    pub fn new(shipped_dir: PathBuf, studio_dir: Option<PathBuf>) -> Resolver {
        let shipped_families = SHIPPED_FAMILIES
            .iter()
            .map(|(stem, text)| {
                let family = FamilyFile::parse(text).expect("a shipped family file parses");
                (*stem, family)
            })
            .collect();
        Resolver {
            shipped_dir,
            studio_dir,
            shipped_families,
        }
    }

    /// The Studio folder in use, if any.
    pub fn studio_dir(&self) -> Option<&Path> {
        self.studio_dir.as_deref()
    }

    /// The family file for a stem: Dew's own copy for a shipped family,
    /// otherwise the Studio install's.
    pub fn family(&self, stem: &str) -> Option<FamilyFile> {
        if let Some(family) = self.shipped_families.get(stem) {
            return Some(family.clone());
        }
        let path = self
            .studio_dir
            .as_ref()?
            .join("families")
            .join(format!("{stem}.json"));
        FamilyFile::parse(&std::fs::read_to_string(path).ok()?).ok()
    }

    /// Resolve a `Font` value.
    pub fn resolve_font(&self, font: &Font) -> Option<ResolvedFace> {
        self.resolve(&font.family, font.weight, font.style)
    }

    /// Resolve a family URI, weight and style to a face file.
    ///
    /// `None` only when not even the default face can be found, which means
    /// the shipped faces could not be written to disk.
    pub fn resolve(
        &self,
        family_uri: &str,
        weight: FontWeight,
        style: FontStyle,
    ) -> Option<ResolvedFace> {
        let wanted = describe(family_uri, weight, style);
        let Some(asked) = family_stem(family_uri) else {
            let reason = "not a family file Dew can read".to_string();
            return self.stand_in(DEFAULT_STAND_IN, 1.0, weight, style, wanted, reason);
        };
        let stem = REMOVED_FAMILIES
            .iter()
            .find(|(gone, _)| *gone == asked)
            .map_or(asked, |(_, now)| now);

        let Some(family) = self.family(stem) else {
            let em_scale = if stem == "LegacyArial" {
                LEGACY_EM_SCALE
            } else {
                1.0
            };
            let (stand_in, reason) = match BUILDER_STAND_INS.iter().find(|(b, _)| *b == stem) {
                Some((_, s)) => (
                    *s,
                    format!(
                        "{stem} is not shipped, because its licence forbids redistribution, \
                     and no Studio install was found"
                    ),
                ),
                None => (
                    DEFAULT_STAND_IN,
                    format!("no family file for {stem} in Dew or in a Studio install"),
                ),
            };
            return self.stand_in(stand_in, em_scale, weight, style, wanted, reason);
        };
        let em_scale = if stem == "LegacyArial" {
            LEGACY_EM_SCALE
        } else {
            1.0
        };
        let (face, reason) = self.pick(&family, weight, style)?;
        Some(self.answer(face, em_scale, reason.map(|r| (wanted, r))))
    }

    /// Resolve within a substitute family, keeping the scale of the family
    /// that was asked for.
    fn stand_in(
        &self,
        stem: &str,
        em_scale: f32,
        weight: FontWeight,
        style: FontStyle,
        wanted: String,
        reason: String,
    ) -> Option<ResolvedFace> {
        let family = self.family(stem)?;
        let (face, _) = self.pick(&family, weight, style)?;
        let reason = format!("{reason}; drawn with {} {}", family.name, face.entry.name);
        Some(self.answer(face, em_scale, Some((wanted, reason))))
    }

    fn answer(
        &self,
        face: Candidate<'_>,
        em_scale: f32,
        fallback: Option<(String, String)>,
    ) -> ResolvedFace {
        let source = match fallback {
            Some((wanted, reason)) => Source::Fallback { wanted, reason },
            None if face.shipped => Source::Shipped,
            None => Source::LocalStudio,
        };
        ResolvedFace {
            path: face.path,
            em_scale,
            source,
            weight: FontWeight::from_u16(face.entry.weight).unwrap_or(FontWeight::Regular),
            style: if face.entry.italic() {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            },
        }
    }

    /// Every face of a family whose file can be opened here.
    fn candidates<'a>(&self, family: &'a FamilyFile) -> Vec<Candidate<'a>> {
        family
            .faces
            .iter()
            .filter_map(|entry| {
                let file = entry.file()?;
                if SHIPPED_FACES.iter().any(|(name, _)| *name == file) {
                    let path = self.shipped_dir.join(file);
                    if path.is_file() {
                        return Some(Candidate {
                            entry,
                            path,
                            shipped: true,
                        });
                    }
                }
                let path = self.studio_dir.as_ref()?.join(file);
                path.is_file().then_some(Candidate {
                    entry,
                    path,
                    shipped: false,
                })
            })
            .collect()
    }

    /// Choose a face: the exact weight and style if it can be opened,
    /// otherwise the nearest that can, with the reason.
    ///
    /// The engine's own rule for a weight a family lacks is not documented.
    /// Dew keeps the style when any face has it, then takes the nearest
    /// weight; on a tie it goes heavier for a weight above 400 and lighter
    /// otherwise, as CSS font matching does.
    fn pick<'a>(
        &self,
        family: &'a FamilyFile,
        weight: FontWeight,
        style: FontStyle,
    ) -> Option<(Candidate<'a>, Option<String>)> {
        let want = weight.as_u16();
        let italic = style == FontStyle::Italic;
        let mut candidates = self.candidates(family);
        if candidates.is_empty() {
            return None;
        }
        if let Some(i) = candidates
            .iter()
            .position(|c| c.entry.weight == want && c.entry.italic() == italic)
        {
            return Some((candidates.swap_remove(i), None));
        }
        let reason = match family
            .faces
            .iter()
            .find(|f| f.weight == want && f.italic() == italic)
        {
            Some(f) if f.file().is_none() => format!(
                "{} {} is a download ({}), not a file in an install",
                family.name, f.name, f.asset_id
            ),
            Some(f) => format!("{} {} is not on this machine", family.name, f.name),
            None => format!(
                "{} has no {} face",
                family.name,
                describe_face(want, italic)
            ),
        };
        let same_style = candidates.iter().any(|c| c.entry.italic() == italic);
        let best = candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| !same_style || c.entry.italic() == italic)
            .min_by_key(|(_, c)| {
                let w = c.entry.weight;
                let distance = w.abs_diff(want);
                let wrong_side = w > want;
                (distance, wrong_side)
            })
            .map(|(i, _)| i)?;
        let chosen = candidates.swap_remove(best);
        let reason = format!("{reason}; nearest is {}", chosen.entry.name);
        Some((chosen, Some(reason)))
    }
}

fn describe(family: &str, weight: FontWeight, style: FontStyle) -> String {
    format!(
        "{family} {}",
        describe_face(weight.as_u16(), style == FontStyle::Italic)
    )
}

fn describe_face(weight: u16, italic: bool) -> String {
    format!("{weight} {}", if italic { "italic" } else { "normal" })
}

/// The process wide resolver: shipped faces written to local data, and the
/// newest Studio install, if any.
pub fn resolver() -> &'static Resolver {
    static RESOLVER: OnceLock<Resolver> = OnceLock::new();
    RESOLVER.get_or_init(|| {
        let shipped = default_shipped_dir().unwrap_or_default();
        Resolver::new(shipped, studio_fonts_dir())
    })
}

/// Resolve a family URI, weight and style with the process wide resolver.
pub fn resolve(family_uri: &str, weight: FontWeight, style: FontStyle) -> Option<ResolvedFace> {
    resolver().resolve(family_uri, weight, style)
}

/// Resolve a `Font` value with the process wide resolver.
pub fn resolve_font(font: &Font) -> Option<ResolvedFace> {
    resolver().resolve_font(font)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn scratch(tag: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("dew-fonts-test-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Shipped faces only, as on a machine with no Studio.
    fn without_studio() -> Resolver {
        Resolver::new(write_shipped(&scratch("shipped")).unwrap(), None)
    }

    fn uri(stem: &str) -> String {
        format!("{FAMILY_PREFIX}{stem}.json")
    }

    fn file_name(face: &ResolvedFace) -> String {
        face.path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn every_family_file_in_the_studio_install_parses() {
        let Some(dir) = studio_fonts_dir() else {
            eprintln!("skipped: no Studio install on this machine");
            return;
        };
        let mut count = 0;
        for entry in std::fs::read_dir(dir.join("families")).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let text = std::fs::read_to_string(&path).unwrap();
                let family =
                    FamilyFile::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                assert!(!family.faces.is_empty(), "{} has no faces", path.display());
                count += 1;
            }
        }
        eprintln!("{count} family files parsed from {}", dir.display());
        assert!(count > 0);
    }

    #[test]
    fn the_shipped_set_resolves_without_studio() {
        let r = without_studio();
        for (stem, text) in SHIPPED_FAMILIES {
            let family = FamilyFile::parse(text).unwrap();
            for face in family.faces.iter().filter(|f| f.file().is_some()) {
                let style = if face.italic() {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                };
                let weight = FontWeight::from_u16(face.weight).unwrap();
                let got = r.resolve(&uri(stem), weight, style).unwrap();
                assert_eq!(got.source, Source::Shipped, "{stem} {}", face.name);
                assert_eq!(file_name(&got), face.file().unwrap());
                assert!(got.path.is_file());
            }
        }
        // The engine default face, by the reflection default's URI.
        let default = r
            .resolve(&uri("LegacyArial"), FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert_eq!(file_name(&default), "Arimo-Regular.ttf");
        let bold = r
            .resolve(&uri("LegacyArial"), FontWeight::Bold, FontStyle::Normal)
            .unwrap();
        assert_eq!(file_name(&bold), "Arimo-Bold.ttf");
    }

    #[test]
    fn every_shipped_face_file_is_named_by_a_shipped_family() {
        for (name, bytes) in SHIPPED_FACES {
            assert!(!bytes.is_empty());
            let named = SHIPPED_FAMILIES.iter().any(|(_, text)| {
                FamilyFile::parse(text)
                    .unwrap()
                    .faces
                    .iter()
                    .any(|f| f.file() == Some(name))
            });
            assert!(named, "{name} is shipped but no family names it");
        }
    }

    #[test]
    fn the_short_form_and_the_full_uri_resolve_alike() {
        let r = without_studio();
        let short = r
            .resolve("SourceSansPro", FontWeight::Bold, FontStyle::Normal)
            .unwrap();
        let full = r
            .resolve(&uri("SourceSansPro"), FontWeight::Bold, FontStyle::Normal)
            .unwrap();
        assert_eq!(short, full);
        assert_eq!(
            family_stem("rbxasset://fonts/families/Foo.json"),
            Some("Foo")
        );
        assert_eq!(family_stem("rbxassetid://123"), None);
        assert_eq!(family_stem("Has Space"), None);
    }

    #[test]
    fn a_missing_weight_takes_the_nearest_and_says_so() {
        let r = without_studio();
        // Source Sans Pro has 400 and 600 as files; 500 is not a face at all.
        // On a tie, the engine chooses lighter (FW Medium 213 matches Regular).
        let medium = r
            .resolve(&uri("SourceSansPro"), FontWeight::Medium, FontStyle::Normal)
            .unwrap();
        assert_eq!(file_name(&medium), "SourceSansPro-Regular.ttf");
        assert!(
            matches!(&medium.source, Source::Fallback { reason, .. } if reason.contains("no 500"))
        );

        // Black (900) is a download; the nearest file is Bold.
        let black = r
            .resolve(&uri("SourceSansPro"), FontWeight::Heavy, FontStyle::Normal)
            .unwrap();
        assert_eq!(file_name(&black), "SourceSansPro-Bold.ttf");
        assert!(
            matches!(&black.source, Source::Fallback { reason, .. } if reason.contains("download"))
        );

        // Extra Light (200) is a download; Light (300) is the nearest file.
        let thin = r
            .resolve(
                &uri("SourceSansPro"),
                FontWeight::ExtraLight,
                FontStyle::Normal,
            )
            .unwrap();
        assert_eq!(file_name(&thin), "SourceSansPro-Light.ttf");
    }

    #[test]
    fn a_missing_style_keeps_the_style_when_it_can() {
        let r = without_studio();
        // Bold Italic is a download; the one italic file wins over Bold.
        let bi = r
            .resolve(&uri("SourceSansPro"), FontWeight::Bold, FontStyle::Italic)
            .unwrap();
        assert_eq!(file_name(&bi), "SourceSansPro-It.ttf");
        assert_eq!(bi.style, FontStyle::Italic);
        // Arimo has no italic file at all, so the style gives way.
        let ai = r
            .resolve(&uri("Arimo"), FontWeight::Regular, FontStyle::Italic)
            .unwrap();
        assert_eq!(file_name(&ai), "Arimo-Regular.ttf");
        assert!(!ai.is_exact());
    }

    #[test]
    fn removed_families_draw_their_replacements() {
        let r = without_studio();
        let gotham = from_enum("GothamBold").unwrap();
        let got = r.resolve_font(&gotham).unwrap();
        assert_eq!(file_name(&got), "Montserrat-Bold.ttf");
        assert_eq!(got.source, Source::Shipped);
        let black = r.resolve_font(&from_enum("GothamBlack").unwrap()).unwrap();
        assert_eq!(file_name(&black), "Montserrat-Black.ttf");
        let arial = r.resolve_font(&from_enum("ArialBold").unwrap()).unwrap();
        assert_eq!(file_name(&arial), "Arimo-Bold.ttf");
    }

    #[test]
    fn every_enum_font_item_maps_to_a_family() {
        let items = enum_items();
        assert!(
            items.len() > 40,
            "reflection database has {} Font items",
            items.len()
        );
        let r = without_studio();
        for item in &items {
            if item == "Unknown" {
                assert!(
                    from_enum(item).is_none(),
                    "Font.fromEnum throws for Unknown"
                );
                continue;
            }
            let font =
                from_enum(item).unwrap_or_else(|| panic!("Enum.Font.{item} maps to nothing"));
            assert!(
                family_stem(&font.family).is_some(),
                "{item}: {}",
                font.family
            );
            assert!(
                r.resolve_font(&font).is_some(),
                "{item} resolves to nothing"
            );
        }
        assert_eq!(from_enum("Legacy").unwrap().family, uri("LegacyArial"));
        let semibold = from_enum("SourceSansSemibold").unwrap();
        assert_eq!(
            (semibold.family.as_str(), semibold.weight),
            (uri("SourceSansPro").as_str(), FontWeight::SemiBold)
        );
        assert_eq!(
            from_enum("SourceSansItalic").unwrap().style,
            FontStyle::Italic
        );
    }

    #[test]
    fn builder_without_studio_falls_back_and_says_why() {
        let r = without_studio();
        let got = r
            .resolve(&uri("BuilderSans"), FontWeight::Bold, FontStyle::Normal)
            .unwrap();
        assert_eq!(file_name(&got), "Arimo-Bold.ttf");
        assert_eq!(got.em_scale, 1.0);
        match &got.source {
            Source::Fallback { wanted, reason } => {
                assert!(wanted.contains("BuilderSans"));
                assert!(reason.contains("licence"), "{reason}");
            }
            other => panic!("expected a fallback, got {other:?}"),
        }
        let mono = r
            .resolve(&uri("BuilderMono"), FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert_eq!(file_name(&mono), "RobotoMono-Regular.ttf");
    }

    #[test]
    fn builder_with_a_studio_folder_reads_it_from_there() {
        // A stand in for an install, so this runs where Studio is absent too.
        let studio = scratch("studio");
        std::fs::create_dir_all(studio.join("families")).unwrap();
        std::fs::write(
            studio.join("families").join("BuilderSans.json"),
            r#"{"name":"Builder Sans","faces":[
                {"name":"Regular","weight":400,"style":"normal","assetId":"rbxasset://fonts/BuilderSans-Regular.otf"}]}"#,
        )
        .unwrap();
        std::fs::write(studio.join("BuilderSans-Regular.otf"), b"not shipped").unwrap();
        let r = Resolver::new(
            write_shipped(&scratch("shipped")).unwrap(),
            Some(studio.clone()),
        );
        let got = r
            .resolve(&uri("BuilderSans"), FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert_eq!(got.source, Source::LocalStudio);
        assert_eq!(got.path, studio.join("BuilderSans-Regular.otf"));
    }

    #[test]
    fn builder_resolves_from_the_real_studio_install_when_present() {
        let Some(dir) = studio_fonts_dir() else {
            eprintln!("skipped: no Studio install on this machine");
            return;
        };
        let r = Resolver::new(write_shipped(&scratch("shipped")).unwrap(), Some(dir));
        let got = r
            .resolve(&uri("BuilderSans"), FontWeight::Bold, FontStyle::Normal)
            .unwrap();
        assert_eq!(got.source, Source::LocalStudio);
        assert_eq!(file_name(&got), "BuilderSans-Bold.otf");
        // A family Dew does not ship, read wholly from the install.
        let oswald = r
            .resolve(&uri("Oswald"), FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert_eq!(oswald.source, Source::LocalStudio);
    }

    #[test]
    fn the_em_scale_is_one_and_a_half_only_for_the_legacy_families() {
        let r = without_studio();
        let legacy = r
            .resolve(&uri("LegacyArial"), FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert_eq!(legacy.em_scale, 1.5);
        let ssp = r
            .resolve(
                &uri("SourceSansPro"),
                FontWeight::Regular,
                FontStyle::Normal,
            )
            .unwrap();
        assert_eq!(ssp.em_scale, 1.0);
        let arimo = r
            .resolve(&uri("Arimo"), FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert_eq!(arimo.em_scale, 1.0);
        let legacy_arimo = r
            .resolve(&uri("LegacyArimo"), FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert_eq!(legacy_arimo.em_scale, 1.0);
        // The default face and LegacyArial are the same file at different scales.
        assert_eq!(legacy.path, arimo.path);
        assert_eq!(
            r.resolve_font(&from_enum("Legacy").unwrap())
                .unwrap()
                .em_scale,
            1.5
        );
        assert_eq!(
            r.resolve_font(&from_enum("Arial").unwrap())
                .unwrap()
                .em_scale,
            1.0
        );
    }

    #[test]
    fn an_unreadable_family_falls_back_to_the_default_face() {
        let r = without_studio();
        let got = r
            .resolve("rbxassetid://12345", FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert_eq!(file_name(&got), "Arimo-Regular.ttf");
        assert!(!got.is_exact());
        let got = r
            .resolve(&uri("NoSuchFamily"), FontWeight::Regular, FontStyle::Normal)
            .unwrap();
        assert!(!got.is_exact());
    }

    #[test]
    fn writing_the_shipped_faces_twice_is_harmless() {
        let dir = scratch("twice");
        write_shipped(&dir).unwrap();
        write_shipped(&dir).unwrap();
        let written = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(written, SHIPPED_FACES.len());
    }
}
