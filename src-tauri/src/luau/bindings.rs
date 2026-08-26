use mlua::prelude::*;
use serde::{Deserialize, Serialize};
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

#[derive(Default)]
pub struct HostState {
    pub widgets: Vec<WidgetDef>,
    pub notifications: Vec<String>,
}

pub type SharedHostState = Arc<Mutex<HostState>>;

pub fn setup_dew_bindings(lua: &Lua, state: SharedHostState) -> LuaResult<()> {
    let dew_table = lua.create_table()?;

    // 1. dew.hud
    let hud_table = lua.create_table()?;
    let state_clone = Arc::clone(&state);

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

        let mut lock = state_clone.lock().unwrap();
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

    let refresh = lua.create_function(|_lua, ()| Ok(()))?;
    hud_table.set("refresh", refresh)?;

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
    let reg_hotkey = lua.create_function(|_lua, (_key, _cb): (String, LuaFunction)| Ok(()))?;
    hotkeys_table.set("register", reg_hotkey)?;
    dew_table.set("hotkeys", hotkeys_table)?;

    // 4. dew.notifications
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

    // 5. dew.audio
    let audio_table = lua.create_table()?;
    let play_chime = lua.create_function(|_lua, chime_type: String| {
        println!("[Dew Audio Chime] {}", chime_type);
        Ok(())
    })?;
    audio_table.set("playChime", play_chime)?;
    dew_table.set("audio", audio_table)?;

    // 6. dew.commandPalette
    let palette_table = lua.create_table()?;
    let open_palette = lua.create_function(|_lua, _opts: LuaValue| Ok(()))?;
    palette_table.set("open", open_palette)?;
    palette_table.set("close", lua.create_function(|_lua, ()| Ok(()))?)?;
    dew_table.set("commandPalette", palette_table)?;

    // Register globally
    lua.globals().set("dew", dew_table)?;

    Ok(())
}
