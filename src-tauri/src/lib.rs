pub mod data;
pub mod error;
pub mod llm;
pub mod memory;
pub mod models;
pub mod tools;
pub mod viewmodel;

use std::sync::{Arc, Mutex};

use memory::conversational_preferences::PreferencesStore;
use memory::facts::FactStore;
use memory::findings::FindingStore;
use memory::macro_conclusion::{LlmCredentials, MacroConclusionStore};
use memory::storage::MemoryStorage;
use viewmodel::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let loader = data::DataLoader::for_development();
    let mut registry = tools::registry::ToolRegistry::new();
    tools::describe::register_describe_tools(&mut registry, &loader);
    tools::query::register_query_tools(&mut registry, &loader);

    let storage = Arc::new(MemoryStorage::for_development());
    let fact_store = Arc::new(FactStore::new(storage.clone()));
    let finding_store = Arc::new(FindingStore::new(storage.clone(), fact_store.clone()));
    let macro_store = Arc::new(MacroConclusionStore::new(storage.clone()));
    let prefs_store = Arc::new(PreferencesStore::new(storage.clone()));

    memory::tools::register_memory_read_tools(
        &mut registry,
        fact_store.clone(),
        finding_store.clone(),
        macro_store.clone(),
        prefs_store.clone(),
    );

    let llm_creds = LlmCredentials {
        api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
        api_base_url: std::env::var("DEEPSEEK_API_BASE")
            .unwrap_or_else(|_| "https://api.deepseek.com/anthropic".into()),
        model: std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".into()),
    };
    memory::tools::register_memory_write_tools(
        &mut registry,
        fact_store,
        finding_store,
        macro_store,
        prefs_store,
        Arc::new(Mutex::new(Some(llm_creds))),
    );

    let llm_client = llm::client::LlmClient::new(Arc::new(registry))
        .expect("failed to create LlmClient");
    let session = Mutex::new(Some(llm::session::ChatSession::new(String::new())));

    tauri::Builder::default()
        .manage(AppState {
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
