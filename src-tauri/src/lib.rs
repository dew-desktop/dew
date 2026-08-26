pub mod luau;

use luau::bindings::WidgetRenderOutput;
use luau::LuauRuntime;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub struct AppState {
    pub runtime: Mutex<LuauRuntime>,
}

#[tauri::command]
fn get_widgets(state: State<'_, Arc<AppState>>) -> Vec<WidgetRenderOutput> {
    let rt = state.runtime.lock().unwrap();
    rt.render_widgets()
}

#[tauri::command]
fn click_widget(
    state: State<'_, Arc<AppState>>,
    widget_id: String,
    secondary: bool,
) -> Result<(), String> {
    let rt = state.runtime.lock().unwrap();
    rt.handle_widget_click(&widget_id, secondary)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn submit_palette(state: State<'_, Arc<AppState>>, query: String) -> Result<(), String> {
    let rt = state.runtime.lock().unwrap();
    rt.submit_palette_query(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_palette_status(state: State<'_, Arc<AppState>>) -> Option<(bool, String)> {
    let rt = state.runtime.lock().unwrap();
    rt.get_palette_status()
}

pub fn run() {
    let runtime = LuauRuntime::new().expect("Failed to initialize Luau runtime");

    // Discover and load all mods from standard paths
    let mut mod_dirs = Vec::new();

    // 1. User global directory (~/.dew/mods)
    if let Some(user_dir) = dirs::home_dir() {
        mod_dirs.push(user_dir.join(".dew").join("mods"));
    }

    // 2. Relative project directory (mods/)
    let project_mods = PathBuf::from("../mods");
    if project_mods.exists() {
        mod_dirs.push(project_mods);
    } else {
        mod_dirs.push(PathBuf::from("mods"));
    }

    let loaded = runtime.load_discovered_mods(&mod_dirs);
    println!("[Dew Core] Successfully loaded {} mod packages", loaded.len());

    let app_state = Arc::new(AppState {
        runtime: Mutex::new(runtime),
    });

    let state_for_setup = Arc::clone(&app_state);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |_app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let shortcut_str = shortcut.to_string();
                        println!("[Dew Global Hotkey Triggered] {}", shortcut_str);
                        let rt = state_for_setup.runtime.lock().unwrap();
                        let _ = rt.trigger_hotkey(&shortcut_str);
                    }
                })
                .build(),
        )
        .manage(Arc::clone(&app_state))
        .invoke_handler(tauri::generate_handler![
            get_widgets,
            click_widget,
            submit_palette,
            get_palette_status
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let state_clone = Arc::clone(&app_state);

            // Register standard global hotkey Alt+Shift+T if available
            if let Ok(shortcut) = "Alt+Shift+T".parse::<Shortcut>() {
                let _ = app.global_shortcut().register(shortcut);
            }

            // High-Precision Tick Loop via Tauri's native async runtime
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let widgets = {
                        let rt = state_clone.runtime.lock().unwrap();
                        rt.render_widgets()
                    };
                    let _ = app_handle.emit("dew:update-widgets", &widgets);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Dew application");
}
