//! `RBXScriptSignal`, `RBXScriptConnection`, and the firing discipline.
//!
//! WHAT A SIGNAL IS HERE
//! An instance id, a [`Kind`], and the arena it lives in. Nothing else -- a
//! signal owns no handler list of its own, because two objects naming the same
//! event (`f.Changed` fetched twice) must reach one list. The handlers live in
//! `Dom` beside the node they belong to, which is also what makes "a connection
//! cannot outlive its instance" a property of the arena rather than a rule
//! somebody has to remember to enforce.
//!
//! THE LOCK IS NEVER HELD WHILE A HANDLER RUNS, and this is the whole of why
//! this module exists rather than a few closures inside `__newindex`. The arena
//! is behind `Arc<Mutex<Dom>>` and `Mutex` is not reentrant, so calling a Luau
//! handler with the lock held deadlocks the moment that handler touches the tree
//! -- and a handler that touches the tree is the NORMAL case: a `Changed` that
//! sets another property, a `Destroying` that tears down a sibling. Every fire
//! below therefore collects, releases, and only then calls. Sprint 7 hit the same
//! shape once already, in `__index` building a method closure under the lock.
//!
//! A HANDLER IS RE-CHECKED BETWEEN CALLS, not snapshotted wholesale. Taking a
//! copy of the function list up front cannot corrupt the iteration, but it does
//! call handlers that were disconnected by an earlier one in the same fire --
//! including handlers on an instance an earlier one destroyed. So the CONNECTION
//! IDS are snapshotted, and each is looked up again just before it is called: a
//! disconnect during a fire is honoured, and a `Destroy` during a fire stops the
//! rest of that instance's handlers rather than calling them on a freed node.
//!
//! A HANDLER'S ERROR DOES NOT PROPAGATE. It is reported and the fire continues.
//! On the engine each handler runs on its own thread, so a failing listener
//! cannot fail the assignment that notified it; here there is no scheduler and a
//! direct call would make `frame.Visible = true` raise because something
//! unrelated was listening. Worse, `Destroy` fires `Destroying` before it frees
//! anything -- propagating there would leave the tree half torn down. Reporting
//! and continuing is the only choice that keeps the arena consistent, and it is
//! what the engine looks like from the guest's side.

use super::{dead_instance, handle, InstanceRef, SharedDom};
use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};

/// Which event on an instance. One value per signal a guest can reach.
///
/// `PropertyChanged` CARRIES THE NAME because `GetPropertyChangedSignal("Text")`
/// and `GetPropertyChangedSignal("Visible")` are different signals on one
/// instance, where every other kind here is one per instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Changed,
    PropertyChanged(String),
    ChildAdded,
    ChildRemoved,
    DescendantAdded,
    DescendantRemoving,
    Destroying,
    // ── Input, sprint 9 ──────────────────────────────────────────────────────
    //
    // ONE ENUM FOR EVERY EVENT, and the input family joins it rather than
    // getting a parallel type. `Dom::connect` stores a `Kind`, `listeners`
    // filters on one, and `fire` calls them -- none of which cares whether the
    // event came from a property write or from a mouse. A second enum would need
    // a second handler table, a second `listeners`, and a second copy of the
    // collect-release-call discipline that is the whole reason this module
    // exists.
    //
    // THE `GuiButton` HALF. `implements` refuses these on a `Frame` by class, so
    // nothing here has to check.
    Activated,
    SecondaryActivated,
    MouseButton1Click,
    MouseButton1Down,
    MouseButton1Up,
    MouseButton2Click,
    MouseButton2Down,
    MouseButton2Up,
    // The `GuiObject` half.
    MouseEnter,
    MouseLeave,
    MouseMoved,
    MouseWheelForward,
    MouseWheelBackward,
    InputBegan,
    InputChanged,
    InputEnded,
    // ── Focus, milestone 4 sprint 3 ──────────────────────────────────────────
    Focused,
    FocusLost,
    // ── `CollectionService`, milestone 27 sprint 1 ───────────────────────────
    //
    // ONE PER TAG NAME, the same shape as `PropertyChanged` above: two
    // different tags are two different signals on one source, so the name has
    // to travel with the kind rather than being a second dimension to index
    // by. The source these fire on is `CollectionService`'s own pseudo-instance
    // (`Dom::collection_service_id`), not the tagged instance itself -- the
    // engine's own `GetInstanceAddedSignal` and `GetInstanceRemovedSignal` are
    // members of the service, not of whatever gets tagged.
    TagAdded(String),
    TagRemoved(String),
}

impl Kind {
    /// What to call this in a message. Not a guest-visible value.
    fn label(&self) -> String {
        match self {
            Kind::Changed => "Changed".into(),
            Kind::PropertyChanged(p) => format!("GetPropertyChangedSignal({p})"),
            Kind::ChildAdded => "ChildAdded".into(),
            Kind::ChildRemoved => "ChildRemoved".into(),
            Kind::DescendantAdded => "DescendantAdded".into(),
            Kind::DescendantRemoving => "DescendantRemoving".into(),
            Kind::Destroying => "Destroying".into(),
            Kind::Activated => "Activated".into(),
            Kind::SecondaryActivated => "SecondaryActivated".into(),
            Kind::MouseButton1Click => "MouseButton1Click".into(),
            Kind::MouseButton1Down => "MouseButton1Down".into(),
            Kind::MouseButton1Up => "MouseButton1Up".into(),
            Kind::MouseButton2Click => "MouseButton2Click".into(),
            Kind::MouseButton2Down => "MouseButton2Down".into(),
            Kind::MouseButton2Up => "MouseButton2Up".into(),
            Kind::MouseEnter => "MouseEnter".into(),
            Kind::MouseLeave => "MouseLeave".into(),
            Kind::MouseMoved => "MouseMoved".into(),
            Kind::MouseWheelForward => "MouseWheelForward".into(),
            Kind::MouseWheelBackward => "MouseWheelBackward".into(),
            Kind::InputBegan => "InputBegan".into(),
            Kind::InputChanged => "InputChanged".into(),
            Kind::InputEnded => "InputEnded".into(),
            Kind::Focused => "Focused".into(),
            Kind::FocusLost => "FocusLost".into(),
            Kind::TagAdded(tag) => format!("GetInstanceAddedSignal({tag})"),
            Kind::TagRemoved(tag) => format!("GetInstanceRemovedSignal({tag})"),
        }
    }
}

/// A guest-held signal. `typeof` reports "RBXScriptSignal".
#[derive(Clone)]
pub struct LuaSignal {
    dom: SharedDom,
    id: usize,
    kind: Kind,
}

/// A guest-held connection. `typeof` reports "RBXScriptConnection".
#[derive(Clone)]
pub struct LuaConnection {
    dom: SharedDom,
    /// An index into the arena's handler table. Never reused, for the same
    /// reason instance ids are not: a stale one must stay stale.
    conn: usize,
}

impl UserData for LuaSignal {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        // A metaFIELD, NOT A METHOD, and mlua fills this in by default with the
        // RUST type name -- so leaving it out does not fail, it answers
        // "LuaSignal", a plausible string that no guest tests for and every
        // guest silently takes the wrong branch on. This has been got wrong
        // twice in this codebase; `typeof(instance)` carries the same comment.
        fields.add_meta_field(MetaMethod::Type, "RBXScriptSignal");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Signal {}", this.kind.label()))
        });

        // TWO SIGNAL OBJECTS FOR ONE EVENT COMPARE EQUAL. `f.Changed` fetched
        // twice makes two userdata, because a signal here is a description and
        // not a stored object; without this, `f.Changed == f.Changed` would be
        // false, which is true of nothing a guest has ever written against.
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            let LuaValue::UserData(ud) = other else {
                return Ok(false);
            };
            let Ok(other) = ud.borrow::<LuaSignal>() else {
                return Ok(false);
            };
            Ok(std::sync::Arc::ptr_eq(&this.dom, &other.dom)
                && this.id == other.id
                && this.kind == other.kind)
        });

        methods.add_method("Connect", |_, this, f: LuaFunction| {
            let mut dom = this.dom.lock().expect("dom");
            let conn = dom
                .connect(this.id, this.kind.clone(), f)
                .ok_or_else(dead_instance)?;
            Ok(LuaConnection {
                dom: this.dom.clone(),
                conn,
            })
        });

        // `Wait` YIELDS ON THE ENGINE, and this host has no task scheduler --
        // the same wall `WaitForChild` hit in sprint 7, and the decision here is
        // deliberately the OPPOSITE SHAPE for a reason worth stating.
        //
        // `WaitForChild` is absent because a non-yielding version of it returns
        // a PLAUSIBLE ANSWER in the case the child already exists: it is
        // `FindFirstChild` wearing the name of the thing it cannot do, and a
        // guest would never learn it had been served the wrong method. There is
        // no such answer for `Wait`. Nothing this host could return would be a
        // half-right value, so the failure cannot be silent, and the only
        // question is whether the guest reads "attempt to call a nil value" --
        // which points at their spelling -- or a sentence naming the actual
        // limitation. It gets the sentence.
        methods.add_method("Wait", |_, this, ()| -> LuaResult<()> {
            Err(LuaError::runtime(format!(
                "{}:Wait() yields, and this host has no task scheduler yet. \
                 Connect a handler instead. See docs/datamodel_scope.md.",
                this.kind.label()
            )))
        });
    }
}

impl UserData for LuaConnection {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "RBXScriptConnection");

        // THE POST-MORTEM STATE OF A CONNECTION IS A QUESTION, NOT AN ERROR, and
        // that is the difference between this and an instance handle. A handle
        // held across a `Destroy` errors because it is a REFERENCE whose subject
        // is gone and every use of it is a mistake. A connection is a
        // SUBSCRIPTION, and "are you still connected" is exactly what a cleanup
        // routine asks about one it may already have lost.
        fields.add_field_method_get("Connected", |_, this| {
            Ok(this.dom.lock().expect("dom").is_connected(this.conn))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, _, ()| Ok("Connection"));

        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            let LuaValue::UserData(ud) = other else {
                return Ok(false);
            };
            let Ok(other) = ud.borrow::<LuaConnection>() else {
                return Ok(false);
            };
            Ok(std::sync::Arc::ptr_eq(&this.dom, &other.dom) && this.conn == other.conn)
        });

        // IDEMPOTENT, including after the instance was destroyed. Disconnecting
        // twice is what a defensive cleanup does, and raising on the second call
        // would make the careful version of a teardown the one that fails.
        methods.add_method("Disconnect", |_, this, ()| {
            this.dom.lock().expect("dom").disconnect(this.conn);
            Ok(())
        });
    }
}

/// The signal value `__index` hands back for an event name.
pub fn signal(lua: &Lua, this: &InstanceRef, kind: Kind) -> LuaResult<LuaValue> {
    LuaSignal {
        dom: this.dom.clone(),
        id: this.id,
        kind,
    }
    .into_lua(lua)
}

/// Call everything connected to `kind` on `id`, with `args`.
///
/// COLLECT, RELEASE, CALL -- see the module comment. The ids are collected under
/// the lock and each handler is fetched under the lock again, but no lock is held
/// across the call itself.
pub fn fire(dom: &SharedDom, id: usize, kind: &Kind, args: &[LuaValue]) {
    let connections = dom.lock().expect("dom").listeners(id, kind);
    for conn in connections {
        // RE-CHECKED, not taken from a snapshot of functions: an earlier handler
        // in this same fire may have disconnected this one, or destroyed the
        // instance it belongs to.
        let handler = dom.lock().expect("dom").handler(conn);
        let Some(handler) = handler else { continue };
        if let Err(error) = handler.call::<()>(LuaMultiValue::from_iter(args.iter().cloned())) {
            eprintln!("[dew] a {} handler errored: {error}", kind.label());
        }
    }
}

/// `Changed` and `GetPropertyChangedSignal(property)`, in that pair's order.
///
/// THE SPECIFIC SIGNAL FIRES FIRST. A guest that listens to both is asking one
/// question at two resolutions, and the narrower one arriving first means a
/// `Changed` handler can already see whatever the specific one did.
pub fn property_changed(lua: &Lua, dom: &SharedDom, id: usize, property: &str) -> LuaResult<()> {
    fire(dom, id, &Kind::PropertyChanged(property.to_string()), &[]);
    let name = lua.create_string(property)?.into_lua(lua)?;
    fire(dom, id, &Kind::Changed, &[name]);
    Ok(())
}

/// Everything under `id`, `id` last-ancestor-first order, depth-first.
fn subtree(dom: &SharedDom, id: usize) -> Vec<usize> {
    let guard = dom.lock().expect("dom");
    let mut out = vec![id];
    let mut stack: Vec<usize> = guard.children(id);
    stack.reverse();
    while let Some(current) = stack.pop() {
        out.push(current);
        let mut kids = guard.children(current);
        kids.reverse();
        stack.extend(kids);
    }
    out
}

/// The ancestors of `id`, closest first. Read before a detach, not after.
fn ancestors(dom: &SharedDom, id: usize) -> Vec<usize> {
    let guard = dom.lock().expect("dom");
    let mut out = Vec::new();
    let mut cursor = guard.parent_of(id);
    while let Some(current) = cursor {
        out.push(current);
        cursor = guard.parent_of(current);
    }
    out
}

fn alive(dom: &SharedDom, id: usize) -> bool {
    dom.lock().expect("dom").exists(id)
}

/// `DescendantRemoving` then `ChildRemoved`, fired BEFORE `id` leaves its parent.
///
/// BEFORE, AND THAT IS THE CONTRACT. `DescendantRemoving` exists so a listener
/// can read the instance one last time; firing it after the detach would hand
/// every handler something already gone. `ChildRemoved` is the engine's
/// after-the-fact notice, but firing it here too keeps one order for the pair
/// rather than splitting the sequence across the mutation -- and a handler that
/// walks the parent during either sees a consistent tree, which is the property
/// worth having.
///
/// THE WHOLE SUBTREE IS ANNOUNCED. An ancestor loses every node under `id`, not
/// just `id`, and a guest counting descendants would otherwise drift.
pub fn leaving(lua: &Lua, dom: &SharedDom, id: usize) -> LuaResult<()> {
    let parent = dom.lock().expect("dom").parent_of(id);
    let Some(parent) = parent else {
        return Ok(());
    };
    let uphill = ancestors(dom, id);
    for node in subtree(dom, id) {
        if !alive(dom, node) {
            continue;
        }
        let value = handle(lua, dom, node)?.into_lua(lua)?;
        for ancestor in &uphill {
            if alive(dom, *ancestor) {
                fire(
                    dom,
                    *ancestor,
                    &Kind::DescendantRemoving,
                    std::slice::from_ref(&value),
                );
            }
        }
    }
    if alive(dom, parent) && alive(dom, id) {
        let value = handle(lua, dom, id)?.into_lua(lua)?;
        fire(dom, parent, &Kind::ChildRemoved, &[value]);
    }
    Ok(())
}

/// `ChildAdded` then `DescendantAdded`, fired AFTER `id` reached its new parent.
pub fn arrived(lua: &Lua, dom: &SharedDom, id: usize) -> LuaResult<()> {
    let parent = dom.lock().expect("dom").parent_of(id);
    let Some(parent) = parent else {
        return Ok(());
    };
    if alive(dom, parent) && alive(dom, id) {
        let value = handle(lua, dom, id)?.into_lua(lua)?;
        fire(dom, parent, &Kind::ChildAdded, &[value]);
    }
    let uphill = ancestors(dom, id);
    for node in subtree(dom, id) {
        if !alive(dom, node) {
            continue;
        }
        let value = handle(lua, dom, node)?.into_lua(lua)?;
        for ancestor in &uphill {
            if alive(dom, *ancestor) {
                fire(
                    dom,
                    *ancestor,
                    &Kind::DescendantAdded,
                    std::slice::from_ref(&value),
                );
            }
        }
    }
    Ok(())
}

/// `Destroy`, with the notices the engine sends and in the engine's order.
///
/// `Destroying` FIRST AND ON EVERY NODE IN THE SUBTREE, because `Destroy` is
/// recursive and each instance it takes is an instance being destroyed. Then the
/// removal notices, then the slots are freed -- so every handler above runs while
/// the tree it is being told about still exists.
pub fn destroy(lua: &Lua, dom: &SharedDom, id: usize) -> LuaResult<()> {
    for node in subtree(dom, id) {
        if alive(dom, node) {
            fire(dom, node, &Kind::Destroying, &[]);
            super::input::on_destroy(lua, node);
            super::collection_service::on_destroy(lua, dom, node);
        }
    }
    if !alive(dom, id) {
        // A `Destroying` handler already took it. Nothing left to free, and the
        // notices below would be about a node that is gone.
        return Ok(());
    }
    leaving(lua, dom, id)?;
    let mut guard = dom.lock().expect("dom");
    guard.destroy(id);
    guard.touch();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamodel::{install, install_vocabulary, Dom};
    use std::sync::{Arc, Mutex};

    /// THE TREE IS BUILT FROM LUAU, as the render and method tests build theirs.
    /// A Rust fixture would exercise the arena and not the dispatch, and the
    /// dispatch is where a signal is handed out -- an event never reached through
    /// `__index` is an event no guest can connect.
    fn vm() -> (Lua, SharedDom) {
        let lua = Lua::new();
        let dom: SharedDom = Arc::new(Mutex::new(Dom::default()));
        install(&lua, &dom).expect("install");
        install_vocabulary(&lua).expect("vocabulary");
        (lua, dom)
    }

    fn eval<T: FromLuaMulti>(src: &str) -> LuaResult<T> {
        let (lua, _dom) = vm();
        lua.load(src).eval()
    }

    fn err(src: &str) -> String {
        eval::<LuaValue>(src)
            .expect_err("expected this to fail")
            .to_string()
    }

    // ── The types themselves ─────────────────────────────────────────────────

    #[test]
    fn typeof_reports_the_engine_names_for_both_types() {
        // THE TRAP, AND IT HAS BEEN SPRUNG TWICE IN THIS CODEBASE. `__type` is a
        // metaFIELD; mlua fills it in by default with the RUST type name, so
        // omitting it does not fail loudly -- it answers "LuaSignal", which reads
        // as a plausible string and which no guest tests for. Asserted by VALUE
        // rather than by "is not nil", for exactly that reason.
        let got: Vec<String> = eval(
            r#"
            local f = Instance.new("Frame")
            local c = f.Changed:Connect(function() end)
            return { typeof(f.Changed), typeof(c), typeof(f:GetPropertyChangedSignal("Visible")) }
        "#,
        )
        .expect("eval");
        assert_eq!(
            got,
            vec![
                "RBXScriptSignal".to_string(),
                "RBXScriptConnection".to_string(),
                "RBXScriptSignal".to_string(),
            ]
        );
    }

    #[test]
    fn two_reads_of_one_signal_compare_equal() {
        // A signal here is a description rather than a stored object, so
        // `f.Changed` builds fresh userdata each time. Without `__eq` the
        // comparison every guest expects to be true would be false.
        let got: Vec<bool> = eval(
            r#"
            local f = Instance.new("Frame")
            local g = Instance.new("Frame")
            return {
                f.Changed == f.Changed,
                f:GetPropertyChangedSignal("Visible") == f:GetPropertyChangedSignal("Visible"),
                f:GetPropertyChangedSignal("Visible") == f:GetPropertyChangedSignal("ZIndex"),
                f.Changed == g.Changed,
                f.Changed == f.ChildAdded,
            }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, true, false, false, false]);
    }

    #[test]
    fn wait_says_why_it_is_not_here_rather_than_reading_as_a_typo() {
        // DECIDED, NOT OVERLOOKED, and deliberately the opposite shape to
        // `WaitForChild`'s absence -- see the comment on the method. Nothing this
        // host could return from `Wait` would be a plausible half-answer, so the
        // failure cannot be silent; the only choice is between "attempt to call a
        // nil value" and a sentence naming the limitation.
        let message = err(r#"return Instance.new("Frame").Changed:Wait()"#);
        assert!(message.contains("yields"), "{message}");
        assert!(message.contains("no task scheduler"), "{message}");
    }

    // ── Changed ──────────────────────────────────────────────────────────────

    #[test]
    fn changed_fires_with_the_property_name() {
        let got: Vec<String> = eval(
            r#"
            local f = Instance.new("Frame")
            local seen = {}
            f.Changed:Connect(function(property) table.insert(seen, property) end)
            f.Visible = false
            f.Name = "renamed"
            f.ZIndex = 4
            return seen
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec!["Visible", "Name", "ZIndex"]);
    }

    #[test]
    fn assigning_the_same_value_fires_nothing() {
        // THE DECISION, WRITTEN AS A TEST. A guest recomputing a tree every tick
        // assigns the value a property already has far more often than it assigns
        // a new one; firing on those turns `Changed` into a clock and makes the
        // dirty flag useless. Compared against the EFFECTIVE value, so writing a
        // property's own engine default over the top of nothing is not a change
        // either -- `Visible` starts true and `Name` starts as the class name.
        let got: Vec<usize> = eval(
            r#"
            local f = Instance.new("Frame")
            local n = 0
            f.Changed:Connect(function() n += 1 end)
            f.Visible = true
            f.Name = "Frame"
            f.Size = UDim2.new(0, 10, 0, 10)
            f.Size = UDim2.new(0, 10, 0, 10)
            local afterRepeats = n
            f.Visible = false
            return { afterRepeats, n }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![1, 2], "only the real changes fire");
    }

    #[test]
    fn a_property_signal_fires_for_its_property_and_no_other() {
        let got: Vec<usize> = eval(
            r#"
            local t = Instance.new("TextLabel")
            local visible, text = 0, 0
            t:GetPropertyChangedSignal("Visible"):Connect(function() visible += 1 end)
            t:GetPropertyChangedSignal("Text"):Connect(function() text += 1 end)
            t.Text = "hello"
            t.Visible = false
            t.ZIndex = 2
            return { visible, text }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![1, 1]);
    }

    #[test]
    fn a_property_signal_is_handed_no_arguments() {
        // The engine's contract, and the difference from `Changed`: the signal
        // already names the property, so passing it again would be the one
        // argument a handler cannot use.
        // `r##"..."##`, because `select("#", ...)` contains the sequence that ends
        // an ordinary raw string.
        let got: i64 = eval(
            r##"
            local f = Instance.new("Frame")
            local count = -1
            f:GetPropertyChangedSignal("Visible"):Connect(function(...) count = select("#", ...) end)
            f.Visible = false
            return count
        "##,
        )
        .expect("eval");
        assert_eq!(got, 0);
    }

    #[test]
    fn asking_for_a_signal_on_a_property_that_does_not_exist_is_loud() {
        // The property path's rule, in the one method that takes a property name
        // as an argument. A silent signal that never fires would have a guest
        // concluding this host does not fire events at all.
        let message = err(r#"return Instance.new("Frame"):GetPropertyChangedSignal("Visibel")"#);
        assert!(message.contains("Visibel"), "{message}");
        assert!(message.contains("not a valid member"), "{message}");
    }

    // ── The deadlock, which is the reason this module exists ─────────────────

    #[test]
    fn a_changed_handler_may_set_another_property() {
        // THE ORDINARY CASE, AND IT WOULD HANG. The arena is behind a `Mutex`,
        // `Mutex` is not reentrant, and firing with the lock held deadlocks the
        // moment a handler writes anything. This test does not assert a value so
        // much as it asserts that the process gets here at all.
        let got: String = eval(
            r#"
            local f = Instance.new("TextLabel")
            f:GetPropertyChangedSignal("Visible"):Connect(function()
                f.Text = "reacted"
            end)
            f.Visible = false
            return f.Text
        "#,
        )
        .expect("eval");
        assert_eq!(got, "reacted");
    }

    #[test]
    fn a_changed_handler_may_destroy_the_tree_it_was_told_about() {
        // The other half of the same trap, and the harsher one: the handler frees
        // the slot the assignment was made against, so the fire loop has to
        // survive its own receiver disappearing mid-flight.
        let got: Vec<bool> = eval(
            r#"
            local root = Instance.new("Folder")
            local child = Instance.new("Frame")
            child.Parent = root
            child.Changed:Connect(function()
                child:Destroy()
            end)
            child.Visible = false
            return { #root:GetChildren() == 0, (pcall(function() return child.Name end)) }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, false]);
    }

    #[test]
    fn a_destroying_handler_may_touch_the_tree() {
        let got: usize = eval(
            r#"
            local root = Instance.new("Folder")
            local a = Instance.new("Frame") a.Name = "a" a.Parent = root
            local b = Instance.new("Frame") b.Name = "b" b.Parent = root
            a.Destroying:Connect(function()
                b.Name = "survivor"
            end)
            a:Destroy()
            return #root:GetChildren()
        "#,
        )
        .expect("eval");
        assert_eq!(got, 1);
    }

    // ── Order, and disconnecting mid-fire ────────────────────────────────────

    #[test]
    fn handlers_fire_in_connection_order() {
        let got: Vec<i64> = eval(
            r#"
            local f = Instance.new("Frame")
            local seen = {}
            for i = 1, 4 do
                f.Changed:Connect(function() table.insert(seen, i) end)
            end
            f.Visible = false
            return seen
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![1, 2, 3, 4]);
    }

    #[test]
    fn a_handler_that_disconnects_another_during_a_fire_is_honoured() {
        // NOT A SNAPSHOT OF FUNCTIONS. Copying the handler list up front cannot
        // corrupt the iteration, but it calls a handler the guest disconnected a
        // microsecond earlier -- so the connection IDS are snapshotted and each is
        // looked up again immediately before it is called.
        let got: Vec<i64> = eval(
            r#"
            local f = Instance.new("Frame")
            local seen = {}
            local second
            f.Changed:Connect(function()
                table.insert(seen, 1)
                second:Disconnect()
            end)
            second = f.Changed:Connect(function() table.insert(seen, 2) end)
            f.Changed:Connect(function() table.insert(seen, 3) end)
            f.Visible = false
            return seen
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![1, 3], "the disconnected handler must not run");
    }

    #[test]
    fn a_handler_that_connects_another_during_a_fire_does_not_call_it_yet() {
        // The listener list is read once at the top of the fire, so a handler
        // added during it waits for the next one. The engine behaves the same way,
        // and the alternative is a fire that can never finish.
        let got: Vec<i64> = eval(
            r#"
            local f = Instance.new("Frame")
            local seen = {}
            f.Changed:Connect(function()
                if #seen == 0 then
                    f.Changed:Connect(function() table.insert(seen, 2) end)
                end
                table.insert(seen, 1)
            end)
            f.Visible = false
            local afterFirst = #seen
            f.ZIndex = 3
            return { afterFirst, #seen }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![1, 3]);
    }

    // ── Connections ──────────────────────────────────────────────────────────

    #[test]
    fn disconnect_stops_a_handler_and_reads_back_as_disconnected() {
        let got: Vec<bool> = eval(
            r#"
            local f = Instance.new("Frame")
            local fired = false
            local c = f.Changed:Connect(function() fired = true end)
            local before = c.Connected
            c:Disconnect()
            f.Visible = false
            return { before, c.Connected, fired }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, false, false]);
    }

    #[test]
    fn disconnecting_twice_is_not_an_error() {
        // What a defensive teardown does. Raising on the second call would make
        // the careful version of a cleanup the one that fails.
        let got: bool = eval(
            r#"
            local f = Instance.new("Frame")
            local c = f.Changed:Connect(function() end)
            c:Disconnect()
            c:Disconnect()
            return c.Connected
        "#,
        )
        .expect("eval");
        assert!(!got);
    }

    #[test]
    fn a_connection_does_not_outlive_its_instance() {
        // THE CONNECTION TWIN of "a handle held across a Destroy errors", and the
        // answer is deliberately different in shape. A handle is a REFERENCE whose
        // subject is gone, so every use of it is a mistake and it raises. A
        // connection is a SUBSCRIPTION, and "are you still connected" is precisely
        // what a cleanup routine asks about one it may already have lost -- so it
        // answers false, disconnects harmlessly, and above all never fires again.
        let got: Vec<bool> = eval(
            r#"
            local root = Instance.new("Folder")
            local child = Instance.new("Frame")
            child.Parent = root
            local fired = false
            local c = child.Changed:Connect(function() fired = true end)
            local before = c.Connected
            child:Destroy()
            c:Disconnect()
            return { before, c.Connected, fired }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, false, false]);
    }

    #[test]
    fn a_descendants_connections_go_with_the_subtree() {
        let got: bool = eval(
            r#"
            local root = Instance.new("Folder")
            local mid = Instance.new("Frame") mid.Parent = root
            local leaf = Instance.new("TextLabel") leaf.Parent = mid
            local c = leaf.Changed:Connect(function() end)
            root:Destroy()
            return c.Connected
        "#,
        )
        .expect("eval");
        assert!(!got);
    }

    #[test]
    fn connecting_to_a_destroyed_instance_says_so() {
        let message = err(r#"
            local f = Instance.new("Frame")
            local signal = f.Changed
            f:Destroy()
            return signal:Connect(function() end)
        "#);
        assert!(message.contains("has been destroyed"), "{message}");
    }

    // ── The tree events ──────────────────────────────────────────────────────

    #[test]
    fn child_added_and_child_removed_name_the_child() {
        let got: Vec<String> = eval(
            r#"
            local root = Instance.new("Folder")
            local seen = {}
            root.ChildAdded:Connect(function(child) table.insert(seen, "+" .. child.Name) end)
            root.ChildRemoved:Connect(function(child) table.insert(seen, "-" .. child.Name) end)

            local a = Instance.new("Frame") a.Name = "a"
            a.Parent = root
            a.Parent = nil
            a.Parent = root
            a:Destroy()
            return seen
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec!["+a", "-a", "+a", "-a"]);
    }

    #[test]
    fn the_descendant_events_reach_past_one_level_and_carry_the_subtree() {
        // AN ANCESTOR LOSES EVERY NODE UNDER THE ONE THAT MOVED, not just that
        // one, which is the part a first implementation gets wrong and a guest
        // counting descendants notices immediately.
        let got: Vec<String> = eval(
            r#"
            local root = Instance.new("Folder")
            local added, removing = {}, {}
            root.DescendantAdded:Connect(function(d) table.insert(added, d.Name) end)
            root.DescendantRemoving:Connect(function(d) table.insert(removing, d.Name) end)

            local mid = Instance.new("Frame") mid.Name = "mid"
            local leaf = Instance.new("TextLabel") leaf.Name = "leaf" leaf.Parent = mid
            mid.Parent = root
            mid:Destroy()

            return { table.concat(added, ","), table.concat(removing, ",") }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec!["mid,leaf".to_string(), "mid,leaf".to_string()]);
    }

    #[test]
    fn a_removal_is_announced_while_the_instance_can_still_be_read() {
        // THE WHOLE CONTRACT OF `DescendantRemoving`, and the reason it fires
        // before the detach rather than after: a handler exists to look at what is
        // leaving. Firing after would hand every one of them something gone.
        let got: String = eval(
            r#"
            local root = Instance.new("Folder")
            local child = Instance.new("Frame") child.Name = "leaving" child.Parent = root
            local seen = "never ran"
            root.DescendantRemoving:Connect(function(d)
                seen = `{d.Name} under {tostring(d.Parent)}`
            end)
            child:Destroy()
            return seen
        "#,
        )
        .expect("eval");
        assert_eq!(got, "leaving under Folder");
    }

    #[test]
    fn destroying_fires_on_every_node_in_the_subtree() {
        // `Destroy` is recursive, so each instance it takes is an instance being
        // destroyed -- and a guest cleaning up per-widget state relies on hearing
        // about every one of them, not just the root of the call.
        let got: Vec<String> = eval(
            r#"
            local root = Instance.new("Folder") root.Name = "root"
            local mid = Instance.new("Frame") mid.Name = "mid" mid.Parent = root
            local leaf = Instance.new("TextLabel") leaf.Name = "leaf" leaf.Parent = mid
            local seen = {}
            for _, node in { root, mid, leaf } do
                node.Destroying:Connect(function() table.insert(seen, node.Name) end)
            end
            root:Destroy()
            return seen
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec!["root", "mid", "leaf"]);
    }

    #[test]
    fn clear_all_children_announces_each_child() {
        let got: Vec<String> = eval(
            r#"
            local root = Instance.new("Folder")
            local seen = {}
            root.ChildRemoved:Connect(function(child) table.insert(seen, child.Name) end)
            for _, name in { "a", "b", "c" } do
                local f = Instance.new("Frame") f.Name = name f.Parent = root
            end
            root:ClearAllChildren()
            return seen
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec!["a", "b", "c"]);
    }

    #[test]
    fn reparenting_fires_changed_for_parent_after_the_tree_moved() {
        let got: Vec<String> = eval(
            r#"
            local a, b = Instance.new("Folder"), Instance.new("Folder")
            a.Name, b.Name = "a", "b"
            local child = Instance.new("Frame")
            local seen = {}
            child.Changed:Connect(function(property)
                table.insert(seen, `{property}={tostring(child.Parent)}`)
            end)
            child.Parent = a
            child.Parent = b
            return seen
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec!["Parent=a", "Parent=b"]);
    }

    // ── A failing handler ────────────────────────────────────────────────────

    #[test]
    fn a_handler_that_errors_is_reported_and_does_not_fail_the_write() {
        // On the engine each handler runs on its own thread, so a failing listener
        // cannot fail the assignment that notified it. There is no scheduler here,
        // so this is the deliberate substitute: report it, keep going, and let the
        // write that fired it complete. Propagating would also mean a `Destroying`
        // handler could abandon a half-torn-down tree.
        let got: Vec<bool> = eval(
            r#"
            local f = Instance.new("Frame")
            local later = false
            f.Changed:Connect(function() error("this handler is broken") end)
            f.Changed:Connect(function() later = true end)
            f.Visible = false
            return { f.Visible == false, later }
        "#,
        )
        .expect("the write must succeed");
        assert_eq!(got, vec![true, true]);
    }

    // ── The repaint, which is what all of this was for ───────────────────────

    #[test]
    fn only_a_real_change_marks_the_tree_dirty() {
        // THE PAYOFF, ASSERTED WHERE IT CAN BE. The live-window number in the
        // sprint record is this same mechanism measured through a window; this is
        // it measured directly, so a regression fails a test rather than costing
        // somebody a core.
        let (lua, dom) = vm();
        lua.load(
            r#"
            _G.f = Instance.new("Frame")
            _G.f.BackgroundTransparency = 0.5
        "#,
        )
        .exec()
        .expect("exec");

        assert!(
            dom.lock().expect("dom").take_dirty(),
            "the build is a change"
        );
        assert!(
            !dom.lock().expect("dom").take_dirty(),
            "and nothing happened since"
        );

        lua.load(r#"_G.f.BackgroundTransparency = 0.5"#)
            .exec()
            .expect("exec");
        assert!(
            !dom.lock().expect("dom").take_dirty(),
            "the same value is not a change"
        );

        lua.load(r#"_G.f.BackgroundTransparency = 0.25"#)
            .exec()
            .expect("exec");
        assert!(dom.lock().expect("dom").take_dirty(), "a new value is");

        lua.load(r#"_G.f:Destroy()"#).exec().expect("exec");
        assert!(dom.lock().expect("dom").take_dirty(), "so is a teardown");
    }
}
