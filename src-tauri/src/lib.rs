pub mod data;
pub mod error;
pub mod models;
pub mod tools;

use std::sync::Mutex;

pub struct AppState {
    pub loader: data::DataLoader,
    pub registry: Mutex<tools::registry::ToolRegistry>,
}

#[tauri::command]
fn status(state: tauri::State<AppState>) -> Result<String, String> {
    let _ = &state.loader;
    let _ = &state.registry;
    Ok(r#"{"status": "ok", "version": "0.1.0"}"#.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let loader = data::DataLoader::for_development();
    let mut registry = tools::registry::ToolRegistry::new();
    tools::registry::register_describe_tools(&mut registry);

    tauri::Builder::default()
        .manage(AppState {
            loader,
            registry: Mutex::new(registry),
        })
        .invoke_handler(tauri::generate_handler![status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
