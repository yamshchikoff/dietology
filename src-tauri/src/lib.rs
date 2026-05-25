pub mod data;
pub mod error;
pub mod llm;
pub mod models;
pub mod tools;
pub mod viewmodel;

use std::sync::{Arc, Mutex};

use viewmodel::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let loader = data::DataLoader::for_development();
    let mut registry = tools::registry::ToolRegistry::new();
    tools::describe::register_describe_tools(&mut registry, &loader);
    tools::query::register_query_tools(&mut registry, &loader);

    let llm_client = llm::client::LlmClient::new(Arc::new(registry))
        .expect("failed to create LlmClient");
    let session = Mutex::new(llm::session::ChatSession::new(String::new()));

    tauri::Builder::default()
        .manage(AppState {
            loader,
            llm_client,
            session,
        })
        .invoke_handler(tauri::generate_handler![
            viewmodel::new_chat,
            viewmodel::send_message,
            viewmodel::get_messages,
            viewmodel::save_session,
            viewmodel::load_session,
            viewmodel::clear_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
