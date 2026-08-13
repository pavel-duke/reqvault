use crate::{
    http,
    models::{EnvironmentFile, HttpError, HttpResponse, RequestFile},
    secrets::{self, KeyringBackend},
    variables::ResolveError,
};

#[tauri::command]
pub async fn send_request(
    request: RequestFile,
    environment: Option<EnvironmentFile>,
    workspace_id: String,
) -> Result<HttpResponse, HttpError> {
    let backend = KeyringBackend::new(&workspace_id).map_err(|error| HttpError {
        message: error.to_string(),
        details: None,
        error_type: "secret_storage".to_string(),
    })?;
    http::send(&request, environment.as_ref(), &mut |name| {
        secrets::get(&backend, name).map_err(|error| match error {
            secrets::SecretError::NotFound(name) => ResolveError::MissingSecret(name),
            _ => ResolveError::SecretStorage,
        })
    })
    .await
}
