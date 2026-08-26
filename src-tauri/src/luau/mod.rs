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
        let state_lock = self.state.lock().unwrap();
        let mut rendered = Vec::new();

        for def in &state_lock.widgets {
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
        let state_lock = self.state.lock().unwrap();
        for def in &state_lock.widgets {
            if def.id == widget_id {
                let key_opt = if secondary {
                    &def.secondary_click_key
                } else {
                    &def.click_key
                };

                if let Some(key) = key_opt {
                    if let Ok(callback) = self.lua.registry_value::<LuaFunction>(&**key) {
                        callback.call::<()>(())?;
                    }
                }
                break;
            }
        }
        Ok(())
    }

    pub fn trigger_hotkey(&self, shortcut: &str) -> Result<bool, LuaError> {
        let state_lock = self.state.lock().unwrap();
        if let Some(key_ref) = state_lock.hotkeys.get(shortcut) {
            if let Ok(callback) = self.lua.registry_value::<LuaFunction>(&**key_ref) {
                callback.call::<()>(())?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn get_palette_status(&self) -> Option<(bool, String)> {
        let state_lock = self.state.lock().unwrap();
        state_lock.palette.as_ref().map(|p| (p.is_open, p.placeholder.clone()))
    }

    pub fn submit_palette_query(&self, query: &str) -> Result<(), LuaError> {
        let state_lock = self.state.lock().unwrap();
        if let Some(ref palette) = state_lock.palette {
            if let Some(ref key) = palette.submit_key {
                if let Ok(submit_fn) = self.lua.registry_value::<LuaFunction>(&**key) {
                    submit_fn.call::<()>(query)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_mod_discovery_and_execution() {
        let runtime = LuauRuntime::new().expect("Failed to initialize Luau VM");
        let mods_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mods");

        let discovered = runtime.load_discovered_mods(&[mods_dir]);
        assert!(discovered.len() >= 2, "Expected at least 2 discovered mods");

        let widgets = runtime.render_widgets();
        assert!(widgets.len() >= 2, "Expected at least 2 rendered widgets (timetracker and calculator)");

        // Verify calculator widget works
        let calc_widget = widgets.iter().find(|w| w.id == "calculator-widget").expect("Calculator widget not found");
        assert_eq!(calc_widget.title, "Calculator");

        // Verify timetracker widget works
        let timer_widget = widgets.iter().find(|w| w.id == "timetracker-main").expect("Timer widget not found");
        assert_eq!(timer_widget.title, "Time Tracker");
    }
}
