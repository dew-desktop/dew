//! Loading a mod: manifest, then VM, then capabilities, then its own code.
//!
//! THE ORDER IS THE POINT. Everything the host needs to decide whether a mod may
//! run is known before the mod's logic executes:
//!
//!   1. `dew.toml` is read from disk. It names the mod, its runtime and its
//!      permissions.
//!   2. A VM is created — deny-by-default, with no ffi, io, os or ambient `dew`.
//!   3. The `dew` table is built from the GRANTED permissions and nothing else.
//!   4. The mod's module is loaded. It returns a declaration and does nothing.
//!   5. `mount` is called once, and builds a tree.
//!
//! A registration-style API collapses 4 and 5 into "loading the mod runs the
//! mod", which puts every one of the earlier steps after the fact.
//!
//! TWO FLAVOURS, ONE LOADER. `manifest.runtime` says which framework — if any —
//! the mod's `mount` was written against, and the branch is narrow on purpose:
//! discovery, the manifest, the sandbox, the capability table, the size and the
//! surface are identical for both, because none of them is a property of the
//! framework the author chose. What differs is exactly three things — which
//! globals the VM gets, whether Aether's desktop ceremony runs, and what `mount`
//! is handed — and they are the three things a runtime IS.

use crate::capabilities::{self, Shared};
use crate::datamodel;
use crate::manifest::{Manifest, Runtime};
use crate::services::{self, Clock, SharedClock};
use crate::surface::Declared;
use dew_runtime::{modules, Session, Vm};
use mlua::prelude::*;
use std::path::{Path, PathBuf};

/// The living half of a mounted mod: what the frame loop drives.
///
/// AN ENUM RATHER THAN A TRAIT, because there are two of these and there will be
/// two. A `Session` is a handle onto Luau objects Aether's `Live.luau` maintains
/// and knows how to diff; a DataModel tree is instances in the host's own arena
/// with nothing watching them. Those are not two implementations of one
/// abstraction, they are two different things that happen to end at the same
/// `Frame`, and the seam they genuinely share is the painter.
pub enum Mounted {
    /// An Aether component, driven through `Driver` and repainted when the
    /// framework says something changed.
    Aether(Session),
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

impl Mounted {
    /// Which flavour this is, for the load line and for `main`'s dispatch.
    pub fn runtime(&self) -> Runtime {
        match self {
            Mounted::Aether(_) => Runtime::Aether,
            Mounted::DataModel { .. } => Runtime::DataModel,
        }
    }
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
    /// clock is not a property of which framework the author chose -- both
    /// flavours subscribe through the same `DewHost.Clock`, and sprint 8 has
    /// Aether reaching it through `Host.Clock` -- so putting it in the enum would
    /// have made "can this mod animate" depend on the runtime it declared.
    pub clock: SharedClock,
    /// The VM this mod lives in. Held because dropping it takes the mod's Lua
    /// handles with it — a mod is exactly as alive as its VM. True of both
    /// flavours: a `Session` is Lua objects, and a `SharedDom` is full of
    /// `InstanceRef`s the guest also holds.
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

pub fn load(
    dir: &Path,
    _aether_root: &Path,
    aliases: &std::collections::HashMap<String, PathBuf>,
    state: &Shared,
) -> Result<Applet, String> {
    // 1 ── the manifest, before anything of the mod's runs.
    let manifest = Manifest::load(dir)?;

    //      SAID OUT LOUD, BEFORE THE MOD RUNS. What the manifest asked for and
    //      the host will not do is reported here rather than discovered by an
    //      author wondering why their keybinding does nothing. Warnings, not
    //      errors: a mod whose hotkeys are inert still renders, and refusing to
    //      load it would be a worse answer than saying which half works.
    for problem in manifest.unhonoured() {
        eprintln!("[dew] {}: {problem}", manifest.id);
    }
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
    //      installed as a GLOBAL rather than passed like `dew`, because it is not
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

    //      AND `DewHost`, ON THE SAME TERMS AND FOR THE SAME REASON. Text metrics
    //      and a frame clock are what the host computes and no guest can: they are
    //      the language of the platform rather than a capability, so they are
    //      installed as a global here beside `Instance` rather than granted in the
    //      table built at step 4. `services.rs` carries the full argument, and the
    //      short version is that a mod refused text metrics cannot lay out -- a
    //      permission with only one sound answer is not a permission.
    //
    //      FOR BOTH RUNTIMES. A DataModel mod calls `DewHost.Text.Measure`
    //      directly; an Aether mod reaches the same functions through the
    //      `Host.Text` and `Host.Clock` seams its interface already declares.
    let clock: SharedClock = std::sync::Arc::new(std::sync::Mutex::new(Clock::default()));
    services::install(vm.lua(), &clock).map_err(|e| format!("{}: {e}", manifest.id))?;

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

    // 3 ── whatever the declared runtime needs in place BEFORE the mod's own
    //      module is loaded. Both arms below produce something step 6 mounts
    //      with, and neither runs a line of the mod.
    enum Ceremony {
        /// Aether's desktop host table, ready to `Mount` through.
        Aether(LuaTable),
        /// The `ScreenGui` in `dom` that the guest parents into.
        DataModel { root: usize },
    }

    let ceremony = match manifest.runtime {
        // 3a ── the framework's own desktop ceremony. Dew ships no Luau of its
        //       own: resolving the host, installing the vocabulary, opening a
        //       reactive scope and opening a session are identical for every
        //       off-engine host, so they live in Aether where the CLI gets them
        //       too.
        Runtime::Aether => {
            //       THROUGH THE MOD'S OWN INSTALL, not through one of ours. The
            //       redirect pesde writes beside a mod is an unversioned path
            //       into the exact Aether that mod declared, so the ceremony and
            //       the widget are the same copy of the framework. Reaching into
            //       a host-side checkout instead is what let a mod and its host
            //       disagree about which Aether they meant.
            let aether: LuaTable =
                modules::load_entry(&vm, &dir.join("roblox_packages/aether.luau"))
                    .and_then(|f| f.call(()))
                    .map_err(|e| {
                        format!(
                            "{}: loading Aether from the mod's own packages: {e}",
                            manifest.id
                        )
                    })?;
            let desktop: LuaTable = aether.get("Desktop").map_err(|e| {
                format!(
                    "{}: this Aether exposes no desktop ceremony: {e}",
                    manifest.id
                )
            })?;
            Ceremony::Aether(desktop)
        }
        // 3b ── no framework: just a root to parent into.
        //
        //       `DewRoot` RATHER THAN `game`, AND THAT IS ABOUT THIS ARM. A
        //       DataModel mod is handed the root it parents into, and `DewRoot`
        //       names it. It needs no vide, so it is not the arm the gate above is
        //       for and it does not get one -- which is the whole of what keeps
        //       `game` from becoming a sentinel a second time.
        //
        //       A `ScreenGui` because that is what an engine application expects
        //       to find above its tree, so the same mod has a chance of running
        //       in both places -- which is a property of the TREE rather than of
        //       anything named `game`.
        //
        //       HANDED TO `mount`, NOT INSTALLED AS A GLOBAL. `examples/standalone`
        //       reaches for a `DewRoot` global because a bare script has no
        //       function to receive one; a mod has `mount`, and a parameter is
        //       the same argument that keeps `dew` off the globals table — what a
        //       mod is GIVEN is visible at its own call site.
        Runtime::DataModel => {
            let root = dom
                .lock()
                .expect("dom")
                .insert("ScreenGui".into(), "DewRoot".into());
            Ceremony::DataModel { root }
        }
    };

    // 4 ── the capability table, from the granted permissions ONLY.
    let dew = capabilities::build(vm.lua(), &manifest.permissions, state)
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    // 5 ── the mod's own module. It returns a declaration; it performs nothing.
    let declaration: LuaTable = modules::load_entry(&vm, &entry)
        .and_then(|f| f.call(()))
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    let (width, height) = size_from(&declaration);
    // The window caption a mod gets when it declares no `title` of its own is its
    // manifest `name`, falling back to the id. An applet called "Time Tracker &
    // Pomodoro HUD" in its manifest should not present itself as `timetracker`.
    let surface = Declared::from_declaration(&declaration, manifest.display_name());

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

    // THE SIGNATURE IS THE RUNTIME'S, and so is the message when it is missing.
    // An author who wrote a DataModel mod and forgot `mount` should not be shown
    // an Aether component's shape to copy.
    let signature = match manifest.runtime {
        Runtime::Aether => "mount = function(dew) … end",
        Runtime::DataModel => "mount = function(dew, root) … end",
    };
    let mount: LuaFunction = declaration.get("mount").map_err(|_| {
        format!(
            "{}: the module returned no `mount` — a Dew applet returns \
             {{ id = …, size = …, {signature} }}. See docs/applet_contract.md",
            manifest.id
        )
    })?;

    // 6 ── build the tree, ONCE.
    let mounted = match ceremony {
        //      INSIDE A REACTIVE SCOPE THE PRELUDE OPENS. The mount function is
        //      handed OVER rather than called here: `derive` and `effect` refuse
        //      to run outside a stable scope, and calling it from Rust would run
        //      it outside one.
        //      `dew` is forwarded to the mod's `mount` through Desktop.Mount's
        //      varargs, so the framework never learns what a capability table is.
        Ceremony::Aether(desktop) => {
            let mount_fn: LuaFunction = desktop.get("Mount").map_err(|e| e.to_string())?;
            let result: LuaTable = mount_fn
                .call((mount, width, height, dew))
                .map_err(|e| format!("{}: while mounting: {e}", manifest.id))?;

            // WHICH HOST THE FRAMEWORK RESOLVED, SAID OUT LOUD.
            //
            // `Host.detect()` chooses between the DataModel host and the Luau
            // test double, and BOTH OF THEM DRAW A CORRECT WIDGET. So "the three
            // Aether mods still render" is true under either branch and is not
            // evidence that either was taken -- the exact shape of green number
            // this project keeps finding. The framework already knows the answer
            // and had no way to say it; `Host.Name` is what is driven and
            // `Host.Environment` is where its text metrics and frame clock came
            // from, so the pair distinguishes every branch that exists.
            //
            // A LINE RATHER THAN AN ASSERTION, because it reports rather than
            // requires: which host a guest framework resolves is the framework's
            // decision, and a mod that draws correctly through the other one is
            // not a mod this file should refuse to run.
            let host_tbl: Option<LuaTable> = result.get("Host").ok();
            let (host_name, environment) = host_tbl
                .map(|h| {
                    (
                        h.get::<String>("Name").unwrap_or_else(|_| "?".into()),
                        h.get::<String>("Environment")
                            .unwrap_or_else(|_| "?".into()),
                    )
                })
                .unwrap_or_else(|| ("?".into(), "?".into()));

            // AND WHAT IT BUILT WITH, WHICH IS A SECOND QUESTION AND THE ONE THIS
            // HOST CAN ANSWER FOR ITSELF.
            //
            // The name above is the framework's own account of its decision, and
            // sprint 6 measured it being TRUE AND NOT ENOUGH: with Aether's
            // `Deps.luau` picking its vide by `typeof(game) == "Instance"`,
            // independently of `Host.detect()`, this line read
            // `DataModel (Dew services)` over a tree of the test double's mock
            // instances. Every mod rendered, every suite passed, and the one thing
            // the sprint existed to change had not changed.
            //
            // A tree root that borrows as an `InstanceRef` is a node in THIS
            // host's arena -- a handle this process issued, holding an id into a
            // `Dom` this file created. Nothing the guest can construct passes it,
            // and the test double's mocks are Luau tables, so the two answers
            // cannot be confused. It is the difference between asking the guest
            // what it did and reading what arrived.
            let built = match result.get::<LuaValue>("Tree") {
                Ok(LuaValue::UserData(ud)) => match ud.borrow::<datamodel::InstanceRef>() {
                    Ok(node) => match node.class_name() {
                        Some(class) => format!("a {class} in this host's DataModel"),
                        None => "a destroyed instance".to_string(),
                    },
                    Err(_) => "userdata this host did not issue".to_string(),
                },
                Ok(LuaValue::Table(_)) => "a Luau table, not this host's DataModel".to_string(),
                _ => "nothing this host recognises".to_string(),
            };

            println!(
                "[dew] {}: mounted through Aether's {host_name} host ({environment} services), \
                 built {built}",
                manifest.id
            );

            let session_tbl: LuaTable = result.get("Session").map_err(|e| e.to_string())?;
            let session = Session::from_lua(vm.lua(), &session_tbl)
                .map_err(|e| format!("{}: {e}", manifest.id))?;
            Mounted::Aether(session)
        }
        //      CALLED DIRECTLY, because there is no scope to be inside. A
        //      DataModel mod's `mount` parents instances and returns; its return
        //      value is deliberately ignored, since the tree the host renders is
        //      the one under the root it was handed rather than one it was given
        //      back. Handing a root over and then reading a returned tree would
        //      be two answers to the same question.
        Ceremony::DataModel { root } => {
            let handle = datamodel::handle(vm.lua(), &dom, root)
                .map_err(|e| format!("{}: {e}", manifest.id))?;
            mount
                .call::<()>((dew, handle))
                .map_err(|e| format!("{}: while mounting: {e}", manifest.id))?;
            Mounted::DataModel {
                dom: dom.clone(),
                root,
            }
        }
    };

    println!(
        "[dew] loaded {} ({}x{}) — {} — {} — granted: {}",
        manifest.id,
        width,
        height,
        surface.describe(),
        mounted.runtime().name(),
        capabilities::describe(&manifest.permissions)
    );

    Ok(Applet {
        manifest,
        width,
        height,
        surface,
        mounted,
        clock,
        vm,
    })
}

/// WHAT THIS COVERS is the branch itself: that a mod declaring
/// `runtime = "datamodel"` reaches `mount(dew, root)` with the vocabulary
/// present and a root to parent into, and that what it parented is what the
/// renderer finds. It goes through the real `load` — manifest, sandbox,
/// capability table and all — because a test that called the DataModel arm
/// directly would pass on the day the manifest stopped selecting it.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A mod on disk, in a directory of its own, removed when the test ends.
    ///
    /// The loader reads `dew.toml` and an entry module from a real directory,
    /// which is the behaviour under test; faking the filesystem here would test
    /// a different loader.
    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str, manifest: &str, entry: &str) -> Fixture {
            let dir = std::env::temp_dir().join(format!("dew-mods-test-{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("temp dir");
            std::fs::write(dir.join("dew.toml"), manifest).expect("dew.toml");
            std::fs::write(dir.join("main.luau"), entry).expect("entry");
            Fixture(dir)
        }

        fn load(&self) -> Result<Applet, String> {
            let state: Shared = Arc::new(Mutex::new(capabilities::HostState::default()));
            // A DataModel mod never reads Aether's source; the path is still a
            // require root, and pointing it at the mod's own directory keeps this
            // test from depending on an install having been run.
            load(&self.0, &self.0, &Default::default(), &state)
        }

        /// The same fixture, loaded against the INSTALLED Aether and vide.
        ///
        /// This one does depend on `pesde install` having run, which the loader
        /// above deliberately does not -- and it has to: what it is testing is
        /// the framework's own behaviour on this host, so there is nothing to
        /// stand in for the framework. `crates/runtime`'s parity tests take the
        /// same dependency for the same reason.
        /// A FIXTURE THAT OWNS ITS FRAMEWORK, because that is now the only way a
        /// mod gets one. The host injects nothing, so the fixture writes the
        /// redirect pesde would have written: an unversioned file beside the mod
        /// naming the Aether this mod declared. Copying the package in would be
        /// truer still and costs seconds per test; the redirect is the same
        /// shape a real mod loads through.
        fn load_aether(&self) -> Result<Applet, String> {
            let state: Shared = Arc::new(Mutex::new(capabilities::HostState::default()));
            let root = dew_runtime::installed_package("aether")
                .expect("no installed aether -- run `pesde install` at the repository root");

            let packages = self.0.join("roblox_packages");
            std::fs::create_dir_all(&packages).expect("fixture roblox_packages");
            let api = root.join("src/api.luau");
            std::fs::write(
                packages.join("aether.luau"),
                format!(
                    "return require(\"@fixture_aether/api\")
-- {}",
                    api.display()
                ),
            )
            .expect("fixture aether redirect");

            let mut aliases = std::collections::HashMap::new();
            aliases.insert("fixture_aether".to_string(), root.join("src"));
            load(&self.0, &root, &aliases, &state)
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
runtime = \"datamodel\"
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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
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

    const PLAIN: &str = r#"
        return {
            id = "plain",
            size = { width = 100, height = 60 },
            mount = function(dew, root)
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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            PLAIN,
        );
        let loaded = fixture.load().expect("the mod loads");

        assert_eq!(loaded.mounted.runtime(), Runtime::DataModel);
        assert_eq!((loaded.width, loaded.height), (100, 60));

        let Mounted::DataModel { dom, root } = &loaded.mounted else {
            panic!("a datamodel manifest must not produce an Aether session");
        };
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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 100, height = 60 },
                    mount = function(dew, root)
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
        let Mounted::DataModel { dom, root } = &loaded.mounted else {
            panic!("a datamodel manifest must not produce an Aether session");
        };

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
        // in `datamodel::services` prove them on a bare VM; this proves `DewHost`
        // survives the manifest, the sandbox and the capability table -- and that
        // it is a GLOBAL, since a mod is handed `dew` and `root` and nothing else.
        let fixture = Fixture::new(
            "services",
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    mount = function(dew, root)
                        assert(DewHost ~= nil, "DewHost is a global")
                        assert(dew.text == nil, "text metrics are not a capability")
                        local w, h = DewHost.Text.Measure("hello", 14)
                        assert(type(w) == "number" and type(h) == "number", "two numbers")
                        local stop = DewHost.Clock.OnFrame(function(dt) end)
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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            r#"
                ticks = 0
                return {
                    id = "plain",
                    mount = function(dew, root)
                        local frame = Instance.new("Frame")
                        frame.Name = "Body"
                        frame.Parent = root
                        DewHost.Clock.OnFrame(function(dt) ticks += 1 end)
                    end,
                }
            "#,
        );
        let loaded = fixture.load().expect("the mod loads");
        let Mounted::DataModel { dom, .. } = &loaded.mounted else {
            panic!("a datamodel manifest must not produce an Aether session");
        };

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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    mount = function(dew, root)
                        local frame = Instance.new("Frame")
                        frame.Name = "Body"
                        frame.Parent = root
                        DewHost.Clock.OnFrame(function(dt)
                            frame.BackgroundTransparency = 0.5
                        end)
                    end,
                }
            "#,
        );
        let loaded = fixture.load().expect("the mod loads");
        let Mounted::DataModel { dom, .. } = &loaded.mounted else {
            panic!("a datamodel manifest must not produce an Aether session");
        };
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
        // and the risk was that `DewHost` picked up the same conditionality by
        // habit. It must not: sprint 8 has Aether's DataModel host filling
        // `Host.Text` and `Host.Clock` from exactly these, so an Aether mod that
        // could not see them would be sprint 8 failing a sprint early.
        //
        // NO AETHER INSTALL IS NEEDED to assert this, and that is deliberate: the
        // install happens before the runtime branch, so this reads the globals of
        // a VM built the same way without depending on `pesde install` having run.
        let lua = mlua::Lua::new();
        let clock: SharedClock = std::sync::Arc::new(std::sync::Mutex::new(Clock::default()));
        services::install(&lua, &clock).expect("install");
        let got: bool = lua
            .load("return DewHost.Text.Measure ~= nil and DewHost.Clock.OnFrame ~= nil")
            .eval()
            .expect("eval");
        assert!(got);
    }

    /// An Aether mod that leaves the one node it built where the host can read
    /// it back. Deliberately minimal: what is under test is the seam, not a
    /// widget.
    ///
    /// `_G` IS THE FIXTURE'S OWN DOING, not a hole in the loader. Each mod gets
    /// its own VM, this one is built and dropped inside a single test, and the
    /// alternative -- teaching `load` to hand back the tree -- would be
    /// production code shaped by a test. `Desktop.Mount` does return it, and
    /// `mods.rs` reads it for the mount line; what it does not do is keep it.
    const AETHER_MOD: &str = r##"
        local Aether = require("./roblox_packages/aether")
        local create = Aether.create

        return {
            id = "plain",
            size = { width = 100, height = 60 },
            mount = function(dew)
                local node = create "Frame" {
                    Name = "Body",
                    Size = UDim2.new(0.5, 4, 0.25, -2),
                    BackgroundColor3 = Color3.fromHex("#336699"),
                }
                _G.__dew_test_tree = node
                --- READ BACK HERE AND NOT LATER, because `Live.Session` commits a
                --- layout pass before `Desktop.Mount` returns and `Host.SetBounds`
                --- overwrites `Size` with the solved rectangle. That is the solver
                --- doing its job; it is also the only window in which the AUTHORED
                --- value is still the stored one.
                _G.__dew_test_size = node.Size
                return node
            end,
        }
    "##;

    #[test]
    fn an_aether_mod_mounts_through_the_datamodel_host() {
        // THE COMPLETION TEST OF STEP G, AS A TEST RATHER THAN AS A LOG LINE.
        //
        // `Host.Name` alone is not enough and this project has the measurement:
        // with Aether's `Deps.luau` choosing its vide by `typeof(game)`,
        // independently of `Host.detect()`, the framework reported `DataModel`
        // over a tree of the test double's mock instances. Both halves are
        // asserted here for that reason -- which host was resolved, AND that what
        // it built is in this host's own arena.
        let fixture = Fixture::new(
            "aetherhost",
            "id = \"plain\"\nruntime = \"aether\"\npermissions = [\"widget\"]\n",
            AETHER_MOD,
        );
        let loaded = fixture.load_aether().expect("the mod loads");
        assert_eq!(loaded.mounted.runtime(), Runtime::Aether);

        // WHICH HOST THE FRAMEWORK RESOLVED, asked of the framework in the mod's
        // own VM. `Host.detect` is memoised per process, so this is the decision
        // the mount above was made under rather than a fresh one.
        let (host_name, environment): (String, String) = loaded
            .vm
            .lua()
            .load(
                r#"
                local host = require("@fixture_aether/api").Host.detect()
                return host.Name, host.Environment
            "#,
            )
            .eval()
            .expect("asking the framework which host it chose");
        assert_eq!(
            host_name, "DataModel",
            "the test double is still on the path"
        );
        assert_eq!(environment, "Dew");

        // AND THAT IT BUILT WITH IT. The pair is the point: the name above was
        // true and insufficient once already.
        let built_here: bool = loaded
            .vm
            .lua()
            .load(r#"return typeof(_G.__dew_test_tree) == "Instance""#)
            .eval()
            .expect("reading the built node back");
        assert!(built_here, "the framework built with something else");

        let globals = loaded.vm.lua().globals();
        // REPOINTED, NOT DELETED. This asserted `game` was a boolean `true`,
        // which was the whole of what the gate ever was. centau/vide#89 and
        // aether#3 removed the need for it, so the assertion turns round: the
        // global must NOT be there.
        //
        // Kept as a guard rather than dropped, because `game` came back once
        // already -- dropped from step F on 2026-09-04 and reinstated the same
        // day -- and the next thing to reach for it should fail here rather than
        // quietly reintroduce a sentinel nothing reads.
        let gate: LuaValue = globals.get("game").expect("reading `game`");
        assert!(
            matches!(gate, LuaValue::Nil),
            "`game` is not installed any more and nothing should reintroduce it, got {gate:?}"
        );
    }

    #[test]
    fn a_udim2_this_host_built_survives_aether_create() {
        // VERIFIED RATHER THAN ASSUMED, and it is the reason this was deferred
        // through the whole of milestone 1: Aether's own vocabulary is Luau
        // TABLES that its `create` consumes, and this host's are USERDATA. The
        // worry was that substituting one for the other was the hard part.
        //
        // It is not, and the reason is worth stating because it retires the
        // worry rather than confirming it: under the DataModel host `create` is
        // vide's own, vide's `create` writes properties onto instances the
        // environment made, and this host's instances take this host's userdata.
        // There was never a translation step on that path -- only on the test
        // double's, which builds Luau tables and therefore needs Luau values.
        //
        // ALL FOUR NUMBERS, AND A NEGATIVE OFFSET. `as_f32` once refused an
        // integer here, so every literal offset in every UDim2 was zero and no
        // test noticed, because none read one back.
        let fixture = Fixture::new(
            "aethervalues",
            "id = \"plain\"\nruntime = \"aether\"\npermissions = [\"widget\"]\n",
            AETHER_MOD,
        );
        let loaded = fixture.load_aether().expect("the mod loads");

        let survived: bool = loaded
            .vm
            .lua()
            .load(
                r##"
                local node = _G.__dew_test_tree
                local size = _G.__dew_test_size
                return typeof(node) == "Instance"
                    and node.ClassName == "Frame"
                    and node.Name == "Body"
                    and typeof(size) == "UDim2"
                    and size.X.Scale == 0.5 and size.X.Offset == 4
                    and size.Y.Scale == 0.25 and size.Y.Offset == -2
                    and node.BackgroundColor3 == Color3.fromHex("#336699")
            "##,
            )
            .eval()
            .expect("reading the built node back");
        assert!(
            survived,
            "a value this host built did not survive Aether's create"
        );

        // AND LAYOUT RAN WITHOUT EATING ITS OWN INPUT, which is the other half and
        // is the bug this sprint actually found.
        //
        // `Live.Session` commits a layout pass during the mount, and the framework
        // reports the solved rectangle through `Host.SetBounds`. That used to
        // write it into `Position` and `Size` -- the properties the solver READS
        // -- so every frame it added the parent's offset to an offset it had
        // already made absolute. On `applets/timetracker` a label walked 286 pixels
        // right per frame and the widget repainted 305 times a second doing
        // nothing. It passed every test in both repositories, because nothing
        // off-engine had ever driven that host before.
        //
        // BOTH HALVES, because either alone is satisfied by something broken. A
        // rectangle with area says layout ran; the authored `Size` still reading
        // `(0.5, 4, 0.25, -2)` says it ran without overwriting what it was
        // solving from.
        let (w, h, intact): (f32, f32, bool) = loaded
            .vm
            .lua()
            .load(
                r##"
                local node = _G.__dew_test_tree
                local host = require("@fixture_aether/api").Host.detect()
                local _, _, w, h = host.Scene.Bounds(node)
                local size = node.Size
                return w, h, size.X.Scale == 0.5 and size.X.Offset == 4
                    and size.Y.Scale == 0.25 and size.Y.Offset == -2
            "##,
            )
            .eval()
            .expect("reading the solved rectangle back");
        assert!(w > 0.0 && h > 0.0, "layout produced no rectangle: {w}x{h}");
        assert!(
            intact,
            "the layout pass overwrote the authored Size it solves from"
        );
    }

    #[test]
    fn the_vocabulary_is_installed_for_a_datamodel_mod() {
        // `Color3` above is the assertion: without `install_vocabulary` the mount
        // fails at the first line that names one. Stated separately so a failure
        // here reads as "the vocabulary went missing" rather than as a render
        // count being wrong.
        let fixture = Fixture::new(
            "vocabulary",
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\", \"storage\"]\n",
            r#"
                return {
                    id = "plain",
                    mount = function(dew, root)
                        assert(dew.storage ~= nil, "storage was granted")
                        assert(dew.clipboard == nil, "clipboard was not asked for")
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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            r#"return { id = "plain" }"#,
        );
        let Err(message) = fixture.load() else {
            panic!("a module with no `mount` must not load");
        };
        assert!(
            message.contains("mount = function(dew, root)"),
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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\", \"rbxassetid\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 100, height = 60 },
                    mount = function(dew, root)
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
        let Mounted::DataModel { dom, root } = &loaded.mounted else {
            panic!("expected DataModel");
        };

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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\", \"storage\"]\n",
            r#"
                return {
                    id = "plain",
                    size = { width = 100, height = 60 },
                    mount = function(dew, root)
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
        let Mounted::DataModel { dom, root } = &loaded.mounted else {
            panic!("expected DataModel");
        };

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
