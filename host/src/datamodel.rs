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

use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use rbx_reflection::{DataType, PropertyDescriptor, Scriptability};
use rbx_types::{Variant, VariantType};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

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
}

/// Every instance in one VM.
#[derive(Default)]
pub struct Dom {
    slots: Vec<Option<Node>>,
}

pub type SharedDom = Arc<Mutex<Dom>>;

impl Dom {
    fn insert(&mut self, class: String, name: String) -> usize {
        self.slots.push(Some(Node {
            class,
            name,
            parent: None,
            children: Vec::new(),
            props: BTreeMap::new(),
        }));
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
        VariantType::Float32 => Variant::Float32(value.as_f32().ok_or_else(|| wrong("a number"))?),
        VariantType::Float64 => Variant::Float64(value.as_f64().ok_or_else(|| wrong("a number"))?),
        VariantType::Int32 => Variant::Int32(value.as_i32().ok_or_else(|| wrong("an integer"))?),
        VariantType::Int64 => Variant::Int64(value.as_i64().ok_or_else(|| wrong("an integer"))?),
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
    matches!(descriptor.data_type, DataType::Value(ty) if supported(ty))
}

/// Turn a stored `Variant` back into something a guest can read.
fn to_lua(lua: &Lua, value: &Variant) -> LuaResult<LuaValue> {
    Ok(match value {
        Variant::Bool(v) => LuaValue::Boolean(*v),
        Variant::String(v) => lua.create_string(v)?.into_lua(lua)?,
        Variant::Float32(v) => LuaValue::Number(*v as f64),
        Variant::Float64(v) => LuaValue::Number(*v),
        Variant::Int32(v) => LuaValue::Integer(*v as i64),
        Variant::Int64(v) => LuaValue::Integer(*v),
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
            let node = dom
                .node(this.id)
                .ok_or_else(|| LuaError::runtime("this instance has been destroyed"))?;

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

            if let Some(stored) = node.props.get(&key) {
                return to_lua(lua, stored);
            }

            // NOT FOUND IS AN ERROR, NOT NIL. Reading a misspelled property on
            // the engine is an error, and a host that answered nil would let a
            // typo travel silently into layout -- which is the exact failure the
            // reflection database was brought in to prevent.
            let Some(descriptor) = describe(&node.class, &key) else {
                return Err(LuaError::runtime(format!(
                    "{} is not a valid member of {}",
                    key, node.class
                )));
            };
            let _ = descriptor;

            match default_for(&node.class, &key) {
                Some(value) => to_lua(lua, &value),
                None => Ok(LuaValue::Nil),
            }
        });

        methods.add_meta_method(
            MetaMethod::NewIndex,
            |_, this, (key, value): (String, LuaValue)| {
                let mut dom = this.dom.lock().expect("dom");
                let class = dom
                    .node(this.id)
                    .ok_or_else(|| LuaError::runtime("this instance has been destroyed"))?
                    .class
                    .clone();

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
                        dom.node_mut(this.id).expect("checked").name = name;
                        return Ok(());
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
                        dom.unparent(this.id);
                        if let Some(parent) = new_parent {
                            dom.node_mut(parent)
                                .expect("checked")
                                .children
                                .push(this.id);
                            dom.node_mut(this.id).expect("checked").parent = Some(parent);
                        }
                        return Ok(());
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
                        return Err(LuaError::runtime(format!(
                            "{class}.{key} is the enum {name}, which this host cannot accept \
                             yet. The property is real; Dew's DataModel implements enums in a \
                             later slice. See docs/datamodel_scope.md."
                        )))
                    }
                    other => {
                        return Err(LuaError::runtime(format!(
                            "{class}.{key} has a data type this host does not understand: \
                             {other:?}"
                        )))
                    }
                };

                let stored = coerce(&value, want, &class, &key)?;
                dom.node_mut(this.id)
                    .expect("checked")
                    .props
                    .insert(key, stored);
                Ok(())
            },
        );
    }
}

/// Install `Instance` into a guest VM.
///
/// A GLOBAL, unlike `dew`. The capability table is passed as an argument because
/// what a mod may DO is per-mod and must be absent when not granted. The
/// DataModel is not a capability: it is the language of the platform, present for
/// every guest on both hosts, and an application that had to be handed it would
/// not be the same application that runs on Roblox.
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
        // `Size` is a UDim2 and this slice has no vocabulary types. The message
        // must not read as "your program is wrong".
        let err = run(r#"Instance.new("Frame").Size = 1"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot accept yet"), "{err}");
        assert!(err.contains("The property is real"), "{err}");
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
