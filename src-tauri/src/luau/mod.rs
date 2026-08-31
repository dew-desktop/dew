pub mod bindings;
pub mod manifest;

use bindings::{setup_dew_bindings, HostState, SharedHostState, WidgetRenderOutput};
use manifest::{discover_mods, DiscoveredMod};
use mlua::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct LuauRuntime {
    lua: Lua,
    state: SharedHostState,
}

impl LuauRuntime {
    pub fn new() -> Result<Self, LuaError> {
        let lua = Lua::new();
        let state = Arc::new(Mutex::new(HostState::default()));

        setup_dew_bindings(&lua, Arc::clone(&state))?;

        Ok(Self { lua, state })
    }

    pub fn load_mod_source(&self, name: &str, source: &str) -> Result<(), LuaError> {
        let clean_source = source.strip_prefix('\u{feff}').unwrap_or(source);
        println!("[Luau Runtime] Loading mod: {}", name);
        self.lua.load(clean_source).set_name(name).exec()?;
        Ok(())
    }

    pub fn load_discovered_mods(&self, base_dirs: &[PathBuf]) -> Vec<DiscoveredMod> {
        let mods = discover_mods(base_dirs);
        for m in &mods {
            if let Ok(source) = fs::read_to_string(&m.script_path) {
                if let Err(e) = self.load_mod_source(&m.manifest.id, &source) {
                    eprintln!("[Luau Runtime] Error executing mod '{}': {}", m.manifest.id, e);
                }
            }
        }
        mods
    }

    pub fn render_widgets(&self) -> Vec<WidgetRenderOutput> {
        let defs: Vec<_> = {
            let state_lock = self.state.lock().unwrap();
            state_lock.widgets.clone()
        };

        let mut rendered = Vec::new();

        for def in defs {
            if let Ok(render_fn) = self.lua.registry_value::<LuaFunction>(&*def.render_key) {
                if let Ok(result_table) = render_fn.call::<LuaTable>(()) {
                    let text: String = result_table.get("text").unwrap_or_default();
                    let subtext: Option<String> = result_table.get("subtext").ok();
                    let icon: Option<String> = result_table.get("icon").ok();
                    let color: Option<String> = result_table.get("color").ok();
                    let glow: Option<bool> = result_table.get("glow").ok();
                    let badge: Option<String> = result_table.get("badge").ok();
                    let progress: Option<f64> = result_table.get("progress").ok();

                    rendered.push(WidgetRenderOutput {
                        id: def.id.clone(),
                        title: def.title.clone(),
                        priority: def.priority,
                        text,
                        subtext,
                        icon,
                        color,
                        glow,
                        badge,
                        progress,
                    });
                }
            }
        }

        rendered.sort_by(|a, b| b.priority.cmp(&a.priority));
        rendered
    }

    pub fn handle_widget_click(&self, widget_id: &str, secondary: bool) -> Result<(), LuaError> {
        let callback_opt = {
            let state_lock = self.state.lock().unwrap();
            state_lock.widgets.iter().find(|w| w.id == widget_id).and_then(|def| {
                if secondary {
                    def.secondary_click_key.clone()
                } else {
                    def.click_key.clone()
                }
            })
        };

        if let Some(key) = callback_opt {
            if let Ok(callback) = self.lua.registry_value::<LuaFunction>(&*key) {
                callback.call::<()>(())?;
            }
        }
        Ok(())
    }

    pub fn trigger_hotkey(&self, shortcut: &str) -> Result<bool, LuaError> {
        let key_opt = {
            let state_lock = self.state.lock().unwrap();
            state_lock.hotkeys.get(shortcut).cloned()
        };

        if let Some(key_ref) = key_opt {
            if let Ok(callback) = self.lua.registry_value::<LuaFunction>(&*key_ref) {
                callback.call::<()>(())?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn take_pending_palette_open(&self) -> Option<String> {
        let mut state_lock = self.state.lock().unwrap();
        state_lock.pending_palette_open.take()
    }

    pub fn submit_palette_query(&self, query: &str) -> Result<(), LuaError> {
        let submit_key_opt = {
            let state_lock = self.state.lock().unwrap();
            state_lock.palette_submit_key.clone()
        };

        if let Some(key) = submit_key_opt {
            if let Ok(submit_fn) = self.lua.registry_value::<LuaFunction>(&*key) {
                submit_fn.call::<()>(query)?;
            }
        }
        Ok(())
    }
}
