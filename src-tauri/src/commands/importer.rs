use std::path::Path;

use crate::{importer, models::ImportResult};

#[tauri::command]
pub fn import_collection(
    workspace_path: String,
    file_path: String,
) -> Result<ImportResult, String> {
    importer::import_file(Path::new(&workspace_path), Path::new(&file_path))
}

#[tauri::command]
pub fn import_curl(workspace_path: String, command: String) -> Result<ImportResult, String> {
    importer::import_curl(Path::new(&workspace_path), &command)
}
