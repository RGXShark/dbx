use std::sync::Arc;
use tauri::State;

use super::connection::AppState;
use dbx_core::ai::AiConfigItem;

#[tauri::command]
pub async fn save_ai_configs(state: State<'_, Arc<AppState>>, configs: Vec<AiConfigItem>) -> Result<(), String> {
    crate::enterprise_ai::ensure_personal_mode()?;
    state.storage.save_ai_configs(&configs).await
}

#[tauri::command]
pub async fn load_ai_configs(state: State<'_, Arc<AppState>>) -> Result<Vec<AiConfigItem>, String> {
    if let Some(item) = crate::enterprise_ai::public_config(&state.storage).await? {
        return Ok(vec![item]);
    }
    state.storage.load_ai_configs().await
}

#[tauri::command]
pub async fn set_default_ai_config(state: State<'_, Arc<AppState>>, config_id: String) -> Result<(), String> {
    crate::enterprise_ai::ensure_personal_mode()?;
    state.storage.set_default_ai_config(&config_id).await
}

#[tauri::command]
pub async fn save_ai_config_item(state: State<'_, Arc<AppState>>, config: AiConfigItem) -> Result<(), String> {
    crate::enterprise_ai::ensure_personal_mode()?;
    state.storage.save_ai_config_item(&config).await
}

#[tauri::command]
pub async fn delete_ai_config(state: State<'_, Arc<AppState>>, config_id: String) -> Result<(), String> {
    crate::enterprise_ai::ensure_personal_mode()?;
    state.storage.delete_ai_config(&config_id).await
}
