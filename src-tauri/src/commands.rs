use tauri::State;

#[tauri::command]
pub async fn test_command() -> Result<String, String> {
    Ok("Test successful".to_string())
}
