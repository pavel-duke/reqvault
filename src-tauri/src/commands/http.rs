use crate::{
    http,
    models::{EnvironmentFile, HttpError, HttpResponse, RequestFile},
};

#[tauri::command]
pub async fn send_request(
    request: RequestFile,
    environment: Option<EnvironmentFile>,
) -> Result<HttpResponse, HttpError> {
    http::send(
        &request,
        environment.as_ref(),
        &mut http::unavailable_secret,
    )
    .await
}
