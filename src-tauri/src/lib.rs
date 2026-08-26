pub mod luau;

use luau::bindings::WidgetRenderOutput;
use luau::LuauRuntime;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, State};

pub struct AppState {
    pub runtime: Mutex<LuauRuntime>,
}

#[tauri::command]
fn get_widgets(state: State<'_, Arc<AppState>>) -> Vec<WidgetRenderOutput> {
    let rt = state.runtime.lock().unwrap();
    rt.render_widgets()
}

#[tauri::command]
fn click_widget(state: State<'_, Arc<AppState>>, widget_id: String, secondary: bool) -> Result<(), String> {
    let rt = state.runtime.lock().unwrap();
    rt.handle_widget_click(&widget_id, secondary)
        .map_err(|e| e.to_string())
}

pub fn run() {
    let runtime = LuauRuntime::new().expect("Failed to initialize Luau runtime");

    // Load initial reference mod: timetracker
    let default_mod = include_str!("../../mods/timetracker/timetracker.luau");
    if let Err(e) = runtime.load_mod_source("timetracker", default_mod) {
        eprintln!("[Dew Core] Error loading timetracker mod: {}", e);
    }

    let app_state = Arc::new(AppState {
        runtime: Mutex::new(runtime),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::clone(&app_state))
        .invoke_handler(tauri::generate_handler![get_widgets, click_widget])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let state_clone = Arc::clone(&app_state);

            // 100ms High-Precision Tick Loop to stream HUD state
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(100));
                loop {
                    interval.tick().await;
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
