use std::path::Path;

use tauri::{Manager, State};

use crate::{
    guard, history, http,
    models::{AuthConfig, EnvironmentFile, HttpError, HttpResponse, RequestFile},
    oauth,
    secrets::{self, KeyringBackend},
    session::SessionState,
    variables::ResolveError,
    workspace,
};

#[tauri::command]
pub async fn send_request(
    app: tauri::AppHandle,
    request: RequestFile,
    environment: Option<EnvironmentFile>,
    workspace_id: String,
    workspace_path: String,
    sessions: State<'_, SessionState>,
) -> Result<HttpResponse, HttpError> {
    let backend = KeyringBackend::new(&workspace_id).map_err(|error| HttpError {
        message: error.to_string(),
        details: None,
        error_type: "secret_storage".to_string(),
    })?;
    let config = workspace::load_config(Path::new(&workspace_path)).map_err(|error| HttpError {
        message: error.to_string(),
        details: None,
        error_type: "workspace".to_string(),
    })?;
    guard::validate(&request, environment.as_ref(), &config.production_guard).map_err(
        |message| HttpError {
            message,
            details: None,
            error_type: "production_guard".to_string(),
        },
    )?;

    let cookie_jar = sessions.jar(&workspace_id).map_err(|message| HttpError {
        message,
        details: None,
        error_type: "session".to_string(),
    })?;
    let mut response = http::send_with_session(
        &request,
        environment.as_ref(),
        &mut |name| {
            secrets::get(&backend, name).map_err(|error| match error {
                secrets::SecretError::NotFound(name) => ResolveError::MissingSecret(name),
                _ => ResolveError::SecretStorage,
            })
        },
        Some(&cookie_jar),
        Some(&config.production_guard),
    )
    .await?;
    if response.status == 401
        && matches!(&request.auth, AuthConfig::OAuth2 { .. })
        && oauth::refresh(&request.auth, environment.as_ref(), &backend)
            .await
            .is_ok()
    {
        response = http::send_with_session(
            &request,
            environment.as_ref(),
            &mut |name| {
                secrets::get(&backend, name).map_err(|error| match error {
                    secrets::SecretError::NotFound(name) => ResolveError::MissingSecret(name),
                    _ => ResolveError::SecretStorage,
                })
            },
            Some(&cookie_jar),
            Some(&config.production_guard),
        )
        .await?;
    }
    if let Ok(root) = app.path().app_local_data_dir() {
        let _ = history::record(&root, &workspace_id, &request, &response);
    }
    Ok(response)
}
