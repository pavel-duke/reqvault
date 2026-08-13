use tauri::State;

use crate::{models::CookieSummary, session::SessionState};

#[tauri::command]
pub fn list_cookies(
    state: State<'_, SessionState>,
    workspace_id: String,
) -> Result<Vec<CookieSummary>, String> {
    state.list(&workspace_id)
}

#[tauri::command]
pub fn delete_cookie(
    state: State<'_, SessionState>,
    workspace_id: String,
    cookie_id: String,
) -> Result<(), String> {
    state.delete(&workspace_id, &cookie_id)
}

#[tauri::command]
pub fn clear_cookies(state: State<'_, SessionState>, workspace_id: String) -> Result<(), String> {
    state.clear(&workspace_id)
}

#[tauri::command]
pub fn close_workspace_session(state: State<'_, SessionState>, workspace_id: String) {
    state.drop_workspace(&workspace_id);
}
