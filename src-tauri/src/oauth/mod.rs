use std::{collections::BTreeMap, time::Duration};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::timeout,
};
use url::Url;
use uuid::Uuid;

use crate::{
    models::{AuthConfig, EnvironmentFile, OAuthResult},
    redaction::redact_text,
    secrets::{self, SecretBackend},
    variables::{ResolveError, resolve_secrets, resolve_variables, secret_names},
};

const OAUTH_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    scope: Option<String>,
}

pub async fn authorize(
    auth: &AuthConfig,
    environment: Option<&EnvironmentFile>,
    backend: &impl SecretBackend,
) -> Result<OAuthResult, String> {
    let AuthConfig::OAuth2 {
        grant_type,
        authorization_url,
        token_url,
        client_id,
        client_secret,
        scopes,
        access_token,
        refresh_token,
    } = auth
    else {
        return Err("В запросе не настроен OAuth 2.0".to_string());
    };

    let variables = environment
        .map(|item| item.variables.clone())
        .unwrap_or_default();
    let mut used_secrets = Vec::new();
    let mut resolve = |value: &str| -> Result<String, String> {
        let with_variables =
            resolve_variables(value, &variables).map_err(|error| error.to_string())?;
        resolve_secrets(
            &with_variables,
            &mut |name| {
                secrets::get(backend, name).map_err(|error| match error {
                    secrets::SecretError::NotFound(name) => ResolveError::MissingSecret(name),
                    _ => ResolveError::SecretStorage,
                })
            },
            &mut used_secrets,
        )
        .map_err(|error| error.to_string())
    };

    let token_url = resolve(token_url)?;
    validate_endpoint(&token_url, "Token URL")?;
    let client_id = resolve(client_id)?;
    if client_id.trim().is_empty() {
        return Err("Укажи Client ID".to_string());
    }
    if !client_secret.trim().is_empty() {
        exact_secret_name(client_secret, "client secret")?;
    }
    let client_secret = resolve(client_secret)?;
    let scopes = resolve(scopes)?;
    let access_secret_name = exact_secret_name(access_token, "access token")?;
    let refresh_secret_name = if refresh_token.trim().is_empty() {
        None
    } else {
        Some(exact_secret_name(refresh_token, "refresh token")?)
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| "Не удалось подготовить OAuth-клиент".to_string())?;

    let token = match grant_type.as_str() {
        "authorization_code_pkce" => {
            let authorization_url = resolve(authorization_url)?;
            validate_endpoint(&authorization_url, "Authorization URL")?;
            authorization_code_pkce(
                &client,
                &authorization_url,
                &token_url,
                &client_id,
                &client_secret,
                &scopes,
                &used_secrets,
            )
            .await?
        }
        "client_credentials" => {
            let mut form = BTreeMap::from([
                ("grant_type", "client_credentials".to_string()),
                ("client_id", client_id),
            ]);
            if !client_secret.is_empty() {
                form.insert("client_secret", client_secret);
            }
            if !scopes.is_empty() {
                form.insert("scope", scopes);
            }
            exchange_token(&client, &token_url, &form, &used_secrets).await?
        }
        _ => return Err("Этот OAuth grant type не поддерживается".to_string()),
    };

    secrets::save(backend, &access_secret_name, &token.access_token)
        .map_err(|error| error.to_string())?;
    let saved_refresh = match (refresh_secret_name, token.refresh_token) {
        (Some(name), Some(value)) => {
            secrets::save(backend, &name, &value).map_err(|error| error.to_string())?;
            Some(name)
        }
        _ => None,
    };

    Ok(OAuthResult {
        access_token_secret: access_secret_name,
        refresh_token_secret: saved_refresh,
        expires_in: token.expires_in,
        scope: token.scope,
    })
}

async fn authorization_code_pkce(
    client: &Client,
    authorization_url: &str,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    scopes: &str,
    used_secrets: &[String],
) -> Result<TokenResponse, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| "Не удалось открыть локальный OAuth callback".to_string())?;
    let address = listener
        .local_addr()
        .map_err(|_| "Не удалось определить OAuth callback".to_string())?;
    let redirect_uri = format!("http://127.0.0.1:{}/callback", address.port());
    let verifier = format!(
        "{}{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let expected_state = Uuid::new_v4().to_string();
    let mut url = Url::parse(authorization_url)
        .map_err(|_| "Укажи корректный Authorization URL".to_string())?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("response_type", "code");
        query.append_pair("client_id", client_id);
        query.append_pair("redirect_uri", &redirect_uri);
        query.append_pair("state", &expected_state);
        query.append_pair("code_challenge", &challenge);
        query.append_pair("code_challenge_method", "S256");
        if !scopes.is_empty() {
            query.append_pair("scope", scopes);
        }
    }
    webbrowser::open(url.as_str())
        .map_err(|_| "Не удалось открыть браузер для OAuth".to_string())?;

    let (mut socket, _) = timeout(OAUTH_TIMEOUT, listener.accept())
        .await
        .map_err(|_| "OAuth-авторизация не завершена за 3 минуты".to_string())?
        .map_err(|_| "Не удалось принять OAuth callback".to_string())?;
    let mut buffer = vec![0_u8; 16 * 1024];
    let read = timeout(Duration::from_secs(10), socket.read(&mut buffer))
        .await
        .map_err(|_| "OAuth callback не содержит ответа".to_string())?
        .map_err(|_| "Не удалось прочитать OAuth callback".to_string())?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| "Некорректный OAuth callback".to_string())?;
    let callback = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| "Некорректный OAuth callback".to_string())?;
    let params = callback
        .query_pairs()
        .into_owned()
        .collect::<BTreeMap<_, _>>();
    let valid_state = params.get("state") == Some(&expected_state);
    let result = if !valid_state {
        Err("OAuth state не совпал. Запрос отклонён".to_string())
    } else if let Some(error) = params.get("error") {
        Err(format!("OAuth-сервер вернул ошибку: {error}"))
    } else {
        params
            .get("code")
            .cloned()
            .ok_or_else(|| "OAuth callback не содержит code".to_string())
    };
    let (title, message) = if result.is_ok() {
        (
            "ReqVault",
            "Авторизация завершена. Можно закрыть эту вкладку.",
        )
    } else {
        (
            "ReqVault",
            "Авторизация не завершена. Вернитесь в приложение.",
        )
    };
    let html =
        format!("<!doctype html><meta charset=\"utf-8\"><title>{title}</title><p>{message}</p>");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let code = result?;

    let mut form = BTreeMap::from([
        ("grant_type", "authorization_code".to_string()),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id.to_string()),
        ("code_verifier", verifier),
    ]);
    if !client_secret.is_empty() {
        form.insert("client_secret", client_secret.to_string());
    }
    exchange_token(client, token_url, &form, used_secrets).await
}

async fn exchange_token(
    client: &Client,
    token_url: &str,
    form: &BTreeMap<&str, String>,
    used_secrets: &[String],
) -> Result<TokenResponse, String> {
    let response = client
        .post(token_url)
        .form(form)
        .send()
        .await
        .map_err(|error| {
            format!(
                "Не удалось обратиться к OAuth token endpoint: {}",
                redact_text(&error.to_string(), used_secrets)
            )
        })?;
    if !response.status().is_success() {
        return Err(format!(
            "OAuth token endpoint вернул статус {}",
            response.status().as_u16()
        ));
    }
    response
        .json::<TokenResponse>()
        .await
        .map_err(|_| "OAuth token endpoint вернул некорректный JSON".to_string())
}

fn validate_endpoint(value: &str, label: &str) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| format!("Укажи корректный {label}"))?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
    if url.scheme() != "https" && !(url.scheme() == "http" && local) {
        return Err(format!("{label} должен использовать HTTPS"));
    }
    Ok(())
}

fn exact_secret_name(value: &str, label: &str) -> Result<String, String> {
    let names = secret_names(value.trim());
    if names.len() == 1 && value.trim() == format!("{{{{secret:{}}}}}", names[0]) {
        Ok(names[0].clone())
    } else {
        Err(format!(
            "Для {label} укажи одну ссылку вида {{{{secret:NAME}}}}"
        ))
    }
}
