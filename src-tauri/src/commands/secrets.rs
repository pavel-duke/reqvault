use crate::secrets::{self, KeyringBackend};

#[tauri::command]
pub fn list_secrets(workspace_id: String) -> Result<Vec<String>, String> {
    let backend = KeyringBackend::new(&workspace_id).map_err(|error| error.to_string())?;
    secrets::list(&backend).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_secret(
    workspace_id: String,
    name: String,
    value: String,
) -> Result<Vec<String>, String> {
    let backend = KeyringBackend::new(&workspace_id).map_err(|error| error.to_string())?;
    secrets::save(&backend, &name, &value).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_secret(workspace_id: String, name: String) -> Result<Vec<String>, String> {
    let backend = KeyringBackend::new(&workspace_id).map_err(|error| error.to_string())?;
    secrets::delete(&backend, &name).map_err(|error| error.to_string())
}
