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
use crate::manifest::{Manifest, Runtime};
use crate::services::{self, Clock, SharedClock};
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

/// Install `game`, for vide's truthiness gate and for nothing else.
///
/// # What this is
///
/// `game = true`. A boolean. Not a DataModel, not a service locator, not a tree,
/// and not something any Aether code path reads.
///
/// # Why anything named `game` exists on this host at all
///
/// vide ships twenty-three modules and seven of them open with one line:
///
/// ```luau
/// local typeof = game and typeof or require "../test/mock".typeof
/// ```
///
/// `game` is NOT one of the four names vide declares its host boundary in --
/// those are `typeof`, `Instance`, `Enum` and `Color3`, and Dew installed all
/// four long before this. It is the CONDITION those names are reached through,
/// and it is read for truthiness alone. With `game` nil the expression never
/// evaluates the real `typeof` however real it is: it evaluates the fallback, and
/// the fallback is a require of `test/mock`, which the wally and pesde artifacts
/// do not ship. Measured on this host in milestone 2's sprint 8: `create`,
/// `apply`, `defaults`, `mount`, `cleanup`, `changed` and `lib` all failed to
/// require, with `Instance` present. Aether's DataModel host takes four of those
/// straight from vide, so without this line it has nothing to take.
///
/// It is a gate in a dependency neither this repository nor Aether's can edit.
///
/// # Why `true` and not something that looks more like a DataModel
///
/// Because every property a bigger `game` could have is one somebody would come
/// to depend on, and ADR-001's 2026-09-04 amendment draws the bound in as many
/// words: not a DataModel, not a service locator, and not reachable by any Aether
/// code path. A boolean satisfies the gate and satisfies nothing else.
///
/// It also FAILS LOUDLY in the one direction worth failing loudly in. `typeof`
/// answers `"boolean"`, so the vendor test this project spent four days
/// unpicking -- `typeof(game) == "Instance"` -- still answers false here, which
/// is the truth: Dew is not Roblox. And anything reaching for `game:GetService`
/// gets "attempt to index boolean", at the call site, naming the line. A table
/// with a `GetService` returning nil would be a service locator that answers
/// every question with silence, which is the shape of failure this project keeps
/// finding rather than a new one.
///
/// vide's own `lib.luau` is the demonstration: it reads
/// `game and game:GetService("RunService").Heartbeat`, so a truthy `game` makes
/// it error at load rather than quietly wire a frame source to nothing. Aether's
/// `VideCore` assembles vide from its modules and never requires `lib.luau`,
/// which is why that error is a property of this design rather than a bug in it.
///
/// # Why here and not beside `Instance`
///
/// `Instance` and the vocabulary are installed for every guest because they are
/// the language of the platform: a guest has them on Roblox and on Dew alike, and
/// an application that had to be handed them would not be the application that
/// runs on both. This is not that. It is a gate ONE DEPENDENCY OF ONE RUNTIME
/// reads, so it is installed for that runtime, at the point where that runtime's
/// ceremony begins, and a `runtime = "datamodel"` mod never sees it.
///
/// That scoping is the whole safeguard. `game` was load-bearing for four days
/// because one line in ADR-001 made it the conformance test; if anything other
/// than vide's gate starts reading it, that is a regression of the amendment and
/// not a convenience.
fn install_gate(lua: &Lua) -> LuaResult<()> {
    lua.globals().set("game", true)
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
    //      supplies the vocabulary, exactly as on Roblox -- so nothing publishes a
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
        //
        //       AND `game`, WHICH IS ONE LINE AND OWES AN EXPLANATION LONGER THAN
        //       ITSELF. See `install_gate` below: it is here rather than beside
        //       `Instance` because it is not part of the language of the platform,
        //       it is a gate ONE DEPENDENCY OF THIS RUNTIME reads, and a DataModel
        //       mod -- which has no vide -- must not be given it.
        Runtime::Aether => {
            install_gate(vm.lua()).map_err(|e| format!("{}: {e}", manifest.id))?;
            let desktop: LuaTable =
                modules::load_entry(&vm, &aether_root.join("src/host/Desktop.luau"))
                    .and_then(|f| f.call(()))
                    .map_err(|e| format!("{}: loading Aether's desktop host: {e}", manifest.id))?;
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

        /// The same fixture, loaded against the INSTALLED Aether and vide.
        ///
        /// This one does depend on `pesde install` having run, which the loader
        /// above deliberately does not -- and it has to: what it is testing is
        /// the framework's own behaviour on this host, so there is nothing to
        /// stand in for the framework. `crates/runtime`'s parity tests take the
        /// same dependency for the same reason.
        fn load_aether(&self) -> Result<Mod, String> {
            let state: Shared = Arc::new(Mutex::new(capabilities::HostState::default()));
            let root = aether_runtime::installed_package("aether")
                .expect("no installed aether -- run `pesde install` at the repository root");
            let vide = aether_runtime::installed_package("vide")
                .expect("no installed vide -- run `pesde install` at the repository root");
            let mut aliases = std::collections::HashMap::new();
            aliases.insert("aether".to_string(), root.join("src"));
            aliases.insert("vide".to_string(), vide.join("src"));
            load(&self.0, &root, &aliases, &state)
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
        local Aether = require("@aether/api")
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
            r#"{ "id": "plain", "runtime": "aether" }"#,
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
                local host = require("@aether/api").Host.detect()
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
        // The gate, and what it is allowed to be. A boolean satisfies vide and
        // satisfies nothing else; `typeof(game) == "Instance"` -- the vendor test
        // this replaced -- still correctly answers false.
        let gate: LuaValue = globals.get("game").expect("game");
        assert!(
            matches!(gate, LuaValue::Boolean(true)),
            "`game` must be a truthy value that is not a DataModel, got {gate:?}"
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
            r#"{ "id": "plain", "runtime": "aether" }"#,
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
        // already made absolute. On `mods/timetracker` a label walked 286 pixels
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
                local host = require("@aether/api").Host.detect()
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
            r#"{ "id": "plain", "runtime": "datamodel" }"#,
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
            r#"{ "id": "plain", "runtime": "datamodel", "permissions": ["storage"] }"#,
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
