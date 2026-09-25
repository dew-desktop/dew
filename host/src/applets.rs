//! Loading a mod: manifest, then VM, then capabilities, then its own code.
//!
//! THE ORDER IS THE POINT. Everything the host needs to decide whether a mod may
//! run is known before the mod's logic executes:
//!
//!   1. `dew.toml` is read from disk. It names the mod and its permissions.
//!   2. A VM is created — deny-by-default, with no ffi, io, os or ambient `desktop`.
//!   3. The `desktop` table is built from the GRANTED permissions and nothing else.
//!   4. The mod's module is loaded. It returns a declaration and does nothing.
//!   5. `mount` is called once, and builds a tree.
//!
//! A registration-style API collapses 4 and 5 into "loading the mod runs the
//! mod", which puts every one of the earlier steps after the fact.
//!
//! ONE LOADER, NO FRAMEWORK NAMED. The host gives a mod a root and the `desktop`
//! table and knows nothing else about what the mod is built from: discovery,
//! the manifest, the sandbox, the capability table, the size and the surface
//! are decided the same way for every mod, and `mount(desktop, root)` is the one
//! signature there is.

use crate::capabilities::{self, Shared};
use crate::datamodel;
use crate::manifest::{Capability, Manifest};
use crate::services::{self, Clock, PointerState, SharedClock, SharedPointer};
use crate::surface::{Declared, Requested};
use dew_runtime::{modules, Vm};
use mlua::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The living half of a mounted mod: what the frame loop drives.
///
/// AN ENUM WITH ONE VARIANT TODAY, because `main.rs` and the tests both match
/// on it (`Mounted::DataModel { .. }`) rather than reading a bare struct.
pub enum Mounted {
    /// A DataModel tree, rendered from the arena with `render::frame_of`.
    ///
    /// NO REACTIVITY, so nothing here can report that a frame is unnecessary.
    /// The host re-renders every frame, which is the honest answer until events
    /// and change signals exist (sprints 8 and beyond) — see `main.rs`.
    DataModel {
        dom: datamodel::SharedDom,
        root: usize,
    },
}

pub struct Applet {
    pub manifest: Manifest,
    pub width: u32,
    pub height: u32,
    pub surface: Declared,
    pub mounted: Mounted,
    /// Frame subscriptions this mod made, for the loop to drive.
    ///
    /// ON `Mod` RATHER THAN INSIDE `Mounted`, and that is the point of it. A
    /// clock is not a property of what `Mounted` holds -- so putting it in the
    /// enum would have made "can this mod animate" depend on that.
    pub clock: SharedClock,
    /// Where this mod's `desktop.Pointer`/`desktop.Input` believe the cursor is.
    ///
    /// ONE PER APPLET, LIKE THE CLOCK ABOVE, and for the same reason a shared
    /// one would be wrong: a process running more than one applet at once has
    /// more than one cursor position to report, one per window, and a single
    /// process-wide cell can only ever hold the last one written -- see
    /// `services::pointer`'s doc comment for the failure this was built to
    /// stop. The frame loop feeds it through `services::pointer_moved_on` and
    /// friends rather than through the process-global `pointer_moved`.
    pub pointer: SharedPointer,
    /// The VM this mod lives in. Held because dropping it takes the mod's Lua
    /// handles with it — a mod is exactly as alive as its VM: a `SharedDom` is
    /// full of `InstanceRef`s the guest also holds.
    pub vm: Vm,
}

/// Default widget size when a mod declares none.
const DEFAULT_SIZE: (u32, u32) = (380, 56);

fn size_from(declaration: &LuaTable) -> (u32, u32) {
    let Ok(size) = declaration.get::<LuaTable>("size") else {
        return DEFAULT_SIZE;
    };
    let width: u32 = size.get("width").unwrap_or(0);
    let height: u32 = size.get("height").unwrap_or(0);
    if width > 0 && height > 0 {
        (width, height)
    } else {
        DEFAULT_SIZE
    }
}

// `install_gate` LIVED HERE AND IS GONE, which is the end of a four-day
// detour worth one comment.
//
// vide read `game` for truthiness alone to decide whether `typeof`, `Instance`,
// `Enum` and `Color3` were real, and fell back to a `require "../test/mock"` the
// published artifact does not ship. So Dew installed `game = true` -- a boolean,
// not a DataModel and not a service locator -- purely to open that gate.
//
// centau/vide#89 has vide guard those names on themselves, and aether#3 has
// `VideCore.full()` mirror it by asking about `Instance`. A host supplying a
// DataModel supplies `Instance`, so there is nothing left for the global to do.
//
// That closes ADR-001's amendment. `game` was dropped from step F on
// 2026-09-04, came back the same day for this one consumer, and is now gone for
// the reason it should always have been gone: nothing reads it.

/// Did this applet load from the coordinator's own bundled directory
/// (ADR-017), rather than `installed::list()`'s user-writable one or an
/// arbitrary `dew run <dir>` path? Canonicalized on both sides so a
/// relative `dir` or a symlink cannot read as bundled by accident.
///
/// `installed.rs`, and the bundled directory it defines, exist only on
/// Windows; off it there is no bundling mechanism at all, so nothing ever
/// reads as bundled and a `Capability::Host` permission can never be
/// granted.
#[cfg(windows)]
fn is_bundled(dir: &Path) -> bool {
    let Some(bundled) = crate::installed::bundled_applets_dir() else {
        return false;
    };
    let (Ok(dir), Ok(bundled)) = (dir.canonicalize(), bundled.canonicalize()) else {
        return false;
    };
    dir.starts_with(&bundled)
}

#[cfg(not(windows))]
fn is_bundled(_dir: &Path) -> bool {
    false
}

pub fn load(
    dir: &Path,
    aliases: &std::collections::HashMap<String, PathBuf>,
    state: &Shared,
) -> Result<Applet, String> {
    // 1 ── the manifest, before anything of the mod's runs.
    let manifest = Manifest::load(dir)?;

    //      A HOST-ONLY PERMISSION IS A LOAD-TIME REFUSAL FROM ANYWHERE BUT
    //      THE BUNDLE (ADR-017), not a permission the mod simply does not
    //      get. `Permission::Widget` asked for and not granted is merely
    //      absent from `desktop` below; `Permission::Install` asked for by a
    //      mod that can never receive it is the same "refused rather than
    //      ignored" treatment `manifest.rs` already gives an unknown
    //      permission, for the same reason -- a mod author believing
    //      something was granted when it silently was not is the worse
    //      failure.
    let host_only: Vec<_> = manifest
        .permissions
        .iter()
        .copied()
        .filter(|p| p.capability() == Capability::Host)
        .collect();
    if !host_only.is_empty() && !is_bundled(dir) {
        let names = host_only
            .iter()
            .map(|p| p.name())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "{}: {names} -- host-only, and this applet did not load from Dew's own bundled directory",
            manifest.id
        ));
    }

    //      SAID OUT LOUD, BEFORE THE MOD RUNS. What the manifest asked for and
    //      the host will not do is reported here rather than discovered by an
    //      author wondering why their keybinding does nothing. Warnings, not
    //      errors: a mod whose hotkeys are inert still renders, and refusing to
    //      load it would be a worse answer than saying which half works.
    for problem in manifest.unhonoured() {
        eprintln!("[dew] {}: {problem}", manifest.id);
    }

    //      EXPERIMENTAL FLAGS, RESOLVED AND SET BEFORE ANY OF THE MOD'S LUAU
    //      RUNS. `experimental_flags` refuses to load on an unknown entry, so
    //      that surfaces here rather than as a property that silently never
    //      unlocks. The effective set -- manifest UNION the local
    //      `DewAppSettings.json` override -- is printed unconditionally and
    //      then set on THIS thread: an applet runs entirely on the thread
    //      that called `load` (its own, once `coordinator::spawn_applet`
    //      exists), so the thread-local this sets is read by the same
    //      `describe`/`class_exists`/enum lookups the mod's own code drives.
    let declared_flags = manifest.experimental_flags()?;
    let effective_flags = datamodel::extensions::effective_flags(&declared_flags);
    datamodel::extensions::print_effective(&effective_flags);
    datamodel::extensions::set_enabled_flags(&effective_flags);

    let entry = manifest
        .entry(dir)
        .ok_or_else(|| format!("{}: no {}.luau or main.luau", dir.display(), manifest.id))?;

    // 2 ── a VM that can reach the mod's own directory and Aether, and nothing
    //      else. Two roots rather than one: a mod requiring a sibling mod's files
    //      is not a thing this platform supports, and the resolver is where that
    //      is enforced rather than checked for later.
    let caps = dew_runtime::Capabilities {
        require_roots: vec![dir.to_path_buf()],
        // A mod's `print` is the author's own debugging and goes to the console
        // the host was launched from.
        print: true,
        // Aether, and the dependencies Aether declares that this host installs.
        aliases: aliases.clone(),
    };
    let vm = Vm::new(caps.clone()).map_err(|e| format!("{}: {e}", manifest.id))?;

    //      THE DATAMODEL IS PER MOD, like the VM. Two mods sharing one instance
    //      tree could reach each other's widgets by walking Parent, which is the
    //      same isolation the require roots above enforce for files. It is
    //      installed as a GLOBAL rather than passed like `desktop`, because it is not
    //      a capability: it is the language of the platform, present for every
    //      guest on the engine and on Dew alike, and an application that had to be
    //      handed it would not be the application that runs on both.
    let dom = datamodel::SharedDom::default();
    //      AND `mod://` POINTS AT THE MOD'S OWN DIRECTORY, which is the same
    //      boundary `require_roots` draws one statement above. A mod reaches its
    //      own files and no others, whether it asks for them with `require` or
    //      with an `Image` property; images arriving later must not become the
    //      way around a rule the requirer already enforces.
    dom.lock().expect("dom").assets.set_root(dir.to_path_buf());
    dom.lock()
        .expect("dom")
        .assets
        .set_permissions(manifest.permissions.clone());
    let dom_weak = std::sync::Arc::downgrade(&dom);
    dom.lock()
        .expect("dom")
        .assets
        .set_dirty_hook(std::sync::Arc::new(move || {
            if let Some(d) = dom_weak.upgrade() {
                d.lock().expect("dom").touch();
            }
        }));
    datamodel::install(vm.lua(), &dom).map_err(|e| format!("{}: {e}", manifest.id))?;

    //      AND `desktop.Text`/`desktop.Clock`, ON THE SAME TERMS AND FOR THE SAME REASON.
    //      Text metrics and a frame clock are what the host computes and no guest
    //      can: they are the language of the platform rather than a capability, so
    //      they are installed as ungated members of the `desktop` global here rather
    //      than granted in the table built at step 4. `services.rs` carries the
    //      full argument, and the short version is that a mod refused text metrics
    //      cannot lay out -- a permission with only one sound answer is not a
    //      permission.
    //
    //      FOR BOTH RUNTIMES. A DataModel mod calls `desktop.Text.Measure`
    //      directly; an Aether mod reaches the same functions through the
    //      `Host.Text` and `Host.Clock` seams its interface already declares.
    let clock: SharedClock = std::sync::Arc::new(std::sync::Mutex::new(Clock::default()));
    //      ITS OWN POINTER, NOT THE PROCESS-WIDE ONE. `services::install` wires
    //      `desktop.Pointer`/`desktop.Input` to the one cell every guest used to share,
    //      which was correct for a process running exactly one applet and is
    //      not any more -- see `SharedPointer`'s doc comment above.
    let pointer: SharedPointer = Arc::new(Mutex::new(PointerState::default()));
    services::install_with_pointer(vm.lua(), &clock, &pointer)
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    //      AND THE VALUE VOCABULARY, FOR BOTH RUNTIMES SINCE SPRINT 6. `UDim2`,
    //      `Color3`, `Enum` and the rest are the language of the platform on the
    //      same terms as `Instance`, and this line used to be in the DataModel arm
    //      alone because a PARTIAL host vocabulary is worse than none: Aether
    //      publishes its own with `if rawget(g, name) == nil` -- first writer wins
    //      -- so Dew's five types did not merge with Aether's eleven, they BLOCKED
    //      them, and `Color3.fromHex` going missing stopped all three mods.
    //
    //      WHAT MADE IT SAFE IS NOT THAT THE CONFLICT WAS RESOLVED, IT IS THAT THE
    //      SECOND WRITER LEFT. Under Aether's DataModel host `InstallVocabulary`
    //      is `function() end` -- what a host says when the environment already
    //      supplies the vocabulary, exactly as on the engine -- so nothing publishes a
    //      second one and there is nothing to win a race against. The other half
    //      is that the host's `available()` probe REQUIRES the vocabulary to be
    //      here: with these names missing the DataModel host is not selected at
    //      all, so this call is not an optimisation, it is the precondition.
    //
    //      Which is why the gap had to close first, in `vocabulary.rs`: whatever
    //      is absent here is now absent everywhere, and it surfaces as a nil index
    //      in a component rather than as anything naming this decision.
    datamodel::install_vocabulary(vm.lua()).map_err(|e| format!("{}: {e}", manifest.id))?;

    modules::install(&vm, &caps).map_err(|e| format!("{}: {e}", manifest.id))?;

    // 3 ── the root every mod parents into, made BEFORE the mod's own module
    //      is loaded.
    //
    //      `DewRoot` RATHER THAN `game`. A mod is handed the root it parents
    //      into, and `DewRoot` names it. It needs no vide, so it is not what
    //      the gate above is for and it does not get one -- which is the
    //      whole of what keeps `game` from becoming a sentinel a second time.
    //
    //      A `ScreenGui` because that is what an engine application expects
    //      to find above its tree, so the same mod has a chance of running in
    //      both places -- which is a property of the TREE rather than of
    //      anything named `game`.
    //
    //      HANDED TO `mount`, NOT INSTALLED AS A GLOBAL. `examples/host/standalone`
    //      reaches for a `DewRoot` global because a bare script has no
    //      function to receive one; a mod has `mount`, and a parameter is the
    //      same argument that keeps `desktop` off the globals table — what a mod
    //      is GIVEN is visible at its own call site.
    let root = dom
        .lock()
        .expect("dom")
        .insert("ScreenGui".into(), "DewRoot".into());

    // 4 ── the capability table, from the granted permissions ONLY.
    let requested: Requested = Arc::new(Mutex::new(None));
    let grant = capabilities::SurfaceGrant {
        requested: Arc::clone(&requested),
        //  THE HANDLE, NOT THE NODE ID. `root` is an index into the DOM and
        //  handing that over parents instances into an integer; the applet
        //  wants the same userdata `mount` was always given.
        root: Some(
            datamodel::handle(vm.lua(), &dom, root)
                .map_err(|e| format!("{}: {e}", manifest.id))?
                .into_lua(vm.lua())
                .map_err(|e| format!("{}: {e}", manifest.id))?,
        ),
        title: manifest.display_name().to_string(),
    };
    let desktop = capabilities::build(vm.lua(), &manifest.permissions, state, &grant)
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    // 5 ── the applet's own module. It asks for a surface, or it returns a
    //      declaration describing one. Both arrive from running it.
    //  `desktop` IS A GLOBAL, the way `game` is one in the engine this host is
    //  shaped after. It was handed to `mount` as an argument, which is why the
    //  old contract had to return a table: there was no other way to be given a
    //  capability table.
    //
    //  NOT THE CHUNK'S VARARG. A chunk's `...` reaches the entry module and stops
    //  there, so an applet split across two files could not see `desktop` from
    //  the second one without threading it through every call that needed it.
    //  It is also not an idiom an applet author has met: a ModuleScript's chunk
    //  receives nothing, so `local desktop = ...` means nothing in the engine.
    //
    //  AN UNGRANTED CAPABILITY IS STILL ABSENT RATHER THAN GUARDED. That comes
    //  from which keys this table has, which `capabilities::build` decides from
    //  the manifest, and not from how the table is delivered.
    vm.lua()
        .globals()
        .set("desktop", desktop.clone())
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    let returned: LuaValue = modules::load_entry(&vm, &entry)
        .and_then(|f| f.call(()))
        .map_err(|e| format!("{}: {e}", manifest.id))?;
    let declaration = match returned {
        LuaValue::Table(table) => table,
        //  NOTHING RETURNED IS THE SHAPE THIS IS MOVING TO. An applet that asked
        //  for a surface has already said everything the host needed, so an
        //  empty table stands in and every read below finds nothing in it.
        _ => vm
            .lua()
            .create_table()
            .map_err(|e| format!("{}: {e}", manifest.id))?,
    };

    let asked = requested.lock().expect("requested").clone();

    //  WHAT THE APPLET ASKED FOR WINS over what it returned. An applet doing
    //  both is mid-migration rather than in conflict, and the call is the newer
    //  of the two statements.
    //
    // The window caption a mod gets when it declares no `title` of its own is its
    // manifest `name`, falling back to the id. An applet called "Time Tracker &
    // Pomodoro HUD" in its manifest should not present itself as `timetracker`.
    let surface = match &asked {
        Some(request) => request.surface.clone(),
        None => Declared::from_declaration(&declaration, manifest.display_name()),
    };
    let (width, height) = match asked.as_ref().and_then(|r| r.size) {
        Some(size) => size,
        None => size_from(&declaration),
    };

    // A SURFACE IS A GRANT (ADR-012), and this is where the asking is checked.
    //
    // Every applet used to get one without saying so. Refusing here rather than
    // at paint time means an applet that wants a screen-covering overlay is
    // turned away before it has run, and the message names the word to add.
    let needed = surface.permission();
    if !manifest.permissions.contains(&needed) {
        return Err(format!(
            "{}: this applet uses a {} surface and dew.toml does not grant it; add {:?} to `permissions`.",
            manifest.id,
            needed.name(),
            needed.name()
        ));
    }

    // THE ONE SIGNATURE THERE IS, named for the message when it is missing.
    let signature = "mount = function(desktop, root) … end";
    //  AN APPLET THAT ASKED FOR ITS SURFACE HAS ALREADY BUILT ITS TREE. It was
    //  handed the root by `desktop.Widget{}` while it ran, so there is nothing left
    //  for the host to call and no declaration to read. That is the shape this
    //  is moving to; the returned table is what it is moving from.
    let mount: Option<LuaFunction> = match declaration.get::<LuaFunction>("mount") {
        Ok(function) => Some(function),
        Err(_) if asked.is_some() => None,
        Err(_) => {
            return Err(format!(
                "{}: the module neither asked for a surface nor returned a `mount`.                  Call `desktop.Widget{{ width = 200, height = 100 }}` and parent your                  tree into what it returns, or return {{ size = ..., {signature} }}.                  See docs/applet_contract.md",
                manifest.id
            ))
        }
    };

    // 6 ── build the tree, ONCE.
    //
    //      CALLED DIRECTLY, because there is no scope to be inside. `mount`
    //      parents instances and returns; its return value is deliberately
    //      ignored, since the tree the host renders is the one under the root
    //      it was handed rather than one it was given back. Handing a root
    //      over and then reading a returned tree would be two answers to the
    //      same question.
    //
    //      NOTHING TO CALL WHEN THE APPLET ALREADY BUILT ITS TREE.
    //      `desktop.Widget{}` handed it this same root while it ran, so the
    //      instances are under there already and calling a second entry point
    //      would ask it to build them twice.
    if let Some(mount) = mount {
        let handle =
            datamodel::handle(vm.lua(), &dom, root).map_err(|e| format!("{}: {e}", manifest.id))?;
        mount
            .call::<()>((desktop, handle))
            .map_err(|e| format!("{}: while mounting: {e}", manifest.id))?;
    }
    let mounted = Mounted::DataModel {
        dom: dom.clone(),
        root,
    };

    println!(
        "[dew] loaded {} ({}x{}) — {} — granted: {}",
        manifest.id,
        width,
        height,
        surface.describe(),
        capabilities::describe(&manifest.permissions)
    );

    Ok(Applet {
        manifest,
        width,
        height,
        surface,
        mounted,
        clock,
        pointer,
        vm,
    })
}

/// WHAT THIS COVERS is the loader itself: that a mod reaches `mount(desktop, root)`
/// with the vocabulary present and a root to parent into, and that what it
/// parented is what the renderer finds. It goes through the real `load` —
/// manifest, sandbox, capability table and all — rather than calling the
/// DataModel machinery directly, because that is the path a mod actually
/// takes.
#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Held for the duration of every test that reads or writes the real
    /// `session.json` -- `dew_marketplace_triggers_with_a_callback_and_never_blocks`
    /// and `dew_account_reports_session_state_and_signs_out` both call
    /// `platform::clear_session()`/`save_session()` against the one file on
    /// disk, and Rust runs tests in parallel by default. Without this, one
    /// test's fixture session can be visible to the other mid-run -- not a
    /// hermetic failure of either test's own logic, just two tests racing on
    /// a real, unpartitioned shared resource.
    #[cfg(windows)]
    static SESSION_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// A mod on disk, in a directory of its own, removed when the test ends.
    ///
    /// The loader reads `dew.toml` and an entry module from a real directory,
    /// which is the behaviour under test; faking the filesystem here would test
    /// a different loader.
    pub struct Fixture(PathBuf);

    impl Fixture {
        pub fn new(name: &str, manifest: &str, entry: &str) -> Fixture {
            let dir = std::env::temp_dir().join(format!("dew-mods-test-{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("temp dir");
            std::fs::write(dir.join("dew.toml"), manifest).expect("dew.toml");
            std::fs::write(dir.join("main.luau"), entry).expect("entry");
            Fixture(dir)
        }

        /// A second file beside the entry, for an applet that is not one file.
        pub fn write(&self, name: &str, source: &str) {
            std::fs::write(self.0.join(name), source).expect("extra module");
        }

        pub fn load(&self) -> Result<Applet, String> {
            let state: Shared = Arc::new(Mutex::new(capabilities::HostState::default()));
            load(&self.0, &Default::default(), &state)
        }
    }

    /// A SURFACE IS A GRANT, AND THE REFUSAL NAMES THE WORD TO ADD.
    ///
    /// Every applet used to get a surface without asking. An author who has just
    /// learned that surfaces are permissions should be told which one, not that
    /// something was denied.
    #[test]
    fn an_applet_may_not_use_a_surface_it_did_not_declare() {
        let fixture = Fixture::new(
            "ungranted",
            "id = \"plain\"
", // no surface on purpose
            r#"
                return {
                    id = "plain",
                    size = { width = 10, height = 10 },
                    mount = function(_dew, _root) end,
                }
            "#,
        );

        let err = match fixture.load() {
            Err(e) => e,
            Ok(_) => panic!("an applet with no surface permission must not load"),
        };

        assert!(
            err.contains("widget") && err.contains("permissions"),
            "the refusal should name the permission to add, got: {err}"
        );
    }

    /// THE ISOLATION IS ASSERTED, NOT ASSUMED.
    ///
    /// Before milestone 6 the host granted every mod Aether's source directory
    /// as a second require root and injected `@aether` and `@vide`. A mod that
    /// forgot to declare a dependency worked anyway, which is why no mod had a
    /// manifest for two years.
    ///
    /// The failure has to name the alias. "attempt to index nil" from three
    /// requires deeper is the same bug with a worse error, and it is what a mod
    /// author sees if this ever regresses.
    #[test]
    fn a_mod_cannot_reach_a_framework_it_did_not_declare() {
        let fixture = Fixture::new(
            "undeclared",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            r#"
                local Aether = require("@aether/api")
                return { id = "plain", size = { width = 10, height = 10 } }
            "#,
        );

        let err = match fixture.load() {
            Err(e) => e,
            Ok(_) => panic!("a mod reaching for an undeclared framework must not load"),
        };

        assert!(
            err.contains("@aether"),
            "the failure should name the alias the mod asked for, got: {err}"
        );
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A mod inside the coordinator's OWN bundled directory, the one place
    /// a `Capability::Host` permission can be granted from (ADR-017).
    /// Writes into the real `bundled_applets_dir()` `applets::load` itself
    /// checks against -- faking that path would test a different check
    /// than the real one, the same reasoning `Fixture` above already gives
    /// for using a real directory rather than a fake filesystem.
    #[cfg(windows)]
    pub struct BundledFixture(PathBuf);

    #[cfg(windows)]
    impl BundledFixture {
        pub fn new(name: &str, manifest: &str, entry: &str) -> BundledFixture {
            let dir = crate::installed::bundled_applets_dir()
                .expect("bundled applets dir")
                .join(format!("dew-mods-test-{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("bundled dir");
            std::fs::write(dir.join("dew.toml"), manifest).expect("dew.toml");
            std::fs::write(dir.join("main.luau"), entry).expect("entry");
            BundledFixture(dir)
        }

        pub fn load(&self) -> Result<Applet, String> {
            let state: Shared = Arc::new(Mutex::new(capabilities::HostState::default()));
            load(&self.0, &Default::default(), &state)
        }
    }

    #[cfg(windows)]
    impl Drop for BundledFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// An applet actually run through `installed::install`, into the real
    /// per-user store `installed::list()`/`dew.Library` themselves read --
    /// faking that store would test a different `List`/`Launch`/`Uninstall`
    /// than the ones a mod actually calls. Removed on drop with
    /// `installed::uninstall`, best-effort: a test that already uninstalled
    /// it (that is what it was testing) leaves nothing for this to do.
    #[cfg(windows)]
    pub struct InstalledFixture {
        pub id: String,
        source: PathBuf,
    }

    #[cfg(windows)]
    impl InstalledFixture {
        pub fn new(name: &str, manifest: &str, entry: &str) -> InstalledFixture {
            let source = std::env::temp_dir().join(format!("dew-library-test-{name}"));
            let _ = std::fs::remove_dir_all(&source);
            std::fs::create_dir_all(&source).expect("temp source dir");
            std::fs::write(source.join("dew.toml"), manifest).expect("dew.toml");
            std::fs::write(source.join("main.luau"), entry).expect("entry");
            let id = crate::installed::install(&source, true).expect("install fixture applet");
            InstalledFixture { id, source }
        }
    }

    #[cfg(windows)]
    impl Drop for InstalledFixture {
        fn drop(&mut self) {
            let _ = crate::installed::uninstall(&self.id);
            let _ = std::fs::remove_dir_all(&self.source);
        }
    }

    /// A HOST-ONLY PERMISSION IS A LOAD REFUSAL, NOT A QUIET ABSENCE
    /// (ADR-017). `install` is a real, known permission -- unlike the
    /// unknown-word case `manifest.rs` already refuses -- and the applet
    /// asking for it here is not bundled, so it must be refused the same
    /// way, naming both the permission and why.
    #[test]
    fn a_host_only_permission_refuses_to_load_outside_the_bundle() {
        let fixture = Fixture::new(
            "install-unbundled",
            "id = \"plain\"\npermissions = [\"widget\", \"install\"]\n",
            PLAIN,
        );

        let err = match fixture.load() {
            Err(e) => e,
            Ok(_) => panic!("a host-only permission from outside the bundle must not load"),
        };

        assert!(
            err.contains("install") && err.contains("bundle"),
            "the refusal should name the permission and why, got: {err}"
        );
    }

    /// THE OTHER HALF OF THE SAME CHECK: the identical manifest, loaded
    /// from the one place that is allowed to hold it, loads clean.
    #[cfg(windows)]
    #[test]
    fn a_host_only_permission_loads_from_the_bundle() {
        let fixture = BundledFixture::new(
            "install-bundled",
            "id = \"plain\"\npermissions = [\"widget\", \"install\"]\n",
            PLAIN,
        );

        assert!(
            fixture.load().is_ok(),
            "a host-only permission declared by a bundled applet must be granted"
        );
    }

    /// `dew` NEVER APPEARS FOR AN ORDINARY APPLET, even one that only ever
    /// asked for `widget`. `dew.Marketplace` is not a thing every applet
    /// gets and merely finds gated members on -- unlike `desktop`, `dew`
    /// itself does not exist unless something granted put a member on it.
    #[test]
    fn an_ordinary_applet_has_no_dew_global_at_all() {
        let fixture = Fixture::new(
            "no-dew",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 10, height = 10 },
                    mount = function(desktop, root)
                        assert(dew == nil, "an ordinary applet must not see a dew global")
                    end,
                }
            "#,
        );
        assert!(fixture.load().is_ok());
    }

    /// THE CALLBACK SHAPE, END TO END (milestone 26 sprint 1; was
    /// trigger-and-poll through milestone 23 sprint 3 -- `Discover()` plus a
    /// separately-polled `Discovered()`). `Discover`/`Install` still return
    /// immediately, proven here by never blocking this test on the network;
    /// the difference is that nothing here ever calls a reader function --
    /// the callback handed to `Discover`/`Install` is what receives the
    /// answer, exactly once, the instant it lands.
    ///
    /// `services::tick` IS CALLED DIRECTLY, on `loaded.clock`, standing in
    /// for `main.rs`'s own frame loop -- see `register_frame_checker`'s doc
    /// comment in `capabilities.rs` for why a `desktop.Clock.OnFrame`
    /// listener, driven by `tick`, is how the callback's answer ever reaches
    /// Luau at all.
    ///
    /// HERMETIC ON PURPOSE: `platform::clear_session()` guarantees no
    /// session file exists before either call, so `platform::discover` and
    /// `package::install_from_marketplace` both fail on `require_session`
    /// before either would ever reach the network -- the same "not signed
    /// in" error `dew discover`/`dew install @owner/id` give from a
    /// terminal in the same state. What is under test is the wiring
    /// (trigger, background thread, callback, shape of the answer), not
    /// `dew-platform`'s own behaviour, which owes this test nothing.
    #[cfg(windows)]
    #[test]
    fn dew_marketplace_triggers_with_a_callback_and_never_blocks() {
        let _session_guard = SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::platform::clear_session();

        let fixture = BundledFixture::new(
            "marketplace-wiring",
            "id = \"plain\"\npermissions = [\"widget\", \"discover\", \"install\"]\n",
            PLAIN,
        );
        let loaded = fixture.load().expect("loads");
        let lua = loaded.vm.lua();

        // AWAITED BY POLLING A LUA GLOBAL THE CALLBACK ITSELF WRITES, not by
        // calling a reader function -- there is no reader function any more.
        // `services::tick` is what actually runs the callback (through the
        // `desktop.Clock.OnFrame` listener `capabilities.rs` registers), so
        // this loop's job is only to keep calling it until that has happened.
        let await_global = |lua: &mlua::Lua, clock: &SharedClock, name: &str| -> mlua::Table {
            for _ in 0..200 {
                services::tick(clock, 0.0);
                if let mlua::Value::Table(t) = lua.globals().get(name).expect("global") {
                    return t;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            panic!("{name} never landed a result within 2 seconds");
        };

        lua.load(
            r#"
                __discover_result = nil
                __discover_calls = 0
                dew.Marketplace.Discover(function(result)
                    __discover_result = result
                    __discover_calls += 1
                end)
            "#,
        )
        .exec()
        .expect("triggering must return immediately, never blocking on the network");

        let discovered = await_global(lua, &loaded.clock, "__discover_result");
        assert!(
            !discovered.get::<bool>("ok").expect("ok field"),
            "no session exists in this test, so discover must fail rather than succeed"
        );
        let error: String = discovered.get("error").expect("error field");
        assert!(error.contains("not signed in"), "got: {error}");

        // FIRES EXACTLY ONCE, not once per frame it happens to still be
        // registered for -- the whole point of moving the "did it land, do
        // not call it twice" bookkeeping into the host.
        for _ in 0..20 {
            services::tick(&loaded.clock, 0.0);
        }
        let calls: i64 = lua.globals().get("__discover_calls").expect("calls");
        assert_eq!(calls, 1, "the callback must not fire more than once per trigger");

        lua.load(
            r#"
                __install_result = nil
                dew.Marketplace.Install("owner", "applet", function(result)
                    __install_result = result
                end)
            "#,
        )
        .exec()
        .expect("triggering must return immediately, never blocking on the network");

        let installed = await_global(lua, &loaded.clock, "__install_result");
        assert!(
            !installed.get::<bool>("ok").expect("ok field"),
            "no session exists in this test, so install must fail rather than succeed"
        );
        let error: String = installed.get("error").expect("error field");
        assert!(error.contains("not signed in"), "got: {error}");
    }

    /// `dew.Library`'S FOUR CALLS, AGAINST A REAL FIXTURE-INSTALLED APPLET --
    /// matching the shape of `dew_marketplace_triggers_with_a_callback_and_never_blocks`
    /// above, but synchronous throughout rather than trigger-and-poll: every
    /// one of `List`/`Launch`/`Uninstall`/`SetEnabled` is a local file
    /// operation or a named-pipe round trip, never a network call, so there
    /// is nothing here to poll for.
    #[cfg(windows)]
    #[test]
    fn dew_library_lists_launches_toggles_and_uninstalls_a_real_applet() {
        // DRAINED FIRST, in case an earlier test on this worker thread queued
        // a load or unload that nothing has since consumed -- asserting a
        // queue's CONTENTS only makes sense starting from empty.
        let _ = crate::library::take_load_requests();
        let _ = crate::library::take_unload_requests();

        let target = InstalledFixture::new("target", "id = \"lib-target\"\n", PLAIN);

        let manager = BundledFixture::new(
            "library-wiring",
            "id = \"plain\"\npermissions = [\"widget\", \"library\"]\n",
            PLAIN,
        );
        let loaded = manager.load().expect("loads");
        let lua = loaded.vm.lua();

        let dew: mlua::Table = lua.globals().get("dew").expect("dew installed");
        let library: mlua::Table = dew.get("Library").expect("Library installed");

        let list_by_id = |library: &mlua::Table, id: &str| -> Option<mlua::Table> {
            let list: mlua::Function = library.get("List").expect("List installed");
            let rows: mlua::Table = list.call(()).expect("List call");
            for pair in rows.sequence_values::<mlua::Table>() {
                let row = pair.expect("row");
                if row.get::<String>("id").expect("id field") == id {
                    return Some(row);
                }
            }
            None
        };

        let row = list_by_id(&library, &target.id).expect("List must include the fixture applet");
        assert_eq!(row.get::<String>("name").expect("name"), target.id);
        assert!(
            row.get::<bool>("enabled").expect("enabled"),
            "a freshly installed applet defaults to enabled"
        );
        assert!(
            !row.get::<bool>("running").expect("running"),
            "nothing was ever actually spawned in this test, so it must read as not running"
        );

        // Launch: not running (no coordinator is listening in this test, so
        // `query_running` reads as false, the same "nothing to orphan"
        // answer `dew uninstall` itself relies on), so it queues the
        // fixture's own directory for the coordinator to pick up.
        let launch: mlua::Function = library.get("Launch").expect("Launch installed");
        let result: mlua::Table = launch.call(target.id.clone()).expect("Launch call");
        assert!(
            result.get::<bool>("ok").expect("ok field"),
            "launching an installed, non-running applet must succeed"
        );
        let queued = crate::library::take_load_requests();
        assert_eq!(
            queued.len(),
            1,
            "Launch must queue exactly the one directory the coordinator should load"
        );
        assert!(queued[0].ends_with(&target.id));

        // Launching an id nothing installed is refused, naming the id.
        let result: mlua::Table = launch
            .call("no-such-applet".to_string())
            .expect("Launch call");
        assert!(!result.get::<bool>("ok").expect("ok field"));
        let error: String = result.get("error").expect("error field");
        assert!(error.contains("no-such-applet"), "got: {error}");

        // SetEnabled(false), then List reflects it. Not running in this
        // test (no coordinator is listening, so `query_running` reads as
        // false), so this is a disk write only -- nothing to live-unload,
        // and the unload queue stays empty.
        let set_enabled: mlua::Function = library.get("SetEnabled").expect("SetEnabled installed");
        let result: mlua::Table = set_enabled
            .call((target.id.clone(), false))
            .expect("SetEnabled call");
        assert!(result.get::<bool>("ok").expect("ok field"));
        let row = list_by_id(&library, &target.id).expect("still installed, only disabled");
        assert!(!row.get::<bool>("enabled").expect("enabled"));
        assert!(
            crate::library::take_unload_requests().is_empty(),
            "disabling an applet that was never running must not queue an unload"
        );

        // SetEnabled(true), on an applet that is not running, queues a live
        // load -- the other half of the same toggle `manage.rs`'s own
        // checkbox already makes -- and then waits for the coordinator to
        // confirm it actually started. NO REAL COORDINATOR IS LISTENING IN
        // THIS TEST (see `dew_marketplace_triggers_with_a_callback_and_never_blocks`'s
        // own doc comment on what `query_running` reads as here), so
        // nothing ever drains the queue this pushes to and the wait times
        // out -- the one part of this call this hermetic test cannot
        // exercise end to end; the live checks in the milestone this
        // shipped under covered that instead. The disk write and the queue
        // push both still happen before the wait, so both are still
        // checked here.
        let result: mlua::Table = set_enabled
            .call((target.id.clone(), true))
            .expect("SetEnabled call");
        assert!(
            !result.get::<bool>("ok").expect("ok field"),
            "nothing drains the load queue in this test, so the wait for it to land must time out"
        );
        let row = list_by_id(&library, &target.id).expect("still installed, now re-enabled");
        assert!(
            row.get::<bool>("enabled").expect("enabled"),
            "the disk write happens before the wait, so it lands even though the wait times out"
        );
        let queued = crate::library::take_load_requests();
        assert_eq!(
            queued.len(),
            1,
            "re-enabling a non-running applet must queue exactly one load, timeout or not"
        );
        assert!(queued[0].ends_with(&target.id));

        // Uninstall removes it from both the Luau-visible list and disk.
        let uninstall: mlua::Function = library.get("Uninstall").expect("Uninstall installed");
        let result: mlua::Table = uninstall.call(target.id.clone()).expect("Uninstall call");
        assert!(result.get::<bool>("ok").expect("ok field"));
        assert!(
            list_by_id(&library, &target.id).is_none(),
            "an uninstalled applet must not appear in List any more"
        );
        assert!(
            !crate::installed::list().iter().any(|e| e.id == target.id),
            "Uninstall must remove the directory from disk, not just hide the row"
        );

        // Uninstalling it again is refused: it is no longer installed.
        let result: mlua::Table = uninstall.call(target.id.clone()).expect("Uninstall call");
        assert!(!result.get::<bool>("ok").expect("ok field"));
    }

    /// `dew.Library.OnChange` FIRES ON THE MODULE'S OWN ACTIONS (milestone 26
    /// sprint 2). `library::launch`/`uninstall`/`set_enabled` bump the
    /// generation `OnChange` watches directly, in-process, so this needs no
    /// live coordinator loop or pipe server to prove -- the cross-process
    /// half (a separate `dew install`/`dew uninstall`) is proved separately
    /// below, against the real `DewLibraryChanged` pipe.
    ///
    /// DRIVEN BY `services::tick`, THE SAME WAY THE CALLBACK SHAPE ABOVE IS
    /// -- `OnChange`'s own dispatch is one more `desktop.Clock.OnFrame`
    /// listener under the hood, registered the same way.
    #[cfg(windows)]
    #[test]
    fn dew_library_on_change_fires_on_its_own_actions() {
        let _ = crate::library::take_load_requests();
        let _ = crate::library::take_unload_requests();

        let target = InstalledFixture::new("onchange-target", "id = \"lib-onchange-target\"\n", PLAIN);

        let manager = BundledFixture::new(
            "library-onchange",
            "id = \"plain\"\npermissions = [\"widget\", \"library\"]\n",
            PLAIN,
        );
        let loaded = manager.load().expect("loads");
        let lua = loaded.vm.lua();

        let dew: mlua::Table = lua.globals().get("dew").expect("dew installed");
        let library: mlua::Table = dew.get("Library").expect("Library installed");
        let on_change: mlua::Function = library.get("OnChange").expect("OnChange installed");
        let set_enabled: mlua::Function = library.get("SetEnabled").expect("SetEnabled installed");

        lua.globals()
            .set("__onchange_calls", 0i64)
            .expect("set global");
        let callback = lua
            .create_function(|lua, ()| {
                let calls: i64 = lua.globals().get("__onchange_calls").expect("calls");
                lua.globals().set("__onchange_calls", calls + 1)
            })
            .expect("create callback");
        on_change.call::<()>(callback).expect("OnChange call");

        // THE BASELINE IS READ AT GRANT TIME. Ticking now, before anything
        // changes, must not fire it -- otherwise every applet holding
        // `library` would see a spurious first call for whatever changed
        // before it ever mounted.
        for _ in 0..5 {
            services::tick(&loaded.clock, 0.0);
        }
        let calls: i64 = lua.globals().get("__onchange_calls").expect("calls");
        assert_eq!(
            calls, 0,
            "OnChange must not fire for a change that predates its own registration"
        );

        // Not running in this test, so this is a disk write only --
        // `set_enabled` bumps the generation directly regardless.
        let result: mlua::Table = set_enabled
            .call((target.id.clone(), false))
            .expect("SetEnabled call");
        assert!(result.get::<bool>("ok").expect("ok field"));

        let mut fired = false;
        for _ in 0..50 {
            services::tick(&loaded.clock, 0.0);
            let calls: i64 = lua.globals().get("__onchange_calls").expect("calls");
            if calls > 0 {
                fired = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(fired, "OnChange never fired after SetEnabled changed the library");

        let calls: i64 = lua.globals().get("__onchange_calls").expect("calls");
        assert_eq!(
            calls, 1,
            "OnChange must fire exactly once per change, not once per frame the generation stays different"
        );
    }

    /// THE CROSS-PROCESS HALF: a bare ping to `DewLibraryChanged`, with
    /// nothing going through `dew.Library` at all, still bumps the
    /// generation -- this is what lets a separate `dew install`/`dew
    /// uninstall` process reach an already-running coordinator's dashboard.
    /// See `coordinator::notify_library_changed`'s own doc comment.
    ///
    /// `spawn_library_changed_server` IS STARTED DIRECTLY, not through
    /// `coordinator::run` -- `run` acquires the single-instance mutex and
    /// would collide with a real `dew` process on this machine; the pipe
    /// server itself does not, and is the one thing this test needs.
    #[cfg(windows)]
    #[test]
    fn dew_library_changed_pipe_bumps_the_generation_from_any_process() {
        crate::coordinator::spawn_library_changed_server();
        // GIVEN A MOMENT TO START LISTENING before the first ping --
        // `CreateNamedPipeW` runs on the spawned thread, not before this
        // call returns.
        std::thread::sleep(std::time::Duration::from_millis(50));

        let before = crate::library::generation();
        crate::coordinator::notify_library_changed();

        let mut bumped = false;
        for _ in 0..100 {
            if crate::library::generation() != before {
                bumped = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            bumped,
            "a ping to DewLibraryChanged must bump the generation within one second"
        );
    }

    /// `dew.Account`'S TWO CALLS, against a real (fixture) session file --
    /// `platform::save_session` writes through the same DPAPI encryption a
    /// real `dew login` would, so this proves `Whoami` against the actual
    /// on-disk format rather than a stand-in for it. Synchronous throughout,
    /// like `dew.Library` above: `platform::load_session()`/`clear_session()`
    /// are local file operations, never a network call.
    #[cfg(windows)]
    #[test]
    fn dew_account_reports_session_state_and_signs_out() {
        let _session_guard = SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::platform::clear_session();

        let manager = BundledFixture::new(
            "account-wiring",
            "id = \"plain\"\npermissions = [\"widget\", \"auth\"]\n",
            PLAIN,
        );
        let loaded = manager.load().expect("loads");
        let lua = loaded.vm.lua();

        let dew: mlua::Table = lua.globals().get("dew").expect("dew installed");
        let account: mlua::Table = dew.get("Account").expect("Account installed");
        let whoami: mlua::Function = account.get("Whoami").expect("Whoami installed");
        let sign_out: mlua::Function = account.get("SignOut").expect("SignOut installed");

        let nobody: mlua::Value = whoami.call(()).expect("Whoami call");
        assert!(
            matches!(nobody, mlua::Value::Nil),
            "no session exists yet, so Whoami must answer nil, not a table"
        );

        crate::platform::save_session(&crate::platform::Session {
            access_token: "at".to_string(),
            refresh_token: "rt".to_string(),
            user_id: "u-1".to_string(),
            email: "person@example.com".to_string(),
        })
        .expect("write fixture session");

        let signed_in: mlua::Table = whoami.call(()).expect("Whoami call");
        assert_eq!(
            signed_in.get::<String>("email").expect("email field"),
            "person@example.com"
        );
        assert_eq!(
            signed_in.get::<String>("userId").expect("userId field"),
            "u-1"
        );

        sign_out.call::<()>(()).expect("SignOut call");
        let nobody_again: mlua::Value = whoami.call(()).expect("Whoami call");
        assert!(
            matches!(nobody_again, mlua::Value::Nil),
            "SignOut must remove the session Whoami reads"
        );
    }

    /// AN UNKNOWN ENTRY REFUSES TO LOAD, naming the entry rather than doing
    /// nothing -- the same choice an unknown permission makes.
    #[test]
    fn an_unknown_experimental_entry_refuses_to_load() {
        let fixture = Fixture::new(
            "gradient-typo",
            "id = \"plain\"\npermissions = [\"widget\"]\n\
             experimentalDatamodel = [\"UIGradient.Typo@1\"]\n",
            PLAIN,
        );

        let err = match fixture.load() {
            Err(e) => e,
            Ok(_) => panic!("an unknown experimentalDatamodel entry must not load"),
        };
        assert!(err.contains("UIGradient.Typo"), "{err}");
    }

    /// THE GATE, PROVEN ON THE REAL EXPERIMENTAL PROPERTY, not the generic
    /// `Tier` mechanism `extensions`'s own tests exercise. An applet that never
    /// declared `GuiObject.BlendingMode` gets the ordinary "not a valid
    /// member" refusal a misspelled property gets -- not a special error, and
    /// not silence.
    ///
    /// ASSIGNED A PLAIN NUMBER, NOT `Enum.BlendMode.Additive` -- unflagged,
    /// the enum category itself does not exist either, and referencing it
    /// would fail on `Enum.BlendMode` before the property assignment this
    /// test is actually about ever ran.
    ///
    /// `datamodel::extensions::set_enabled_flags` IS THREAD-LOCAL, and this
    /// test runs on its own worker thread reused across the suite -- reset it
    /// before mounting, in case a prior test on this thread left a flag set.
    #[test]
    fn an_applet_that_never_declared_blending_mode_cannot_set_it() {
        datamodel::extensions::set_enabled_flags(&[]);
        let fixture = Fixture::new(
            "blend-undeclared",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 100, height = 60 },
                    mount = function(_dew, root)
                        local frame = Instance.new("Frame")
                        frame.BlendingMode = 0
                        frame.Parent = root
                    end,
                }
            "#,
        );

        let err = match fixture.load() {
            Err(e) => e,
            Ok(_) => panic!("BlendingMode must not be settable without declaring it"),
        };
        assert!(err.contains("BlendingMode"), "{err}");
        assert!(err.contains("not a valid member"), "{err}");
    }

    /// THE SAME PROPERTY, DECLARED. It becomes settable, and the value round
    /// trips through the DataModel it was written to -- not merely accepted
    /// and discarded.
    #[test]
    fn an_applet_that_declares_blending_mode_can_set_and_read_it_back() {
        let fixture = Fixture::new(
            "blend-declared",
            "id = \"plain\"\npermissions = [\"widget\"]\n\
             experimentalDatamodel = [\"GuiObject.BlendingMode@1\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 100, height = 60 },
                    mount = function(_dew, root)
                        local frame = Instance.new("Frame")
                        frame.Name = "Body"
                        frame.Size = UDim2.new(1, 0, 1, 0)
                        frame.BlendingMode = Enum.BlendMode.Multiply
                        assert(frame.BlendingMode == Enum.BlendMode.Multiply, "did not round-trip")
                        frame.Parent = root
                    end,
                }
            "#,
        );

        let result = fixture.load();
        // RESET BEFORE UNWRAPPING: a successful `load` leaves the flag enabled
        // on this thread via the real `set_enabled_flags` call inside it (see
        // `applets::load`), and this worker thread is reused by later tests
        // that assume no flags are set.
        datamodel::extensions::set_enabled_flags(&[]);
        let loaded = result.expect("a declared experimental entry loads");

        let Mounted::DataModel { dom, root } = &loaded.mounted;
        let frame = datamodel::render::frame_of(dom, *root, 100.0, 60.0);
        assert_eq!(
            frame.nodes.first().map(|n| n.blend_mode),
            Some(dew_runtime::frame::BlendMode::Multiply),
            "the value set in Luau reached the display list the painter reads"
        );
    }

    const PLAIN: &str = r#"
        return {
            id = "plain",
            size = { width = 100, height = 60 },
            mount = function(desktop, root)
                local frame = Instance.new("Frame")
                frame.Name = "Body"
                frame.Size = UDim2.new(1, 0, 1, 0)
                frame.BackgroundColor3 = Color3.fromRGB(10, 20, 30)
                frame.Parent = root
            end,
        }
    "#;

    #[test]
    fn a_datamodel_mod_mounts_and_the_renderer_finds_what_it_parented() {
        let fixture = Fixture::new(
            "mounts",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            PLAIN,
        );
        let loaded = fixture.load().expect("the mod loads");

        assert_eq!((loaded.width, loaded.height), (100, 60));

        let Mounted::DataModel { dom, root } = &loaded.mounted;
        // The tree is reachable from the root the host made and handed over —
        // which is the claim, rather than "mount ran without erroring".
        let frame = datamodel::render::frame_of(dom, *root, 100.0, 60.0);
        assert_eq!(frame.nodes.len(), 1);
    }

    #[test]
    fn a_datamodel_mod_responds_to_a_click_through_the_ordinary_loader() {
        // THE MILESTONE'S DONE-TEST, ON THE PATH A MOD REALLY TAKES. The unit
        // tests in `datamodel::input` prove the hit test and the firing on a bare
        // VM; this proves it survives the loader — a manifest, a capability table,
        // the root the host made, and the VM a mod is actually given.
        let fixture = Fixture::new(
            "clickable",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 100, height = 60 },
                    mount = function(desktop, root)
                        local b = Instance.new("TextButton")
                        b.Name = "Go"
                        b.Size = UDim2.new(1, 0, 1, 0)
                        b.Text = "before"
                        b.Parent = root
                        b.Activated:Connect(function()
                            b.Text = "after"
                        end)
                    end,
                }
            "#,
        );
        let loaded = fixture.load().expect("the mod loads");
        let Mounted::DataModel { dom, root } = &loaded.mounted;

        let surface = datamodel::input::Surface {
            lua: loaded.vm.lua(),
            dom,
            root: *root,
            size: (100.0, 60.0),
        };
        let mut pointer = datamodel::input::Pointer::default();
        let button = datamodel::input::Button::Left;
        // A SYNTHETIC PRESS AND RELEASE, headless. There is no window in this test
        // — `Pointer` takes coordinates, and what the frame loop does with a real
        // `Event::PointerDown` is one translation away.
        pointer.down(&surface, button, 50.0, 30.0).expect("down");
        pointer.up(&surface, button, 50.0, 30.0).expect("up");

        // READ BACK THROUGH THE ARENA, because a mod is HANDED its root as an
        // argument rather than given a global to reach it by — there is no
        // `DewRoot` in a mod's VM to evaluate against. The renderer reads the tree
        // the same way.
        let dom = dom.lock().expect("dom");
        let go = dom.children(*root)[0];
        assert_eq!(
            dom.property(go, "Text"),
            Some(rbx_types::Variant::String("after".into())),
            "a click on a mod loaded through the ordinary loader did not reach its handler"
        );
    }

    #[test]
    fn a_mod_can_measure_a_string_and_subscribe_to_frames() {
        // THE SPRINT'S TWO SERVICES, THROUGH THE ORDINARY LOADER. The unit tests
        // in `datamodel::services` prove them on a bare VM; this proves `desktop.Text`
        // and `desktop.Clock` survive the manifest, the sandbox and the capability
        // table -- ungated members of the same `desktop` a mod is handed, on the same
        // terms as `desktop.Time`.
        let fixture = Fixture::new(
            "services",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    mount = function(desktop, root)
                        assert(desktop.Text ~= nil, "desktop.Text is present")
                        local w, h = desktop.Text.Measure("hello", 14)
                        assert(type(w) == "number" and type(h) == "number", "two numbers")
                        local stop = desktop.Clock.OnFrame(function(dt) end)
                        assert(type(stop) == "function", "OnFrame returns an unsubscribe")
                        stop()
                    end,
                }
            "#,
        );
        assert!(fixture.load().is_ok());
    }

    #[test]
    fn subscribing_to_frames_does_not_dirty_the_tree() {
        // THE REGRESSION THIS SPRINT WAS MOST LIKELY TO CAUSE, asserted rather
        // than left to the live window. Milestone 1's sprint 9 stopped the host
        // repainting a static DataModel mod every frame; a clock that ticked into
        // `invalidate`, or a subscribe that touched the arena, hands that straight
        // back. `--stats` sees it as `painted` climbing to meet `fps`, and this
        // sees it as `take_dirty` answering true for a mod that changed nothing.
        let fixture = Fixture::new(
            "idleclock",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            r#"
                ticks = 0
                return {
                    id = "plain",
                    mount = function(desktop, root)
                        local frame = Instance.new("Frame")
                        frame.Name = "Body"
                        frame.Parent = root
                        desktop.Clock.OnFrame(function(dt) ticks += 1 end)
                    end,
                }
            "#,
        );
        let loaded = fixture.load().expect("the mod loads");
        let Mounted::DataModel { dom, .. } = &loaded.mounted;

        // The mount itself dirtied the tree, correctly: a new tree has to reach
        // the screen once. Clear it the way the first painted frame would.
        assert!(dom.lock().expect("dom").take_dirty());

        for _ in 0..10 {
            services::tick(&loaded.clock, 1.0 / 60.0);
            assert!(
                !dom.lock().expect("dom").take_dirty(),
                "a frame listener that assigns nothing dirtied the tree"
            );
        }

        // AND THE LISTENER REALLY RAN. Without this the test above passes just as
        // well when `tick` does nothing at all, which is the shape of green number
        // this project keeps finding.
        let ticks: u32 = loaded.vm.lua().globals().get("ticks").expect("ticks");
        assert_eq!(ticks, 10);
    }

    #[test]
    fn a_frame_listener_that_assigns_does_dirty_the_tree() {
        // The other direction, and the reason the test above is not just "the
        // clock is inert". A mod that animates by assigning inside a frame
        // listener must repaint -- the paint follows the change, not the tick.
        let fixture = Fixture::new(
            "animclock",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    mount = function(desktop, root)
                        local frame = Instance.new("Frame")
                        frame.Name = "Body"
                        frame.Parent = root
                        desktop.Clock.OnFrame(function(dt)
                            frame.BackgroundTransparency = 0.5
                        end)
                    end,
                }
            "#,
        );
        let loaded = fixture.load().expect("the mod loads");
        let Mounted::DataModel { dom, .. } = &loaded.mounted;
        assert!(dom.lock().expect("dom").take_dirty());

        services::tick(&loaded.clock, 1.0 / 60.0);
        assert!(
            dom.lock().expect("dom").take_dirty(),
            "a frame listener that assigned a property left the tree clean"
        );
    }

    #[test]
    fn an_aether_mod_gets_the_same_services() {
        // NOT A DIFFERENT PLATFORM PER RUNTIME. `install_vocabulary` genuinely is
        // conditional -- Aether carries its own and a partial host one blocks it --
        // and the risk was that `desktop.Text`/`desktop.Clock` picked up the same
        // conditionality by habit. They must not: sprint 8 has Aether's DataModel
        // host filling `Host.Text` and `Host.Clock` from exactly these, so an
        // Aether mod that could not see them would be sprint 8 failing a sprint
        // early.
        //
        // NO AETHER INSTALL IS NEEDED to assert this, and that is deliberate: the
        // install happens before the runtime branch, so this reads the globals of
        // a VM built the same way without depending on `pesde install` having run.
        let lua = mlua::Lua::new();
        let clock: SharedClock = std::sync::Arc::new(std::sync::Mutex::new(Clock::default()));
        services::install(&lua, &clock).expect("install");
        let got: bool = lua
            .load("return desktop.Text.Measure ~= nil and desktop.Clock.OnFrame ~= nil")
            .eval()
            .expect("eval");
        assert!(got);
    }

    #[test]
    fn the_vocabulary_is_installed_for_a_datamodel_mod() {
        // `Color3` above is the assertion: without `install_vocabulary` the mount
        // fails at the first line that names one. Stated separately so a failure
        // here reads as "the vocabulary went missing" rather than as a render
        // count being wrong.
        let fixture = Fixture::new(
            "vocabulary",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            PLAIN,
        );
        assert!(fixture.load().is_ok());
    }

    #[test]
    fn a_datamodel_mod_is_handed_only_what_its_manifest_granted() {
        // The capability table reaches this branch too, and it is still built
        // from the permissions ONLY: `storage` is a table, `clipboard` is absent.
        //
        // AN ASSERTION WAS REMOVED FROM THIS FIXTURE ON 2026-09-04, and the reason
        // is worth more than the line was. It read:
        //
        //     assert(rawget(_G, "game") == nil, "`game` is not on the plan; see the roadmap")
        //
        // It did its job -- it pinned a plan decision at the moment that decision
        // was easy to drift away from -- and then the decision changed. `game` is
        // back in scope for vide's truthiness gate, as sprint 6's item 0a, so a
        // test demanding its absence enforces a position the roadmap no longer
        // holds.
        //
        // IT IS NOT REPLACED WITH THE OPPOSITE. Where `game` gets installed, and
        // for which runtime, is sprint 6's to decide: vide is an Aether-mod
        // concern and this fixture is a DataModel mod, which needs no vide and
        // would be the wrong place to pin either answer. It was also never about
        // this test's subject, which is that a mod is handed only what its
        // manifest granted.
        let fixture = Fixture::new(
            "caps",
            "id = \"plain\"\npermissions = [\"widget\", \"storage\"]\n",
            r#"
                return {
                    id = "plain",
                    mount = function(desktop, root)
                        assert(desktop.Storage ~= nil, "storage was granted")
                        assert(desktop.Clipboard == nil, "clipboard was not asked for")
                        assert(root.Name == "DewRoot", "the root is named DewRoot")
                    end,
                }
            "#,
        );
        assert!(fixture.load().is_ok());
    }

    #[test]
    fn a_datamodel_mod_without_mount_is_told_the_signature_it_needed() {
        let fixture = Fixture::new(
            "nomount",
            "id = \"plain\"\npermissions = [\"widget\"]\n",
            r#"return { id = "plain" }"#,
        );
        let Err(message) = fixture.load() else {
            panic!("a module with no `mount` must not load");
        };
        assert!(
            message.contains("mount = function(desktop, root)"),
            "an author of a DataModel mod must not be shown an Aether signature: {message}"
        );
    }

    fn test_png() -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        let img = image::RgbaImage::from_raw(1, 1, vec![50, 100, 150, 255]).expect("1x1");
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .expect("encode");
        buffer.into_inner()
    }

    #[test]
    fn a_mod_resolves_an_image_through_rbxassetid_with_grant() {
        let fixture = Fixture::new(
            "rbxgrant",
            "id = \"plain\"\npermissions = [\"widget\", \"rbxassetid\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 100, height = 60 },
                    mount = function(desktop, root)
                        local img = Instance.new("ImageLabel")
                        img.Name = "RbxIcon"
                        img.Size = UDim2.new(1, 0, 1, 0)
                        img.ImageContent = Content.fromUri("rbxassetid://12345")
                        img.Parent = root
                    end,
                }
            "#,
        );
        let loaded = fixture.load().expect("the mod loads");
        let Mounted::DataModel { dom, root } = &loaded.mounted;

        let png_bytes = test_png();
        let expected_hash = dew_host::assets::hash_bytes(&png_bytes);

        // Supply mock transport
        dom.lock()
            .expect("dom")
            .assets
            .set_transport(std::sync::Arc::new(dew_host::assets::MockTransport(
                move |_| Ok(png_bytes.clone()),
            )));

        let frame = datamodel::render::frame_of(dom, *root, 100.0, 60.0);
        assert_eq!(frame.nodes.len(), 1);
        let node_image = frame.nodes[0].image.as_ref().expect("image on node");
        assert_eq!(node_image.uri, "rbxassetid://12345");
        let bitmap = node_image.bitmap.as_ref().expect("resolved bitmap");
        assert_eq!(bitmap.rgba, vec![50, 100, 150, 255]);

        let cache = dom.lock().expect("dom").assets.content_cache();
        assert_eq!(cache.lock().unwrap().misses(), 1);
        assert_eq!(cache.lock().unwrap().hits(), 0);
        assert!(cache.lock().unwrap().contains_hash(&expected_hash));
    }

    #[test]
    fn a_mod_is_refused_an_image_through_rbxassetid_without_grant() {
        let fixture = Fixture::new(
            "rbxnogrant",
            "id = \"plain\"\npermissions = [\"widget\", \"storage\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 100, height = 60 },
                    mount = function(desktop, root)
                        local img = Instance.new("ImageLabel")
                        img.Name = "RbxIcon"
                        img.Size = UDim2.new(1, 0, 1, 0)
                        img.ImageContent = Content.fromUri("rbxassetid://12345")
                        img.Parent = root
                    end,
                }
            "#,
        );
        let loaded = fixture.load().expect("the mod loads");
        let Mounted::DataModel { dom, root } = &loaded.mounted;

        // Transport must NEVER be called without grant
        dom.lock()
            .expect("dom")
            .assets
            .set_transport(std::sync::Arc::new(dew_host::assets::MockTransport(|_| {
                panic!("transport must not be called when permission is missing");
            })));

        let frame = datamodel::render::frame_of(dom, *root, 100.0, 60.0);
        assert_eq!(frame.nodes.len(), 1);
        let node_image = frame.nodes[0].image.as_ref().expect("image on node");
        assert_eq!(node_image.uri, "rbxassetid://12345");
        // Refusal means bitmap is None: draws missing placeholder marker, property keeps value
        assert!(node_image.bitmap.is_none());
    }
}

#[cfg(test)]
mod asking_for_a_surface {
    use super::tests::Fixture;

    const ASKS: &str = r#"
local root = desktop.Widget({ width = 120, height = 60 })
local frame = Instance.new("Frame")
frame.Name = "Asked"
frame.Size = UDim2.new(1, 0, 1, 0)
frame.BackgroundColor3 = Color3.fromRGB(20, 20, 20)
frame.Parent = root
"#;

    /// The shape this is all moving to: nothing returned at all.
    ///
    /// AN APPLET USED TO HAVE TO RETURN A TABLE to be given anything, because
    /// `desktop` only ever reached it through `mount`. It arrives as the chunk's
    /// vararg now, so asking is possible before there is anything to return.
    #[test]
    fn an_applet_that_asks_returns_nothing() {
        let fixture = Fixture::new("asks", "id = \"asks\"\npermissions = [\"widget\"]\n", ASKS);
        let applet = fixture.load().expect("an applet that asks should load");
        assert_eq!((applet.width, applet.height), (120, 60));
    }

    /// An ungranted surface is absent, not refused.
    ///
    /// THE DIFFERENCE IS WHERE THE ERROR POINTS. A permission check somewhere
    /// else says the applet was turned away; a nil index says which line asked
    /// for what it did not declare.
    #[test]
    fn an_ungranted_surface_is_not_on_the_table() {
        let fixture = Fixture::new(
            "asks-ungranted",
            "id = \"ungranted\"\npermissions = [\"widget\"]\n",
            "assert(desktop.Widget ~= nil, \"widget was granted\")\n\
             assert(desktop.Overlay == nil, \"overlay was not granted and must be absent\")\n\
             local root = desktop.Widget({ width = 10, height = 10 })\n",
        );
        fixture
            .load()
            .expect("the applet asserts about its own table");
    }

    /// Neither asking nor returning a `mount` is the one real mistake.
    #[test]
    fn an_applet_that_does_neither_is_told_both_ways_out() {
        let fixture = Fixture::new(
            "asks-neither",
            "id = \"neither\"\npermissions = [\"widget\"]\n",
            "return { size = { width = 10, height = 10 } }\n",
        );
        let error = match fixture.load() {
            Err(error) => error,
            Ok(_) => panic!("an applet with no surface and no mount should not load"),
        };
        assert!(
            error.contains("desktop.Widget") && error.contains("mount"),
            "the message should name both ways out, got: {error}"
        );
    }

    /// A second file in the applet can reach `desktop` without being handed it.
    ///
    /// THE REASON IT IS A GLOBAL RATHER THAN THE CHUNK'S VARARG. `...` reaches
    /// the entry module and stops there, so an applet split across two files
    /// would see nothing from the second one, and `desktop` would have to be
    /// threaded through every call that wanted it.
    #[test]
    fn a_required_module_can_reach_desktop() {
        let fixture = Fixture::new(
            "asks-submodule",
            "id = \"sub\"
permissions = [\"widget\"]
",
            "local helper = require(\"./helper\")
helper()
",
        );
        fixture.write(
            "helper.luau",
            "return function()
  assert(desktop ~= nil, \"a required module should see desktop\")
             local root = desktop.Widget({ width = 12, height = 12 })
             assert(root ~= nil, \"and should be answered by it\")
end
",
        );
        fixture
            .load()
            .expect("a submodule should be able to ask for the surface");
    }

    /// What was asked for beats what was returned.
    #[test]
    fn asking_wins_over_a_stale_declaration() {
        let fixture = Fixture::new(
            "asks-both",
            "id = \"both\"\npermissions = [\"widget\"]\n",
            "local root = desktop.Widget({ width = 33, height = 44 })\n\
             return { size = { width = 999, height = 999 }, mount = function() end }\n",
        );
        let applet = fixture.load().expect("loads");
        assert_eq!(
            (applet.width, applet.height),
            (33, 44),
            "the call is the newer of the two statements"
        );
    }
}

#[cfg(test)]
mod a_pressable_responds {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Aether's `Pressable` responds to a press the host delivered.
    ///
    /// NOTHING DRIVES A SESSION HERE. The applet performed its own mount and the
    /// host holds no session to step: the press reaches the framework because the
    /// host offers a `UserInputService` and the framework connects to it, which is
    /// the arrangement on the engine.
    ///
    /// THE APPLET IS THE REAL ONE, not a fixture, because what is under test is
    /// whether a framework's own hit testing survives this path at all.
    #[test]
    #[ignore = "blocked on Aether's own repo detecting `desktop` instead of \
                `dew` (ADR-018); see \
                .artifacts/project/upstream/aether-host-detection-needs-desktop.md. \
                Dew's own rename does not wait on that catching up."]
    fn timetracker_toggles_when_its_button_is_pressed() {
        let dir = PathBuf::from("../examples/aether/timetracker");
        let dir = if dir.is_dir() {
            dir
        } else {
            PathBuf::from("examples/aether/timetracker")
        };
        if !dir.join("roblox_packages/aether.luau").is_file() {
            //  A MISSING INSTALL IS NOT A FAILING TEST, and it is not a silent
            //  pass either: this says which command is missing.
            panic!(
                "no installed aether at {} -- run `pesde install` in that directory",
                dir.display()
            );
        }

        let state: Shared = Arc::new(Mutex::new(capabilities::HostState::default()));
        let loaded = load(&dir, &Default::default(), &state).expect("timetracker should load");

        let Mounted::DataModel { dom, root } = &loaded.mounted;

        let label_before = button_label(dom, *root);

        let surface = datamodel::input::Surface {
            lua: loaded.vm.lua(),
            dom,
            root: *root,
            size: (loaded.width as f32, loaded.height as f32),
        };
        let mut pointer = datamodel::input::Pointer::default();
        let button = datamodel::input::Button::Left;

        //  LAY THE TREE OUT BEFORE AIMING AT IT. An applet that parents itself
        //  into its surface leaves `AbsolutePosition` at the origin until
        //  something computes geometry, and a framework hit-testing against that
        //  finds every rectangle stacked at (0, 0). Rendering a frame is what a
        //  window does between pointer events, so this does it too.
        let settle = || {
            let mut guard = dom.lock().expect("dom");
            datamodel::render::commit_geometry(
                &mut guard,
                *root,
                loaded.width as f32,
                loaded.height as f32,
            );
        };
        settle();

        //  THE CENTRE OF THE TOGGLE, which sits at (286, 16) and is 74 by 24.
        let (x, y) = (286.0 + 37.0, 16.0 + 12.0);

        //  A FRAME BETWEEN EACH STEP, because the framework polls the pointer on
        //  a heartbeat rather than being pushed at. Without one it never sees the
        //  press it is being asked about.
        //  RECORDED BEFORE DISPATCHED, which is the order the renderer uses. A
        //  framework polls `services` for where the pointer is and whether a
        //  button is down; dispatching without recording delivers the event to
        //  an instance and leaves the poller reading a pointer that never moved.
        //  ON THE APPLET'S OWN CELL, not the process-global one: `load` wires
        //  `desktop.Pointer`/`desktop.Input` to `loaded.pointer` now, so recording on
        //  the shared cell here would leave the applet's own poll reading a
        //  pointer that never moved.
        services::pointer_moved_on(&loaded.pointer, x, y);
        pointer.moved(&surface, x, y).expect("moved");
        services::tick(&loaded.clock, 1.0 / 60.0);
        settle();

        services::pointer_button_on(&loaded.pointer, button as usize, true);
        pointer.down(&surface, button, x, y).expect("down");
        services::tick(&loaded.clock, 1.0 / 60.0);
        settle();

        services::pointer_button_on(&loaded.pointer, button as usize, false);
        pointer.up(&surface, button, x, y).expect("up");
        services::tick(&loaded.clock, 1.0 / 60.0);
        settle();

        let label_after = button_label(dom, *root);
        assert_ne!(
            label_before, label_after,
            "pressing the toggle should change its label, was {label_before:?} and still is"
        );
    }

    /// The toggle's caption, read out of the tree by name.
    fn button_label(dom: &datamodel::SharedDom, root: usize) -> String {
        let guard = dom.lock().expect("dom");
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if guard.name_of(id).as_deref() == Some("Label") {
                if let Some(rbx_types::Variant::String(text)) = guard.property(id, "Text") {
                    return text;
                }
            }
            stack.extend(guard.children(id));
        }
        String::new()
    }
}
