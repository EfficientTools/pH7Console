use crate::{AppState};
use crate::ai::{AIResponse};
use crate::terminal::CommandExecution;
use tauri::State;

#[tauri::command]
pub async fn create_terminal(
    state: State<'_, AppState>,
    title: Option<String>
) -> Result<String, String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    
    terminal_manager.create_session(title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_command(
    state: State<'_, AppState>,
    session_id: String,
    command: String
) -> Result<CommandExecution, String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    
    terminal_manager.execute_command(&session_id, &command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_terminal_output(
    state: State<'_, AppState>,
    _session_id: String,
    limit: Option<usize>
) -> Result<Vec<CommandExecution>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    
    let history = terminal_manager.get_command_history(limit);
    Ok(history.into_iter().cloned().collect())
}

#[tauri::command]
pub async fn ai_suggest_command(
    state: State<'_, AppState>,
    context: String,
    intent: Option<String>
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    
    let prompt = match intent {
        Some(i) => format!("Suggest commands for: {}. Context: {}", i, context),
        None => format!("Suggest next commands based on context: {}", context),
    };
    
    Ok(model_manager.generate_response(&prompt, Some(&context)).await)
}

#[tauri::command]
pub async fn ai_explain_command(
    state: State<'_, AppState>,
    command: String
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    let prompt = format!("Explain this command: {}", command);
    
    Ok(model_manager.generate_response(&prompt, None).await)
}

#[tauri::command]
pub async fn ai_fix_error(
    state: State<'_, AppState>,
    error_output: String,
    command: String,
    context: Option<String>
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    
    let prompt = format!(
        "Fix this error - Command: '{}', Error: '{}', Context: '{}'",
        command, error_output, context.unwrap_or_default()
    );
    
    Ok(model_manager.generate_response(&prompt, Some(&error_output)).await)
}

#[tauri::command]
pub async fn ai_analyze_output(
    state: State<'_, AppState>,
    output: String,
    command: String
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    
    let prompt = format!(
        "Analyze this command output and provide insights: Command: '{}', Output: '{}'",
        command, output
    );
    
    Ok(model_manager.generate_response(&prompt, Some(&output)).await)
}

#[tauri::command]
pub async fn get_smart_completions(
    state: State<'_, AppState>,
    partial_command: String,
    session_id: String
) -> Result<Vec<String>, String> {
    let model_manager = state.inner().model_manager.lock().await;
    let terminal_manager = state.inner().terminal_manager.lock().await;
    
    let context = terminal_manager.get_smart_context(&session_id);
    
    Ok(model_manager.get_smart_completions(&partial_command, &context).await)
}

#[tauri::command]
pub async fn ai_translate_natural_language(
    state: State<'_, AppState>,
    natural_language: String,
    context: String
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    
    let prompt = format!(
        "Translate this natural language request to shell commands: '{}'. Context: {}",
        natural_language, context
    );
    
    Ok(model_manager.generate_response(&prompt, Some(&context)).await)
}

#[tauri::command]
pub async fn test_command() -> Result<String, String> {
    Ok("Test successful".to_string())
}
