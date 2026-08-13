use crate::{
    models::{EnvironmentFile, RequestFile, SecurityReport},
    security,
};

#[tauri::command]
pub fn inspect_request(
    request: RequestFile,
    environment: Option<EnvironmentFile>,
) -> SecurityReport {
    security::analyze(&request, environment.as_ref())
}

#[tauri::command]
pub fn generate_safe_curl(
    request: RequestFile,
    environment: Option<EnvironmentFile>,
) -> Result<String, String> {
    security::curl(&request, environment.as_ref())
}
