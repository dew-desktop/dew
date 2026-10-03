//! Embeds a monotonic build number into `dew`'s build identifier at compile
//! time, and on Windows the bundled applets' files into `dew` itself (see
//! `embed_bundled_applets`).
//!
//! COMPILE TIME, NOT RUNTIME. `dew.exe` ships to machines with no `.git`
//! directory at all, so a `git` invocation inside the running program would
//! fail exactly where the identifier matters most -- a crash report from a
//! real install. `cargo:rustc-env` bakes the answer in once, here, where a
//! checkout is expected to exist.
//!
//! `git rev-list --count HEAD`, NOT A COMMIT HASH. Sprint 3 of milestone 12
//! originally embedded a short sha here, reasoning from the plan's own
//! "SemVer build metadata on a short commit hash" wording. That did not
//! actually match the reference design it was modeled on: Roblox's real
//! version string, `0.739.0.7390687`, is entirely numeric -- there is no VCS
//! hash anywhere in it, and its fourth segment is an opaque, monotonically
//! incrementing build number from Roblox's own build system. `git rev-list
//! --count HEAD` -- the number of commits reachable from `HEAD` -- is the
//! natural local analogue of that counter: it needs no new infrastructure,
//! and unlike a CI run number it is available identically to a developer
//! running `cargo build` at their desk and to a CI job, since a CI run
//! number only exists inside that CI job.
//!
//! NO `.git` IS NOT A BUILD FAILURE. A source tarball or a vendored crate
//! directory has no `.git` either, and refusing to compile because of it
//! would be strictly worse than an honest `0` in the identifier -- `0` is a
//! count `git rev-list --count` can never actually produce for a real
//! checkout (every repository has at least one commit once `HEAD` resolves
//! at all), so it unambiguously means "not built from a git checkout"
//! without mixing a word into an otherwise all-numeric, Roblox-shaped
//! identifier.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The applet folders at the repository root that are built into `dew`.
/// The same set as `bundled::APPLETS`, which a test in `host/src` checks
/// against the table generated from this list.
const BUNDLED_IDS: &[&str] = &["dashboard", "quickpanel"];

/// The file `bundled.rs` includes, in `OUT_DIR`.
const BUNDLED_TABLE: &str = "bundled_applets.rs";

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn main() {
    // dew.ico, embedded into dew.exe itself: see host/Cargo.toml's own
    // comment on the embed-resource dependency for why a file shipped
    // beside the exe was never actually reachable from a real release.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/dew.rc");
        println!("cargo:rerun-if-changed=assets/dew.ico");
        embed_resource::compile("assets/dew.rc", embed_resource::NONE);
    }

    // The bundled applets exist only on Windows, as `bundled.rs` does.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_bundled_applets();
    }

    let count = git(&["rev-list", "--count", "HEAD"]).unwrap_or_else(|| "0".to_string());
    println!("cargo:rustc-env=DEW_BUILD_NUMBER={count}");

    // REBUILD WHEN THE COMMIT CHANGES, not just when a source file does --
    // otherwise cargo sees no reason to rerun this script and the embedded
    // count goes stale the moment a commit lands with no code change.
    //
    // `git rev-parse --git-dir` resolves the REAL git directory, which
    // `.git/HEAD` assumes is a directory beside the checkout and is wrong for
    // a linked worktree, where `.git` is a file pointing elsewhere. A plain
    // commit on a branch changes the ref file HEAD points at, not HEAD
    // itself (`ref: refs/heads/main` never changes from a commit alone), so
    // both are watched; a detached HEAD has no such ref and is covered by
    // watching HEAD alone, since detached commits rewrite it directly.
    if let Some(git_dir) = git(&["rev-parse", "--git-dir"]) {
        let git_dir = PathBuf::from(git_dir);
        let head = git_dir.join("HEAD");
        println!("cargo:rerun-if-changed={}", head.display());
        if let Ok(contents) = std::fs::read_to_string(&head) {
            if let Some(ref_path) = contents.trim().strip_prefix("ref: ") {
                println!(
                    "cargo:rerun-if-changed={}",
                    git_dir.join(ref_path).display()
                );
            }
        }
    }
}

/// Write the table of every file in each bundled applet's folder, as
/// `include_bytes!` of its absolute path, so `dew.exe` carries its own
/// dashboard and quick panel and needs no checkout beside it.
///
/// THE SAME FILES THE APPLET HAS ON DISK, packages included. An applet
/// `require`s from its own `roblox_packages/`, which `pesde install` fills
/// and git ignores, so it is embedded like any other folder. Regular files
/// are embedded and directories recursed into; a symlink, a junction or
/// anything else is skipped rather than followed, so a link cannot pull a
/// file from outside the folder into the binary. A hardlink is a regular
/// file and is embedded.
fn embed_bundled_applets() {
    let root = Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("host/ has a parent")
        .to_path_buf();

    let mut out = String::from(
        "// Generated by host/build.rs from the bundled applet folders.\n\
         static EMBEDDED: &[EmbeddedApplet] = &[\n",
    );
    for id in BUNDLED_IDS {
        let dir = root.join(id);
        // A folder that appears or changes anywhere below the applet,
        // `roblox_packages/` included, reruns this script.
        println!("cargo:rerun-if-changed={}", dir.display());
        require_installed_packages(&dir);

        let mut dirs = Vec::new();
        let mut files = Vec::new();
        walk(&dir, "", &mut dirs, &mut files);
        assert!(
            files.iter().any(|(rel, _)| rel == "dew.toml"),
            "{}: no dew.toml, so this is not the bundled applet's folder",
            dir.display()
        );

        writeln!(
            out,
            "    EmbeddedApplet {{\n        id: {id:?},\n        dirs: &["
        )
        .unwrap();
        for rel in &dirs {
            println!("cargo:rerun-if-changed={}", dir.join(rel).display());
            writeln!(out, "            {rel:?},").unwrap();
        }
        out.push_str("        ],\n        files: &[\n");
        for (rel, path) in &files {
            println!("cargo:rerun-if-changed={}", path.display());
            let path = path
                .to_str()
                .unwrap_or_else(|| panic!("{}: not a UTF-8 path", path.display()));
            writeln!(out, "            ({rel:?}, include_bytes!({path:?})),").unwrap();
        }
        out.push_str("        ],\n    },\n");
    }
    out.push_str("];\n");

    let dest = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join(BUNDLED_TABLE);
    std::fs::write(&dest, out).unwrap_or_else(|e| panic!("{}: {e}", dest.display()));
}

/// Collect the directories and regular files under `dir`, as paths relative
/// to the applet folder with `/` between components, in name order.
fn walk(dir: &Path, rel: &str, dirs: &mut Vec<String>, files: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap_or_else(|e| panic!("{}: {e}", dir.display())))
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name
            .to_str()
            .unwrap_or_else(|| panic!("{}: not a UTF-8 name", path.display()));
        let child = if rel.is_empty() {
            name.to_string()
        } else {
            format!("{rel}/{name}")
        };
        // `DirEntry::file_type` does not follow links: a symlink or a
        // junction reports as neither a directory nor a file.
        let file_type = entry
            .file_type()
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if file_type.is_dir() {
            dirs.push(child.clone());
            walk(&path, &child, dirs, files);
        } else if file_type.is_file() {
            files.push((child, path));
        }
    }
}

/// Fail the build when an applet declares packages and they are not
/// installed. Without them the embedded applet could not `require` its
/// framework, and nothing would say so until a user opened it.
fn require_installed_packages(dir: &Path) {
    let manifest = dir.join("pesde.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return;
    };
    let packages = dir.join("roblox_packages");
    let missing: Vec<&str> = declared_dependencies(&text)
        .into_iter()
        .filter(|name| !packages.join(format!("{name}.luau")).is_file())
        .collect();
    if !missing.is_empty() {
        panic!(
            "\n{} has no installed {}, which {} declares.\n\
             The bundled applets are built into dew with their packages, \
             so run `pesde install` in {} and build again.\n",
            packages.display(),
            missing.join(", "),
            manifest.display(),
            dir.display()
        );
    }
}

/// The names under `[dependencies]` in a `pesde.toml`.
fn declared_dependencies(text: &str) -> Vec<&str> {
    let mut in_section = false;
    let mut names = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == "[dependencies]";
            continue;
        }
        if !in_section || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim();
            if !name.is_empty() {
                names.push(name);
            }
        }
    }
    names
}
