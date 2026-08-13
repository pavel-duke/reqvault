use std::path::Path;

use crate::{
    models::{
        EnvironmentFile, EnvironmentSummary, MigrationPlan, MigrationResult, RequestFile,
        RequestSummary, WorkspaceConfig, WorkspaceDiagnostics, WorkspaceSnapshot,
    },
    secrets::{self, KeyringBackend},
    workspace,
};

#[tauri::command]
pub fn create_workspace(path: String, name: Option<String>) -> Result<WorkspaceSnapshot, String> {
    workspace::create(Path::new(&path), name).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_workspace(path: String) -> Result<WorkspaceSnapshot, String> {
    workspace::open(Path::new(&path)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_workspace_config(
    workspace_path: String,
    config: WorkspaceConfig,
) -> Result<WorkspaceConfig, String> {
    workspace::save_config(Path::new(&workspace_path), &config).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_workspace(workspace_path: String, destination_path: String) -> Result<(), String> {
    workspace::export_bundle(Path::new(&workspace_path), Path::new(&destination_path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_workspace(
    source_path: String,
    target_path: String,
) -> Result<WorkspaceSnapshot, String> {
    workspace::import_bundle(Path::new(&source_path), Path::new(&target_path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_request(
    workspace_path: String,
    relative_path: Option<String>,
    collection: Option<String>,
    request: RequestFile,
) -> Result<RequestSummary, String> {
    workspace::save_request(
        Path::new(&workspace_path),
        relative_path.as_deref(),
        collection.as_deref(),
        &request,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_request(workspace_path: String, relative_path: String) -> Result<(), String> {
    workspace::delete_request(Path::new(&workspace_path), &relative_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_environment(
    workspace_path: String,
    relative_path: Option<String>,
    environment: EnvironmentFile,
) -> Result<EnvironmentSummary, String> {
    workspace::save_environment(
        Path::new(&workspace_path),
        relative_path.as_deref(),
        &environment,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_environment(workspace_path: String, relative_path: String) -> Result<(), String> {
    workspace::delete_environment(Path::new(&workspace_path), &relative_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn workspace_fingerprint(workspace_path: String) -> Result<String, String> {
    workspace::reliability::fingerprint(Path::new(&workspace_path))
}

#[tauri::command]
pub fn diagnose_workspace(workspace_path: String) -> Result<WorkspaceDiagnostics, String> {
    let root = Path::new(&workspace_path);
    let available_secrets = workspace::load_config(root)
        .ok()
        .and_then(|config| KeyringBackend::new(&config.id).ok())
        .and_then(|backend| secrets::list(&backend).ok())
        .unwrap_or_default();
    workspace::reliability::diagnose(root, &available_secrets)
}

#[tauri::command]
pub fn preview_workspace_migration(workspace_path: String) -> Result<MigrationPlan, String> {
    workspace::reliability::migration_plan(Path::new(&workspace_path))
}

#[tauri::command]
pub fn migrate_workspace(workspace_path: String) -> Result<MigrationResult, String> {
    let root = Path::new(&workspace_path);
    let backup_id = workspace::reliability::apply_migration(root)?;
    let workspace = workspace::open(root).map_err(|error| error.to_string())?;
    Ok(MigrationResult {
        backup_id,
        workspace,
    })
}

#[tauri::command]
pub fn rollback_workspace_migration(
    workspace_path: String,
    backup_id: String,
) -> Result<WorkspaceSnapshot, String> {
    let root = Path::new(&workspace_path);
    workspace::reliability::restore_backup(root, &backup_id)?;
    workspace::open(root).map_err(|error| error.to_string())
}
