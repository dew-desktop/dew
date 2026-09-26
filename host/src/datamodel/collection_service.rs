//! `CollectionService`: a general-purpose tag <-> instance multimap.
//!
//! A REAL, SHIPPED ROBLOX SERVICE, not a private lookalike built for
//! `StyleSheet`'s `.class` selector alone. The engine's own `CollectionService`
//! is used for physics groups, highlights and ad hoc tag queries well beyond
//! styling, and this is the same surface: `AddTag`/`RemoveTag`/`HasTag`,
//! `GetTagged`/`GetTags`, and `GetInstanceAddedSignal`/`GetInstanceRemovedSignal`.
//! There is no `StyleRule`, `.class` or selector code here at all -- that reads
//! this service, and is a later sprint's work.
//!
//! WHY THE TWO SIGNALS FIRE THROUGH `Dom`'s EXISTING ARENA. `signal.rs`'s own
//! module comment is explicit that a signal here is `(id, Kind)` looked up
//! against handler storage living beside the node -- collect, release, call,
//! never under the lock. `GetInstanceAddedSignal(tag)` and
//! `GetInstanceRemovedSignal(tag)` are service members rather than members of
//! any tagged instance, so `Dom::collection_service_id` mints one pseudo-node
//! the first time either is asked for, and `Kind::TagAdded`/`TagRemoved` fire on
//! it exactly as `Changed` fires on a real one. That reuses `Dom::connect`,
//! `listeners`, `disconnect` and `signal::fire` unchanged, rather than a second
//! handler list and a second collect-release-call discipline for one service.

use super::{dead_instance, handle, signal, InstanceRef, SharedDom};
use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataMethods};

pub struct CollectionServiceHandle {
    dom: SharedDom,
}

impl CollectionServiceHandle {
    pub fn new(dom: SharedDom) -> Self {
        Self { dom }
    }
}

/// The arena id behind a guest's `Instance` handle, or the destroyed-instance
/// error every other method on a dead handle already gives.
fn instance_id(dom: &SharedDom, instance: &LuaAnyUserData) -> LuaResult<usize> {
    let id = instance.borrow::<InstanceRef>()?.id;
    if dom.lock().expect("dom").exists(id) {
        Ok(id)
    } else {
        Err(dead_instance())
    }
}

impl UserData for CollectionServiceHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, _, ()| {
            Ok("CollectionService".to_string())
        });

        // IDEMPOTENT: tagging an already-tagged instance is a no-op on the
        // engine, not an error, and `Dom::add_tag` answers whether it actually
        // added anything so the signal only fires on a real change.
        methods.add_method(
            "AddTag",
            |_, this, (instance, tag): (LuaAnyUserData, String)| {
                let id = instance_id(&this.dom, &instance)?;
                let added = this.dom.lock().expect("dom").add_tag(id, &tag);
                if added {
                    let source = this.dom.lock().expect("dom").collection_service_id();
                    signal::fire(
                        &this.dom,
                        source,
                        &signal::Kind::TagAdded(tag),
                        &[LuaValue::UserData(instance)],
                    );
                }
                Ok(())
            },
        );

        // IDEMPOTENT THE OTHER WAY: removing a tag the instance never held is a
        // no-op, not an error, and fires nothing.
        methods.add_method(
            "RemoveTag",
            |_, this, (instance, tag): (LuaAnyUserData, String)| {
                let id = instance_id(&this.dom, &instance)?;
                let removed = this.dom.lock().expect("dom").remove_tag(id, &tag);
                if removed {
                    // THE GUARD MUST NOT OUTLIVE THIS STATEMENT: naming it as the
                    // scrutinee of the `if let` below would extend it across
                    // `signal::fire`'s own `dom.lock()`, on the same non-reentrant
                    // `Mutex` -- a self-deadlock this sprint hit once already.
                    let source = this.dom.lock().expect("dom").collection_service_id;
                    if let Some(source) = source {
                        signal::fire(
                            &this.dom,
                            source,
                            &signal::Kind::TagRemoved(tag),
                            &[LuaValue::UserData(instance)],
                        );
                    }
                }
                Ok(())
            },
        );

        methods.add_method(
            "HasTag",
            |_, this, (instance, tag): (LuaAnyUserData, String)| {
                let id = instance_id(&this.dom, &instance)?;
                Ok(this.dom.lock().expect("dom").has_tag(id, &tag))
            },
        );

        // EMPTY, NOT NIL, for a tag nothing holds -- the same answer
        // `GetChildren` gives an childless instance.
        methods.add_method("GetTagged", |lua, this, tag: String| {
            let ids = this.dom.lock().expect("dom").get_tagged(&tag);
            let out = lua.create_table()?;
            for (index, id) in ids.into_iter().enumerate() {
                out.raw_set(index + 1, handle(lua, &this.dom, id)?)?;
            }
            Ok(out)
        });

        methods.add_method("GetTags", |lua, this, instance: LuaAnyUserData| {
            let id = instance_id(&this.dom, &instance)?;
            let tags = this.dom.lock().expect("dom").get_tags(id);
            let out = lua.create_table()?;
            for (index, tag) in tags.into_iter().enumerate() {
                out.raw_set(index + 1, tag)?;
            }
            Ok(out)
        });

        methods.add_method("GetInstanceAddedSignal", |lua, this, tag: String| {
            let source = this.dom.lock().expect("dom").collection_service_id();
            let pseudo = InstanceRef {
                dom: this.dom.clone(),
                id: source,
            };
            signal::signal(lua, &pseudo, signal::Kind::TagAdded(tag))
        });

        methods.add_method("GetInstanceRemovedSignal", |lua, this, tag: String| {
            let source = this.dom.lock().expect("dom").collection_service_id();
            let pseudo = InstanceRef {
                dom: this.dom.clone(),
                id: source,
            };
            signal::signal(lua, &pseudo, signal::Kind::TagRemoved(tag))
        });
    }
}

/// `Destroy`'s hook into the tag multimap, called from [`signal::destroy`]
/// alongside `input::on_destroy` while `id` is still alive: an instance loses
/// its tags on destruction exactly as it does on an explicit `RemoveTag`, and
/// the engine fires `InstanceRemoved` for both.
pub fn on_destroy(lua: &Lua, dom: &SharedDom, id: usize) {
    let (removed, source) = {
        let mut guard = dom.lock().expect("dom");
        let removed = guard.untag_all(id);
        (removed, guard.collection_service_id)
    };
    if removed.is_empty() {
        return;
    }
    // A pseudo-instance is minted lazily by the first `Add`/`GetInstanceAdded`
    // call, so `None` here means nothing has ever been able to listen and
    // there is nothing to fire to.
    let Some(source) = source else { return };
    let Ok(instance) = handle(lua, dom, id) else {
        return;
    };
    let arg = LuaValue::UserData(instance);
    for tag in removed {
        signal::fire(
            dom,
            source,
            &signal::Kind::TagRemoved(tag),
            std::slice::from_ref(&arg),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamodel::{install, install_vocabulary, SharedDom};

    /// THE TREE IS BUILT FROM LUAU, as `members.rs`'s own tests build theirs --
    /// a service reached only from Rust is not proof a guest can reach it.
    fn eval<T: FromLuaMulti>(src: &str) -> LuaResult<T> {
        let lua = Lua::new();
        let dom = SharedDom::default();
        install(&lua, &dom).expect("install");
        install_vocabulary(&lua).expect("vocabulary");
        lua.load(src).eval()
    }

    #[test]
    fn get_service_answers_the_same_object_every_time() {
        let got: bool = eval(
            r#"
            return services:GetService("CollectionService") == services:GetService("CollectionService")
        "#,
        )
        .expect("eval");
        assert!(got);
    }

    #[test]
    fn get_service_refuses_a_name_this_host_does_not_provide() {
        let err = eval::<LuaValue>(r#"return services:GetService("Lighting")"#)
            .expect_err("expected this to fail")
            .to_string();
        assert!(err.contains("Lighting"), "{err}");
    }

    #[test]
    fn a_tag_round_trips_through_get_tagged_and_get_tags_then_clears() {
        let got: (usize, bool, usize, String, usize, usize) = eval(
            r#"
            local CollectionService = services:GetService("CollectionService")
            local f = Instance.new("Frame")
            CollectionService:AddTag(f, "Enemy")

            local tagged = CollectionService:GetTagged("Enemy")
            local tags = CollectionService:GetTags(f)

            CollectionService:RemoveTag(f, "Enemy")

            local taggedAfter = CollectionService:GetTagged("Enemy")
            local tagsAfter = CollectionService:GetTags(f)

            return #tagged, tagged[1] == f, #tags, tags[1], #taggedAfter, #tagsAfter
        "#,
        )
        .expect("eval");
        assert_eq!(got, (1, true, 1, "Enemy".to_string(), 0, 0));
    }

    #[test]
    fn an_unused_tag_reads_as_an_empty_table_not_nil() {
        let got: bool = eval(
            r#"
            local CollectionService = services:GetService("CollectionService")
            local tagged = CollectionService:GetTagged("NothingHoldsThis")
            return type(tagged) == "table" and #tagged == 0
        "#,
        )
        .expect("eval");
        assert!(got);
    }

    #[test]
    fn tagging_twice_and_untagging_an_absent_tag_are_both_no_ops() {
        let got: (usize, bool) = eval(
            r#"
            local CollectionService = services:GetService("CollectionService")
            local f = Instance.new("Frame")
            local adds = 0
            CollectionService:GetInstanceAddedSignal("Enemy"):Connect(function() adds += 1 end)

            CollectionService:AddTag(f, "Enemy")
            CollectionService:AddTag(f, "Enemy")

            -- Never held, and removing it must not error.
            CollectionService:RemoveTag(f, "NeverHeld")

            return adds, CollectionService:HasTag(f, "Enemy")
        "#,
        )
        .expect("eval");
        assert_eq!(got, (1, true));
    }

    #[test]
    fn added_and_removed_signals_fire_in_order_and_after_the_mutation_is_visible() {
        // THE TRAP THIS CATCHES: a signal firing before its own mutation lands
        // would let a handler observe a tag that `HasTag` still denies, or miss
        // one `HasTag` already granted.
        let got: (String, String, bool, bool) = eval(
            r#"
            local CollectionService = services:GetService("CollectionService")
            local f = Instance.new("Frame")
            local order = {}
            local hadTagWhenAdded, hadTagWhenRemoved

            CollectionService:GetInstanceAddedSignal("Enemy"):Connect(function(inst)
                table.insert(order, "added")
                hadTagWhenAdded = CollectionService:HasTag(inst, "Enemy")
            end)
            CollectionService:GetInstanceRemovedSignal("Enemy"):Connect(function(inst)
                table.insert(order, "removed")
                hadTagWhenRemoved = CollectionService:HasTag(inst, "Enemy")
            end)

            CollectionService:AddTag(f, "Enemy")
            CollectionService:RemoveTag(f, "Enemy")

            return order[1], order[2], hadTagWhenAdded, hadTagWhenRemoved
        "#,
        )
        .expect("eval");
        assert_eq!(
            got,
            ("added".to_string(), "removed".to_string(), true, false)
        );
    }

    #[test]
    fn destroying_a_tagged_instance_fires_instance_removed_and_clears_the_tag() {
        let got: (String, usize) = eval(
            r#"
            local CollectionService = services:GetService("CollectionService")
            local f = Instance.new("Frame")
            CollectionService:AddTag(f, "Enemy")

            local seen = "none"
            CollectionService:GetInstanceRemovedSignal("Enemy"):Connect(function()
                seen = "removed"
            end)

            f:Destroy()

            return seen, #CollectionService:GetTagged("Enemy")
        "#,
        )
        .expect("eval");
        assert_eq!(got, ("removed".to_string(), 0));
    }

    #[test]
    fn tagging_and_untagging_mark_the_tree_dirty() {
        // A `.class` selector reads a tag, so a tag change can change what
        // the cascade resolves -- the same reason `AddTag`/`RemoveTag` have
        // to mark the tree dirty that any other property write already does.
        let lua = Lua::new();
        let dom = SharedDom::default();
        install(&lua, &dom).expect("install");

        lua.load(
            r#"
            CollectionService = services:GetService("CollectionService")
            frame = Instance.new("Frame")
        "#,
        )
        .exec()
        .expect("setup");
        dom.lock().expect("dom").take_dirty();

        lua.load(r#"CollectionService:AddTag(frame, "Enemy")"#)
            .exec()
            .expect("add tag");
        assert!(
            dom.lock().expect("dom").take_dirty(),
            "AddTag should mark the tree dirty"
        );

        lua.load(r#"CollectionService:RemoveTag(frame, "Enemy")"#)
            .exec()
            .expect("remove tag");
        assert!(
            dom.lock().expect("dom").take_dirty(),
            "RemoveTag should mark the tree dirty"
        );
    }
}
