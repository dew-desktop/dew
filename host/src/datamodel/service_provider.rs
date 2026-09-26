//! `services`, the real `ServiceProvider` (`Instance` -> `ServiceProvider` ->
//! `DataModel` in the reflection database), exposed without the `DataModel`
//! wrapped around it.
//!
//! WHY NOT `game`. `game:GetService` is `DataModel`'s inherited method, and
//! `DataModel` is a tree root with a `ClassName`, a `Parent` slot, and every
//! other `Instance` member besides -- none of which any service lookup needs.
//! Building that just to host one method would drag in a second, mostly-empty
//! `Instance` implementation for a shape nothing here asks for yet. Roblox
//! itself keeps `GetService` on `ServiceProvider`, a class with no properties
//! of its own (confirmed against `rbx_reflection_database`, not assumed from
//! the name) -- so exposing exactly that class, and no more, is not inventing
//! a shape, it is declining to add one Roblox itself did not need either.
//!
//! WHY A METHOD, NOT A FIELD LOOKUP. `services:GetService("CollectionService")`
//! is byte-for-byte what a script portable to the real engine already writes
//! as `game:GetService("CollectionService")`. A script that wants both needs
//! nothing more than `(game or services):GetService(name)` -- one `or`,
//! because the receiver is a `ServiceProvider` either way.
//!
//! IDENTITY IS CACHED, NOT MINTED PER CALL. `this.collection_service` is
//! built once, in [`super::install`], and `GetService` clones the handle
//! (cheap: `LuaAnyUserData` is a reference to one Lua object) rather than
//! creating a fresh userdata each time -- so two calls answer the same
//! object, and `==` on them agrees without a custom `Eq`, the same as two
//! `game:GetService("X")` calls do on the engine.

use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataMethods};

pub struct ServiceProviderHandle {
    collection_service: LuaAnyUserData,
}

impl ServiceProviderHandle {
    pub fn new(collection_service: LuaAnyUserData) -> Self {
        Self { collection_service }
    }
}

impl UserData for ServiceProviderHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, _, ()| Ok("services".to_string()));

        methods.add_method("GetService", |_, this, name: String| match name.as_str() {
            "CollectionService" => Ok(this.collection_service.clone()),
            other => Err(LuaError::runtime(format!(
                "{other} is not a service this host provides yet"
            ))),
        });
    }
}
