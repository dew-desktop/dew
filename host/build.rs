//! Embeds a monotonic build number into `dew`'s build identifier at compile
//! time.
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
    // dew.ico, embedded into dew.exe itself: see host/Cargo.toml's own
    // comment on the embed-resource dependency for why a file shipped
    // beside the exe was never actually reachable from a real release.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/dew.rc");
        println!("cargo:rerun-if-changed=assets/dew.ico");
        embed_resource::compile("assets/dew.rc", embed_resource::NONE);
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
