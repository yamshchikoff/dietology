pub mod data;
pub mod error;
pub mod llm;
pub mod models;
pub mod tools;

use std::sync::Mutex;

pub struct AppState {
    pub loader: data::DataLoader,
    pub registry: Mutex<tools::registry::ToolRegistry>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let loader = data::DataLoader::for_development();
    let mut registry = tools::registry::ToolRegistry::new();
    tools::describe::register_describe_tools(&mut registry, &loader);
    tools::query::register_query_tools(&mut registry, &loader);

    tauri::Builder::default()
        .manage(AppState {
            loader,
            registry: Mutex::new(registry),
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
