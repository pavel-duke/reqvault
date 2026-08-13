use tauri::Manager;

use crate::{
    history,
    models::{HistoryEntry, HistorySettings, HistorySummary},
};

fn history_root(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| "Не удалось определить папку данных ReqVault".to_string())
}

#[tauri::command]
pub fn get_history_settings(
    app: tauri::AppHandle,
    workspace_id: String,
) -> Result<HistorySettings, String> {
    history::settings(&history_root(&app)?, &workspace_id)
}

#[tauri::command]
pub fn set_history_settings(
    app: tauri::AppHandle,
    workspace_id: String,
    settings: HistorySettings,
) -> Result<HistorySettings, String> {
    history::set_settings(&history_root(&app)?, &workspace_id, settings)
}

#[tauri::command]
pub fn list_history(
    app: tauri::AppHandle,
    workspace_id: String,
) -> Result<Vec<HistorySummary>, String> {
    history::list(&history_root(&app)?, &workspace_id)
}

#[tauri::command]
pub fn get_history_entry(
    app: tauri::AppHandle,
    workspace_id: String,
    id: String,
) -> Result<HistoryEntry, String> {
    history::get(&history_root(&app)?, &workspace_id, &id)
}

#[tauri::command]
pub fn delete_history_entry(
    app: tauri::AppHandle,
    workspace_id: String,
    id: String,
) -> Result<(), String> {
    history::delete(&history_root(&app)?, &workspace_id, &id)
}

#[tauri::command]
pub fn clear_history(app: tauri::AppHandle, workspace_id: String) -> Result<(), String> {
    history::clear(&history_root(&app)?, &workspace_id)
}
