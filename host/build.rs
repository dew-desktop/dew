//! Embeds a short commit sha into `dew`'s build identifier at compile time.
//!
//! COMPILE TIME, NOT RUNTIME. `dew.exe` ships to machines with no `.git`
//! directory at all, so a `git` invocation inside the running program would
//! fail exactly where the identifier matters most -- a crash report from a
//! real install. `cargo:rustc-env` bakes the answer in once, here, where a
//! checkout is expected to exist.
//!
//! NO `.git` IS NOT A BUILD FAILURE. A source tarball or a vendored crate
//! directory has no `.git` either, and refusing to compile because of it
//! would be strictly worse than an honest `nogit` in the identifier.

use std::path::PathBuf;
use std::process::Command;

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
    let sha = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "nogit".to_string());
    println!("cargo:rustc-env=DEW_BUILD_SHA={sha}");

    // REBUILD WHEN THE COMMIT CHANGES, not just when a source file does --
    // otherwise cargo sees no reason to rerun this script and the embedded
    // sha goes stale the moment a commit lands with no code change.
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
