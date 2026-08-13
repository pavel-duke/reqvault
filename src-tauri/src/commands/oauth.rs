use crate::{
    models::{EnvironmentFile, OAuthResult, RequestFile},
    oauth,
    secrets::KeyringBackend,
};

#[tauri::command]
pub async fn authorize_oauth(
    request: RequestFile,
    environment: Option<EnvironmentFile>,
    workspace_id: String,
) -> Result<OAuthResult, String> {
    let backend = KeyringBackend::new(&workspace_id).map_err(|error| error.to_string())?;
    oauth::authorize(&request.auth, environment.as_ref(), &backend).await
}

#[tauri::command]
pub async fn refresh_oauth(
    request: RequestFile,
    environment: Option<EnvironmentFile>,
    workspace_id: String,
) -> Result<OAuthResult, String> {
    let backend = KeyringBackend::new(&workspace_id).map_err(|error| error.to_string())?;
    oauth::refresh(&request.auth, environment.as_ref(), &backend).await
}
