//! Loading a mod: manifest, then VM, then capabilities, then its own code.
//!
//! THE ORDER IS THE POINT. Everything the host needs to decide whether a mod may
//! run is known before the mod's logic executes:
//!
//!   1. `mod.json` is read from disk. It names the mod, its runtime and its
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
use crate::services::{self, Clock, SharedClock};
use crate::manifest::{Manifest, Runtime};
use crate::surface::Declared;
use aether_runtime::{modules, Session, Vm};
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

pub struct Mod {
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

pub fn load(
    dir: &Path,
    aether_root: &Path,
    aliases: &std::collections::HashMap<String, PathBuf>,
    state: &Shared,
) -> Result<Mod, String> {
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
    let caps = aether_runtime::Capabilities {
        require_roots: vec![dir.to_path_buf(), aether_root.to_path_buf()],
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
    //      guest on Roblox and on Dew alike, and an application that had to be
    //      handed it would not be the application that runs on both.
    let dom = datamodel::SharedDom::default();
    //      AND `mod://` POINTS AT THE MOD'S OWN DIRECTORY, which is the same
    //      boundary `require_roots` draws one statement above. A mod reaches its
    //      own files and no others, whether it asks for them with `require` or
    //      with an `Image` property; images arriving later must not become the
    //      way around a rule the requirer already enforces.
    dom.lock().expect("dom").assets.set_root(dir.to_path_buf());
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

    modules::install(&vm, &caps).map_err(|e| format!("{}: {e}", manifest.id))?;

    // 3 ── whatever the declared runtime needs in place BEFORE the mod's own
    //      module is loaded. Both arms below produce something step 6 mounts
    //      with, and neither runs a line of the mod.
    //
    //      `Instance` ONLY FOR AN AETHER MOD, NOT THE VOCABULARY. Aether carries
    //      its own `UDim2`, `Color3` and the rest for off-engine hosts and
    //      publishes them with `if rawget(g, name) == nil` -- first writer wins --
    //      so a partial host vocabulary does not merge with Aether's, it blocks
    //      it. Installing the five host types for an Aether mod took
    //      `Color3.fromHex` away and all three mods stopped loading, in either
    //      order. `datamodel::install_vocabulary` says what closing that costs;
    //      it is the change where Aether consumes the host's vocabulary rather
    //      than carrying one, and it retires `Headless.luau` at the same time.
    //
    //      A DATAMODEL MOD HAS NO SUCH CONFLICT and needs the vocabulary to write
    //      a single line, so it gets it. That the two arms differ here is not an
    //      inconsistency to tidy away later: it is the whole reason the runtime is
    //      declared rather than sniffed.
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
            let desktop: LuaTable =
                modules::load_entry(&vm, &aether_root.join("src/host/Desktop.luau"))
                    .and_then(|f| f.call(()))
                    .map_err(|e| format!("{}: loading Aether's desktop host: {e}", manifest.id))?;
            Ceremony::Aether(desktop)
        }
        // 3b ── no framework: the vocabulary, and a root to parent into.
        //
        //       `DewRoot` RATHER THAN `game`, and `game` IS NOT ON THE PLAN.
        //       This used to say the name "arrives in sprint 9, when the services
        //       and the member surface behind it make it true" -- which was the
        //       plan under the old numbering and is now wrong about it twice over.
        //       `game` was DROPPED from step F by decision on 2026-09-04: it was
        //       never a capability here but a SENTINEL, the proxy `Host.detect()`
        //       uses for "are these four globals real", and Dew installs all four.
        //       The roadmap's "Why `game` was dropped from step F" carries the
        //       measurement. It is not forbidden forever; it is simply not what
        //       makes a guest framework run here.
        //
        //       A `ScreenGui` because that is what a Roblox application expects
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
            datamodel::install_vocabulary(vm.lua()).map_err(|e| format!("{}: {e}", manifest.id))?;
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

    // THE SIGNATURE IS THE RUNTIME'S, and so is the message when it is missing.
    // An author who wrote a DataModel mod and forgot `mount` should not be shown
    // an Aether component's shape to copy.
    let signature = match manifest.runtime {
        Runtime::Aether => "mount = function(dew) … end",
        Runtime::DataModel => "mount = function(dew, root) … end",
    };
    let mount: LuaFunction = declaration.get("mount").map_err(|_| {
        format!(
            "{}: the module returned no `mount` — a Dew mod returns \
             {{ id = …, size = …, {signature} }}. See docs/mod_contract.md",
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
            let handle = datamodel::handle(&dom, root);
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

    Ok(Mod {
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
    /// The loader reads `mod.json` and an entry module from a real directory,
    /// which is the behaviour under test; faking the filesystem here would test
    /// a different loader.
    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str, manifest: &str, entry: &str) -> Fixture {
            let dir = std::env::temp_dir().join(format!("dew-mods-test-{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("temp dir");
            std::fs::write(dir.join("mod.json"), manifest).expect("mod.json");
            std::fs::write(dir.join("main.luau"), entry).expect("entry");
            Fixture(dir)
        }

        fn load(&self) -> Result<Mod, String> {
            let state: Shared = Arc::new(Mutex::new(capabilities::HostState::default()));
            // A DataModel mod never reads Aether's source; the path is still a
            // require root, and pointing it at the mod's own directory keeps this
            // test from depending on an install having been run.
            load(&self.0, &self.0, &Default::default(), &state)
        }
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
            r#"{ "id": "plain", "runtime": "datamodel" }"#,
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
            r#"{ "id": "plain", "runtime": "datamodel" }"#,
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
            r#"{ "id": "plain", "runtime": "datamodel" }"#,
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
            r#"{ "id": "plain", "runtime": "datamodel" }"#,
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
            r#"{ "id": "plain", "runtime": "datamodel" }"#,
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

    #[test]
    fn the_vocabulary_is_installed_for_a_datamodel_mod() {
        // `Color3` above is the assertion: without `install_vocabulary` the mount
        // fails at the first line that names one. Stated separately so a failure
        // here reads as "the vocabulary went missing" rather than as a render
        // count being wrong.
        let fixture = Fixture::new(
            "vocabulary",
            r#"{ "id": "plain", "runtime": "datamodel" }"#,
            PLAIN,
        );
        assert!(fixture.load().is_ok());
    }

    #[test]
    fn a_datamodel_mod_is_handed_only_what_its_manifest_granted() {
        // The capability table reaches this branch too, and it is still built
        // from the permissions ONLY: `storage` is a table, `clipboard` is absent.
        let fixture = Fixture::new(
            "caps",
            r#"{ "id": "plain", "runtime": "datamodel", "permissions": ["storage"] }"#,
            r#"
                return {
                    id = "plain",
                    mount = function(dew, root)
                        assert(dew.storage ~= nil, "storage was granted")
                        assert(dew.clipboard == nil, "clipboard was not asked for")
                        assert(root.Name == "DewRoot", "the root is named DewRoot")
                        assert(rawget(_G, "game") == nil, "`game` is not on the plan; see the roadmap")
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
            r#"{ "id": "plain", "runtime": "datamodel" }"#,
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
}
