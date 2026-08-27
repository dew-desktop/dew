pub mod bindings;

use bindings::{setup_dew_bindings, ComponentDef, DrawCommand, HostState, SharedHostState};
use mlua::prelude::*;
use std::fs;
use std::path::Path;
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

    pub fn load_script(&self, name: &str, source: &str) -> Result<(), LuaError> {
        let clean_source = source.strip_prefix('\u{feff}').unwrap_or(source);
        println!("[Dew Luau Runtime] Executing module: {}", name);
        self.lua.load(clean_source).set_name(name).exec()?;
        Ok(())
    }

    pub fn load_mods_from_dir(&self, base_dir: &Path) {
        if let Ok(entries) = fs::read_dir(base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let script_path = path.join(format!("{}.luau", path.file_name().unwrap().to_str().unwrap()));
                    let target = if script_path.exists() {
                        script_path
                    } else {
                        path.join("main.luau")
                    };

                    if target.exists() {
                        if let Ok(content) = fs::read_to_string(&target) {
                            let mod_id = path.file_name().unwrap().to_str().unwrap();
                            if let Err(e) = self.load_script(mod_id, &content) {
                                eprintln!("[Dew Luau Runtime] Error loading mod '{}': {}", mod_id, e);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn render_frame(&self) -> Vec<DrawCommand> {
        let components: Vec<ComponentDef> = {
            let state_lock = self.state.lock().unwrap();
            state_lock.components.clone()
        };

        let mut commands = Vec::new();

        for comp in components {
            if let Ok(render_fn) = self.lua.registry_value::<LuaFunction>(&*comp.render_fn) {
                if let Ok(res_table) = render_fn.call::<LuaTable>(()) {
                    // 1. If result is a Roblox/Aether Virtual Instance (has ClassName), compile it!
                    let compiled_table = if res_table.contains_key("ClassName").unwrap_or(false) {
                        if let Ok(aether_global) = self.lua.globals().get::<LuaTable>("Aether") {
                            if let Ok(compile_fn) = aether_global.get::<LuaFunction>("compileTree") {
                                let bounds = self.lua.create_table().unwrap();
                                bounds.set("x", 0.0).unwrap();
                                bounds.set("y", 0.0).unwrap();
                                bounds.set("w", 380.0).unwrap();
                                bounds.set("h", 56.0).unwrap();
                                compile_fn.call::<LuaTable>((res_table.clone(), bounds)).unwrap_or(res_table)
                            } else {
                                res_table
                            }
                        } else {
                            res_table
                        }
                    } else {
                        res_table
                    };

                    // 2. Extract compiled draw commands
                    if let Ok(cmds) = compiled_table.get::<LuaTable>("commands") {
                        for pair in cmds.pairs::<i32, LuaTable>() {
                            if let Ok((_, cmd_tbl)) = pair {
                                let kind: String = cmd_tbl.get("kind").unwrap_or_else(|_| "rect".into());
                                let x: f32 = cmd_tbl.get("x").unwrap_or(0.0);
                                let y: f32 = cmd_tbl.get("y").unwrap_or(0.0);
                                let w: f32 = cmd_tbl.get("w").unwrap_or(0.0);
                                let h: f32 = cmd_tbl.get("h").unwrap_or(0.0);
                                let radius: f32 = cmd_tbl.get("radius").unwrap_or(0.0);
                                let stroke_width: f32 = cmd_tbl.get("stroke_width").unwrap_or(0.0);
                                let color_r: u8 = cmd_tbl.get("r").unwrap_or(255);
                                let color_g: u8 = cmd_tbl.get("g").unwrap_or(255);
                                let color_b: u8 = cmd_tbl.get("b").unwrap_or(255);
                                let color_a: u8 = cmd_tbl.get("a").unwrap_or(255);
                                let text: Option<String> = cmd_tbl.get("text").ok();

                                commands.push(DrawCommand {
                                    kind,
                                    x,
                                    y,
                                    w,
                                    h,
                                    radius,
                                    stroke_width,
                                    color_r,
                                    color_g,
                                    color_b,
                                    color_a,
                                    text,
                                });
                            }
                        }
                    }
                }
            }
        }

        commands
    }

    pub fn handle_click(&self, comp_id: &str) -> Result<(), LuaError> {
        let click_fn_opt = {
            let state_lock = self.state.lock().unwrap();
            state_lock.components.iter().find(|c| c.id == comp_id).and_then(|c| c.click_fn.clone())
        };

        if let Some(key) = click_fn_opt {
            if let Ok(callback) = self.lua.registry_value::<LuaFunction>(&*key) {
                callback.call::<()>(())?;
            }
        }
        Ok(())
    }
}
