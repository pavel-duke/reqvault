use std::path::Path;

use crate::{
    models::{HttpResponse, RequestFile},
    response_tools,
};

#[tauri::command]
pub fn export_response(
    destination_path: String,
    format: String,
    request: RequestFile,
    response: HttpResponse,
) -> Result<(), String> {
    response_tools::export(Path::new(&destination_path), &format, &request, &response)
}

#[tauri::command]
pub fn save_response_fixture(
    workspace_path: String,
    name: String,
    response: HttpResponse,
) -> Result<String, String> {
    response_tools::save_fixture(Path::new(&workspace_path), &name, &response)
}
