//! `aether_runtime` — the Rust-owned host runtime for Aether.
//!
//! ## What this is for
//!
//! An Aether application is a Luau module. On Roblox, the engine hosts it: it
//! lays out, it paints, and `UserInputService` reports input. Off Roblox, THIS
//! crate is the engine — it owns the process, embeds Luau as a guest, drives
//! frames, and hands a display list to a painter.
//!
//! The same application source runs on both. That is the point, and it is
//! structural rather than aspirational: `Host.detect()` in the framework picks
//! its host by environment, and everything that decides anything about a UI
//! stays in Luau on both sides. Rust never decides where a thing goes or what a
//! click means.
//!
//! ```text
//!   Aether (Luau)  ──Live.Frame──>  aether_runtime  ──>  impl Painter
//!        ^                                │
//!        └────── Pointer / Key / Step ────┘
//! ```
//!
//! ## Two shells, one runtime
//!
//! The `aether` CLI and the Dew desktop platform both build on this crate and
//! differ ONLY in which capabilities they grant. The CLI runs the author's own
//! code, the way `cargo run` does; Dew runs code fetched from strangers. Neither
//! gets a different pipeline, and neither gets a VM that skips the sandbox — see
//! [`vm`] for why the trusted shell takes the guarded path too.
//!
//! ## What this crate deliberately does not contain
//!
//! No window, no swapchain, no `ffi`, and no `libloading`. Windows and surfaces
//! belong to a shell; native code belongs to a painter behind [`painter::Painter`].
//! The guest reaches the outside world only through capability tables the host
//! installs by name, which is the property that makes a permission model possible
//! at all.

pub mod driver;
pub mod font;
pub mod frame;
pub mod modules;
pub mod painter;
pub mod requirer;

#[cfg(feature = "raster")]
pub mod raster;
pub mod session;
pub mod vm;

pub use driver::Driver;
pub use frame::{Delta, Frame, Node, Rect, Rgb};
pub use painter::Painter;
#[cfg(feature = "raster")]
pub use raster::RasterPainter;
pub use session::{Modifiers, Pointer, Session, Stats};
pub use vm::{Capabilities, Vm};

use mlua::prelude::*;
use std::path::{Path, PathBuf};

/// Drop Windows' extended-length path prefix.
///
/// `canonicalize` returns the extended-length form (backslash, backslash,
/// question mark, backslash, then the drive), and almost nothing downstream
/// accepts it: `FsRequirer` cannot reset its context to one, and it survives into
/// a `.luaurc` as `//?/C:/...`, where it reads as a UNC host rather than a drive.
/// Both failures point somewhere other than the path.
///
/// The prefix is spelled out in prose above because it is four characters of
/// pure backslash-escaping, and a draft of this function shipped with one too
/// few — matching nothing, stripping nothing, and leaving every symptom intact.
pub fn strip_extended_prefix(path: PathBuf) -> PathBuf {
    const PREFIX: &str = r"\\?\";
    match path.to_string_lossy().strip_prefix(PREFIX) {
        Some(stripped) => PathBuf::from(stripped),
        None => path,
    }
}

/// Where a pesde-installed guest package's own tree sits.
///
/// THE PREMISE CHANGED WITH ADR-004, AND THE FUNCTION HAD TO CHANGE WITH IT.
/// This used to be `luau_source_root()`, and it answered `CARGO_MANIFEST_DIR/../..`
/// — which worked because this crate lived in Aether's repository, so the
/// checkout Cargo made for the pinned revision WAS the framework's Luau source,
/// same revision by construction. That is exactly the arrangement this crate
/// moving to Dew dissolves: there is no Aether checkout above this file any
/// more, and `../..` is now Dew's own root.
///
/// So Aether arrives the way every other guest package arrives — through pesde,
/// pinned by commit in `pesde.toml`, beside vide. `roblox_packages/` is found by
/// walking up from the working directory, the same walk `mods` already does, and
/// the version directory pesde writes is GLOBBED rather than spelled out: naming
/// one here would go stale on the next resolve and report itself as "no aether"
/// rather than as "a different aether".
///
/// The two layouts pesde writes are both searched, because a wally package and a
/// git package do not nest alike:
///
/// ```text
///   roblox_packages/.pesde/centau_vide@0.4.1/vide/src        (wally)
///   roblox_packages/.pesde/spektr+aether/<version>/aether/src (git)
/// ```
///
/// Returns the directory CONTAINING `src`, not `src` itself: an Aether consumer
/// needs the package root to reach `src/host/Desktop.luau`, and the `@aether`
/// alias is that root's `src`.
pub fn installed_package(name: &str) -> Option<PathBuf> {
    let mut cur = std::env::current_dir().ok()?;
    let base = loop {
        let candidate = cur.join("roblox_packages").join(".pesde");
        if candidate.is_dir() {
            break candidate;
        }
        if !cur.pop() {
            return None;
        }
    };

    /// A directory named `name` with a `src` inside it, at most `depth` levels
    /// down. Bounded rather than unbounded: a package's own vendored
    /// `roblox_packages` is further down than either layout above, and finding
    /// Aether's copy of vide instead of ours would be a bug that looks like a
    /// version skew.
    fn search(dir: &Path, name: &str, depth: usize) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if entry.file_name().to_string_lossy() == name && path.join("src").is_dir() {
                return Some(path);
            }
            if depth > 0 {
                if let Some(found) = search(&path, name, depth - 1) {
                    return Some(found);
                }
            }
        }
        None
    }

    search(&base, name, 2)
        .map(|p| strip_extended_prefix(p.canonicalize().unwrap_or_else(|_| p.clone())))
}

/// A loaded application, before it is driven.
pub struct Application {
    vm: Vm,
    /// The value the entry module returned. Held as a registry-independent handle
    /// so the shell can hand it back to the framework when it opens a session.
    entry: LuaTable,
}

impl Application {
    /// Load an application's entry module under a fresh guest VM.
    ///
    /// The entry module must return a table carrying a `Session` field — the
    /// result of `Live.Session(host, root, router, w, h)`. That indirection is
    /// deliberate: it keeps the decision of HOW to mount (which root, which
    /// router, which dimensions) in Luau, where the Roblox entry point makes the
    /// same decision with the same code.
    pub fn load(caps: Capabilities, entry: &Path) -> LuaResult<Self> {
        let vm = Vm::new(caps.clone())?;
        modules::install(&vm, &caps)?;
        let chunk = modules::load_entry(&vm, entry)?;
        let entry: LuaTable = chunk.call(())?;
        Ok(Application { vm, entry })
    }

    pub fn vm(&self) -> &Vm {
        &self.vm
    }

    /// Read any other field the entry module returned.
    ///
    /// Entry points expose more than a `Session` — a size to open a window at, a
    /// transition a shell can drive, a title. Rather than growing a struct field
    /// per convention, a shell asks for what it knows it needs and handles the
    /// absence: an entry written for the CLI should not fail to load under Dew
    /// merely because it omitted something Dew never reads.
    pub fn get<T: FromLua>(&self, field: &str) -> LuaResult<T> {
        self.entry.get(field)
    }

    /// Bind to the application's `Live.Session`.
    pub fn session(&self) -> LuaResult<Session> {
        let t: LuaTable = self.entry.get("Session").map_err(|_| {
            LuaError::RuntimeError(
                "the entry module returned no `Session` field — an Aether application's \
                 entry point returns { Session = Live.Session(...) }"
                    .into(),
            )
        })?;
        Session::from_lua(self.vm.lua(), &t)
    }
}
