//! `.dewpkg` -- a plain zip archive. `dew package` writes one from an applet
//! directory; `dew install` reads one back by extracting it to a fresh
//! temporary directory and handing that directory to `installed::install`
//! exactly as it would a directory an author pointed it at directly.
//!
//! `installed::install` NEVER LEARNS THE DIFFERENCE. A zip is a second way of
//! producing a `&Path`, nothing more -- extraction happens entirely here and
//! is done by the time `install` sees anything.
//!
//! A LOCAL FILE A PERSON CHOSE TO OPEN IS ALREADY AS TRUSTED AS A DIRECTORY
//! THEY CHOSE TO POINT AT. Zipping it changes nothing about that, so the only
//! checks below are safety ones -- a malformed or hostile archive must not
//! write outside the directory it is being extracted into -- not signing or
//! provenance.

#![cfg(windows)]

use crate::installed::SKIP_DIRS;
use crate::manifest::Manifest;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A loose ceiling on what a package can contain. Not a precise security
/// boundary -- no real applet is within two orders of magnitude of either
/// number -- just a refusal for a zip that is obviously not one.
const MAX_ENTRIES: usize = 20_000;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

/// A temporary extraction directory, removed on drop regardless of whether
/// the install that follows succeeds. Derefs to the path so callers hand it
/// to `installed::install` exactly as they would any other directory.
struct TempExtract(PathBuf);

impl std::ops::Deref for TempExtract {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempExtract {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fresh_temp_dir() -> Result<TempExtract, String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "dew-install-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(TempExtract(dir))
}

/// A zip entry's name, checked component by component and turned into a path
/// relative to the extraction root.
///
/// THIS IS THE ZIP-SLIP REFUSAL. Any component that could step outside that
/// root -- `..`, a drive letter, a leading `/` or `\` -- refuses the WHOLE
/// archive rather than skipping the one entry: a package that contains one
/// unsafe path is not a package the rest of it should be trusted from either.
fn safe_relative_path(name: &str) -> Result<PathBuf, String> {
    let mut out = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "'{name}' is not a safe path inside the package (escapes the install directory)"
                ));
            }
        }
    }
    Ok(out)
}

/// Is this entry a symlink? Read from the Unix mode bits a zip stores in its
/// external file attributes, the same field `unzip` and `7z` read. Never
/// followed on extraction, for the reason `copy_dir` already skips anything
/// exotic in a plain directory copy: it could point anywhere at all.
fn is_symlink(mode: Option<u32>) -> bool {
    const S_IFLNK: u32 = 0o120000;
    const S_IFMT: u32 = 0o170000;
    mode.map(|m| m & S_IFMT == S_IFLNK).unwrap_or(false)
}

/// If every entry is nested under the same single top-level directory, that
/// directory's name -- the shape Windows Explorer's right-click "compress"
/// produces on a folder. `None` means the archive is already rooted the way
/// `dew package` writes one, or the shape is ambiguous, in which case it is
/// extracted as-is rather than guessed at.
fn shared_top_level(entries: &[(PathBuf, bool)]) -> Option<PathBuf> {
    let mut top: Option<PathBuf> = None;
    for (path, is_dir) in entries {
        let mut components = path.components();
        let first = PathBuf::from(components.next()?.as_os_str());
        let nested = components.next().is_some();

        // A file sitting directly at the archive root means it is not fully
        // wrapped in one folder, whatever else is nested elsewhere.
        if !nested && !is_dir {
            return None;
        }

        match &top {
            None => top = Some(first),
            Some(t) if *t == first => {}
            Some(_) => return None,
        }
    }
    top
}

/// Extract `zip_path` into a fresh temporary directory, refusing the whole
/// archive on the first unsafe entry rather than extracting anything at all.
fn extract(zip_path: &Path) -> Result<TempExtract, String> {
    let file = File::open(zip_path).map_err(|e| format!("{}: {e}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("{}: not a valid zip archive ({e})", zip_path.display()))?;

    if archive.len() > MAX_ENTRIES {
        return Err(format!(
            "{}: {} entries is more than dew install will extract",
            zip_path.display(),
            archive.len()
        ));
    }

    // FIRST PASS: every name is validated and nothing is written to disk yet.
    // This is what makes the zip-slip refusal whole-archive rather than
    // per-entry -- an unsafe entry found on file 400 of 500 must not leave
    // the first 399 already extracted on disk.
    let mut entries: Vec<(PathBuf, bool)> = Vec::with_capacity(archive.len());
    let mut total_bytes: u64 = 0;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("{}: {e}", zip_path.display()))?;
        let rel = safe_relative_path(entry.name())?;
        total_bytes = total_bytes.saturating_add(entry.size());
        if total_bytes > MAX_TOTAL_BYTES {
            return Err(format!(
                "{}: uncompressed contents exceed what dew install will extract",
                zip_path.display()
            ));
        }
        entries.push((rel, entry.is_dir()));
    }

    let strip = shared_top_level(&entries);
    let dest = fresh_temp_dir()?;

    for (i, (rel, is_dir)) in entries.iter().enumerate() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("{}: {e}", zip_path.display()))?;

        if is_symlink(entry.unix_mode()) {
            continue;
        }

        let rel = match &strip {
            Some(prefix) => rel.strip_prefix(prefix).unwrap_or(rel).to_path_buf(),
            None => rel.clone(),
        };
        if rel.as_os_str().is_empty() {
            continue;
        }

        let out_path = dest.join(&rel);
        // BELT AND SUSPENDERS: `safe_relative_path` already refused anything
        // that could climb out, but the resolved path is re-checked against
        // the extraction root before anything is written, in case a future
        // change to that sanitiser ever disagrees with this one.
        if !out_path.starts_with(&*dest) {
            return Err(format!(
                "'{}' is not a safe path inside the package (escapes the install directory)",
                entry.name()
            ));
        }

        if *is_dir {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("{}: {e}", out_path.display()))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let mut out_file =
            File::create(&out_path).map_err(|e| format!("{}: {e}", out_path.display()))?;
        std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| format!("{}: {e}", out_path.display()))?;
    }

    Ok(dest)
}

/// Extract `zip_path` to a fresh temporary directory and hand it to
/// `installed::install` exactly as a directory source would be, cleaning up
/// the temporary directory whether that call succeeds or fails.
pub fn install_from_archive(zip_path: &Path, force: bool) -> Result<String, String> {
    let extracted = extract(zip_path)?;
    crate::installed::install(&extracted, force)
}

/// Zip `dir` into a `.dewpkg` file, excluding the same directories
/// `installed::install` never copies, with the manifest at the archive's own
/// root rather than nested under a folder. Defaults the output filename to
/// the manifest's own id, read the same way `install` reads it.
pub fn package(dir: &Path, output: Option<PathBuf>) -> Result<PathBuf, String> {
    let manifest = Manifest::load(dir)?;
    let out_path = output.unwrap_or_else(|| PathBuf::from(format!("{}.dewpkg", manifest.id)));

    let file = File::create(&out_path).map_err(|e| format!("{}: {e}", out_path.display()))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_dir(&mut writer, dir, Path::new(""), &options).inspect_err(|_| {
        // A half-written archive is worse than none: an install of it would
        // fail in a way that looks like a corrupt download rather than what
        // actually happened.
        let _ = std::fs::remove_file(&out_path);
    })?;
    writer
        .finish()
        .map_err(|e| format!("{}: {e}", out_path.display()))?;

    Ok(out_path)
}

fn add_dir(
    writer: &mut zip::ZipWriter<File>,
    src: &Path,
    prefix: &Path,
    options: &zip::write::SimpleFileOptions,
) -> Result<(), String> {
    for entry in std::fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", src.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        let name = entry.file_name();
        let rel = prefix.join(&name);

        if file_type.is_dir() {
            if SKIP_DIRS.contains(&name.to_string_lossy().as_ref()) {
                continue;
            }
            add_dir(writer, &entry.path(), &rel, options)?;
        } else if file_type.is_file() {
            // ZIP NAMES ARE FORWARD-SLASHED, by the format's own spec,
            // regardless of the platform writing them.
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            writer
                .start_file(rel_str, *options)
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
            let mut contents = Vec::new();
            File::open(entry.path())
                .and_then(|mut f| f.read_to_end(&mut contents))
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
            writer
                .write_all(&contents)
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        }
        // A symlink or anything else exotic: skipped, matching `copy_dir`'s
        // own posture on a plain directory copy.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_traversal_entries_are_refused() {
        assert!(safe_relative_path("../../../../evil.txt").is_err());
        assert!(safe_relative_path("a/../../b").is_err());
        assert!(safe_relative_path("/etc/passwd").is_err());
        assert!(safe_relative_path("C:\\Windows\\evil.txt").is_err());
        assert!(safe_relative_path("dew.toml").is_ok());
        assert!(safe_relative_path("my-widget/dew.toml").is_ok());
    }

    #[test]
    fn a_shared_wrapper_directory_is_detected() {
        let entries = vec![
            (PathBuf::from("my-widget/dew.toml"), false),
            (PathBuf::from("my-widget/entry.luau"), false),
        ];
        assert_eq!(shared_top_level(&entries), Some(PathBuf::from("my-widget")));
    }

    #[test]
    fn an_already_rooted_archive_is_not_unwrapped() {
        let entries = vec![
            (PathBuf::from("dew.toml"), false),
            (PathBuf::from("entry.luau"), false),
        ];
        assert_eq!(shared_top_level(&entries), None);
    }

    #[test]
    fn a_mixed_shape_is_not_unwrapped() {
        let entries = vec![
            (PathBuf::from("readme.txt"), false),
            (PathBuf::from("my-widget/dew.toml"), false),
        ];
        assert_eq!(shared_top_level(&entries), None);
    }

    #[test]
    fn a_malicious_archive_is_refused_and_nothing_lands_outside_the_temp_dir() {
        let dir = fresh_temp_dir().expect("temp dir");
        let zip_path = dir.join("evil.dewpkg");
        {
            let file = File::create(&zip_path).expect("create zip");
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer
                .start_file("../../../../evil.txt", options)
                .expect("start entry");
            writer.write_all(b"pwned").expect("write entry");
            writer.finish().expect("finish zip");
        }

        let result = extract(&zip_path);
        assert!(result.is_err(), "a path-traversal entry must be refused");

        // Nothing should have escaped anywhere near a real path outside the
        // extraction area -- there is no directory to check because
        // `extract` never got past validating names before writing anything.
        let escaped = std::env::temp_dir().join("evil.txt");
        assert!(!escaped.exists());
    }
}
