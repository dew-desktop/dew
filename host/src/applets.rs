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
//! ONE LOADER. `manifest.runtime` names the framework — if any — the mod's
//! `mount` was written against. There is one runtime today, `datamodel`:
//! discovery, the manifest, the sandbox, the capability table, the size and the
//! surface are all the same regardless, because none of them is a property of
//! the runtime; what a runtime actually decides is which globals the VM gets
//! and what `mount` is handed.

use crate::capabilities::{self, Shared};
use crate::datamodel;
use crate::manifest::{Manifest, Runtime};
use crate::services::{self, Clock, SharedClock};
use crate::surface::{Declared, Requested};
use dew_runtime::{modules, Vm};
use mlua::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The living half of a mounted mod: what the frame loop drives.
///
/// AN ENUM WITH ONE VARIANT TODAY rather than the DataModel shape on its own,
/// because `main.rs` and the tests both match on it (`Mounted::DataModel { .. }`)
/// and a second runtime is the kind of thing this seam exists to make room for.
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

impl Mounted {
    /// Which flavour this is, for the load line and for `main`'s dispatch.
    pub fn runtime(&self) -> Runtime {
        match self {
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
    /// clock is not a property of which runtime the author chose -- so putting
    /// it in the enum would have made "can this mod animate" depend on that.
    pub clock: SharedClock,
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

    //      AND `dew.Text`/`dew.Clock`, ON THE SAME TERMS AND FOR THE SAME REASON.
    //      Text metrics and a frame clock are what the host computes and no guest
    //      can: they are the language of the platform rather than a capability, so
    //      they are installed as ungated members of the `dew` global here rather
    //      than granted in the table built at step 4. `services.rs` carries the
    //      full argument, and the short version is that a mod refused text metrics
    //      cannot lay out -- a permission with only one sound answer is not a
    //      permission.
    //
    //      FOR BOTH RUNTIMES. A DataModel mod calls `dew.Text.Measure`
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
    //      module is loaded. Produces something step 6 mounts with, and does
    //      not run a line of the mod.
    enum Ceremony {
        /// The `ScreenGui` in `dom` that the guest parents into.
        DataModel { root: usize },
    }

    let ceremony = match manifest.runtime {
        // no framework: just a root to parent into.
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
        //       HANDED TO `mount`, NOT INSTALLED AS A GLOBAL. `examples/host/standalone`
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
    let requested: Requested = Arc::new(Mutex::new(None));
    let grant = capabilities::SurfaceGrant {
        requested: Arc::clone(&requested),
        root: match &ceremony {
            //  THE HANDLE, NOT THE NODE ID. `root` is an index into the DOM and
            //  handing that over parents instances into an integer; the applet
            //  wants the same userdata `mount` was always given.
            Ceremony::DataModel { root } => Some(
                datamodel::handle(vm.lua(), &dom, *root)
                    .map_err(|e| format!("{}: {e}", manifest.id))?
                    .into_lua(vm.lua())
                    .map_err(|e| format!("{}: {e}", manifest.id))?,
            ),
        },
        title: manifest.display_name().to_string(),
    };
    let dew = capabilities::build(vm.lua(), &manifest.permissions, state, &grant)
        .map_err(|e| format!("{}: {e}", manifest.id))?;

    // 5 ── the applet's own module. It asks for a surface, or it returns a
    //      declaration describing one. Both arrive from running it.
    //  `dew` IS A GLOBAL, the way `game` is one in the engine this host is shaped
    //  after. It was handed to `mount` as an argument, which is why the old
    //  contract had to return a table: there was no other way to be given a
    //  capability table.
    //
    //  NOT THE CHUNK'S VARARG. A chunk's `...` reaches the entry module and stops
    //  there, so an applet split across two files could not see `dew` from the
    //  second one without threading it through every call that needed it. It is
    //  also not an idiom an applet author has met: a ModuleScript's chunk
    //  receives nothing, so `local dew = ...` means nothing in the engine.
    //
    //  AN UNGRANTED CAPABILITY IS STILL ABSENT RATHER THAN GUARDED. That comes
    //  from which keys this table has, which `capabilities::build` decides from
    //  the manifest, and not from how the table is delivered.
    vm.lua()
        .globals()
        .set("dew", dew.clone())
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

    // THE SIGNATURE IS THE RUNTIME'S, and so is the message when it is missing.
    let signature = match manifest.runtime {
        Runtime::DataModel => "mount = function(dew, root) … end",
    };
    //  AN APPLET THAT ASKED FOR ITS SURFACE HAS ALREADY BUILT ITS TREE. It was
    //  handed the root by `dew.Widget{}` while it ran, so there is nothing left
    //  for the host to call and no declaration to read. That is the shape this
    //  is moving to; the returned table is what it is moving from.
    let mount: Option<LuaFunction> = match declaration.get::<LuaFunction>("mount") {
        Ok(function) => Some(function),
        Err(_) if asked.is_some() => None,
        Err(_) => {
            return Err(format!(
                "{}: the module neither asked for a surface nor returned a `mount`.                  Call `dew.Widget{{ width = 200, height = 100 }}` and parent your                  tree into what it returns, or return {{ size = ..., {signature} }}.                  See docs/applet_contract.md",
                manifest.id
            ))
        }
    };

    // 6 ── build the tree, ONCE.
    let mounted = match ceremony {
        //      CALLED DIRECTLY, because there is no scope to be inside. A
        //      DataModel mod's `mount` parents instances and returns; its return
        //      value is deliberately ignored, since the tree the host renders is
        //      the one under the root it was handed rather than one it was given
        //      back. Handing a root over and then reading a returned tree would
        //      be two answers to the same question.
        Ceremony::DataModel { root } => {
            //      NOTHING TO CALL WHEN THE APPLET ALREADY BUILT ITS TREE.
            //      `dew.Widget{}` handed it this same root while it ran, so the
            //      instances are under there already and calling a second
            //      entry point would ask it to build them twice.
            if let Some(mount) = mount {
                let handle = datamodel::handle(vm.lua(), &dom, root)
                    .map_err(|e| format!("{}: {e}", manifest.id))?;
                mount
                    .call::<()>((dew, handle))
                    .map_err(|e| format!("{}: while mounting: {e}", manifest.id))?;
            }
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
pub mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

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
            // The path is still a require root, and pointing it at the mod's own
            // directory keeps this test from depending on an install having run.
            load(&self.0, &self.0, &Default::default(), &state)
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
        // in `datamodel::services` prove them on a bare VM; this proves `dew.Text`
        // and `dew.Clock` survive the manifest, the sandbox and the capability
        // table -- ungated members of the same `dew` a mod is handed, on the same
        // terms as `dew.Time`.
        let fixture = Fixture::new(
            "services",
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    mount = function(dew, root)
                        assert(dew.Text ~= nil, "dew.Text is present")
                        local w, h = dew.Text.Measure("hello", 14)
                        assert(type(w) == "number" and type(h) == "number", "two numbers")
                        local stop = dew.Clock.OnFrame(function(dt) end)
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
                        dew.Clock.OnFrame(function(dt) ticks += 1 end)
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
            "id = \"plain\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            r#"
                return {
                    id = "plain",
                    mount = function(dew, root)
                        local frame = Instance.new("Frame")
                        frame.Name = "Body"
                        frame.Parent = root
                        dew.Clock.OnFrame(function(dt)
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
        // and the risk was that `dew.Text`/`dew.Clock` picked up the same
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
            .load("return dew.Text.Measure ~= nil and dew.Clock.OnFrame ~= nil")
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
                        assert(dew.Storage ~= nil, "storage was granted")
                        assert(dew.Clipboard == nil, "clipboard was not asked for")
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
local root = dew.Widget({ width = 120, height = 60 })
local frame = Instance.new("Frame")
frame.Name = "Asked"
frame.Size = UDim2.new(1, 0, 1, 0)
frame.BackgroundColor3 = Color3.fromRGB(20, 20, 20)
frame.Parent = root
"#;

    /// The shape this is all moving to: nothing returned at all.
    ///
    /// AN APPLET USED TO HAVE TO RETURN A TABLE to be given anything, because
    /// `dew` only ever reached it through `mount`. It arrives as the chunk's
    /// vararg now, so asking is possible before there is anything to return.
    #[test]
    fn an_applet_that_asks_returns_nothing() {
        let fixture = Fixture::new(
            "asks",
            "id = \"asks\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            ASKS,
        );
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
            "id = \"ungranted\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            "assert(dew.Widget ~= nil, \"widget was granted\")\n\
             assert(dew.Overlay == nil, \"overlay was not granted and must be absent\")\n\
             local root = dew.Widget({ width = 10, height = 10 })\n",
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
            "id = \"neither\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            "return { size = { width = 10, height = 10 } }\n",
        );
        let error = match fixture.load() {
            Err(error) => error,
            Ok(_) => panic!("an applet with no surface and no mount should not load"),
        };
        assert!(
            error.contains("dew.Widget") && error.contains("mount"),
            "the message should name both ways out, got: {error}"
        );
    }

    /// A second file in the applet can reach `dew` without being handed it.
    ///
    /// THE REASON IT IS A GLOBAL RATHER THAN THE CHUNK'S VARARG. `...` reaches
    /// the entry module and stops there, so an applet split across two files
    /// would see nothing from the second one, and `dew` would have to be
    /// threaded through every call that wanted it.
    #[test]
    fn a_required_module_can_reach_dew() {
        let fixture = Fixture::new(
            "asks-submodule",
            "id = \"sub\"
runtime = \"datamodel\"
permissions = [\"widget\"]
",
            "local helper = require(\"./helper\")
helper()
",
        );
        fixture.write(
            "helper.luau",
            "return function()
  assert(dew ~= nil, \"a required module should see dew\")
             local root = dew.Widget({ width = 12, height = 12 })
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
            "id = \"both\"\nruntime = \"datamodel\"\npermissions = [\"widget\"]\n",
            "local root = dew.Widget({ width = 33, height = 44 })\n\
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
        let aether_root = dew_runtime::installed_package_in(&dir, "aether")
            .expect("timetracker has no aether installed");
        let loaded =
            load(&dir, &aether_root, &Default::default(), &state).expect("timetracker should load");

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
        services::pointer_moved(x, y);
        pointer.moved(&surface, x, y).expect("moved");
        services::tick(&loaded.clock, 1.0 / 60.0);
        settle();

        services::pointer_button(button as usize, true);
        pointer.down(&surface, button, x, y).expect("down");
        services::tick(&loaded.clock, 1.0 / 60.0);
        settle();

        services::pointer_button(button as usize, false);
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
