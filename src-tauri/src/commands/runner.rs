use std::path::Path;

use crate::{
    models::{CollectionRunOptions, CollectionRunReport},
    runner,
};

#[tauri::command]
pub async fn run_collection(
    workspace_path: String,
    options: CollectionRunOptions,
) -> Result<CollectionRunReport, String> {
    runner::run_workspace(Path::new(&workspace_path), &options).await
}
