use mlua::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCommand {
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub stroke_width: f32,
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub color_a: u8,
    pub text: Option<String>,
}

#[derive(Clone)]
pub struct ComponentDef {
    pub id: String,
    pub render_fn: Arc<mlua::RegistryKey>,
    pub click_fn: Option<Arc<mlua::RegistryKey>>,
}

#[derive(Default)]
pub struct HostState {
    pub components: Vec<ComponentDef>,
    pub storage: HashMap<String, String>,
    pub clipboard_text: String,
}

pub type SharedHostState = Arc<Mutex<HostState>>;

pub fn setup_dew_bindings(lua: &Lua, state: SharedHostState) -> LuaResult<()> {
    let dew_table = lua.create_table()?;

    // 1. dew.hud
    let hud_table = lua.create_table()?;
    let state_hud = Arc::clone(&state);

    let register_component = lua.create_function(move |lua, def_table: LuaTable| {
        let id: String = def_table.get("id")?;
        let render_fn: LuaFunction = def_table.get("render")?;
        let render_key = Arc::new(lua.create_registry_value(render_fn)?);

        let click_key = if let Ok(click_fn) = def_table.get::<LuaFunction>("onClick") {
            Some(Arc::new(lua.create_registry_value(click_fn)?))
        } else {
            None
        };

        let mut lock = state_hud.lock().unwrap();
        lock.components.retain(|c| c.id != id);
        lock.components.push(ComponentDef {
            id,
            render_fn: render_key,
            click_fn: click_key,
        });

        Ok(())
    })?;
    hud_table.set("registerComponent", register_component)?;
    hud_table.set("refresh", lua.create_function(|_lua, ()| Ok(()))?)?;
    dew_table.set("hud", hud_table)?;

    // 2. dew.time
    let time_table = lua.create_table()?;
    let time_now = lua.create_function(|_lua, ()| {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        Ok(now)
    })?;
    time_table.set("now", time_now)?;

    let format_duration = lua.create_function(|_lua, seconds: f64| {
        let total_secs = seconds.floor() as u64;
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        let s = total_secs % 60;
        if h > 0 {
            Ok(format!("{:02}:{:02}:{:02}", h, m, s))
        } else {
            Ok(format!("{:02}:{:02}", m, s))
        }
    })?;
    time_table.set("formatDuration", format_duration)?;
    dew_table.set("time", time_table)?;

    // 3. dew.storage
    let storage_table = lua.create_table()?;
    let state_storage_get = Arc::clone(&state);
    let storage_get = lua.create_function(move |_lua, key: String| {
        let lock = state_storage_get.lock().unwrap();
        Ok(lock.storage.get(&key).cloned())
    })?;
    storage_table.set("get", storage_get)?;

    let state_storage_set = Arc::clone(&state);
    let storage_set = lua.create_function(move |_lua, (key, value): (String, String)| {
        let mut lock = state_storage_set.lock().unwrap();
        lock.storage.insert(key, value);
        Ok(())
    })?;
    storage_table.set("set", storage_set)?;
    dew_table.set("storage", storage_table)?;

    // 4. dew.clipboard
    let clip_table = lua.create_table()?;
    let state_clip_read = Arc::clone(&state);
    let clip_read = lua.create_function(move |_lua, ()| {
        let lock = state_clip_read.lock().unwrap();
        Ok(lock.clipboard_text.clone())
    })?;
    clip_table.set("readText", clip_read)?;

    let state_clip_write = Arc::clone(&state);
    let clip_write = lua.create_function(move |_lua, text: String| {
        let mut lock = state_clip_write.lock().unwrap();
        lock.clipboard_text = text;
        Ok(())
    })?;
    clip_table.set("writeText", clip_write)?;
    dew_table.set("clipboard", clip_table)?;

    // 5. dew.notifications & audio
    let notif_table = lua.create_table()?;
    let show_notif = lua.create_function(|_lua, msg: LuaValue| {
        if let LuaValue::String(s) = msg {
            println!("[Dew Notification] {}", s.to_string_lossy());
        }
        Ok(())
    })?;
    notif_table.set("show", show_notif)?;
    dew_table.set("notifications", notif_table)?;

    let audio_table = lua.create_table()?;
    let play_chime = lua.create_function(|_lua, chime: String| {
        println!("[Dew Chime] {}", chime);
        Ok(())
    })?;
    audio_table.set("playChime", play_chime)?;
    dew_table.set("audio", audio_table)?;

    // Bind to globals
    lua.globals().set("dew", dew_table)?;

    // Load & Bind @dew/aether into globals
    let aether_source = include_str!("../../../types/aether.luau");
    let clean_aether = aether_source.strip_prefix('\u{feff}').unwrap_or(aether_source);
    let aether_module: LuaTable = lua.load(clean_aether).set_name("@dew/aether").eval()?;
    lua.globals().set("Aether", aether_module)?;

    Ok(())
}
