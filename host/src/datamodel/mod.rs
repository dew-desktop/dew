//! The host-owned DataModel: instances, the tree, and property validation.
//!
//! WHY THIS EXISTS
//! Dew is its own product. A developer arriving from the engine writes Luau against
//! a DataModel, and that experience is the whole promise -- so Dew has to provide
//! one rather than require Aether. Aether is a headless framework that runs on
//! top of a host, the way Ark UI runs on top of a DOM; it is a consumer of what
//! is built here, never a substitute for it.
//!
//! WHAT WAS HERE BEFORE
//! Nothing. `host/Cargo.toml` has carried `rbx_types`, `rbx_reflection` and
//! `rbx_reflection_database` since they were added, with a comment saying they
//! are "what lets the host reject a property that does not exist on a class
//! before the value reaches layout, which is the security argument for a
//! host-owned datamodel made concrete". No file in the host binary imported any
//! of them. The only consumer was the reporting tool that prints
//! `docs/datamodel_scope.md`. The mechanism described in the present tense had
//! never existed, and nothing said so. This file is that comment becoming true.
//!
//! THE SHAPE, and why it is an arena
//! Instances form a graph with parent and child edges pointing both ways.
//! `Rc<RefCell<Node>>` on both sides leaks every tree that is ever built, and
//! `Rc` is not `Send` besides -- the VM is created with mlua's `send` feature, so
//! anything a guest can hold must cross threads. Ids into an arena behind one
//! `Arc<Mutex<_>>` avoid the cycle entirely, make `Destroy` a matter of clearing
//! a slot, and match how `capabilities::HostState` is already shared.
//!
//! WHAT THIS SLICE COVERS, and what it does not
//! `Instance.new`, `ClassName`, `Name`, `Parent`, and property get and set for
//! properties whose declared type is a primitive. No events, no methods, no
//! layout, and no vocabulary types -- `UDim2`, `Color3` and `Vector2` need
//! constructors in the guest before a property of that type can be assigned, and
//! that is the next slice. An unsupported type says so IN THOSE WORDS rather
//! than reporting a rejection, because "this host has not built that yet" and
//! "your program is wrong" must never arrive as the same message.

mod collection_service;
mod content;
pub mod enums;
pub mod extensions;
pub mod input;
pub mod members;
pub mod render;
mod service_provider;
pub mod signal;
mod vocabulary;

use content::{LuaContent, LuaFont};
use enums::LuaEnumItem;
use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use rbx_reflection::{DataType, PropertyDescriptor, Scriptability};
use rbx_types::{Variant, VariantType};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use vocabulary::{
    LuaColor3, LuaColorSequence, LuaNumberSequence, LuaRect, LuaUDim, LuaUDim2, LuaVector2,
};

/// One instance's state. Never handed to a guest directly.
struct Node {
    class: String,
    name: String,
    parent: Option<usize>,
    children: Vec<usize>,
    /// Only what has been ASSIGNED. An unset property reads its default from the
    /// reflection database, so this map is the diff from a fresh instance rather
    /// than a full property set -- which is also what makes "was this touched"
    /// answerable later.
    props: BTreeMap<String, Variant>,
    attributes: BTreeMap<String, Variant>,
    /// Indices into `Dom::handlers`, IN CONNECT ORDER. Firing order is connect
    /// order, and this is the only place that order is recorded -- a map keyed by
    /// event would lose it, and a guest that connects a logger and then a
    /// mutator is relying on it.
    connections: Vec<usize>,
}

/// One `Connect`, kept beside the arena rather than inside the signal object.
///
/// A SIGNAL OWNS NOTHING. `f.Changed` builds a fresh userdata every time it is
/// read, so a handler list living in the signal would be a list per read. They
/// live here, and the node owns the ids, which is also what makes `Destroy` drop
/// every connection on the way past instead of leaving live closures pointing at
/// a freed slot.
struct Handler {
    instance: usize,
    kind: signal::Kind,
    f: LuaFunction,
}

/// Every instance in one VM.
pub struct Dom {
    slots: Vec<Option<Node>>,
    handlers: Vec<Option<Handler>>,
    /// Has anything a guest can see changed since the last frame was drawn?
    ///
    /// THE ANSWER USED TO BE "ASSUME SO", and the DataModel arm of the frame loop
    /// repainted unconditionally because of it -- 1424 painted frames a second on
    /// a static mod, measured in sprint 7. Every write a guest can make now goes
    /// through a path that fires `Changed`, and the same path sets this. Layout's
    /// own writes do NOT: `set_internal` is how `AbsolutePosition` gets there, and
    /// marking it dirty would make the render pass re-dirty the tree it just
    /// rendered, which is a repaint loop wearing the costume of a fix.
    dirty: bool,
    /// What this guest's `Image` and `ImageContent` properties resolve to.
    ///
    /// PER DOM, WHICH IS PER MOD, and that is the same isolation the require
    /// roots enforce for files: `mod://` means "beside THIS mod", and a store
    /// shared between two guests would make one mod's assets reachable by name
    /// from another. It also makes the decoded cache die with the mod, which is
    /// what a `Destroy` of the last node holding an image should cost.
    pub assets: crate::assets::Assets,
    /// Ids whose slot has been emptied and whose guest handle can be forgotten.
    ///
    /// A LIST RATHER THAN A CALL, because [`Dom::destroy`] has no `&Lua` and the
    /// handle cache is a Lua table. [`handle`] drains this before it answers, so
    /// the cache is pruned by the next thing that needs it. Bounded by the nodes
    /// destroyed since the last handle was asked for, and the whole VM goes when
    /// the mod does.
    released: Vec<usize>,
    /// `CollectionService`'s tag multimap: tag name to the instances holding it.
    ///
    /// PER DOM, the same as `assets` above -- a mod's tags are its own, and this
    /// is what makes them die with the mod rather than leaking into the next one
    /// loaded into the same process.
    tags: BTreeMap<String, BTreeSet<usize>>,
    /// The reverse index of `tags`, for `GetTags` and for `destroy` to find what
    /// to clean up without a scan over every tag.
    instance_tags: BTreeMap<usize, BTreeSet<String>>,
    /// The id of `CollectionService`'s own pseudo-instance, minted the first time
    /// a tag signal is asked for.
    ///
    /// A REAL ARENA SLOT, so `GetInstanceAddedSignal`/`GetInstanceRemovedSignal`
    /// reuse `Dom::connect`/`listeners`/`disconnect` and `signal::fire` exactly as
    /// `Changed` does, rather than a second handler list and a second discipline
    /// for collecting, releasing and calling. It holds no children, is never
    /// reachable from a guest as an `Instance`, and is never destroyed -- there is
    /// nothing that would call `Dom::destroy` on it.
    collection_service_id: Option<usize>,
}

/// STARTS DIRTY. A tree nothing has touched still has to reach the screen once,
/// and a derived `Default` would leave the first frame unpainted -- a black
/// window that fixes itself the first time anything moves, which is the worst
/// possible way for this to be wrong.
impl Default for Dom {
    fn default() -> Self {
        Dom {
            slots: Vec::new(),
            handlers: Vec::new(),
            dirty: true,
            assets: crate::assets::Assets::default(),
            released: Vec::new(),
            tags: BTreeMap::new(),
            instance_tags: BTreeMap::new(),
            collection_service_id: None,
        }
    }
}

pub type SharedDom = Arc<Mutex<Dom>>;

impl Dom {
    pub fn insert(&mut self, class: String, name: String) -> usize {
        self.slots.push(Some(Node {
            class,
            name,
            parent: None,
            children: Vec::new(),
            props: BTreeMap::new(),
            attributes: BTreeMap::new(),
            connections: Vec::new(),
        }));
        self.dirty = true;
        self.slots.len() - 1
    }

    fn node(&self, id: usize) -> Option<&Node> {
        self.slots.get(id).and_then(|s| s.as_ref())
    }

    fn node_mut(&mut self, id: usize) -> Option<&mut Node> {
        self.slots.get_mut(id).and_then(|s| s.as_mut())
    }

    /// Detach `id` from whatever currently holds it.
    fn unparent(&mut self, id: usize) {
        let Some(old) = self.node(id).and_then(|n| n.parent) else {
            return;
        };
        if let Some(parent) = self.node_mut(old) {
            parent.children.retain(|c| *c != id);
        }
        if let Some(node) = self.node_mut(id) {
            node.parent = None;
        }
    }

    // ── What the renderer reads ──────────────────────────────────────────────
    //
    // The arena is private and stays private; these are the questions a display
    // pass asks, answered without handing out a `&Node`. Keeping the struct
    // sealed is what stops the renderer growing a second idea of what a property
    // means -- it must go through the same default resolution a guest read does.

    /// A property as stored, or the engine default when nothing assigned it.
    pub fn property(&self, id: usize, key: &str) -> Option<Variant> {
        let node = self.node(id)?;
        if let Some(stored) = node.props.get(key) {
            return Some(stored.clone());
        }
        default_for(&node.class, key)
    }

    pub fn children(&self, id: usize) -> Vec<usize> {
        self.node(id)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    pub fn class_of(&self, id: usize) -> Option<String> {
        self.node(id).map(|n| n.class.clone())
    }

    pub fn name_of(&self, id: usize) -> Option<String> {
        self.node(id).map(|n| n.name.clone())
    }

    pub fn parent_of(&self, id: usize) -> Option<usize> {
        self.node(id).and_then(|n| n.parent)
    }

    pub fn exists(&self, id: usize) -> bool {
        self.node(id).is_some()
    }

    pub fn get_attribute(&self, id: usize, name: &str) -> Option<Variant> {
        self.node(id).and_then(|n| n.attributes.get(name).cloned())
    }

    pub fn set_attribute(&mut self, id: usize, name: &str, val: Option<Variant>) {
        if let Some(node) = self.node_mut(id) {
            match val {
                Some(v) => {
                    node.attributes.insert(name.to_string(), v);
                }
                None => {
                    node.attributes.remove(name);
                }
            }
            self.dirty = true;
        }
    }

    pub fn get_attributes(&self, id: usize) -> BTreeMap<String, Variant> {
        self.node(id)
            .map(|n| n.attributes.clone())
            .unwrap_or_default()
    }

    // ── Tags (`CollectionService`) ───────────────────────────────────────────

    /// The id of `CollectionService`'s own pseudo-instance, minting it on first
    /// use. See the field's own doc comment for why it exists at all.
    fn collection_service_id(&mut self) -> usize {
        if let Some(id) = self.collection_service_id {
            return id;
        }
        let id = self.insert(
            "CollectionService".to_string(),
            "CollectionService".to_string(),
        );
        self.collection_service_id = Some(id);
        id
    }

    /// Add `tag` to `id`. Answers whether it was newly added, which is what a
    /// caller needs to decide whether `InstanceAdded` should fire -- adding a tag
    /// an instance already holds is a no-op on the engine, not an error.
    fn add_tag(&mut self, id: usize, tag: &str) -> bool {
        if self.node(id).is_none() {
            return false;
        }
        let added = self.tags.entry(tag.to_string()).or_default().insert(id);
        if added {
            self.instance_tags
                .entry(id)
                .or_default()
                .insert(tag.to_string());
        }
        added
    }

    /// Remove `tag` from `id`. Answers whether it was actually removed -- a tag
    /// the instance never held is a no-op on the engine, not an error.
    fn remove_tag(&mut self, id: usize, tag: &str) -> bool {
        let removed = self
            .tags
            .get_mut(tag)
            .is_some_and(|holders| holders.remove(&id));
        if removed {
            if self.tags.get(tag).is_some_and(|holders| holders.is_empty()) {
                self.tags.remove(tag);
            }
            if let Some(held) = self.instance_tags.get_mut(&id) {
                held.remove(tag);
                if held.is_empty() {
                    self.instance_tags.remove(&id);
                }
            }
        }
        removed
    }

    fn has_tag(&self, id: usize, tag: &str) -> bool {
        self.tags
            .get(tag)
            .is_some_and(|holders| holders.contains(&id))
    }

    /// Every instance holding `tag`, or empty when nothing does -- never nil and
    /// never an error, matching the engine's own answer for an unused tag.
    fn get_tagged(&self, tag: &str) -> Vec<usize> {
        self.tags
            .get(tag)
            .map(|holders| holders.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Every tag `id` holds, or empty once it holds none.
    fn get_tags(&self, id: usize) -> Vec<String> {
        self.instance_tags
            .get(&id)
            .map(|held| held.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Detach `id` from every tag it holds, for `Destroy`. Answers the tags it
    /// held, so the caller can fire `InstanceRemoved` for each -- the engine
    /// fires that signal for a destroyed instance exactly as it does for an
    /// explicit `RemoveTag`, and this is called from inside the destroy walk
    /// while the node is still alive, before `Dom::destroy` frees its slot.
    fn untag_all(&mut self, id: usize) -> Vec<String> {
        let Some(held) = self.instance_tags.remove(&id) else {
            return Vec::new();
        };
        for tag in &held {
            if let Some(holders) = self.tags.get_mut(tag) {
                holders.remove(&id);
                if holders.is_empty() {
                    self.tags.remove(tag);
                }
            }
        }
        held.into_iter().collect()
    }

    // ── Connections ──────────────────────────────────────────────────────────
    //
    // THE HANDLER TABLE IS AN ARENA TOO, and ids are never reused for the same
    // reason instance ids are not: a `RBXScriptConnection` a guest kept must stay
    // stale rather than come back pointing at somebody else's handler.

    /// Record a handler, or `None` when the instance is gone.
    pub fn connect(&mut self, id: usize, kind: signal::Kind, f: LuaFunction) -> Option<usize> {
        self.node(id)?;
        self.handlers.push(Some(Handler {
            instance: id,
            kind,
            f,
        }));
        let conn = self.handlers.len() - 1;
        self.node_mut(id).expect("checked").connections.push(conn);
        Some(conn)
    }

    /// Drop a handler. Already gone is not an error -- see `Disconnect`.
    pub fn disconnect(&mut self, conn: usize) {
        let Some(handler) = self.handlers.get_mut(conn).and_then(Option::take) else {
            return;
        };
        if let Some(node) = self.node_mut(handler.instance) {
            node.connections.retain(|c| *c != conn);
        }
    }

    pub fn is_connected(&self, conn: usize) -> bool {
        self.handlers.get(conn).is_some_and(Option::is_some)
    }

    /// The handlers connected to `kind` on `id`, in connect order.
    ///
    /// RETURNS IDS, NOT FUNCTIONS. The caller re-looks-up each one immediately
    /// before calling it, so that a handler which disconnects another during the
    /// same fire is honoured rather than raced.
    pub fn listeners(&self, id: usize, kind: &signal::Kind) -> Vec<usize> {
        let Some(node) = self.node(id) else {
            return Vec::new();
        };
        node.connections
            .iter()
            .copied()
            .filter(|c| {
                self.handlers
                    .get(*c)
                    .and_then(Option::as_ref)
                    .is_some_and(|h| &h.kind == kind)
            })
            .collect()
    }

    pub fn handler(&self, conn: usize) -> Option<LuaFunction> {
        self.handlers
            .get(conn)
            .and_then(Option::as_ref)
            .map(|h| h.f.clone())
    }

    // ── Repaint ──────────────────────────────────────────────────────────────

    /// Something changed; the next frame has to be drawn.
    pub fn touch(&mut self) {
        self.dirty = true;
    }

    /// Is a repaint owed, and clear the debt.
    ///
    /// TAKING RATHER THAN READING, because the caller is about to draw. A
    /// separate `is_dirty` and `clear` is the shape where a `?` between them
    /// loses a frame that will never be asked for again.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    /// Free `id` and everything under it, and detach it from its parent.
    ///
    /// THE SLOT IS EMPTIED, NOT MARKED. `Index` and `NewIndex` already answer
    /// "this instance has been destroyed" when `node` comes back `None`, so
    /// clearing the slot is what makes a handle held across a `Destroy` fail
    /// loudly instead of reading a stale name -- which is the whole difference
    /// between `Destroy` and `Parent = nil`.
    ///
    /// IDS ARE NEVER REUSED: `insert` pushes, so a freed slot stays `None`
    /// forever and a stale id cannot come back pointing at a new instance.
    /// That costs one `Option` per destroyed node and buys the guarantee that
    /// makes handles safe to hold.
    ///
    /// DESCENDANTS GO TOO. The engine destroys the subtree, and leaving children in
    /// the arena would strand a set of nodes with a parent id pointing at an
    /// empty slot -- reachable from nothing, freed by nothing.
    ///
    /// AND THE CONNECTIONS GO WITH THEM. A handler outliving its instance is a
    /// closure the guest can no longer see, still reachable from the arena, that
    /// would fire on a node the arena no longer has. Dropping them here is what
    /// makes `connection.Connected` answer false after a `Destroy` without the
    /// signal type having to know anything about the tree.
    pub fn destroy(&mut self, id: usize) {
        if self.node(id).is_none() {
            return;
        }
        self.unparent(id);
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            let Some(node) = self.slots.get_mut(current).and_then(Option::take) else {
                continue;
            };
            for conn in node.connections {
                if let Some(slot) = self.handlers.get_mut(conn) {
                    *slot = None;
                }
            }
            // AND ITS GUEST HANDLE, once something with a `&Lua` comes past.
            // Ids are never recycled -- `insert` pushes -- so a cache entry that
            // outlives this by a moment cannot come to mean a different instance.
            self.released.push(current);
            stack.extend(node.children);
        }
        self.dirty = true;
    }

    /// Write a property the HOST computed, bypassing the guest's rules.
    ///
    /// `AbsolutePosition` and `AbsoluteSize` are read-only to a guest and written
    /// by whatever laid out the tree. Going through the assignment path would
    /// refuse them, correctly, so the host writes them here instead -- the one
    /// door, named so it is greppable, rather than making the public path lenient.
    pub fn set_internal(&mut self, id: usize, key: &str, value: Variant) {
        if let Some(node) = self.node_mut(id) {
            node.props.insert(key.to_string(), value);
        }
    }

    /// Is `maybe_ancestor` at or above `id`? Used to refuse a cycle.
    fn is_ancestor_of(&self, maybe_ancestor: usize, id: usize) -> bool {
        let mut cursor = Some(id);
        while let Some(current) = cursor {
            if current == maybe_ancestor {
                return true;
            }
            cursor = self.node(current).and_then(|n| n.parent);
        }
        false
    }
}

/// A handle whose slot has been cleared.
///
/// ONE MESSAGE FOR EVERY PATH that finds an empty slot -- reading a property,
/// writing one, or calling a method on a handle held across a `Destroy`. A guest
/// cannot tell which of those it hit and should not have to.
pub(crate) fn dead_instance() -> LuaError {
    LuaError::runtime("this instance has been destroyed")
}

/// A handle a guest holds. `typeof` reports "Instance".
#[derive(Clone)]
pub struct InstanceRef {
    dom: SharedDom,
    id: usize,
}

impl InstanceRef {
    /// The class of the node this handle points at, or `None` once it is
    /// destroyed.
    ///
    /// FOR THE HOST TO REPORT WHAT A GUEST BUILT, and it exists because "which
    /// host did the framework resolve" and "what did the framework build with"
    /// turned out to be two questions. A guest framework can report the first
    /// truthfully and still be assembling a tree out of something else entirely;
    /// this is the only side of that pair the HOST can answer for itself, because
    /// a handle it did not issue cannot be borrowed as one of these.
    pub fn class_name(&self) -> Option<String> {
        self.dom.lock().ok()?.class_of(self.id)
    }
}

/// Look up a property on a class or any of its ancestors.
///
/// WALKS THE CHAIN because the reflection database declares a property on the
/// class that introduces it and does not repeat it on descendants. `Frame` has
/// no `Name` of its own; it has `Instance`'s.
fn describe(class: &str, property: &str) -> Option<&'static PropertyDescriptor<'static>> {
    let db = rbx_reflection_database::get().ok()?;
    let mut cursor = Some(class);
    while let Some(c) = cursor {
        // THE EXTENSION REGISTRY IS CHECKED FIRST ON EVERY STEP of the walk,
        // whether `c` is a class the reflection database also knows about or
        // one this host synthesizes whole (`InputActionLabel.InputAction`
        // below). It is invisible unless its tier says so -- see
        // `extensions::describe`.
        if let Some(descriptor) = extensions::describe(c, property) {
            return Some(descriptor);
        }
        if let Some(current) = db.classes.get(c) {
            if let Some(descriptor) = current.properties.get(property) {
                return Some(descriptor);
            }
            cursor = current.superclass;
        } else if extensions::class_exists(c) {
            // InputActionLabel is introduced in the engine 0.736 and is not yet in
            // rbx_reflection_database 0.728. It inherits from GuiObject and declares
            // text and image properties matching TextLabel and ImageLabel. Its own
            // `InputAction` property is a row in `extensions::PROPERTIES`, checked
            // above; what is left here is the fallback through two sibling classes
            // the registry does not need to know about.
            if let Some(desc) = db
                .classes
                .get("TextLabel")
                .and_then(|cl| cl.properties.get(property))
            {
                return Some(desc);
            }
            if let Some(desc) = db
                .classes
                .get("ImageLabel")
                .and_then(|cl| cl.properties.get(property))
            {
                return Some(desc);
            }
            cursor = Some("GuiObject");
        } else {
            break;
        }
    }
    None
}

fn class_exists(class: &str) -> bool {
    if extensions::class_exists(class) {
        return true;
    }
    rbx_reflection_database::get()
        .map(|db| db.classes.contains_key(class))
        .unwrap_or(false)
}

/// The default a property reads before anything assigns it.
fn default_for(class: &str, property: &str) -> Option<Variant> {
    let db = rbx_reflection_database::get().ok()?;
    let mut cursor = Some(class);
    while let Some(c) = cursor {
        if let Some(value) = extensions::default_for(c, property) {
            return Some(value);
        }
        if let Some(current) = db.classes.get(c) {
            if let Some(value) = current.default_properties.get(property) {
                return Some(value.clone());
            }
            cursor = current.superclass;
        } else if extensions::class_exists(c) {
            if let Some(val) = db
                .classes
                .get("TextLabel")
                .and_then(|cl| cl.default_properties.get(property))
            {
                return Some(val.clone());
            }
            if let Some(val) = db
                .classes
                .get("ImageLabel")
                .and_then(|cl| cl.default_properties.get(property))
            {
                return Some(val.clone());
            }
            cursor = Some("GuiObject");
        } else {
            break;
        }
    }
    None
}

/// A Lua number, whichever way the VM is holding it.
///
/// `Value::as_f64` matches ONLY `Value::Number` and `Value::as_i32` matches ONLY
/// `Value::Integer`; neither crosses the boundary. Luau has one number type and a
/// guest writing `10` cannot know or care which variant mlua produced, so reading
/// through the strict accessors made `UDim2.new(0, 10, 0, 20)` silently yield
/// offsets of zero -- every literal offset in every layout, discarded.
///
/// Nothing caught it because no test had set a numeric property. The tests below
/// now set one of each kind.
pub(crate) fn number(value: &LuaValue) -> Option<f64> {
    match value {
        LuaValue::Integer(i) => Some(*i as f64),
        LuaValue::Number(n) => Some(*n),
        _ => None,
    }
}

/// A whole number of pixels, refusing a fraction rather than truncating.
///
/// `ZIndex` and `LayoutOrder` are integers in the engine. Truncating 10.7 to 10
/// produces a UI that is subtly wrong everywhere and blames nobody, so this asks
/// instead.
///
/// THIS USED TO GUARD THE VOCABULARY CONSTRUCTORS TOO, AND IT WAS WRONG THERE.
/// See [`pixel_i32`]: the engine accepts a fraction in `UDim.new` and truncates
/// it, so refusing one is a divergence rather than a stricter reading. What is
/// left here is the PROPERTY path -- writing an `Int32`-typed member -- and
/// whether the engine coerces or refuses there has NOT been measured, so it is
/// deliberately left alone. Two different questions that happened to share a
/// function.
pub(crate) fn whole_i32(value: f64, what: &str) -> LuaResult<i32> {
    if value.fract() != 0.0 {
        return Err(LuaError::runtime(format!(
            "{what} is a whole number; got {value}"
        )));
    }
    Ok(value as i32)
}

/// A `UDim` offset, truncated towards zero the way the engine truncates it.
///
/// # Why this is not [`whole_i32`], which is what it was until sprint 6
///
/// `UDim.Offset` is an `i32`, and the rule above read that as "a fraction is a
/// mistake worth reporting". The engine reads it as "a fraction is a number that
/// has to become an `i32`", and converts. Measured against lune's implementation
/// of the datatype, which is built on the same `rbx_types` this host is and is
/// what every one of Aether's suites is calibrated against:
///
/// ```text
/// UDim2.fromOffset(0, 8.7138671875).Y.Offset  ->   8
/// UDim.new(0, -2.5).Offset                    ->  -2
/// ```
///
/// Truncation towards zero, which is what `as i32` does.
///
/// # What refusing cost
///
/// A whole class of correct program. The fraction that found this was
/// `8.7138671875`, and it came from a TEXT MEASUREMENT -- the height of one line
/// in the face that will draw it -- travelling through Aether's layout solver
/// into `Host.SetBounds`. Text metrics are fractional by nature, so a host that
/// refuses a fractional offset cannot be told the result of laying out text; the
/// mod that hit it renders in the engine and could not mount here.
///
/// The original argument is not wrong about authors, and it is not what this
/// function is for: it was written about a guest typing `10.7` into a literal,
/// and it was applied to every number that reaches a constructor including the
/// ones a solver computed. A host is entitled to be stricter than the engine
/// about very little, and ADR-001 sets the bar exactly here -- either the same
/// code runs on both, or the DataModel is not faithful yet.
///
/// # What is not fixed by this
///
/// The offset a guest reads back is still an integer, because it always was, so
/// a fractional offset does not round-trip on this host or on the engine. This
/// changes where that is discovered, not whether it is true.
pub(crate) fn pixel_i32(value: f64) -> i32 {
    value as i32
}

/// Turn a Lua value into a `Variant` of the declared type, or say why not.
///
/// THE TWO FAILURES ARE DIFFERENT MESSAGES. A value of the wrong shape is the
/// guest's mistake; a declared type this host has not implemented is the host's,
/// and reporting the second as the first would have an author rewriting correct
/// code to satisfy a limitation nobody told them about.
fn coerce(value: &LuaValue, want: VariantType, class: &str, property: &str) -> LuaResult<Variant> {
    if !supported(want) {
        return Err(LuaError::runtime(format!(
            "{class}.{property} is a {want:?}, which this host cannot accept yet.              The property is real and the value may well be correct; Dew's DataModel              implements primitive-typed properties in this slice and the vocabulary              types in the next. See docs/datamodel_scope.md."
        )));
    }
    let wrong = |expected: &str| {
        LuaError::runtime(format!(
            "{class}.{property} expects {expected}, got {}",
            value.type_name()
        ))
    };
    Ok(match want {
        VariantType::Bool => Variant::Bool(value.as_boolean().ok_or_else(|| wrong("a boolean"))?),
        VariantType::String => Variant::String(
            value
                .as_string()
                .ok_or_else(|| wrong("a string"))?
                .to_string_lossy(),
        ),
        VariantType::Float32 => {
            Variant::Float32(number(value).ok_or_else(|| wrong("a number"))? as f32)
        }
        VariantType::Float64 => Variant::Float64(number(value).ok_or_else(|| wrong("a number"))?),
        VariantType::Int32 => Variant::Int32(whole_i32(
            number(value).ok_or_else(|| wrong("a number"))?,
            &format!("{class}.{property}"),
        )?),
        VariantType::Int64 => {
            let n = number(value).ok_or_else(|| wrong("a number"))?;
            if n.fract() != 0.0 {
                return Err(LuaError::runtime(format!(
                    "{class}.{property} is a whole number; got {n}"
                )));
            }
            Variant::Int64(n as i64)
        }
        VariantType::UDim => {
            Variant::UDim(LuaUDim::from_value(value).ok_or_else(|| wrong("a UDim"))?)
        }
        VariantType::UDim2 => {
            Variant::UDim2(LuaUDim2::from_value(value).ok_or_else(|| wrong("a UDim2"))?)
        }
        VariantType::Vector2 => {
            Variant::Vector2(LuaVector2::from_value(value).ok_or_else(|| wrong("a Vector2"))?)
        }
        VariantType::Color3 => {
            Variant::Color3(LuaColor3::from_value(value).ok_or_else(|| wrong("a Color3"))?)
        }
        VariantType::Rect => {
            Variant::Rect(LuaRect::from_value(value).ok_or_else(|| wrong("a Rect"))?)
        }
        VariantType::Content => {
            Variant::Content(LuaContent::from_value(value).ok_or_else(|| wrong("a Content"))?)
        }
        // THE LEGACY FORM IS A BARE STRING. `Image` is a `ContentId` and
        // `ImageContent` is a `Content`; the same asset is named both ways
        // depending on which generation of the property a guest reaches for.
        VariantType::ContentId => Variant::ContentId(
            value
                .as_string()
                .ok_or_else(|| wrong("a content string"))?
                .to_string_lossy()
                .into(),
        ),
        VariantType::Font => {
            Variant::Font(LuaFont::from_value(value).ok_or_else(|| wrong("a Font"))?)
        }
        VariantType::ColorSequence => Variant::ColorSequence(
            LuaColorSequence::from_value(value).ok_or_else(|| wrong("a ColorSequence"))?,
        ),
        VariantType::NumberSequence => Variant::NumberSequence(
            LuaNumberSequence::from_value(value).ok_or_else(|| wrong("a NumberSequence"))?,
        ),
        // `supported` was checked before the match, so every remaining type has
        // an arm above. Written as unreachable rather than as a second copy of
        // the message, because two copies is how a predicate and its error drift
        // apart.
        other => unreachable!("supported() admitted {other:?} and coerce has no arm for it"),
    })
}

/// The declared property types this host can store today.
///
/// ONE PREDICATE, READ BY THREE THINGS: `coerce` refuses anything it rejects,
/// `accepts` reports on it, and `datamodel-surface` counts with it to generate
/// `docs/datamodel_scope.md`. A hand-kept list in the reporting tool says
/// whatever it was last edited to say, which is how that document came to report
/// Aether's numbers under the host's name.
pub fn supported(ty: VariantType) -> bool {
    matches!(
        ty,
        VariantType::Bool
            | VariantType::String
            | VariantType::Float32
            | VariantType::Float64
            | VariantType::Int32
            | VariantType::Int64
            | VariantType::UDim
            | VariantType::UDim2
            | VariantType::Vector2
            | VariantType::Color3
            | VariantType::Rect
            | VariantType::Content
            | VariantType::ContentId
            | VariantType::Font
            | VariantType::ColorSequence
            | VariantType::NumberSequence
    )
}

/// Would this host accept an assignment to `class.property` from a guest?
///
/// The measurement behind the number in `docs/datamodel_scope.md`. It answers
/// for the REAL code path: the property must exist on the class or an ancestor,
/// be writable, and carry a type `coerce` can store.
pub fn accepts(class: &str, property: &str) -> bool {
    // `Name` and `Parent` are handled ahead of the reflection lookup in
    // `NewIndex`, so they are accepted whatever their declared type says.
    if matches!(property, "Name" | "Parent") {
        return describe(class, property).is_some();
    }
    let Some(descriptor) = describe(class, property) else {
        return false;
    };
    if !matches!(
        descriptor.scriptability,
        Scriptability::ReadWrite | Scriptability::Write
    ) {
        return false;
    }
    match descriptor.data_type {
        DataType::Value(ty) => supported(ty),
        // Every enum in the reflection database is reachable, so a property typed
        // by one is accepted whatever the enum is.
        DataType::Enum(_) => true,
        _ => false,
    }
}

/// The value a property of this type reads before anything has set it.
///
/// THE REFLECTION DATABASE HAS NO DEFAULT FOR A COMPUTED PROPERTY.
/// `AbsolutePosition` and `AbsoluteSize` are results of layout, so nothing
/// serialises them and `default_properties` does not carry them -- which made a
/// guest reading one before the first layout get NIL, and then fail at
/// `.X` with a message about indexing nil rather than about layout.
///
/// The engine answers `(0, 0)` there. A zero is the honest reading of "nothing
/// has laid this out yet", and it is the same number the engine gives, so this
/// is parity rather than a convenience.
fn zero_for(ty: VariantType) -> Option<Variant> {
    Some(match ty {
        VariantType::Bool => Variant::Bool(false),
        VariantType::String => Variant::String(String::new()),
        VariantType::Float32 => Variant::Float32(0.0),
        VariantType::Float64 => Variant::Float64(0.0),
        VariantType::Int32 => Variant::Int32(0),
        VariantType::Int64 => Variant::Int64(0),
        VariantType::UDim => Variant::UDim(rbx_types::UDim::new(0.0, 0)),
        VariantType::UDim2 => Variant::UDim2(rbx_types::UDim2::new(
            rbx_types::UDim::new(0.0, 0),
            rbx_types::UDim::new(0.0, 0),
        )),
        VariantType::Vector2 => Variant::Vector2(rbx_types::Vector2::new(0.0, 0.0)),
        VariantType::Color3 => Variant::Color3(rbx_types::Color3::new(0.0, 0.0, 0.0)),
        VariantType::ColorSequence => Variant::ColorSequence(rbx_types::ColorSequence {
            keypoints: vec![
                rbx_types::ColorSequenceKeypoint::new(0.0, rbx_types::Color3::new(1.0, 1.0, 1.0)),
                rbx_types::ColorSequenceKeypoint::new(1.0, rbx_types::Color3::new(1.0, 1.0, 1.0)),
            ],
        }),
        VariantType::NumberSequence => Variant::NumberSequence(rbx_types::NumberSequence {
            keypoints: vec![
                rbx_types::NumberSequenceKeypoint::new(0.0, 0.0, 0.0),
                rbx_types::NumberSequenceKeypoint::new(1.0, 0.0, 0.0),
            ],
        }),
        VariantType::Content => Variant::Content(rbx_types::Content::none()),
        VariantType::ContentId => Variant::ContentId(String::new().into()),
        // Anything else keeps the old behaviour of reading nil. Inventing a value
        // for a type this host cannot present would be worse than the nil.
        _ => return None,
    })
}

/// Turn a Lua value into an enum member of `ty`.
///
/// ACCEPTS A NUMBER AS WELL AS AN `EnumItem`, because the engine does and a guest
/// written against it may pass either. A number is checked against the enum's own
/// members rather than stored blindly: enum values are not a contiguous range, and
/// storing an unnamed one produces a property that reads back as nothing.
fn coerce_enum(
    value: &LuaValue,
    ty: &str,
    class: &str,
    property: &str,
) -> LuaResult<rbx_types::Enum> {
    if let LuaValue::UserData(ud) = value {
        if let Ok(item) = ud.borrow::<LuaEnumItem>() {
            // THE ENUM TYPE IS CHECKED, not just the shape. `Enum.FillDirection`
            // and `Enum.AutomaticSize` both have a member numbered 1, so a
            // mismatched item would otherwise be stored as a plausible wrong
            // answer rather than refused.
            if item.ty != ty {
                return Err(LuaError::runtime(format!(
                    "{class}.{property} expects an Enum.{ty}, got an Enum.{}",
                    item.ty
                )));
            }
            return Ok(rbx_types::Enum::from_u32(item.value));
        }
    }
    if let Some(n) = number(value) {
        let raw = whole_i32(n, &format!("{class}.{property}"))?;
        let raw = u32::try_from(raw).map_err(|_| {
            LuaError::runtime(format!(
                "{class}.{property} has no Enum.{ty} numbered {raw}"
            ))
        })?;
        if !enums::value_is_valid(ty, raw) {
            return Err(LuaError::runtime(format!(
                "{class}.{property} has no Enum.{ty} numbered {raw}"
            )));
        }
        return Ok(rbx_types::Enum::from_u32(raw));
    }
    Err(LuaError::runtime(format!(
        "{class}.{property} expects an Enum.{ty}, got {}",
        value.type_name()
    )))
}

/// Coerce a Lua value into a `Variant` for an attribute.
pub(crate) fn coerce_attribute_value(value: &LuaValue) -> LuaResult<Option<Variant>> {
    match value {
        LuaValue::Nil => Ok(None),
        LuaValue::Boolean(b) => Ok(Some(Variant::Bool(*b))),
        LuaValue::String(s) => Ok(Some(Variant::String(s.to_string_lossy()))),
        LuaValue::Integer(i) => Ok(Some(Variant::Float64(*i as f64))),
        LuaValue::Number(n) => Ok(Some(Variant::Float64(*n))),
        LuaValue::UserData(_) => {
            if let Some(v) = LuaUDim::from_value(value) {
                return Ok(Some(Variant::UDim(v)));
            }
            if let Some(v) = LuaUDim2::from_value(value) {
                return Ok(Some(Variant::UDim2(v)));
            }
            if let Some(v) = LuaVector2::from_value(value) {
                return Ok(Some(Variant::Vector2(v)));
            }
            if let Some(v) = LuaColor3::from_value(value) {
                return Ok(Some(Variant::Color3(v)));
            }
            if let Some(v) = LuaRect::from_value(value) {
                return Ok(Some(Variant::Rect(v)));
            }
            if let Some(v) = LuaFont::from_value(value) {
                return Ok(Some(Variant::Font(v)));
            }
            Err(LuaError::runtime(format!(
                "SetAttribute: unsupported UserData type {}",
                value.type_name()
            )))
        }
        _ => Err(LuaError::runtime(format!(
            "SetAttribute: unsupported attribute type {}",
            value.type_name()
        ))),
    }
}

/// Turn a stored `Variant` back into something a guest can read.
pub(crate) fn to_lua(lua: &Lua, value: &Variant, enum_type: Option<&str>) -> LuaResult<LuaValue> {
    Ok(match value {
        // `Variant::Enum` is a bare `u32` with no record of which enum it belongs
        // to, so naming it needs the property's declared type. Without that a
        // guest reads a number where the engine hands back an EnumItem.
        Variant::Enum(raw) => {
            let ty = enum_type.ok_or_else(|| {
                LuaError::runtime("an enum value was stored without its declared type")
            })?;
            match enums::item_by_value(ty, raw.to_u32()) {
                Some(item) => item.into_lua(lua)?,
                None => {
                    return Err(LuaError::runtime(format!(
                        "Enum.{ty} has no member numbered {}",
                        raw.to_u32()
                    )))
                }
            }
        }
        Variant::Bool(v) => LuaValue::Boolean(*v),
        Variant::String(v) => lua.create_string(v)?.into_lua(lua)?,
        Variant::Float32(v) => LuaValue::Number(*v as f64),
        Variant::Float64(v) => LuaValue::Number(*v),
        Variant::Int32(v) => LuaValue::Integer(*v as i64),
        Variant::Int64(v) => LuaValue::Integer(*v),
        Variant::UDim(v) => LuaUDim(*v).into_lua(lua)?,
        Variant::UDim2(v) => LuaUDim2(*v).into_lua(lua)?,
        Variant::Vector2(v) => LuaVector2(*v).into_lua(lua)?,
        Variant::Color3(v) => LuaColor3(*v).into_lua(lua)?,
        Variant::Rect(v) => LuaRect(*v).into_lua(lua)?,
        Variant::Content(v) => LuaContent(v.clone()).into_lua(lua)?,
        Variant::ContentId(v) => lua.create_string(v.as_str())?.into_lua(lua)?,
        Variant::Font(v) => LuaFont(v.clone()).into_lua(lua)?,
        Variant::ColorSequence(v) => LuaColorSequence(v.clone()).into_lua(lua)?,
        Variant::NumberSequence(v) => LuaNumberSequence(v.clone()).into_lua(lua)?,
        // The engine stores some colours as bytes. A guest reads a Color3 either
        // way; presenting two Luau types for one engine concept would make
        // `typeof` answer differently depending on which property was read.
        Variant::Color3uint8(v) => LuaColor3(rbx_types::Color3::from(*v)).into_lua(lua)?,
        // A property whose stored type this slice cannot present. Reading it is
        // not an error the way writing it is -- the value exists and is correct,
        // this host simply has no representation for it yet -- but returning nil
        // silently would make a real value look absent, so it is refused loudly.
        other => {
            return Err(LuaError::runtime(format!(
                "this host cannot yet present a {:?} to Luau",
                other.ty()
            )))
        }
    })
}

impl UserData for InstanceRef {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        // `typeof(instance) == "Instance"` IS THE CONFORMANCE TEST, not a nicety.
        // Aether picks its host with `the engineHost.available()`, which is exactly
        // `typeof(game) == "Instance"` -- so a faithful DataModel here means
        // Aether's existing host runs on Dew unmodified, with no Dew branch and
        // no adapter.
        //
        // A FIELD, NOT A METHOD. Luau's `typeof` reads `__type` as a STRING off
        // the metatable; registering a function there leaves it unread. And mlua
        // fills `__type` in by default with the Rust type name, so without this
        // the answer is not "wrong" in a way that fails loudly -- it is the string
        // "InstanceRef", which every guest written for the engine silently takes
        // the non-engine branch on. Verified by a test that asserts the value
        // rather than asserting it is not nil.
        fields.add_meta_field(MetaMethod::Type, "Instance");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            let dom = this.dom.lock().expect("dom");
            Ok(dom
                .node(this.id)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| "<destroyed>".into()))
        });

        // TAKES A `LuaValue`, not an `InstanceRef`. A typed parameter would make
        // `frame == 3` a type error raised from inside `==`, and comparing an
        // instance with a number is false in Luau rather than a mistake.
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            let LuaValue::UserData(ud) = other else {
                return Ok(false);
            };
            let Ok(other) = ud.borrow::<InstanceRef>() else {
                return Ok(false);
            };
            Ok(Arc::ptr_eq(&this.dom, &other.dom) && this.id == other.id)
        });

        methods.add_meta_method(MetaMethod::Index, |lua, this, key: String| {
            let dom = this.dom.lock().expect("dom");
            let node = dom.node(this.id).ok_or_else(dead_instance)?;

            match key.as_str() {
                "ClassName" => return lua.create_string(&node.class)?.into_lua(lua),
                "Name" => return lua.create_string(&node.name)?.into_lua(lua),
                "Parent" => {
                    // THROUGH THE CACHE, so `child.Parent` read twice is one
                    // object. See `handle` for why that has to be true.
                    //
                    // AND THE LOCK GOES FIRST, exactly as the child-name arm
                    // below does it: `handle` takes the same `Mutex` to prune
                    // destroyed ids, and this one is not reentrant. Reading the
                    // id out and dropping the guard is the whole of it.
                    let parent = node.parent;
                    drop(dom);
                    return match parent {
                        Some(parent) => handle(lua, &this.dom, parent)?.into_lua(lua),
                        None => Ok(LuaValue::Nil),
                    };
                }
                _ => {}
            }

            // METHODS BEFORE PROPERTIES, and the lock is released first because
            // `members::lookup` builds a closure that will take it again when the
            // guest calls the function. `Mutex` is not reentrant; holding it
            // across the lookup deadlocked on the first `GetChildren`.
            //
            // BEFORE THE "not a valid member" ERROR, since a method is not in the
            // reflection database at all -- that crate carries properties only,
            // which is why the member surface needed its own predicate.
            let class = node.class.clone();
            drop(dom);
            if let Some(method) = members::lookup(lua, this, &class, &key)? {
                return Ok(method);
            }
            let dom = this.dom.lock().expect("dom");
            let node = dom.node(this.id).ok_or_else(dead_instance)?;

            // THE DESCRIPTOR IS LOOKED UP FIRST, EVEN FOR A STORED VALUE, because
            // an enum cannot name itself. `Variant::Enum` is a bare number, so
            // presenting one needs the declared type from the reflection database
            // whether the value was assigned or defaulted.
            //
            // NOT FOUND IS AN ERROR, NOT NIL. Reading a misspelled property on
            // The engine is an error, and a host that answered nil would let a
            // typo travel silently into layout -- the exact failure the
            // reflection database was brought in to prevent.
            let Some(descriptor) = describe(&node.class, &key) else {
                // `parent.ChildName`, AND IT GOES HERE RATHER THAN EARLIER.
                // A property SHADOWS a child on the engine -- a `Frame` called
                // `Size` does not take `frame.Size` away from you -- so the
                // reflection database is asked first and this is the fallback,
                // not the other way round.
                //
                // NOT FOUND IS STILL AN ERROR, and that is the whole delicacy of
                // this arm. `FindFirstChild` answers nil because "does a child
                // called this exist" is a question with an honest negative;
                // `parent.Panel` is a guest saying the child IS there, exactly
                // as `frame.Sze` is a guest saying the property is. One message
                // serves both because on the engine one message serves both --
                // and a misspelled property that fell through to here finds no
                // child either, so it still errors rather than reading nil.
                let child = dom
                    .children(this.id)
                    .into_iter()
                    .find(|c| dom.name_of(*c).as_deref() == Some(key.as_str()));
                if let Some(child) = child {
                    // THE LOCK GOES FIRST, for the reason the method arm above
                    // gives: `handle` is cheap, but every value handed to a guest
                    // is something the guest may immediately call back into, and
                    // this `Mutex` is not reentrant.
                    drop(dom);
                    return handle(lua, &this.dom, child)?.into_lua(lua);
                }
                return Err(LuaError::runtime(format!(
                    "{} is not a valid member of {}",
                    key, node.class
                )));
            };
            let enum_type = match &descriptor.data_type {
                DataType::Enum(name) => Some(*name),
                _ => None,
            };

            if let Some(stored) = node.props.get(&key) {
                return to_lua(lua, stored, enum_type);
            }

            if let Some(value) = default_for(&node.class, &key) {
                return to_lua(lua, &value, enum_type);
            }
            match &descriptor.data_type {
                DataType::Value(ty) => match zero_for(*ty) {
                    Some(value) => to_lua(lua, &value, enum_type),
                    None => Ok(LuaValue::Nil),
                },
                _ => Ok(LuaValue::Nil),
            }
        });

        // EVERY WRITE PATH BELOW ENDS THE SAME WAY: mutate, DROP THE LOCK, fire.
        //
        // The arena is behind a `Mutex` and `Mutex` is not reentrant, so a
        // `Changed` handler that sets another property -- which is the ordinary
        // thing for one to do, not an exotic one -- deadlocks the instant it is
        // called with the lock still held. The `drop(dom)` lines below are load
        // bearing and there are tests for both shapes of handler that would hang
        // without them.
        methods.add_meta_method(
            MetaMethod::NewIndex,
            |lua, this, (key, value): (String, LuaValue)| {
                let mut dom = this.dom.lock().expect("dom");
                let class = dom.node(this.id).ok_or_else(dead_instance)?.class.clone();

                match key.as_str() {
                    "ClassName" => {
                        return Err(LuaError::runtime(
                            "ClassName is read-only; it is what the instance IS",
                        ))
                    }
                    "Name" => {
                        let name = value
                            .as_string()
                            .ok_or_else(|| {
                                LuaError::runtime(format!(
                                    "Name expects a string, got {}",
                                    value.type_name()
                                ))
                            })?
                            .to_string_lossy();
                        // ASSIGNING THE SAME VALUE IS NOT A CHANGE. See the
                        // comment on the property path below; the rule is one
                        // rule and it applies to `Name` first because a mod that
                        // rewrites a label every frame writes the same string
                        // most of those frames.
                        if dom.node(this.id).expect("checked").name == name {
                            return Ok(());
                        }
                        dom.node_mut(this.id).expect("checked").name = name;
                        dom.touch();
                        drop(dom);
                        return signal::property_changed(lua, &this.dom, this.id, "Name");
                    }
                    "Parent" => {
                        let new_parent = match &value {
                            LuaValue::Nil => None,
                            LuaValue::UserData(ud) => Some(ud.borrow::<InstanceRef>()?.id),
                            other => {
                                return Err(LuaError::runtime(format!(
                                    "Parent expects an Instance or nil, got {}",
                                    other.type_name()
                                )))
                            }
                        };
                        if let Some(parent) = new_parent {
                            // A CYCLE IS REFUSED RATHER THAN BUILT. The engine raises
                            // here too, and an arena would happily store one and
                            // hang the first traversal that walked it.
                            if dom.is_ancestor_of(this.id, parent) {
                                return Err(LuaError::runtime(
                                    "that would make an instance its own ancestor",
                                ));
                            }
                            if dom.node(parent).is_none() {
                                return Err(LuaError::runtime("the new Parent has been destroyed"));
                            }
                        }
                        if dom.node(this.id).expect("checked").parent == new_parent {
                            return Ok(());
                        }
                        // THE DEPARTURE IS ANNOUNCED BEFORE IT HAPPENS, so a
                        // `ChildRemoved` or `DescendantRemoving` handler can read
                        // the instance one last time -- which is the only thing
                        // those events are for. That means the lock is released
                        // and taken again either side of the notice, and the
                        // guards `leaving` carries exist because a handler may
                        // have moved or destroyed something in between.
                        drop(dom);
                        signal::leaving(lua, &this.dom, this.id)?;
                        let mut dom = this.dom.lock().expect("dom");
                        if dom.node(this.id).is_none() {
                            return Err(dead_instance());
                        }
                        if new_parent.is_some_and(|p| dom.node(p).is_none()) {
                            return Err(LuaError::runtime("the new Parent has been destroyed"));
                        }
                        dom.unparent(this.id);
                        if let Some(parent) = new_parent {
                            dom.node_mut(parent)
                                .expect("checked")
                                .children
                                .push(this.id);
                            dom.node_mut(this.id).expect("checked").parent = Some(parent);
                        }
                        dom.touch();
                        drop(dom);
                        signal::arrived(lua, &this.dom, this.id)?;
                        return signal::property_changed(lua, &this.dom, this.id, "Parent");
                    }
                    _ => {}
                }

                let Some(descriptor) = describe(&class, &key) else {
                    return Err(LuaError::runtime(format!(
                        "{key} is not a valid member of {class}"
                    )));
                };

                // READ-ONLY IS REFUSED. `AbsolutePosition` is a result of layout,
                // and a host that accepted an assignment to it would store a value
                // the next solve overwrites -- which reads as a mystery rather
                // than as a mistake.
                if !matches!(
                    descriptor.scriptability,
                    Scriptability::ReadWrite | Scriptability::Write
                ) {
                    return Err(LuaError::runtime(format!("{class}.{key} is read-only")));
                }

                let want = match &descriptor.data_type {
                    DataType::Value(ty) => *ty,
                    DataType::Enum(name) => {
                        let stored = coerce_enum(&value, name, &class, &key)?;
                        if dom.property(this.id, &key) == Some(Variant::Enum(stored)) {
                            return Ok(());
                        }
                        dom.node_mut(this.id)
                            .expect("checked")
                            .props
                            .insert(key.clone(), Variant::Enum(stored));
                        dom.touch();
                        drop(dom);
                        return signal::property_changed(lua, &this.dom, this.id, &key);
                    }
                    other => {
                        return Err(LuaError::runtime(format!(
                            "{class}.{key} has a data type this host does not understand: \
                             {other:?}"
                        )))
                    }
                };

                let stored = coerce(&value, want, &class, &key)?;

                // ASSIGNING THE SAME VALUE FIRES NOTHING, and this is a decision
                // rather than an optimisation that fell out.
                //
                // A guest recomputing a whole tree on every tick -- which is what
                // an engine developer writes, and what Aether's own reconciler
                // does -- assigns the value a property already has far more often
                // than it assigns a new one. Firing on those turns `Changed` into
                // a tick, and a `Changed` handler that writes another property is
                // then a repaint loop that never settles: exactly the failure the
                // dirty flag below is meant to end. The other way round -- firing
                // only sometimes on a real change -- produces a UI that misses
                // updates, and both are miserable to attribute after the fact.
                //
                // COMPARED AGAINST THE EFFECTIVE VALUE, not the stored one. An
                // unset property reads its engine default, so writing `true` to a
                // `Visible` nothing has touched is not a change either, and
                // comparing against the (absent) stored value would say it was.
                if dom.property(this.id, &key).as_ref() == Some(&stored) {
                    return Ok(());
                }

                dom.node_mut(this.id)
                    .expect("checked")
                    .props
                    .insert(key.clone(), stored);
                dom.touch();
                drop(dom);
                signal::property_changed(lua, &this.dom, this.id, &key)
            },
        );
    }
}

/// Where the one-userdata-per-instance table lives in a VM's registry.
const HANDLES: &str = "dew.datamodel.handles";

/// The guest-facing handle onto an instance, and the SAME one every time.
///
/// # Why this is a cache and not a constructor
///
/// It was a constructor: every read of `Parent`, every `GetChildren`, every
/// `FindFirstChild` built a fresh userdata around the same arena id. `__eq`
/// made `a == b` answer true, so nothing looked wrong -- and Luau indexes a
/// table by RAW identity, never through `__eq`, so `t[a]` and `t[b]` were two
/// different keys.
///
/// In the engine an Instance is a reference with one identity, and a guest
/// memoising anything per node relies on that. Aether does, in the place where
/// it costs the most: `Live.luau` assigns each node a stable display id through
/// a weak-keyed table, so a fresh handle per frame meant every node was a NEW
/// node every frame, the delta reported the entire tree as changed, and a static
/// widget repainted at the full frame rate. Measured at `painted 827` against
/// `827 fps` on an idle calculator -- the exact shape of regression `--stats`
/// exists to catch, and it was invisible to all 268 tests and to the eye.
///
/// Nothing about that is Aether's mistake to fix. `t[instance] = x` is ordinary
/// Luau against a DataModel, and a host on which it silently fails to memoise is
/// a host the standard's own pass-or-fail is about.
///
/// # Strong, and pruned rather than weak
///
/// Weak values would collect a handle nothing else holds, and what holds these
/// BETWEEN frames is a weak table on the guest's side -- so the identity would
/// survive a garbage collection only by luck, which is worse than not surviving
/// it at all: a repaint storm that appears under memory pressure is one nobody
/// reproduces. The entry lives as long as the instance instead, and `destroy`
/// records the ids to forget.
///
/// ONE DOM PER VM, which is what lets this key by id alone. `install` is called
/// once per guest, `mods.rs` builds one `Dom` per mod, and two guests sharing a
/// tree is the thing that isolation forbids in the first place.
pub fn handle(lua: &Lua, dom: &SharedDom, id: usize) -> LuaResult<LuaAnyUserData> {
    let cache: LuaTable = match lua.named_registry_value::<LuaValue>(HANDLES)? {
        LuaValue::Table(existing) => existing,
        _ => {
            let fresh = lua.create_table()?;
            lua.set_named_registry_value(HANDLES, &fresh)?;
            fresh
        }
    };

    // Destroyed nodes first, so a handle is never handed out for a slot the
    // arena has emptied and then re-answered from the cache.
    // NIL THE KEY, NEVER `raw_remove`. The ids are integers, so `raw_remove`
    // takes the `table.remove` path and SHIFTS every entry above it down one --
    // which does not empty a slot, it renumbers the whole cache, and the next
    // `Instance.new` is answered with somebody else's handle. It presented as
    // `available()` passing in `Host.detect` and failing one call later inside
    // `DataModel.new`, which is a fine description of a cache that answers a
    // question with the previous question's answer.
    let released = std::mem::take(&mut dom.lock().expect("dom").released);
    for gone in released {
        cache.raw_set(gone, LuaValue::Nil)?;
    }

    if let Ok(LuaValue::UserData(existing)) = cache.raw_get::<LuaValue>(id) {
        return Ok(existing);
    }
    let made = lua.create_userdata(InstanceRef {
        dom: dom.clone(),
        id,
    })?;
    cache.raw_set(id, &made)?;
    Ok(made)
}

/// Install `Instance` into a guest VM.
///
/// A GLOBAL, unlike `dew`. The capability table is passed as an argument because
/// what a mod may DO is per-mod and must be absent when not granted. The
/// DataModel is not a capability: it is the language of the platform, present for
/// every guest on both hosts, and an application that had to be handed it would
/// not be the same application that runs on the engine.
/// Install the value vocabulary: `UDim`, `UDim2`, `Vector2`, `Color3`, `Rect`,
/// `Font`, the two sequence types and their keypoints, `Enum`, and `Content`.
///
/// SEPARATE FROM `install`, AND CALLED FOR BOTH RUNTIMES SINCE SPRINT 6. It was
/// called for a DataModel mod only, and the reason is worth keeping because it is
/// the shape of the trap rather than a fact about a past release: Aether carries
/// its own vocabulary for off-engine hosts and publishes it with
/// `if rawget(g, name) == nil` -- FIRST WRITER WINS. So a partial host vocabulary
/// does not merge with Aether's, it BLOCKS it. Installing five types into an
/// Aether mod VM took `Color3.fromHex` away and all three mods stopped loading,
/// whichever order the two ran in.
///
/// TWO THINGS CHANGED, AND ONLY TOGETHER ARE THEY ENOUGH.
///
/// The gap closed. `vocabulary.rs` now answers every one of the eleven names
/// Aether's `REQUIRED_VOCABULARY` requires, `fromHex` and `fromHSV` included, so
/// there is no member for a first writer to take away.
///
/// And the second writer left. Under Aether's DataModel host `InstallVocabulary`
/// is `function() end`, which is what a host says when the environment already
/// supplies the vocabulary -- it is the same line on the engine and for the same
/// reason. Nothing publishes a second vocabulary, so nothing races.
///
/// THIS IS NOT OPTIONAL FOR AN AETHER MOD, WHICH IS THE PART WORTH KNOWING. The
/// DataModel host's `available()` probe asks the environment for `UDim`, `UDim2`,
/// `Vector2`, `Color3`, `Rect` and `Enum` before it will consent to drive
/// anything, and refuses a host that lacks one. So on a VM where this was not
/// called, Aether does not fall back to a lesser vocabulary -- it selects the
/// Luau test double instead, silently, and draws a correct-looking widget through
/// it. A missing name here does not surface here.
///
/// The prediction this replaces was that closing the gap would mean substituting
/// Aether's Luau tables for this host's userdata inside its own `create`, and
/// that turned out to be a thing that did not need doing: on the DataModel host
/// `create` is vide's, `create` writes properties onto real instances, and this
/// host's instances take this host's userdata. There was no substitution because
/// there were never two vocabularies on that path -- only on the test double's.
pub fn install_vocabulary(lua: &Lua) -> LuaResult<()> {
    vocabulary::install(lua)?;
    enums::install(lua)?;
    content::install(lua)
}

pub fn install(lua: &Lua, dom: &SharedDom) -> LuaResult<()> {
    let instance = lua.create_table()?;
    let shared = dom.clone();
    instance.set(
        "new",
        lua.create_function(move |lua, class: String| {
            if !class_exists(&class) {
                return Err(LuaError::runtime(format!(
                    "{class} is not a valid class name"
                )));
            }
            let id = shared
                .lock()
                .expect("dom")
                .insert(class.clone(), class.clone());
            handle(lua, &shared, id)
        })?,
    )?;
    lua.globals().set("Instance", instance)?;

    // `services`, NOT `game`. See `service_provider`'s own module comment for
    // why: it is the real `ServiceProvider` the engine's `game:GetService`
    // already inherits from, exposed without the `DataModel` tree root
    // wrapped around it. A script portable to the real engine reaches the
    // same service through `(game or services):GetService(name)`.
    let collection_service = lua.create_userdata(
        collection_service::CollectionServiceHandle::new(dom.clone()),
    )?;
    let services = lua.create_userdata(service_provider::ServiceProviderHandle::new(
        collection_service,
    ))?;
    lua.globals().set("services", services)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm() -> (Lua, SharedDom) {
        let lua = Lua::new();
        let dom: SharedDom = Arc::new(Mutex::new(Dom::default()));
        install(&lua, &dom).expect("install");
        install_vocabulary(&lua).expect("vocabulary");
        (lua, dom)
    }

    fn run(src: &str) -> LuaResult<()> {
        let (lua, _dom) = vm();
        lua.load(src).exec()
    }

    fn eval<T: FromLuaMulti>(src: &str) -> LuaResult<T> {
        let (lua, _dom) = vm();
        lua.load(src).eval()
    }

    #[test]
    fn typeof_an_instance_is_instance() {
        // The whole conformance test in one assertion: Aether picks its host with
        // `typeof(game) == "Instance"`, so this is what lets its existing host
        // run on Dew with no Dew-specific branch.
        let got: String = eval(r#"return typeof(Instance.new("Frame"))"#).expect("eval");
        assert_eq!(got, "Instance");
    }

    #[test]
    fn a_new_instance_is_named_for_its_class() {
        let got: String = eval(r#"return Instance.new("Frame").Name"#).expect("eval");
        assert_eq!(got, "Frame");
    }

    #[test]
    fn an_unknown_class_is_refused() {
        let err = run(r#"Instance.new("Frmae")"#).unwrap_err().to_string();
        assert!(err.contains("Frmae"), "{err}");
        assert!(err.contains("not a valid class name"), "{err}");
    }

    #[test]
    fn a_misspelled_property_is_an_error_rather_than_nil() {
        // The reason the reflection database was brought in. Answering nil lets a
        // typo travel silently into layout.
        let err = run(r#"Instance.new("Frame").Visibel = true"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Visibel"), "{err}");
        assert!(err.contains("Frame"), "{err}");

        let err = run(r#"local _ = Instance.new("Frame").Visibel"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a valid member"), "{err}");
    }

    #[test]
    fn a_child_is_reachable_by_name() {
        let got: Vec<bool> = eval(
            r#"
            local root = Instance.new("Frame")
            local panel = Instance.new("Frame")
            panel.Name = "Panel"
            panel.Parent = root
            return { root.Panel == panel, root.Panel.Name == "Panel" }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, true]);
    }

    #[test]
    fn a_property_shadows_a_child_of_the_same_name() {
        // The ordering, asserted rather than assumed. A child called `Visible`
        // must not take `frame.Visible` away, because on the engine it does not.
        let got: bool = eval(
            r#"
            local root = Instance.new("Frame")
            local decoy = Instance.new("Frame")
            decoy.Name = "Visible"
            decoy.Parent = root
            root.Visible = false
            return root.Visible == false
        "#,
        )
        .expect("eval");
        assert!(got);
    }

    #[test]
    fn a_misspelled_property_still_errors_beside_a_child_lookup() {
        // THE TRAP THIS SPRINT WAS WARNED ABOUT. A host that answered nil for a
        // missing child would have to answer nil for a misspelled property too,
        // since `__index` cannot tell which one the guest meant. Both arrive
        // here and both must still be an error.
        let err = run(r#"
            local root = Instance.new("Frame")
            local panel = Instance.new("Frame")
            panel.Name = "Panel"
            panel.Parent = root
            local _ = root.Visibel
        "#)
        .unwrap_err()
        .to_string();
        assert!(err.contains("Visibel"), "{err}");
        assert!(err.contains("not a valid member"), "{err}");
    }

    #[test]
    fn a_missing_child_is_an_error_but_find_first_child_is_nil() {
        // The two spellings answer differently ON PURPOSE, and this is the test
        // that says so. `FindFirstChild` asks whether a child exists; `.Panel`
        // asserts that it does.
        let err = run(r#"local _ = Instance.new("Frame").Panel"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a valid member"), "{err}");

        let got: bool =
            eval(r#"return Instance.new("Frame"):FindFirstChild("Panel") == nil"#).expect("eval");
        assert!(got);
    }

    #[test]
    fn a_reparented_child_stops_answering_by_name() {
        let got: Vec<bool> = eval(
            r#"
            local root = Instance.new("Frame")
            local panel = Instance.new("Frame")
            panel.Name = "Panel"
            panel.Parent = root
            local before = root.Panel == panel
            panel.Parent = nil
            local after = pcall(function() return root.Panel end)
            return { before, after }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, false]);
    }

    #[test]
    fn a_method_still_wins_over_a_child_of_the_same_name() {
        // Methods resolve before properties and therefore before children. A mod
        // that names a child `Destroy` must not disarm `Destroy`.
        let got: bool = eval(
            r#"
            local root = Instance.new("Frame")
            local decoy = Instance.new("Frame")
            decoy.Name = "GetChildren"
            decoy.Parent = root
            return type(root.GetChildren) == "function"
        "#,
        )
        .expect("eval");
        assert!(got);
    }

    #[test]
    fn a_property_of_the_wrong_type_is_refused() {
        let err = run(r#"Instance.new("Frame").Visible = "yes""#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("expects a boolean"), "{err}");
    }

    #[test]
    fn a_primitive_property_round_trips() {
        let got: bool = eval(
            r#"
            local f = Instance.new("Frame")
            f.Visible = false
            return f.Visible
        "#,
        )
        .expect("eval");
        assert!(!got);
    }

    #[test]
    fn an_unset_property_reads_its_engine_default() {
        // Not nil, and not a guess: whatever `Instance.new` produces on the
        // engine, from the same database the scope document is generated from.
        let got: bool = eval(r#"return Instance.new("Frame").Visible"#).expect("eval");
        assert!(got);
    }

    #[test]
    fn class_name_is_read_only() {
        let err = run(r#"Instance.new("Frame").ClassName = "TextLabel""#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("read-only"), "{err}");
    }

    #[test]
    fn a_read_only_property_is_refused() {
        let err = run(r#"Instance.new("Frame").AbsolutePosition = 1"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("read-only"), "{err}");
    }

    #[test]
    fn a_number_is_a_number_however_the_vm_holds_it() {
        // THE BUG THIS EXISTS FOR: `Value::as_f32` matches only `Value::Number`
        // and `as_i32` only `Value::Integer`, so an integer literal assigned to a
        // float property was refused, and every literal offset in a UDim2 came
        // back zero. Nothing caught it because no test set a numeric property.
        let got: f64 = eval(
            r#"
            local f = Instance.new("Frame")
            f.BackgroundTransparency = 1
            return f.BackgroundTransparency
        "#,
        )
        .expect("integer into a float property");
        assert_eq!(got, 1.0);

        let got: f64 = eval(
            r#"
            local f = Instance.new("Frame")
            f.BackgroundTransparency = 0.25
            return f.BackgroundTransparency
        "#,
        )
        .expect("fraction into a float property");
        assert_eq!(got, 0.25);

        let got: i64 = eval(
            r#"
            local f = Instance.new("Frame")
            f.ZIndex = 3
            return f.ZIndex
        "#,
        )
        .expect("integer into an int property");
        assert_eq!(got, 3);
    }

    #[test]
    fn a_fraction_in_an_integer_property_is_refused() {
        let err = run(r#"Instance.new("Frame").ZIndex = 3.5"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("whole number"), "{err}");
        assert!(err.contains("3.5"), "{err}");
    }

    #[test]
    fn a_udim2_property_round_trips_through_the_vocabulary() {
        let got: Vec<f64> = eval(
            r#"
            local f = Instance.new("Frame")
            f.Size = UDim2.new(0.5, 10, 0, 40)
            return { f.Size.X.Scale, f.Size.X.Offset, f.Size.Y.Scale, f.Size.Y.Offset }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![0.5, 10.0, 0.0, 40.0]);
    }

    #[test]
    fn a_colour_property_round_trips() {
        let got: f32 = eval(
            r#"
            local f = Instance.new("Frame")
            f.BackgroundColor3 = Color3.fromRGB(255, 128, 0)
            return f.BackgroundColor3.R
        "#,
        )
        .expect("eval");
        assert_eq!(got, 1.0);
    }

    #[test]
    fn an_enum_property_round_trips_as_an_enum_item() {
        let got: String = eval(
            r#"
            local f = Instance.new("Frame")
            f.AutomaticSize = Enum.AutomaticSize.Y
            return f.AutomaticSize.Name
        "#,
        )
        .expect("eval");
        assert_eq!(got, "Y");
    }

    #[test]
    fn an_unset_enum_property_reads_its_default_as_an_enum_item() {
        // Not a number. `Variant::Enum` is a bare u32, so this only works because
        // the read path resolves the declared type before presenting the value.
        let got: String =
            eval(r#"return typeof(Instance.new("Frame").AutomaticSize)"#).expect("eval");
        assert_eq!(got, "EnumItem");
    }

    #[test]
    fn a_member_of_the_wrong_enum_is_refused() {
        // Both are numbered 1 in their own enum, so a host that compared values
        // and not types would store this as a plausible wrong answer.
        let err = run(r#"Instance.new("Frame").AutomaticSize = Enum.FillDirection.Vertical"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("expects an Enum.AutomaticSize"), "{err}");
        assert!(err.contains("got an Enum.FillDirection"), "{err}");
    }

    #[test]
    fn a_number_is_accepted_for_an_enum_only_when_the_enum_has_it() {
        let got: String = eval(
            r#"
            local f = Instance.new("Frame")
            f.AutomaticSize = Enum.AutomaticSize.XY.Value
            return f.AutomaticSize.Name
        "#,
        )
        .expect("eval");
        assert_eq!(got, "XY");

        let err = run(r#"Instance.new("Frame").AutomaticSize = 99"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no Enum.AutomaticSize numbered 99"), "{err}");
    }

    #[test]
    fn a_content_property_round_trips() {
        let got: String = eval(
            r#"
            local i = Instance.new("ImageLabel")
            i.ImageContent = Content.fromUri("rbxassetid://12345")
            return i.ImageContent.Uri
        "#,
        )
        .expect("eval");
        assert_eq!(got, "rbxassetid://12345");
    }

    #[test]
    fn a_content_property_accepts_a_plain_string_like_the_engine() {
        let got: String = eval(
            r#"
            local i = Instance.new("ImageLabel")
            i.ImageContent = "rbxassetid://7"
            return i.ImageContent.Uri
        "#,
        )
        .expect("eval");
        assert_eq!(got, "rbxassetid://7");
    }

    #[test]
    fn the_legacy_content_id_property_stays_a_string() {
        // `Image` is a ContentId and `ImageContent` is a Content. The same asset
        // is named both ways depending on which generation the guest reaches for,
        // and each must read back as its own type.
        let got: String = eval(
            r#"
            local i = Instance.new("ImageLabel")
            i.Image = "rbxassetid://9"
            return typeof(i.Image) .. ":" .. i.Image
        "#,
        )
        .expect("eval");
        assert_eq!(got, "string:rbxassetid://9");
    }

    #[test]
    fn any_well_formed_uri_is_accepted_whatever_its_scheme() {
        // ADR-003: an unresolvable Content is a rendering outcome, not a property
        // error. An engine application moved here with a grant withheld is a
        // correct application missing an image, and refusing the assignment would
        // make it a broken one.
        for uri in [
            "rbxassetid://1",
            "dew://icon.png",
            "https://example.invalid/a.png",
        ] {
            let got: String = eval(&format!(
                r#"
                local i = Instance.new("ImageLabel")
                i.ImageContent = Content.fromUri("{uri}")
                return i.ImageContent.Uri
            "#
            ))
            .expect("eval");
            assert_eq!(got, uri);
        }
    }

    #[test]
    fn a_font_property_round_trips() {
        let got: (String, u32) = eval(
            r#"
            local t = Instance.new("TextLabel")
            t.FontFace = Font.new("Inter", Enum.FontWeight.Bold)
            return t.FontFace.Family, t.FontFace.Weight
        "#,
        )
        .expect("eval");
        assert_eq!(got.0, "Inter");
        assert_eq!(got.1, 700);
    }

    #[test]
    fn a_vocabulary_value_of_the_wrong_type_is_refused() {
        let err = run(r#"Instance.new("Frame").Size = Vector2.new(1, 2)"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("expects a UDim2"), "{err}");
    }

    #[test]
    fn parent_builds_a_tree_both_ways() {
        let (lua, dom) = vm();
        lua.load(
            r#"
            local parent = Instance.new("Frame")
            local child = Instance.new("Frame")
            child.Name = "Child"
            child.Parent = parent
            _G.parent, _G.child = parent, child
        "#,
        )
        .exec()
        .expect("exec");

        let d = dom.lock().expect("dom");
        assert_eq!(d.node(0).expect("parent").children, vec![1]);
        assert_eq!(d.node(1).expect("child").parent, Some(0));
        assert_eq!(d.node(1).expect("child").name, "Child");
    }

    #[test]
    fn reparenting_detaches_from_the_old_parent() {
        let (lua, dom) = vm();
        lua.load(
            r#"
            local a, b = Instance.new("Frame"), Instance.new("Frame")
            local child = Instance.new("Frame")
            child.Parent = a
            child.Parent = b
            child.Parent = nil
            _G.a = a
        "#,
        )
        .exec()
        .expect("exec");

        let d = dom.lock().expect("dom");
        assert!(d.node(0).expect("a").children.is_empty());
        assert!(d.node(1).expect("b").children.is_empty());
        assert_eq!(d.node(2).expect("child").parent, None);
    }

    #[test]
    fn a_parent_cycle_is_refused() {
        let err = run(r#"
            local a, b = Instance.new("Frame"), Instance.new("Frame")
            b.Parent = a
            a.Parent = b
        "#)
        .unwrap_err()
        .to_string();
        assert!(err.contains("its own ancestor"), "{err}");
    }

    #[test]
    fn an_instance_is_its_own_parent_never() {
        let err = run(r#"
            local a = Instance.new("Frame")
            a.Parent = a
        "#)
        .unwrap_err()
        .to_string();
        assert!(err.contains("its own ancestor"), "{err}");
    }

    #[test]
    fn two_handles_to_one_instance_compare_equal() {
        let got: bool = eval(
            r#"
            local parent = Instance.new("Frame")
            local child = Instance.new("Frame")
            child.Parent = parent
            return child.Parent == parent
        "#,
        )
        .expect("eval");
        assert!(got);
    }
    #[test]
    fn zz_probe_handle_identity() {
        let got: bool = eval(
            r#"
            local parent = Instance.new("Frame")
            local child = Instance.new("Frame")
            child.Parent = parent
            local a = parent:GetChildren()[1]
            local b = parent:GetChildren()[1]
            local t = {}
            t[a] = 1
            return rawequal(a, b) and t[b] == 1
        "#,
        )
        .expect("eval");
        assert!(got, "two reads of one instance are not the same table key");
    }

    #[test]
    fn uigradient_accepts_color_and_transparency_sequences() {
        let ok: bool = eval(
            r#"
            local g = Instance.new("UIGradient")
            g.Color = ColorSequence.new(Color3.new(1, 0, 0), Color3.new(0, 0, 1))
            g.Transparency = NumberSequence.new(0.25, 0.75)
            local cs = g.Color
            local ts = g.Transparency
            return typeof(cs) == "ColorSequence"
                and #cs.Keypoints == 2
                and cs.Keypoints[1].Value == Color3.new(1, 0, 0)
                and typeof(ts) == "NumberSequence"
                and #ts.Keypoints == 2
                and ts.Keypoints[1].Value == 0.25
        "#,
        )
        .expect("eval");
        assert!(ok);
    }

    #[test]
    fn input_action_label_accepts_input_action() {
        let got: String = eval(
            r#"
            local label = Instance.new("InputActionLabel")
            label.InputAction = "Interact"
            return label.InputAction
        "#,
        )
        .expect("eval");
        assert_eq!(got, "Interact");
    }

    /// PROVES THE ANCESTOR WALK, NOT JUST THE REGISTRY ROW. `BlendingMode` is
    /// declared on `GuiObject`, and nothing inherits `GuiObject` directly --
    /// `describe`/`default_for` must climb from the CONCRETE class a guest
    /// actually instantiates. Two unrelated descendants, not one, because a
    /// walk that happens to work for `Frame` alone would not prove the climb
    /// is real rather than a coincidence of `Frame` being the first class the
    /// reflection database's superclass chain reaches.
    #[test]
    fn blending_mode_resolves_through_two_different_guiobject_descendants() {
        // RESET BEFORE `expect`, NOT AFTER: the flag is thread-local and this
        // test's worker thread is reused by later tests, so a panic on the
        // `expect` below must not leave it enabled for whichever test runs
        // next on the same thread.
        extensions::set_enabled_flags(&[("FFlagDewGuiObjectBlendingMode", 1)]);
        let result: LuaResult<Vec<String>> = eval(
            r#"
            local frame = Instance.new("Frame")
            frame.BlendingMode = Enum.BlendMode.Additive
            local button = Instance.new("TextButton")
            button.BlendingMode = Enum.BlendMode.Multiply
            return { tostring(frame.BlendingMode), tostring(button.BlendingMode) }
        "#,
        );
        extensions::set_enabled_flags(&[]);
        assert_eq!(
            result.expect("eval"),
            vec!["Enum.BlendMode.Additive", "Enum.BlendMode.Multiply"]
        );
    }

    /// THE GATE ITSELF, on the same property. Unflagged, `BlendingMode` is not
    /// a member of `GuiObject` at all -- not merely stuck at its default.
    ///
    /// ASSIGNED A PLAIN NUMBER, NOT `Enum.BlendMode.Additive`: unflagged, the
    /// enum category itself does not exist either, and referencing it would
    /// fail on `Enum.BlendMode` before the property assignment this test is
    /// actually about ever ran.
    #[test]
    fn blending_mode_is_not_a_member_of_frame_without_its_flag() {
        extensions::set_enabled_flags(&[]);
        let err = run(r#"Instance.new("Frame").BlendingMode = 0"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("BlendingMode"), "{err}");
        assert!(err.contains("not a valid member"), "{err}");
    }
}
