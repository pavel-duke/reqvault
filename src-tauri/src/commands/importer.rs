use std::path::Path;

use crate::{importer, models::ImportResult};

#[tauri::command]
pub fn import_collection(
    workspace_path: String,
    file_path: String,
) -> Result<ImportResult, String> {
    importer::import_file(Path::new(&workspace_path), Path::new(&file_path))
}
