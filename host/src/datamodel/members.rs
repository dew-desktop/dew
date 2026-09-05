//! Methods a guest can call on an instance, and the predicate that says which.
//!
//! WHY A PREDICATE AND NOT A LIST
//! The property half of `datamodel-surface` used to keep a hand-written array of
//! names, and it reported 35 of 138 for a surface the host had never implemented
//! -- the names in it were Aether's. [`super::accepts`] closed that by making the
//! tool ask the code path a guest actually takes. The member half had the same
//! hole in a more honest shape, `IMPLEMENTED_MEMBERS: &[&str] = &[]`, and this is
//! its `accepts`: [`implements`] answers for one class and one member, [`lookup`]
//! is what `__index` calls, and `lookup` refuses to hand out anything
//! [`implements`] does not admit. A number printed from here cannot be a number
//! the dispatch does not honour.
//!
//! WHY THE CLASS IS A PARAMETER when every entry below says `Instance`
//! IT IS NO LONGER VACUOUS, and sprint 9 is the sprint that made it matter.
//! `Activated` and the `MouseButton` pairs are `GuiButton`'s, so
//! `frame.Activated` is not a valid member of a `Frame` and this predicate says
//! so; the pointer events are `GuiObject`'s and reach a `Frame` but not a
//! `UIListLayout`. Sprint 7 built the check while every answer was still "yes",
//! which was cheaper than retrofitting it on the day a wrong answer became
//! possible -- and this is that day.
//!
//! WHAT THIS SLICE COVERS: tree navigation, lifecycle, the signals an instance
//! raises ABOUT ITSELF -- `Changed`, `GetPropertyChangedSignal`, and the five
//! tree-and-lifecycle events -- and, since sprint 9, the sixteen input events.
//! The signal TYPE lives in [`super::signal`], and the hit test and the pointer
//! state machine in [`super::input`]; this file is only where the dispatch hands
//! a signal out. That split is the point: `__index` cannot tell an input event
//! from a property one, and it should not have to.
//!
//! STILL NOT HERE, AND EACH FOR A STATED REASON. `Focused`, `FocusLost`,
//! `CaptureFocus`, `ReleaseFocus` and `IsFocused` need an owner and keyboard
//! routing, which is a mechanism of its own rather than the second half of this
//! one. `PageEnter`, `PageLeave`, `Stopped`, `Next`, `Previous`, `JumpTo`,
//! `JumpToIndex`, `GetScrollVelocity` and `ResetScrollVelocity` are
//! `UIPageLayout` and `ScrollingFrame` behaviour, and neither class is rendered
//! yet -- offering those members would be offering a scroll that cannot scroll.
//!
//! WHY `WaitForChild` IS NOT HERE, decided rather than overlooked
//! On the engine it YIELDS: it returns the child if one exists and otherwise
//! parks the calling thread until something creates one. Dew has no task
//! scheduler, so the only version this host could write returns immediately when
//! the child is already there and errors when it is not -- which is
//! `FindFirstChild` with a worse failure mode, under the one name whose entire
//! purpose is the case it cannot serve. A guest writing `WaitForChild` is saying
//! the child may not exist yet, and an error is a WRONG answer to that, not a
//! partial one. So it stays on the backlog in `docs/datamodel_scope.md`, where a
//! method this host does not implement belongs, and it arrives with the scheduler.

use super::{handle, signal, InstanceRef};
use mlua::prelude::*;

/// One method this host answers, and the class a receiver must be to call it.
struct Member {
    name: &'static str,
    /// The class that introduces it. A receiver qualifies if it IS that class or
    /// descends from it, by the reflection database's own superclass chain.
    ///
    /// WHERE **DEW** INTRODUCES IT, NOT WHERE ROBLOX DOES. The pinned API dump
    /// puts `IsA` on `Object`, a class `rbx_reflection_database` does not carry
    /// -- its root is `Instance`. Naming a class the database has never heard of
    /// would make the descendant check answer false for every instance in the
    /// arena, so this names the class the dispatch actually enforces against.
    introduced_on: &'static str,
}

/// Tree navigation, lifecycle, and the signals an instance raises about itself.
///
/// EVENTS SIT IN THE SAME LIST AS METHODS, because `__index` makes no
/// distinction: `f:GetChildren()` fetches a function and `f.Changed` fetches a
/// signal, and both are a member this host either answers or does not. Splitting
/// them would give `implements` two lists to stay consistent with, which is the
/// one thing this file exists to avoid.
const MEMBERS: &[Member] = &[
    // -- Input, and THE CLASS PARAMETER STOPS BEING VACUOUS HERE --------------
    //
    // Every entry in this list said `Instance` until this sprint, and the module
    // comment above promised that would change. It has: `Activated` and the
    // `MouseButton` pairs are `GuiButton`'s, so `frame.Activated` is not a valid
    // member of a `Frame` and `implements` says so by walking the superclass
    // chain rather than by matching a name. A name-only predicate would have
    // offered `Activated` on a `UIListLayout` and reported it reachable
    // everywhere.
    //
    // THE CLASSES COME FROM THE PINNED API DUMP, not from memory:
    // `host/datamodel/api_surface.json` puts these eight on `GuiButton` and the
    // next eight on `GuiObject`, and both names exist in
    // `rbx_reflection_database` -- which is the thing that had to be checked,
    // because naming a class the database has never heard of makes the descendant
    // check answer false for every instance in the arena.
    Member {
        name: "Activated",
        introduced_on: "GuiButton",
    },
    Member {
        name: "SecondaryActivated",
        introduced_on: "GuiButton",
    },
    Member {
        name: "MouseButton1Click",
        introduced_on: "GuiButton",
    },
    Member {
        name: "MouseButton1Down",
        introduced_on: "GuiButton",
    },
    Member {
        name: "MouseButton1Up",
        introduced_on: "GuiButton",
    },
    Member {
        name: "MouseButton2Click",
        introduced_on: "GuiButton",
    },
    Member {
        name: "MouseButton2Down",
        introduced_on: "GuiButton",
    },
    Member {
        name: "MouseButton2Up",
        introduced_on: "GuiButton",
    },
    Member {
        name: "MouseEnter",
        introduced_on: "GuiObject",
    },
    Member {
        name: "MouseLeave",
        introduced_on: "GuiObject",
    },
    Member {
        name: "MouseMoved",
        introduced_on: "GuiObject",
    },
    Member {
        name: "MouseWheelForward",
        introduced_on: "GuiObject",
    },
    Member {
        name: "MouseWheelBackward",
        introduced_on: "GuiObject",
    },
    Member {
        name: "InputBegan",
        introduced_on: "GuiObject",
    },
    Member {
        name: "InputChanged",
        introduced_on: "GuiObject",
    },
    Member {
        name: "InputEnded",
        introduced_on: "GuiObject",
    },
    // -- Everything an instance says about itself -----------------------------
    Member {
        name: "Changed",
        introduced_on: "Instance",
    },
    Member {
        name: "ChildAdded",
        introduced_on: "Instance",
    },
    Member {
        name: "ChildRemoved",
        introduced_on: "Instance",
    },
    Member {
        name: "ClearAllChildren",
        introduced_on: "Instance",
    },
    Member {
        name: "DescendantAdded",
        introduced_on: "Instance",
    },
    Member {
        name: "DescendantRemoving",
        introduced_on: "Instance",
    },
    Member {
        name: "Destroying",
        introduced_on: "Instance",
    },
    Member {
        name: "Destroy",
        introduced_on: "Instance",
    },
    Member {
        name: "FindFirstAncestor",
        introduced_on: "Instance",
    },
    Member {
        name: "FindFirstAncestorOfClass",
        introduced_on: "Instance",
    },
    Member {
        name: "FindFirstAncestorWhichIsA",
        introduced_on: "Instance",
    },
    Member {
        name: "FindFirstChild",
        introduced_on: "Instance",
    },
    Member {
        name: "FindFirstChildOfClass",
        introduced_on: "Instance",
    },
    Member {
        name: "FindFirstChildWhichIsA",
        introduced_on: "Instance",
    },
    Member {
        name: "FindFirstDescendant",
        introduced_on: "Instance",
    },
    Member {
        name: "GetChildren",
        introduced_on: "Instance",
    },
    Member {
        name: "GetDescendants",
        introduced_on: "Instance",
    },
    Member {
        name: "GetPropertyChangedSignal",
        introduced_on: "Instance",
    },
    Member {
        name: "IsA",
        introduced_on: "Instance",
    },
    Member {
        name: "IsAncestorOf",
        introduced_on: "Instance",
    },
    Member {
        name: "IsDescendantOf",
        introduced_on: "Instance",
    },
];

/// Does `class` match `ancestor`, or descend from it?
///
/// THE SUPERCLASS CHAIN, NOT A STRING COMPARISON, and this is the function most
/// likely to be written as equality by accident. Equality makes a `Frame` fail to
/// be a `GuiObject`, which is precisely the question every caller of `IsA` is
/// asking -- nobody writes `x:IsA("Frame")` when they already hold a `Frame`.
/// Aether's host layer asks about `GuiBase2d`; a host answering only exact
/// matches would report a screen full of frames as containing no UI at all.
pub fn class_is_a(class: &str, ancestor: &str) -> bool {
    let Ok(db) = rbx_reflection_database::get() else {
        return false;
    };
    let mut cursor = db.classes.get(class);
    while let Some(current) = cursor {
        if current.name == ancestor {
            return true;
        }
        cursor = current.superclass.and_then(|s| db.classes.get(s));
    }
    false
}

/// Does this host answer `member` when it is called on an instance of `class`?
///
/// The member twin of [`super::accepts`], and the measurement behind the second
/// half of `docs/datamodel_scope.md`. `datamodel-surface` calls this and nothing
/// else, so that document cannot report a method the dispatch will not hand out.
pub fn implements(class: &str, member: &str) -> bool {
    MEMBERS
        .iter()
        .any(|m| m.name == member && class_is_a(class, m.introduced_on))
}

/// A destroyed handle says so rather than reading as an empty instance.
fn dead() -> LuaError {
    LuaError::runtime("this instance has been destroyed")
}

/// Every descendant of `id`, depth-first and in child order.
///
/// ONE ORDER FOR EVERY DESCENDANT WALK here, and it is a decision worth naming:
/// `GetDescendants`, the recursive arm of `FindFirstChild` and
/// `FindFirstChildWhichIsA`, and `FindFirstDescendant` all use it. Order is only
/// observable when two descendants share a name at different depths, and one
/// order for all of them means a guest comparing two of these gets a consistent
/// answer rather than three functions that disagree about the same tree.
fn descendants(dom: &super::Dom, id: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack: Vec<usize> = dom.children(id);
    stack.reverse();
    while let Some(current) = stack.pop() {
        out.push(current);
        let mut kids = dom.children(current);
        kids.reverse();
        stack.extend(kids);
    }
    out
}

/// Ancestors of `id`, closest first.
fn ancestors(dom: &super::Dom, id: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut cursor = dom.parent_of(id);
    while let Some(current) = cursor {
        out.push(current);
        cursor = dom.parent_of(current);
    }
    out
}

/// A found instance as a handle, or nil.
///
/// NIL IS THE RIGHT ANSWER, and this is the one place in the DataModel where it
/// is. Reading a misspelled PROPERTY is an error because the name was meant to
/// exist; asking whether a child exists is a question whose honest negative
/// answer is "it does not", and every guest written for the engine tests it with
/// `if child then`. Erroring here to match the property path would take away the
/// entire reason `FindFirstChild` is spelled differently from `.Child`.
fn found(lua: &Lua, this: &InstanceRef, id: Option<usize>) -> LuaResult<LuaValue> {
    match id {
        Some(id) => handle(lua, &this.dom, id)?.into_lua(lua),
        None => Ok(LuaValue::Nil),
    }
}

/// The id behind an argument that has to be an Instance.
fn instance_arg(value: &LuaValue, method: &str) -> LuaResult<InstanceRef> {
    let wrong = || {
        LuaError::runtime(format!(
            "{method} expects an Instance, got {}",
            value.type_name()
        ))
    };
    let LuaValue::UserData(ud) = value else {
        return Err(wrong());
    };
    let other = ud.borrow::<InstanceRef>().map_err(|_| wrong())?;
    Ok(other.clone())
}

/// The method named `key`, already bound to `this`, or `None` when this host has
/// no such member on this class.
///
/// GOES THROUGH [`implements`] FIRST and only then matches on the name -- the
/// same shape `supported` and `coerce` have above it. Two independent lists is
/// how a predicate and the thing it describes drift apart, so the fallthrough is
/// `unreachable!` rather than a second copy of the answer.
///
/// THE RECEIVER IS BOUND AT LOOKUP, so the `self` Luau passes for `a:Method()`
/// arrives as a leading argument that every arm below ignores. Binding rather
/// than reading `self` off the call is what makes `local f = a.GetChildren`
/// behave the way it does on the engine, where the method is fetched from the
/// instance and carries it.
pub fn lookup(
    lua: &Lua,
    this: &InstanceRef,
    class: &str,
    key: &str,
) -> LuaResult<Option<LuaValue>> {
    if !implements(class, key) {
        return Ok(None);
    }

    // AN EVENT IS NOT CALLED, IT IS READ. `f.Changed` is a property access that
    // yields a signal; `f:Changed()` is a mistake. So these return before the
    // function table below rather than being a function that returns a signal --
    // which is what `GetPropertyChangedSignal` genuinely is, and the two live one
    // above the other here so the difference is visible rather than inferred.
    let event = match key {
        "Changed" => Some(signal::Kind::Changed),
        "ChildAdded" => Some(signal::Kind::ChildAdded),
        "ChildRemoved" => Some(signal::Kind::ChildRemoved),
        "DescendantAdded" => Some(signal::Kind::DescendantAdded),
        "DescendantRemoving" => Some(signal::Kind::DescendantRemoving),
        "Destroying" => Some(signal::Kind::Destroying),
        // INPUT IS READ, NOT CALLED, LIKE EVERY OTHER EVENT. There is nothing
        // special about these sixteen at the dispatch: `button.Activated` fetches
        // a signal from the same list by the same rule, and what makes them
        // different lives entirely in who FIRES them -- `datamodel::input`, driven
        // by the window rather than by a guest's assignment.
        "Activated" => Some(signal::Kind::Activated),
        "SecondaryActivated" => Some(signal::Kind::SecondaryActivated),
        "MouseButton1Click" => Some(signal::Kind::MouseButton1Click),
        "MouseButton1Down" => Some(signal::Kind::MouseButton1Down),
        "MouseButton1Up" => Some(signal::Kind::MouseButton1Up),
        "MouseButton2Click" => Some(signal::Kind::MouseButton2Click),
        "MouseButton2Down" => Some(signal::Kind::MouseButton2Down),
        "MouseButton2Up" => Some(signal::Kind::MouseButton2Up),
        "MouseEnter" => Some(signal::Kind::MouseEnter),
        "MouseLeave" => Some(signal::Kind::MouseLeave),
        "MouseMoved" => Some(signal::Kind::MouseMoved),
        "MouseWheelForward" => Some(signal::Kind::MouseWheelForward),
        "MouseWheelBackward" => Some(signal::Kind::MouseWheelBackward),
        "InputBegan" => Some(signal::Kind::InputBegan),
        "InputChanged" => Some(signal::Kind::InputChanged),
        "InputEnded" => Some(signal::Kind::InputEnded),
        _ => None,
    };
    if let Some(kind) = event {
        return Ok(Some(signal::signal(lua, this, kind)?));
    }

    let this = this.clone();

    let f = match key {
        "GetChildren" => lua.create_function(move |lua, _: LuaValue| {
            let dom = this.dom.lock().expect("dom");
            if dom.node(this.id).is_none() {
                return Err(dead());
            }
            // THE LOCK GOES FIRST. `handle` takes the same `Mutex` to prune
            // destroyed ids and it is not reentrant, so the ids come out and the
            // guard goes before any handle is built.
            let children = dom.children(this.id);
            drop(dom);
            let out = lua.create_table()?;
            for (i, child) in children.into_iter().enumerate() {
                out.set(i + 1, handle(lua, &this.dom, child)?)?;
            }
            Ok(out)
        })?,
        "GetDescendants" => lua.create_function(move |lua, _: LuaValue| {
            let dom = this.dom.lock().expect("dom");
            if dom.node(this.id).is_none() {
                return Err(dead());
            }
            let all = descendants(&dom, this.id);
            drop(dom);
            let out = lua.create_table()?;
            for (i, node) in all.into_iter().enumerate() {
                out.set(i + 1, handle(lua, &this.dom, node)?)?;
            }
            Ok(out)
        })?,
        "FindFirstChild" => lua.create_function(
            move |lua, (_, name, recursive): (LuaValue, String, Option<bool>)| {
                let dom = this.dom.lock().expect("dom");
                if dom.node(this.id).is_none() {
                    return Err(dead());
                }
                let pool = if recursive.unwrap_or(false) {
                    descendants(&dom, this.id)
                } else {
                    dom.children(this.id)
                };
                let hit = pool
                    .into_iter()
                    .find(|c| dom.name_of(*c).as_deref() == Some(name.as_str()));
                // THE LOCK GOES FIRST, for `handle`'s sake. See `GetChildren`.
                drop(dom);
                found(lua, &this, hit)
            },
        )?,
        "FindFirstChildOfClass" => {
            lua.create_function(move |lua, (_, class): (LuaValue, String)| {
                let dom = this.dom.lock().expect("dom");
                if dom.node(this.id).is_none() {
                    return Err(dead());
                }
                let hit = dom
                    .children(this.id)
                    .into_iter()
                    .find(|c| dom.class_of(*c).as_deref() == Some(class.as_str()));
                // THE LOCK GOES FIRST, for `handle`'s sake. See `GetChildren`.
                drop(dom);
                found(lua, &this, hit)
            })?
        }
        "FindFirstChildWhichIsA" => lua.create_function(
            move |lua, (_, class, recursive): (LuaValue, String, Option<bool>)| {
                let dom = this.dom.lock().expect("dom");
                if dom.node(this.id).is_none() {
                    return Err(dead());
                }
                let pool = if recursive.unwrap_or(false) {
                    descendants(&dom, this.id)
                } else {
                    dom.children(this.id)
                };
                let hit = pool
                    .into_iter()
                    .find(|c| dom.class_of(*c).is_some_and(|k| class_is_a(&k, &class)));
                // THE LOCK GOES FIRST, for `handle`'s sake. See `GetChildren`.
                drop(dom);
                found(lua, &this, hit)
            },
        )?,
        "FindFirstDescendant" => {
            lua.create_function(move |lua, (_, name): (LuaValue, String)| {
                let dom = this.dom.lock().expect("dom");
                if dom.node(this.id).is_none() {
                    return Err(dead());
                }
                let hit = descendants(&dom, this.id)
                    .into_iter()
                    .find(|c| dom.name_of(*c).as_deref() == Some(name.as_str()));
                // THE LOCK GOES FIRST, for `handle`'s sake. See `GetChildren`.
                drop(dom);
                found(lua, &this, hit)
            })?
        }
        "FindFirstAncestor" => lua.create_function(move |lua, (_, name): (LuaValue, String)| {
            let dom = this.dom.lock().expect("dom");
            if dom.node(this.id).is_none() {
                return Err(dead());
            }
            let hit = ancestors(&dom, this.id)
                .into_iter()
                .find(|a| dom.name_of(*a).as_deref() == Some(name.as_str()));
            // THE LOCK GOES FIRST, for `handle`'s sake. See `GetChildren`.
            drop(dom);
            found(lua, &this, hit)
        })?,
        "FindFirstAncestorOfClass" => {
            lua.create_function(move |lua, (_, class): (LuaValue, String)| {
                let dom = this.dom.lock().expect("dom");
                if dom.node(this.id).is_none() {
                    return Err(dead());
                }
                let hit = ancestors(&dom, this.id)
                    .into_iter()
                    .find(|a| dom.class_of(*a).as_deref() == Some(class.as_str()));
                // THE LOCK GOES FIRST, for `handle`'s sake. See `GetChildren`.
                drop(dom);
                found(lua, &this, hit)
            })?
        }
        "FindFirstAncestorWhichIsA" => {
            lua.create_function(move |lua, (_, class): (LuaValue, String)| {
                let dom = this.dom.lock().expect("dom");
                if dom.node(this.id).is_none() {
                    return Err(dead());
                }
                let hit = ancestors(&dom, this.id)
                    .into_iter()
                    .find(|a| dom.class_of(*a).is_some_and(|k| class_is_a(&k, &class)));
                // THE LOCK GOES FIRST, for `handle`'s sake. See `GetChildren`.
                drop(dom);
                found(lua, &this, hit)
            })?
        }
        // A CLASS NAME THE DATABASE HAS NEVER HEARD OF ANSWERS FALSE rather than
        // raising, and that is a deliberate departure from the property path's
        // "a typo is loud" rule. `IsA` exists to be handed an arbitrary string
        // and to give back a boolean; the engine answers false, Aether's host
        // layer asks it inside conditions, and a host that raised would turn a
        // guarded branch into a crash. `FindFirstChildOfClass` takes the same
        // view of the same string, because two answers to one question is worse
        // than either answer.
        "IsA" => lua.create_function(move |_, (_, class): (LuaValue, String)| {
            let dom = this.dom.lock().expect("dom");
            let own = dom.class_of(this.id).ok_or_else(dead)?;
            Ok(class_is_a(&own, &class))
        })?,
        // NOT REFLEXIVE, on either of these. An instance is not its own ancestor
        // and not its own descendant, and the arena's `is_ancestor_of` answers
        // true for the self case because it exists to refuse a cycle -- where
        // reflexive is the correct reading. Reusing it without the guard would
        // make `x:IsDescendantOf(x)` true.
        "IsAncestorOf" => lua.create_function(move |_, (_, other): (LuaValue, LuaValue)| {
            let other = instance_arg(&other, "IsAncestorOf")?;
            let dom = this.dom.lock().expect("dom");
            if dom.node(this.id).is_none() {
                return Err(dead());
            }
            // A HANDLE FROM ANOTHER VM'S ARENA is unrelated to this one however
            // the ids compare, and two arenas both number from zero.
            if !std::sync::Arc::ptr_eq(&this.dom, &other.dom) {
                return Ok(false);
            }
            Ok(this.id != other.id && dom.is_ancestor_of(this.id, other.id))
        })?,
        "IsDescendantOf" => lua.create_function(move |_, (_, other): (LuaValue, LuaValue)| {
            let other = instance_arg(&other, "IsDescendantOf")?;
            let dom = this.dom.lock().expect("dom");
            if dom.node(this.id).is_none() {
                return Err(dead());
            }
            if !std::sync::Arc::ptr_eq(&this.dom, &other.dom) {
                return Ok(false);
            }
            Ok(this.id != other.id && dom.is_ancestor_of(other.id, this.id))
        })?,
        // THE PROPERTY NAME IS CHECKED AGAINST THE CLASS, and a misspelling is
        // loud here where `IsA`'s is not. `IsA` is handed arbitrary strings by
        // design and answers a boolean; this is asked for a signal that will
        // never fire, and returning a silent one would leave a guest watching a
        // property that does not exist and concluding the host does not fire
        // events. It is the property path's rule, in the one method that takes a
        // property name as an argument.
        "GetPropertyChangedSignal" => {
            lua.create_function(move |lua, (_, property): (LuaValue, String)| {
                let class = {
                    let dom = this.dom.lock().expect("dom");
                    dom.class_of(this.id).ok_or_else(dead)?
                };
                if super::describe(&class, &property).is_none() {
                    return Err(LuaError::runtime(format!(
                        "{property} is not a valid member of {class}"
                    )));
                }
                signal::signal(lua, &this, signal::Kind::PropertyChanged(property))
            })?
        }
        // THE SLOT IS CLEARED, so every handle already held fails loudly on its
        // next read instead of reporting a stale name. `Index` and `NewIndex`
        // have answered "this instance has been destroyed" on an empty slot since
        // the arena existed; this is the thing that finally makes that branch
        // reachable. A `Parent = nil` that left the node in place would be the
        // version where a destroyed widget keeps answering questions about itself.
        //
        // THE NOTICES COME FIRST AND THE LOCK IS NOT HELD FOR THEM.
        // `signal::destroy` fires `Destroying` over the whole subtree, then the
        // removal notices, and only then frees the slots -- so a handler runs
        // while the instance it is being told about still exists, and a handler
        // that touches the tree does not meet a lock this call is holding.
        "Destroy" => {
            lua.create_function(move |lua, _: LuaValue| signal::destroy(lua, &this.dom, this.id))?
        }
        "ClearAllChildren" => lua.create_function(move |lua, _: LuaValue| {
            let children = {
                let dom = this.dom.lock().expect("dom");
                if dom.node(this.id).is_none() {
                    return Err(dead());
                }
                dom.children(this.id)
            };
            for child in children {
                // RE-CHECKED EACH TIME. A `Destroying` handler on the first child
                // is free to destroy the rest, and this list was taken before any
                // of them ran.
                if this.dom.lock().expect("dom").exists(child) {
                    signal::destroy(lua, &this.dom, child)?;
                }
            }
            Ok(())
        })?,
        other => unreachable!("implements() admitted {other} and lookup has no arm for it"),
    };
    Ok(Some(LuaValue::Function(f)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamodel::{install, install_vocabulary, SharedDom};

    /// THE TREE IS BUILT FROM LUAU, as the render tests build theirs. A Rust
    /// fixture would exercise the arena and not the dispatch, and the dispatch is
    /// what this sprint added -- a method never reached through `__index` is a
    /// method no guest can call.
    fn eval<T: FromLuaMulti>(src: &str) -> LuaResult<T> {
        let lua = Lua::new();
        let dom = SharedDom::default();
        install(&lua, &dom).expect("install");
        install_vocabulary(&lua).expect("vocabulary");
        lua.load(src).eval()
    }

    fn err(src: &str) -> String {
        eval::<LuaValue>(src)
            .expect_err("expected this to fail")
            .to_string()
    }

    // ── The predicate ────────────────────────────────────────────────────────

    #[test]
    fn the_predicate_answers_for_the_classes_a_guest_holds() {
        assert!(implements("Frame", "GetChildren"));
        assert!(implements("TextButton", "Destroy"));
        assert!(implements("Instance", "IsA"));
        assert!(!implements("Frame", "Clone"));
    }

    #[test]
    fn the_predicate_refuses_what_this_sprint_deferred() {
        // WHAT IS STILL DEFERRED, named here because a predicate that answered
        // true for any of it would print a number the dispatch cannot honour --
        // exactly the failure the hand-written list produced.
        //
        // THIS TEST HAS NOW BEEN REPOINTED TWICE, and that is it working rather
        // than it failing at its job. Sprint 8 found `Changed` and
        // `GetPropertyChangedSignal` in it; this sprint found `Activated` and
        // `InputBegan`. Whatever is left is what the next sprint owes.
        //
        // FOCUS, which needs an owner and keyboard routing.
        for member in [
            "CaptureFocus",
            "ReleaseFocus",
            "IsFocused",
            "Focused",
            "FocusLost",
        ] {
            assert!(!implements("TextBox", member), "{member}");
        }
        // `UIPageLayout` AND `ScrollingFrame`, neither of which is rendered.
        for member in ["GetScrollVelocity", "ResetScrollVelocity"] {
            assert!(!implements("ScrollingFrame", member), "{member}");
        }
        for member in [
            "PageEnter",
            "PageLeave",
            "Stopped",
            "Next",
            "Previous",
            "JumpToIndex",
        ] {
            assert!(!implements("UIPageLayout", member), "{member}");
        }
        // Decided, not overlooked. See the module comment.
        assert!(!implements("Frame", "WaitForChild"));
    }

    #[test]
    fn the_class_parameter_is_no_longer_vacuous() {
        // EVERY ENTRY IN `MEMBERS` SAID `Instance` UNTIL THIS SPRINT, so every
        // answer was "yes" and the class parameter could have been deleted
        // without a single test noticing. It could not be now.
        //
        // `Activated` is a `GuiButton`'s, so a `Frame` does not have it; the
        // pointer events are a `GuiObject`'s, so a `Frame` does and a
        // `UIListLayout` does not. A name-only predicate answers true to all five
        // of these.
        assert!(implements("TextButton", "Activated"));
        assert!(!implements("Frame", "Activated"));
        assert!(implements("Frame", "MouseEnter"));
        assert!(!implements("UIListLayout", "MouseEnter"));
        assert!(!implements("Folder", "InputBegan"));
        // AND THE SUPERCLASS WALK STILL RUNS BOTH WAYS: a `TextButton` has the
        // `GuiObject` half by descent rather than by being listed twice, and the
        // `Instance` members by descent from further up again.
        assert!(implements("TextButton", "InputBegan"));
        assert!(implements("TextButton", "GetChildren"));
    }

    // ── IsA ──────────────────────────────────────────────────────────────────

    #[test]
    fn is_a_walks_the_superclass_chain() {
        // The whole point of the method, and what it would fail at if it were
        // written as a string comparison.
        let got: Vec<bool> = eval(
            r#"
            local f = Instance.new("Frame")
            return { f:IsA("Frame"), f:IsA("GuiObject"), f:IsA("GuiBase2d"), f:IsA("Instance") }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, true, true, true]);
    }

    #[test]
    fn is_a_is_false_for_a_sibling_and_for_a_class_below() {
        // The chain runs upward only: a Frame is not a TextLabel, and a Folder is
        // not a Frame however far down it sits.
        let got: Vec<bool> = eval(
            r#"
            local f = Instance.new("Frame")
            local label = Instance.new("TextLabel")
            return { f:IsA("TextLabel"), label:IsA("Frame"), Instance.new("Folder"):IsA("Frame") }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![false, false, false]);
    }

    #[test]
    fn is_a_answers_false_for_a_class_that_does_not_exist() {
        // The engine's contract, and a departure from "a typo is loud" made on
        // purpose: `IsA` is asked inside conditions.
        let got: bool = eval(r#"return Instance.new("Frame"):IsA("Frmae")"#).expect("eval");
        assert!(!got);
    }

    // ── Children and descendants ─────────────────────────────────────────────

    #[test]
    fn get_children_is_in_parent_order_and_one_level_deep() {
        let got: Vec<String> = eval(
            r#"
            local root = Instance.new("Folder")
            for _, name in { "a", "b", "c" } do
                local f = Instance.new("Frame")
                f.Name = name
                f.Parent = root
            end
            local deep = Instance.new("Frame")
            deep.Name = "deep"
            deep.Parent = root:GetChildren()[1]

            local names = {}
            for _, child in root:GetChildren() do
                table.insert(names, child.Name)
            end
            return names
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec!["a", "b", "c"]);
    }

    #[test]
    fn get_descendants_is_depth_first_in_child_order() {
        let got: Vec<String> = eval(
            r#"
            local root = Instance.new("Folder")
            local a = Instance.new("Frame") a.Name = "a" a.Parent = root
            local a1 = Instance.new("Frame") a1.Name = "a1" a1.Parent = a
            local a2 = Instance.new("Frame") a2.Name = "a2" a2.Parent = a
            local b = Instance.new("Frame") b.Name = "b" b.Parent = root

            local names = {}
            for _, d in root:GetDescendants() do
                table.insert(names, d.Name)
            end
            return names
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec!["a", "a1", "a2", "b"]);
    }

    #[test]
    fn an_empty_instance_has_no_children_rather_than_nil() {
        let got: usize = eval(r#"return #Instance.new("Frame"):GetChildren()"#).expect("eval");
        assert_eq!(got, 0);
    }

    // ── Finding ──────────────────────────────────────────────────────────────

    #[test]
    fn find_first_child_returns_nil_when_there_is_no_such_child() {
        // NIL, NOT AN ERROR. The one place in this DataModel where nil is the
        // correct answer, and the reason the method exists at all.
        let got: bool = eval(r#"return Instance.new("Folder"):FindFirstChild("nothing") == nil"#)
            .expect("eval");
        assert!(got);
    }

    #[test]
    fn find_first_child_is_one_level_deep_until_it_is_asked_to_recurse() {
        let got: Vec<bool> = eval(
            r#"
            local root = Instance.new("Folder")
            local mid = Instance.new("Frame") mid.Parent = root
            local deep = Instance.new("Frame") deep.Name = "deep" deep.Parent = mid
            return {
                root:FindFirstChild("deep") == nil,
                root:FindFirstChild("deep", true) == deep,
            }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, true]);
    }

    #[test]
    fn of_class_matches_exactly_and_which_is_a_matches_the_chain() {
        // The pair exists because they answer differently, and this is the tree
        // where they do: a TextLabel is a GuiObject but is not one by class name.
        let got: Vec<bool> = eval(
            r#"
            local root = Instance.new("Folder")
            local label = Instance.new("TextLabel") label.Parent = root
            return {
                root:FindFirstChildOfClass("TextLabel") == label,
                root:FindFirstChildOfClass("GuiObject") == nil,
                root:FindFirstChildWhichIsA("GuiObject") == label,
            }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, true, true]);
    }

    #[test]
    fn find_first_descendant_reaches_past_the_first_level() {
        let got: bool = eval(
            r#"
            local root = Instance.new("Folder")
            local mid = Instance.new("Frame") mid.Parent = root
            local deep = Instance.new("Frame") deep.Name = "deep" deep.Parent = mid
            return root:FindFirstDescendant("deep") == deep
        "#,
        )
        .expect("eval");
        assert!(got);
    }

    // ── Ancestors ────────────────────────────────────────────────────────────

    #[test]
    fn the_ancestor_finders_walk_upward_and_stop_at_the_root() {
        let got: Vec<bool> = eval(
            r#"
            local screen = Instance.new("ScreenGui") screen.Name = "DewRoot"
            local frame = Instance.new("Frame") frame.Parent = screen
            local label = Instance.new("TextLabel") label.Parent = frame
            return {
                label:FindFirstAncestor("DewRoot") == screen,
                label:FindFirstAncestorOfClass("Frame") == frame,
                label:FindFirstAncestorWhichIsA("LayerCollector") == screen,
                label:FindFirstAncestor("nothing") == nil,
            }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, true, true, true]);
    }

    #[test]
    fn ancestry_is_not_reflexive() {
        // `Dom::is_ancestor_of` answers true for the self case because it exists
        // to refuse a cycle, where reflexive is the right reading. These two are
        // not that question, and reusing it unguarded made `x:IsDescendantOf(x)`
        // come back true.
        let got: Vec<bool> = eval(
            r#"
            local root = Instance.new("Folder")
            local child = Instance.new("Frame") child.Parent = root
            return {
                root:IsAncestorOf(child),
                child:IsDescendantOf(root),
                root:IsAncestorOf(root),
                root:IsDescendantOf(root),
                child:IsAncestorOf(root),
            }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, true, false, false, false]);
    }

    #[test]
    fn ancestry_spans_more_than_one_level() {
        let got: bool = eval(
            r#"
            local root = Instance.new("Folder")
            local mid = Instance.new("Frame") mid.Parent = root
            local leaf = Instance.new("TextLabel") leaf.Parent = mid
            return root:IsAncestorOf(leaf) and leaf:IsDescendantOf(root)
        "#,
        )
        .expect("eval");
        assert!(got);
    }

    // ── Destroy ──────────────────────────────────────────────────────────────

    #[test]
    fn a_handle_held_across_a_destroy_errors_rather_than_reading_a_stale_value() {
        // THE POINT OF CLEARING THE SLOT. A `Parent = nil` implementation passes
        // every other test in this file and fails this one, which is why it is
        // written as a read of a property that used to have a value.
        let message = err(r#"
            local f = Instance.new("Frame")
            f.Name = "gone"
            f:Destroy()
            return f.Name
        "#);
        assert!(message.contains("has been destroyed"), "{message}");

        let message = err(r#"
            local f = Instance.new("Frame")
            f:Destroy()
            f.Visible = false
        "#);
        assert!(message.contains("has been destroyed"), "{message}");
    }

    #[test]
    fn destroy_detaches_from_the_parent() {
        let got: Vec<bool> = eval(
            r#"
            local root = Instance.new("Folder")
            local child = Instance.new("Frame") child.Parent = root
            child:Destroy()
            return { #root:GetChildren() == 0, root:FindFirstChildOfClass("Frame") == nil }
        "#,
        )
        .expect("eval");
        assert_eq!(got, vec![true, true]);
    }

    #[test]
    fn destroy_takes_the_whole_subtree() {
        let message = err(r#"
            local root = Instance.new("Folder")
            local mid = Instance.new("Frame") mid.Parent = root
            local leaf = Instance.new("TextLabel") leaf.Parent = mid
            root:Destroy()
            return leaf.Name
        "#);
        assert!(message.contains("has been destroyed"), "{message}");
    }

    #[test]
    fn a_destroyed_handle_cannot_be_destroyed_again() {
        // A DIVERGENCE FROM THE ENGINE, TAKEN KNOWINGLY. Roblox leaves the
        // instance in existence -- parented to nil and locked -- so a second
        // `Destroy` is a no-op. Dew frees the slot, and the rule that follows from
        // that is one rule: every access through a handle whose slot is gone says
        // so. Carving `Destroy` out would mean `f.Name` raised and `f:Destroy()`
        // did not, which is a harder thing to hold in your head than "a destroyed
        // handle is finished", and it would be the only member that could be
        // reached on a node the arena no longer has.
        let message = err(r#"
            local f = Instance.new("Frame")
            f:Destroy()
            f:Destroy()
        "#);
        assert!(message.contains("has been destroyed"), "{message}");
    }

    #[test]
    fn clear_all_children_empties_one_level_and_destroys_what_it_removed() {
        let got: usize = eval(
            r#"
            local root = Instance.new("Folder")
            for _ = 1, 3 do
                Instance.new("Frame").Parent = root
            end
            root:ClearAllChildren()
            return #root:GetChildren()
        "#,
        )
        .expect("eval");
        assert_eq!(got, 0);

        let message = err(r#"
            local root = Instance.new("Folder")
            local child = Instance.new("Frame") child.Parent = root
            root:ClearAllChildren()
            return child.Name
        "#);
        assert!(message.contains("has been destroyed"), "{message}");
    }

    #[test]
    fn calling_a_method_on_a_destroyed_handle_says_so() {
        let message = err(r#"
            local f = Instance.new("Frame")
            f:Destroy()
            return f:GetChildren()
        "#);
        assert!(message.contains("has been destroyed"), "{message}");
    }

    // ── Dispatch ─────────────────────────────────────────────────────────────

    #[test]
    fn a_method_this_host_does_not_implement_is_still_not_a_valid_member() {
        // The dispatch runs BEFORE the reflection lookup, so this checks that it
        // falls through rather than swallowing everything shaped like a method.
        let message = err(r#"return Instance.new("Frame"):Clone()"#);
        assert!(message.contains("not a valid member"), "{message}");
    }

    #[test]
    fn a_method_is_a_value_that_carries_its_receiver() {
        // `local get = root.GetChildren` behaves as it does on the engine,
        // because the receiver is bound at lookup rather than read off the call.
        let got: usize = eval(
            r#"
            local root = Instance.new("Folder")
            Instance.new("Frame").Parent = root
            local get = root.GetChildren
            return #get()
        "#,
        )
        .expect("eval");
        assert_eq!(got, 1);
    }
}
