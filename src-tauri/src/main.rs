// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod terminal;
mod commands;
mod models;

use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

use ai::ModelManager;
use terminal::TerminalManager;

#[derive(Clone)]
pub struct AppState {
    pub model_manager: Arc<Mutex<ModelManager>>,
    pub terminal_manager: Arc<Mutex<TerminalManager>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize app state
            let model_manager = Arc::new(Mutex::new(ModelManager::new()));
            let terminal_manager = Arc::new(Mutex::new(TerminalManager::new()));
            
            let app_state = AppState {
                model_manager,
                terminal_manager,
            };
            
            app.manage(app_state);
            
            // Initialize local AI models on startup
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                println!("🤖 Initializing local AI models...");
                // Model initialization will happen here
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
