use mlua::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetRenderOutput {
    pub id: String,
    pub title: String,
    pub priority: i32,
    pub text: String,
    pub subtext: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub glow: Option<bool>,
    pub badge: Option<String>,
    pub progress: Option<f64>,
}

#[derive(Clone)]
pub struct WidgetDef {
    pub id: String,
    pub title: String,
    pub priority: i32,
    pub render_key: Arc<mlua::RegistryKey>,
    pub click_key: Option<Arc<mlua::RegistryKey>>,
    pub secondary_click_key: Option<Arc<mlua::RegistryKey>>,
}

#[derive(Clone)]
pub struct PaletteState {
    pub is_open: bool,
    pub placeholder: String,
    pub submit_key: Option<Arc<mlua::RegistryKey>>,
}

#[derive(Default)]
pub struct HostState {
    pub widgets: Vec<WidgetDef>,
    pub hotkeys: HashMap<String, Arc<mlua::RegistryKey>>,
    pub palette: Option<PaletteState>,
    pub storage: HashMap<String, String>,
    pub clipboard_text: String,
}

pub type SharedHostState = Arc<Mutex<HostState>>;

pub fn setup_dew_bindings(lua: &Lua, state: SharedHostState) -> LuaResult<()> {
    let dew_table = lua.create_table()?;

    // 1. dew.hud
    let hud_table = lua.create_table()?;
    let state_hud = Arc::clone(&state);

    let register_widget = lua.create_function(move |lua, def_table: LuaTable| {
        let id: String = def_table.get("id")?;
        let title: String = def_table.get("title").unwrap_or_else(|_| id.clone());
        let priority: i32 = def_table.get("priority").unwrap_or(0);

        let render_fn: LuaFunction = def_table.get("render")?;
        let render_key = Arc::new(lua.create_registry_value(render_fn)?);

        let click_key = if let Ok(click_fn) = def_table.get::<LuaFunction>("onClick") {
            Some(Arc::new(lua.create_registry_value(click_fn)?))
        } else {
            None
        };

        let sec_click_key = if let Ok(sec_fn) = def_table.get::<LuaFunction>("onSecondaryClick") {
            Some(Arc::new(lua.create_registry_value(sec_fn)?))
        } else {
            None
        };

        let mut lock = state_hud.lock().unwrap();
        lock.widgets.retain(|w| w.id != id);
        lock.widgets.push(WidgetDef {
            id,
            title,
            priority,
            render_key,
            click_key,
            secondary_click_key: sec_click_key,
        });

        Ok(())
    })?;

    hud_table.set("registerWidget", register_widget)?;
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

    // 3. dew.hotkeys
    let hotkeys_table = lua.create_table()?;
    let state_hotkeys = Arc::clone(&state);
    let reg_hotkey = lua.create_function(move |lua, (key_combo, callback): (String, LuaFunction)| {
        let key_ref = Arc::new(lua.create_registry_value(callback)?);
        let mut lock = state_hotkeys.lock().unwrap();
        lock.hotkeys.insert(key_combo, key_ref);
        Ok(())
    })?;
    hotkeys_table.set("register", reg_hotkey)?;
    dew_table.set("hotkeys", hotkeys_table)?;

    // 4. dew.commandPalette
    let palette_table = lua.create_table()?;
    let state_palette = Arc::clone(&state);
    let open_palette = lua.create_function(move |lua, opts_table: LuaTable| {
        let placeholder: String = opts_table.get("placeholder").unwrap_or_else(|_| "Search or execute...".into());
        let submit_key = if let Ok(submit_fn) = opts_table.get::<LuaFunction>("onSubmit") {
            Some(Arc::new(lua.create_registry_value(submit_fn)?))
        } else {
            None
        };

        let mut lock = state_palette.lock().unwrap();
        lock.palette = Some(PaletteState {
            is_open: true,
            placeholder,
            submit_key,
        });

        Ok(())
    })?;
    palette_table.set("open", open_palette)?;

    let state_palette_close = Arc::clone(&state);
    let close_palette = lua.create_function(move |_lua, ()| {
        let mut lock = state_palette_close.lock().unwrap();
        if let Some(ref mut p) = lock.palette {
            p.is_open = false;
        }
        Ok(())
    })?;
    palette_table.set("close", close_palette)?;
    dew_table.set("commandPalette", palette_table)?;

    // 5. dew.storage
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

    // 6. dew.clipboard
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

    // 7. dew.notifications
    let notif_table = lua.create_table()?;
    let show_notif = lua.create_function(|_lua, msg: LuaValue| {
        match msg {
            LuaValue::String(s) => {
                println!("[Dew Notification] {}", s.to_string_lossy());
            }
            LuaValue::Table(t) => {
                if let Ok(body) = t.get::<String>("body") {
                    println!("[Dew Notification] {}", body);
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    notif_table.set("show", show_notif)?;
    dew_table.set("notifications", notif_table)?;

    // 8. dew.audio
    let audio_table = lua.create_table()?;
    let play_chime = lua.create_function(|_lua, chime_type: String| {
        println!("[Dew Audio Chime] {}", chime_type);
        Ok(())
    })?;
    audio_table.set("playChime", play_chime)?;
    dew_table.set("audio", audio_table)?;

    // Bind to globals
    lua.globals().set("dew", dew_table)?;

    Ok(())
}
