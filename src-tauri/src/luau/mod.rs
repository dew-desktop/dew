pub mod bindings;

use bindings::{setup_dew_bindings, HostState, SharedHostState, WidgetRenderOutput};
use mlua::prelude::*;
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
        // Strip potential UTF-8 Byte Order Mark (BOM) common on Windows
        let clean_source = source.strip_prefix('\u{feff}').unwrap_or(source);
        println!("[Luau Runtime] Loading mod: {}", name);
        self.lua.load(clean_source).set_name(name).exec()?;
        Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luau_runtime_and_timetracker_mod() {
        let runtime = LuauRuntime::new().expect("Failed to initialize Luau VM");
        let mod_source = include_str!("../../../mods/timetracker/timetracker.luau");

        runtime.load_mod_source("timetracker", mod_source).expect("Failed to execute timetracker.luau");

        // 1. Check initial render
        let widgets = runtime.render_widgets();
        assert_eq!(widgets.len(), 1);
        assert_eq!(widgets[0].id, "timetracker-main");
        assert_eq!(widgets[0].text, "00:00");
        assert_eq!(widgets[0].glow, Some(false));

        // 2. Click widget to toggle timer
        runtime.handle_widget_click("timetracker-main", false).expect("Click failed");

        // 3. Check updated render after click
        let updated_widgets = runtime.render_widgets();
        assert_eq!(updated_widgets[0].glow, Some(true)); // Timer is running!
    }
}
