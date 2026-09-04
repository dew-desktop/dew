//! The host-owned DataModel: instances, the tree, and property validation.
//!
//! WHY THIS EXISTS
//! Dew is its own product. A developer arriving from Roblox writes Luau against
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

mod content;
mod enums;
pub mod input;
pub mod members;
pub mod render;
pub mod signal;
mod vocabulary;

use content::{LuaContent, LuaFont};
use enums::LuaEnumItem;
use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use rbx_reflection::{DataType, PropertyDescriptor, Scriptability};
use rbx_types::{Variant, VariantType};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use vocabulary::{LuaColor3, LuaRect, LuaUDim, LuaUDim2, LuaVector2};

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
    /// DESCENDANTS GO TOO. Roblox destroys the subtree, and leaving children in
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

/// Look up a property on a class or any of its ancestors.
///
/// WALKS THE CHAIN because the reflection database declares a property on the
/// class that introduces it and does not repeat it on descendants. `Frame` has
/// no `Name` of its own; it has `Instance`'s.
fn describe(class: &str, property: &str) -> Option<&'static PropertyDescriptor<'static>> {
    let db = rbx_reflection_database::get().ok()?;
    let mut cursor = db.classes.get(class);
    while let Some(current) = cursor {
        if let Some(descriptor) = current.properties.get(property) {
            return Some(descriptor);
        }
        cursor = current.superclass.and_then(|s| db.classes.get(s));
    }
    None
}

fn class_exists(class: &str) -> bool {
    rbx_reflection_database::get()
        .map(|db| db.classes.contains_key(class))
        .unwrap_or(false)
}

/// The default a property reads before anything assigns it.
fn default_for(class: &str, property: &str) -> Option<Variant> {
    let db = rbx_reflection_database::get().ok()?;
    let mut cursor = db.classes.get(class);
    while let Some(current) = cursor {
        if let Some(value) = current.default_properties.get(property) {
            return Some(value.clone());
        }
        cursor = current.superclass.and_then(|s| db.classes.get(s));
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
/// `Offset`, `ZIndex` and `LayoutOrder` are integers in the engine. Truncating
/// 10.7 to 10 produces a UI that is subtly wrong everywhere and blames nobody, so
/// this asks instead.
pub(crate) fn whole_i32(value: f64, what: &str) -> LuaResult<i32> {
    if value.fract() != 0.0 {
        return Err(LuaError::runtime(format!(
            "{what} is a whole number; got {value}"
        )));
    }
    Ok(value as i32)
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

/// Turn a stored `Variant` back into something a guest can read.
fn to_lua(lua: &Lua, value: &Variant, enum_type: Option<&str>) -> LuaResult<LuaValue> {
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
        // Aether picks its host with `RobloxHost.available()`, which is exactly
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
                    return match node.parent {
                        Some(parent) => InstanceRef {
                            dom: this.dom.clone(),
                            id: parent,
                        }
                        .into_lua(lua),
                        None => Ok(LuaValue::Nil),
                    }
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
            // the engine is an error, and a host that answered nil would let a
            // typo travel silently into layout -- the exact failure the
            // reflection database was brought in to prevent.
            let Some(descriptor) = describe(&node.class, &key) else {
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
                            // A CYCLE IS REFUSED RATHER THAN BUILT. Roblox raises
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
                // a Roblox developer writes, and what Aether's own reconciler
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

/// A guest-facing handle onto an instance the HOST made.
///
/// The renderer needs a root to lay out against and a guest needs something to
/// parent into. Both want the same handle, and it is not `Instance.new`'s job to
/// hand out one for a node the host created.
pub fn handle(dom: &SharedDom, id: usize) -> InstanceRef {
    InstanceRef {
        dom: dom.clone(),
        id,
    }
}

/// Install `Instance` into a guest VM.
///
/// A GLOBAL, unlike `dew`. The capability table is passed as an argument because
/// what a mod may DO is per-mod and must be absent when not granted. The
/// DataModel is not a capability: it is the language of the platform, present for
/// every guest on both hosts, and an application that had to be handed it would
/// not be the same application that runs on Roblox.
/// Install `UDim2`, `Color3`, `Vector2`, `UDim` and `Rect`.
///
/// SEPARATE FROM `install`, AND NOT CALLED FOR AN AETHER MOD YET. Aether carries
/// its own vocabulary for off-engine hosts, and `Headless.InstallVocabulary`
/// publishes it with `if rawget(g, name) == nil` -- first writer wins. So a
/// partial host vocabulary does not merge with Aether's, it BLOCKS it: installing
/// these into a mod VM took `Color3.fromHex` away and all three mods stopped
/// loading, whichever order the two ran in.
///
/// Closing the gap is not a matter of adding `fromHex`. Aether's values are Luau
/// tables its own `create` consumes, and these are userdata; substituting one for
/// the other is the change where Aether starts consuming the host's vocabulary
/// instead of carrying its own, which is the same change that retires
/// `Headless.luau`.
///
/// Until then this is the language for a guest whose host IS the whole story --
/// the tests below, and `dew run` for a raw Luau application when it exists. On
/// Roblox `InstallVocabulary` is already a no-op because the engine provides the
/// vocabulary; on Dew it should be a no-op for the same reason, and that is the
/// end state this is waiting for rather than working around.
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
        lua.create_function(move |_, class: String| {
            if !class_exists(&class) {
                return Err(LuaError::runtime(format!(
                    "{class} is not a valid class name"
                )));
            }
            let id = shared
                .lock()
                .expect("dom")
                .insert(class.clone(), class.clone());
            Ok(InstanceRef {
                dom: shared.clone(),
                id,
            })
        })?,
    )?;
    lua.globals().set("Instance", instance)?;
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
    fn an_unimplemented_type_says_so_rather_than_rejecting_the_program() {
        // `UIGradient.Color` is a `ColorSequence`, which no slice has reached.
        // The message must not read as "your program is wrong".
        //
        // NAMED `Size`, THEN `FontFace`, NOW THIS. A test for "unsupported" has
        // to be repointed every time support arrives, which is the test doing its
        // job rather than failing at it. When nothing in scope is unsupported it
        // has no subject and should be deleted rather than weakened.
        let err = run(r#"Instance.new("UIGradient").Color = 1"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot accept yet"), "{err}");
        assert!(err.contains("The property is real"), "{err}");
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
        // error. A Roblox application moved here with a grant withheld is a
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
}
