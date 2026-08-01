// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod commands;
mod history;
#[cfg(target_os = "macos")]
mod macos_windowing;
mod models;
mod shell_integration;
mod terminal;
mod voice;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use ai::ModelManager;
use history::{HistoryConfig, HistoryStore};
use shell_integration::ShellIntegration;
use terminal::{PtyEventSinks, TerminalManager};

#[derive(Clone)]
pub struct AppState {
    pub model_manager: Arc<Mutex<ModelManager>>,
    pub terminal_manager: Arc<Mutex<TerminalManager>>,
    pub history_store: Option<HistoryStore>,
}

fn main() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(target_os = "macos")]
    let builder = builder
        .menu(macos_windowing::build_menu)
        .on_menu_event(macos_windowing::handle_menu_event)
        .on_window_event(macos_windowing::handle_window_event);

    let app = builder
        .setup(|app| {
            voice::initialize(app.handle().clone());
            let app_data_directory = app.path().app_data_dir()?;
            let resource_directory = app.path().resource_dir()?;
            let data_directory = app_data_directory.join("ai_data");
            std::fs::create_dir_all(&data_directory)?;

            #[cfg(target_os = "macos")]
            let history_store = match HistoryStore::open_with_keychain(HistoryConfig::new(
                app_data_directory.join("history.sqlite3"),
            )) {
                Ok(store) => Some(store),
                Err(error) => {
                    // Fail closed: terminal startup and in-memory history stay
                    // available, but a Keychain/decryption/migration failure
                    // must never fall back to a plaintext database.
                    eprintln!(
                        "Encrypted command history is unavailable; using in-memory history only: {error}"
                    );
                    None
                }
            };
            #[cfg(not(target_os = "macos"))]
            let history_store = {
                // Other platform releases need their own native secure-key
                // adapter before durable history can be enabled safely.
                eprintln!(
                    "Encrypted command history has no secure key provider on this platform; using in-memory history only"
                );
                None
            };
            let shell_integration = match ShellIntegration::install(&app_data_directory) {
                Ok(integration) => Some(integration),
                Err(error) => {
                    eprintln!("Shell integration is unavailable: {error}");
                    None
                }
            };

            // Initialize app state
            let model_manager = Arc::new(Mutex::new(ModelManager::new(
                data_directory,
                vec![
                    resource_directory.join("resources/models/terminal-assistant.gguf"),
                    resource_directory.join("models/terminal-assistant.gguf"),
                ],
            )));
            let encrypted_history_for_learning = history_store.clone();
            let output_app = app.handle().clone();
            let exit_app = app.handle().clone();
            let pty_event_sinks = PtyEventSinks::new(
                move |event| {
                    let _ = output_app.emit("terminal-output", event);
                },
                move |event| {
                    let _ = exit_app.emit("terminal-exit", event);
                },
            );
            let terminal_manager = Arc::new(Mutex::new(
                TerminalManager::with_runtime_integrations(pty_event_sinks, shell_integration),
            ));

            let app_state = AppState {
                model_manager: model_manager.clone(),
                terminal_manager,
                history_store,
            };

            app.manage(app_state);

            // Initialize local AI models on startup
            let _app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                println!("Initializing local command intelligence...");
                match model_manager.lock().await.load_model().await {
                    Ok(_) => {
                        println!("Local command intelligence is ready");

                        // Rebuild private adaptive state from the newest
                        // encrypted completions. The learning engine itself is
                        // memory-only, so no second plaintext command store is
                        // introduced and clearing encrypted history remains the
                        // single durable privacy control.
                        if let Some(history_store) = encrypted_history_for_learning {
                            let records = tauri::async_runtime::spawn_blocking(move || {
                                history_store.recent(None, 500)
                            })
                            .await;
                            if let Ok(Ok(mut records)) = records {
                                records.reverse();
                                let manager = model_manager.lock().await.clone();
                                for record in records {
                                    let Some(exit_code) = record.exit_code else {
                                        continue;
                                    };
                                    let context = format!(
                                        "cwd:{}\nshell:{}",
                                        record.cwd,
                                        record.shell.as_deref().unwrap_or("unknown")
                                    );
                                    manager
                                        .learn_from_completed_command(
                                            &record.session_id,
                                            &record.command,
                                            &context,
                                            exit_code == 0,
                                            record.duration_ms,
                                        )
                                        .await;
                                }
                            }
                        }
                    }
                    Err(e) => println!("Failed to initialize local command intelligence: {}", e),
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_terminal,
            commands::restart_terminal_session,
            commands::execute_command,
            commands::create_command_plan,
            commands::get_terminal_output,
            commands::ai_suggest_command,
            commands::ai_explain_command,
            commands::ai_fix_error,
            commands::ai_analyze_output,
            commands::get_smart_completions,
            commands::ai_translate_natural_language,
            commands::get_user_analytics,
            commands::update_ai_feedback,
            commands::close_terminal_session,
            commands::update_session_title,
            commands::resize_terminal,
            commands::write_to_terminal,
            commands::write_bytes_to_terminal,
            commands::get_terminal_snapshot,
            commands::attach_terminal_stream,
            commands::detach_terminal_stream,
            commands::sync_terminal_working_directory,
            commands::get_system_info,
            commands::get_context_suggestions,
            commands::get_all_sessions,
            commands::get_path_completions,
            commands::get_history_persistence_status,
            commands::clear_command_history,
            commands::get_command_history_for_navigation,
            commands::search_command_history,
            commands::search_command_history_records,
            commands::store_command_in_history,
            commands::record_shell_command,
            commands::get_recent_command_history,
            commands::initialize_ml_system,
            commands::get_local_llm_status,
            commands::cancel_ai_generation,
            commands::get_repo_info,
            commands::get_runtime_info,
            commands::get_parent_directories,
            commands::get_child_directories,
            commands::change_directory,
            commands::execute_file,
            commands::file_action,
            commands::validate_frequent_directories,
            commands::find_path_in_common_locations,
            commands::validate_and_correct_path,
            commands::get_voice_input_status,
            commands::request_voice_input_access,
            commands::start_voice_input,
            commands::stop_voice_input,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        #[cfg(target_os = "macos")]
        macos_windowing::handle_run_event(app_handle, &event);
    });
}
