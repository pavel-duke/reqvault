use std::{collections::BTreeMap, fs, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use md5::Md5;
use reqwest::{
    Certificate, Client, Identity, Method, Proxy,
    header::{
        AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, COOKIE, HeaderMap, HeaderName, HeaderValue,
        LOCATION, WWW_AUTHENTICATE,
    },
    redirect::Policy,
};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, macros::format_description};
use url::Url;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::{
    guard,
    models::{
        AuthConfig, BodyConfig, EnvironmentFile, HttpError, HttpResponse, ProductionGuard,
        ProxyConfig, RequestFile, ResponseHeader,
    },
    redaction::{is_sensitive_header, redact_header, redact_text},
    session::CookieJar,
    variables::{
        ResolveError, is_exact_secret_reference, resolve_secrets, resolve_variables, secret_names,
    },
};

const ALLOWED_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const MAX_RESPONSE_BODY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone)]
struct PreparedRequest {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: BodyConfig,
    timeout: Duration,
    follow_redirects: bool,
    secret_values: Zeroizing<Vec<String>>,
    transport: PreparedTransport,
    auth: PreparedAuth,
}

impl Drop for PreparedRequest {
    fn drop(&mut self) {
        zeroize_body(&mut self.body);
        self.headers.clear();
        self.url.set_query(None);
        self.url.set_fragment(None);
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
enum PreparedAuth {
    None,
    Digest {
        username: String,
        password: String,
    },
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        session_token: String,
        region: String,
        service: String,
    },
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
struct PreparedTransport {
    proxy: PreparedProxy,
    custom_ca: Option<Vec<u8>>,
    identity: Option<Vec<u8>>,
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
enum PreparedProxy {
    None,
    System,
    Custom {
        url: String,
        username: String,
        password: String,
    },
}

fn zeroize_body(body: &mut BodyConfig) {
    match body {
        BodyConfig::None => {}
        BodyConfig::Json { value } => value.zeroize(),
        BodyConfig::Graphql {
            query,
            variables,
            operation_name,
        } => {
            query.zeroize();
            variables.zeroize();
            operation_name.zeroize();
        }
        BodyConfig::Raw {
            value,
            content_type,
        } => {
            value.zeroize();
            content_type.zeroize();
        }
        BodyConfig::FormUrlencoded { fields } => {
            for field in fields {
                field.name.zeroize();
                field.value.zeroize();
            }
        }
        BodyConfig::Multipart { fields } => {
            for field in fields {
                match field {
                    crate::models::MultipartField::Text { name, value, .. } => {
                        name.zeroize();
                        value.zeroize();
                    }
                    crate::models::MultipartField::File {
                        name,
                        path,
                        content_type,
                        ..
                    } => {
                        name.zeroize();
                        path.zeroize();
                        content_type.zeroize();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
async fn send<F>(
    request: &RequestFile,
    environment: Option<&EnvironmentFile>,
    resolve_secret: &mut F,
) -> Result<HttpResponse, HttpError>
where
    F: FnMut(&str) -> Result<String, ResolveError>,
{
    send_with_session(request, environment, resolve_secret, None, None).await
}

pub async fn send_with_session<F>(
    request: &RequestFile,
    environment: Option<&EnvironmentFile>,
    resolve_secret: &mut F,
    cookie_jar: Option<&CookieJar>,
    production_guard: Option<&ProductionGuard>,
) -> Result<HttpResponse, HttpError>
where
    F: FnMut(&str) -> Result<String, ResolveError>,
{
    let prepared = prepare(request, environment, resolve_secret).map_err(resolve_error)?;
    execute(prepared, cookie_jar, production_guard).await
}

fn prepare<F>(
    request: &RequestFile,
    environment: Option<&EnvironmentFile>,
    resolve_secret: &mut F,
) -> Result<PreparedRequest, PrepareError>
where
    F: FnMut(&str) -> Result<String, ResolveError>,
{
    validate_credentials(request)?;
    let variables = environment
        .map(|environment| &environment.variables)
        .cloned()
        .unwrap_or_default();
    let mut secret_values = Zeroizing::new(Vec::new());
    let mut resolve = |value: &str| -> Result<String, PrepareError> {
        let variables_resolved = resolve_variables(value, &variables)?;
        Ok(resolve_secrets(
            &variables_resolved,
            resolve_secret,
            &mut secret_values,
        )?)
    };

    let normalized_method = request.method.trim().to_uppercase();
    if !ALLOWED_METHODS.contains(&normalized_method.as_str()) {
        return Err(PrepareError::InvalidMethod(normalized_method));
    }
    let method = Method::from_bytes(normalized_method.as_bytes())
        .map_err(|_| PrepareError::InvalidMethod(normalized_method.clone()))?;
    let raw_url = resolve(&request.url)?;
    let mut url = Url::parse(&raw_url).map_err(|_| PrepareError::InvalidUrl)?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(PrepareError::InvalidUrl);
    }

    {
        let mut query = url.query_pairs_mut();
        for item in request
            .query
            .iter()
            .filter(|item| item.enabled && !item.name.is_empty())
        {
            query.append_pair(&resolve(&item.name)?, &resolve(&item.value)?);
        }
        if let AuthConfig::ApiKeyQuery { name, value } = &request.auth {
            query.append_pair(&resolve(name)?, &resolve(value)?);
        }
    }

    let mut headers = HeaderMap::new();
    for (name, value) in &request.headers {
        let contains_secret = !secret_names(name).is_empty() || !secret_names(value).is_empty();
        let resolved_name = resolve(name)?;
        let resolved_value = resolve(value)?;
        let header_name = HeaderName::from_bytes(resolved_name.as_bytes())
            .map_err(|_| PrepareError::InvalidHeaderName(resolved_name.clone()))?;
        let mut header_value = HeaderValue::from_str(&resolved_value)
            .map_err(|_| PrepareError::InvalidHeaderValue(resolved_name.clone()))?;
        if contains_secret || is_sensitive_header(header_name.as_str()) {
            header_value.set_sensitive(true);
        }
        headers.insert(header_name, header_value);
    }

    let prepared_auth = match &request.auth {
        AuthConfig::None | AuthConfig::ApiKeyQuery { .. } => PreparedAuth::None,
        AuthConfig::Bearer { token } => {
            let mut value = HeaderValue::from_str(&format!("Bearer {}", resolve(token)?))
                .map_err(|_| PrepareError::InvalidHeaderValue("Authorization".to_string()))?;
            value.set_sensitive(true);
            headers.insert(reqwest::header::AUTHORIZATION, value);
            PreparedAuth::None
        }
        AuthConfig::Basic { username, password } => {
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            let credentials = format!("{}:{}", resolve(username)?, resolve(password)?);
            let mut value =
                HeaderValue::from_str(&format!("Basic {}", STANDARD.encode(credentials)))
                    .map_err(|_| PrepareError::InvalidHeaderValue("Authorization".to_string()))?;
            value.set_sensitive(true);
            headers.insert(reqwest::header::AUTHORIZATION, value);
            PreparedAuth::None
        }
        AuthConfig::Digest { username, password } => PreparedAuth::Digest {
            username: resolve(username)?,
            password: resolve(password)?,
        },
        AuthConfig::ApiKeyHeader { name, value } => {
            let resolved_name = resolve(name)?;
            let header_name = HeaderName::from_bytes(resolved_name.as_bytes())
                .map_err(|_| PrepareError::InvalidHeaderName(resolved_name.clone()))?;
            let mut header_value = HeaderValue::from_str(&resolve(value)?)
                .map_err(|_| PrepareError::InvalidHeaderValue(resolved_name))?;
            header_value.set_sensitive(true);
            headers.insert(header_name, header_value);
            PreparedAuth::None
        }
        AuthConfig::OAuth2 { access_token, .. } => {
            let mut value = HeaderValue::from_str(&format!("Bearer {}", resolve(access_token)?))
                .map_err(|_| PrepareError::InvalidHeaderValue("Authorization".to_string()))?;
            value.set_sensitive(true);
            headers.insert(reqwest::header::AUTHORIZATION, value);
            PreparedAuth::None
        }
        AuthConfig::AwsSigV4 {
            access_key,
            secret_key,
            session_token,
            region,
            service,
        } => PreparedAuth::AwsSigV4 {
            access_key: resolve(access_key)?,
            secret_key: resolve(secret_key)?,
            session_token: resolve(session_token)?,
            region: resolve(region)?,
            service: resolve(service)?,
        },
    };

    let body = match &request.body {
        BodyConfig::None => BodyConfig::None,
        BodyConfig::Json { value } => {
            let value = resolve(value)?;
            serde_json::from_str::<serde_json::Value>(&value)
                .map_err(|error| PrepareError::InvalidJson(error.to_string()))?;
            BodyConfig::Json { value }
        }
        BodyConfig::Graphql {
            query,
            variables,
            operation_name,
        } => {
            let query = resolve(query)?;
            if query.trim().is_empty() {
                return Err(PrepareError::InvalidGraphql(
                    "query не может быть пустым".to_string(),
                ));
            }
            let variables = resolve(variables)?;
            let parsed = serde_json::from_str::<serde_json::Value>(&variables)
                .map_err(|error| PrepareError::InvalidGraphql(error.to_string()))?;
            if !parsed.is_object() {
                return Err(PrepareError::InvalidGraphql(
                    "variables должны быть JSON-объектом".to_string(),
                ));
            }
            BodyConfig::Graphql {
                query,
                variables,
                operation_name: resolve(operation_name)?,
            }
        }
        BodyConfig::Raw {
            value,
            content_type,
        } => BodyConfig::Raw {
            value: resolve(value)?,
            content_type: resolve(content_type)?,
        },
        BodyConfig::FormUrlencoded { fields } => BodyConfig::FormUrlencoded {
            fields: fields
                .iter()
                .filter(|field| field.enabled && !field.name.is_empty())
                .map(|field| {
                    Ok(crate::models::KeyValue {
                        name: resolve(&field.name)?,
                        value: resolve(&field.value)?,
                        enabled: true,
                    })
                })
                .collect::<Result<Vec<_>, PrepareError>>()?,
        },
        BodyConfig::Multipart { fields } => BodyConfig::Multipart {
            fields: fields
                .iter()
                .filter(|field| match field {
                    crate::models::MultipartField::Text { name, enabled, .. }
                    | crate::models::MultipartField::File { name, enabled, .. } => {
                        *enabled && !name.is_empty()
                    }
                })
                .map(|field| match field {
                    crate::models::MultipartField::Text { name, value, .. } => {
                        Ok(crate::models::MultipartField::Text {
                            name: resolve(name)?,
                            value: resolve(value)?,
                            enabled: true,
                        })
                    }
                    crate::models::MultipartField::File {
                        name,
                        path,
                        content_type,
                        ..
                    } => Ok(crate::models::MultipartField::File {
                        name: resolve(name)?,
                        path: resolve(path)?,
                        content_type: resolve(content_type)?,
                        enabled: true,
                    }),
                })
                .collect::<Result<Vec<_>, PrepareError>>()?,
        },
    };

    let proxy = match &request.transport.proxy {
        ProxyConfig::None => PreparedProxy::None,
        ProxyConfig::System => PreparedProxy::System,
        ProxyConfig::Custom {
            url,
            username,
            password,
        } => PreparedProxy::Custom {
            url: resolve(url)?,
            username: resolve(username)?,
            password: resolve(password)?,
        },
    };
    let custom_ca = read_optional_pem(&resolve(&request.transport.custom_ca_path)?, "custom CA")?;
    let certificate_path = resolve(&request.transport.client_certificate_path)?;
    let key_path = resolve(&request.transport.client_key_path)?;
    let identity = match (certificate_path.is_empty(), key_path.is_empty()) {
        (true, true) => None,
        (false, false) => {
            let mut certificate = fs::read(&certificate_path).map_err(|error| {
                PrepareError::ReadTlsFile("клиентский сертификат", error.to_string())
            })?;
            let key = fs::read(&key_path)
                .map_err(|error| PrepareError::ReadTlsFile("приватный ключ", error.to_string()))?;
            certificate.push(b'\n');
            certificate.extend(key);
            Some(certificate)
        }
        _ => return Err(PrepareError::IncompleteIdentity),
    };

    Ok(PreparedRequest {
        method,
        url,
        headers,
        body,
        timeout: Duration::from_millis(request.timeout_ms.clamp(1, 600_000)),
        follow_redirects: request.follow_redirects,
        secret_values,
        transport: PreparedTransport {
            proxy,
            custom_ca,
            identity,
        },
        auth: prepared_auth,
    })
}

fn validate_credentials(request: &RequestFile) -> Result<(), PrepareError> {
    let require_reference = |value: &str, label: &'static str| {
        if value.trim().is_empty() || is_exact_secret_reference(value) {
            Ok(())
        } else {
            Err(PrepareError::UnsafeCredential(label))
        }
    };
    match &request.auth {
        AuthConfig::None => {}
        AuthConfig::Bearer { token } => require_reference(token, "Bearer token")?,
        AuthConfig::Basic { password, .. } => require_reference(password, "пароль Basic Auth")?,
        AuthConfig::Digest { password, .. } => require_reference(password, "пароль Digest Auth")?,
        AuthConfig::ApiKeyHeader { value, .. } | AuthConfig::ApiKeyQuery { value, .. } => {
            require_reference(value, "API key")?
        }
        AuthConfig::OAuth2 {
            client_secret,
            access_token,
            refresh_token,
            ..
        } => {
            require_reference(client_secret, "OAuth client secret")?;
            require_reference(access_token, "OAuth access token")?;
            require_reference(refresh_token, "OAuth refresh token")?;
        }
        AuthConfig::AwsSigV4 {
            access_key,
            secret_key,
            session_token,
            ..
        } => {
            require_reference(access_key, "AWS access key")?;
            require_reference(secret_key, "AWS secret key")?;
            require_reference(session_token, "AWS session token")?;
        }
    }
    if let ProxyConfig::Custom { password, .. } = &request.transport.proxy {
        require_reference(password, "пароль proxy")?;
    }
    Ok(())
}

fn read_optional_pem(path: &str, purpose: &'static str) -> Result<Option<Vec<u8>>, PrepareError> {
    if path.trim().is_empty() {
        return Ok(None);
    }
    fs::read(path)
        .map(Some)
        .map_err(|error| PrepareError::ReadTlsFile(purpose, error.to_string()))
}

async fn execute(
    mut prepared: PreparedRequest,
    cookie_jar: Option<&CookieJar>,
    production_guard: Option<&ProductionGuard>,
) -> Result<HttpResponse, HttpError> {
    let request_id = Uuid::new_v4().to_string();
    let mut client_builder = Client::builder()
        .redirect(Policy::none())
        .timeout(prepared.timeout);
    client_builder = match &prepared.transport.proxy {
        PreparedProxy::None => client_builder.no_proxy(),
        PreparedProxy::System => client_builder,
        PreparedProxy::Custom {
            url,
            username,
            password,
        } => {
            let mut proxy = Proxy::all(url).map_err(|error| {
                request_error(
                    "Не удалось настроить proxy",
                    error,
                    &prepared.secret_values,
                    "proxy",
                )
            })?;
            if !username.is_empty() || !password.is_empty() {
                proxy = proxy.basic_auth(username, password);
            }
            client_builder.proxy(proxy)
        }
    };
    if let Some(pem) = &prepared.transport.custom_ca {
        let certificate = Certificate::from_pem(pem).map_err(|error| {
            request_error(
                "Не удалось прочитать custom CA",
                error,
                &prepared.secret_values,
                "tls",
            )
        })?;
        client_builder = client_builder.add_root_certificate(certificate);
    }
    if let Some(pem) = &prepared.transport.identity {
        let identity = Identity::from_pem(pem).map_err(|error| {
            request_error(
                "Не удалось прочитать клиентский сертификат или ключ",
                error,
                &prepared.secret_values,
                "tls",
            )
        })?;
        client_builder = client_builder.identity(identity);
    }
    let client = client_builder.build().map_err(|error| {
        request_error(
            "Не удалось подготовить HTTP-клиент",
            error,
            &prepared.secret_values,
            "client",
        )
    })?;

    let started = std::time::Instant::now();
    let mut current_url = prepared.url.clone();
    let mut current_method = prepared.method.clone();
    let mut current_headers = prepared.headers.clone();
    let mut current_body = prepared.body.clone();
    let mut explicit_cookie = current_headers.contains_key(COOKIE);
    let mut redirect_count = 0_u8;
    let response = loop {
        if let Some(guard) = production_guard {
            guard::validate_resolved_target(&current_url, guard)
                .await
                .map_err(redirect_guard_error)?;
        }
        if matches!(prepared.auth, PreparedAuth::AwsSigV4 { .. }) {
            prepared.url = current_url.clone();
            prepared.method = current_method.clone();
            prepared.headers = current_headers;
            prepared.body = current_body.clone();
            sign_aws_request(&mut prepared, OffsetDateTime::now_utc())?;
            current_headers = prepared.headers.clone();
        }
        if !current_headers.contains_key(COOKIE)
            && let Some(cookie) = cookie_jar.and_then(|jar| jar.request_header(&current_url))
            && let Ok(mut value) = HeaderValue::from_str(&cookie)
        {
            value.set_sensitive(true);
            current_headers.insert(COOKIE, value);
        }
        let response = send_prepared(
            &client,
            &current_method,
            &current_url,
            &current_headers,
            &current_body,
            &prepared.secret_values,
        )
        .await?;
        store_response_cookies(cookie_jar, &current_url, &response);
        if !prepared.follow_redirects || !is_redirect_status(response.status()) {
            break response;
        }
        let Some(location) = response.headers().get(LOCATION) else {
            break response;
        };
        let location = location
            .to_str()
            .map_err(|_| redirect_error("Сервер вернул некорректный адрес редиректа"))?;
        if redirect_count >= 10 {
            return Err(redirect_error("Превышен лимит из 10 редиректов"));
        }
        let next_url = current_url
            .join(location)
            .map_err(|_| redirect_error("Сервер вернул некорректный адрес редиректа"))?;
        if !matches!(next_url.scheme(), "http" | "https") {
            return Err(redirect_error("Редирект использует неподдерживаемую схему"));
        }
        if let Some(guard) = production_guard {
            guard::validate_redirect(&current_url, &next_url, guard)
                .map_err(redirect_guard_error)?;
        }
        let cross_origin = !guard::same_origin(&current_url, &next_url);
        if cross_origin && matches!(prepared.auth, PreparedAuth::AwsSigV4 { .. }) {
            return Err(redirect_error(
                "Междоменный редирект для AWS SigV4 заблокирован: запрос нужно подписать для нового адреса",
            ));
        }
        if cross_origin {
            remove_sensitive_headers(&mut current_headers);
            explicit_cookie = false;
        } else if !explicit_cookie {
            current_headers.remove(COOKIE);
        }
        if redirect_switches_to_get(response.status(), &current_method) {
            current_method = Method::GET;
            current_body = BodyConfig::None;
            current_headers.remove(CONTENT_TYPE);
            current_headers.remove(CONTENT_LENGTH);
        }
        current_url = next_url;
        redirect_count += 1;
    };
    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        && let PreparedAuth::Digest { username, password } = &prepared.auth
        && let Some(challenge) = response.headers().get(WWW_AUTHENTICATE)
        && let Ok(challenge) = challenge.to_str()
        && challenge
            .split_once(' ')
            .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("digest"))
    {
        let authorization =
            digest_authorization(challenge, &current_method, &current_url, username, password)?;
        let challenge_duration = started.elapsed().as_millis();
        let mut retry = prepared.clone();
        retry.auth = PreparedAuth::None;
        retry.method = current_method;
        retry.url = current_url;
        retry.body = current_body;
        retry.headers = current_headers;
        let mut authorization = HeaderValue::from_str(&authorization)
            .map_err(|_| auth_error("Некорректный Digest Authorization"))?;
        authorization.set_sensitive(true);
        retry.headers.insert(AUTHORIZATION, authorization);
        let mut result = Box::pin(execute(retry, cookie_jar, production_guard)).await?;
        result.duration_ms += challenge_duration;
        return Ok(result);
    }
    let duration_ms = started.elapsed().as_millis();
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let declared_size = response
        .content_length()
        .and_then(|value| usize::try_from(value).ok());
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| ResponseHeader {
            name: name.to_string(),
            value: redact_header(
                name.as_str(),
                value.to_str().unwrap_or("[не удалось прочитать]"),
                &prepared.secret_values,
            ),
        })
        .collect();
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::with_capacity(
        declared_size
            .unwrap_or_default()
            .min(MAX_RESPONSE_BODY_BYTES),
    );
    let mut truncated = declared_size.is_some_and(|size| size > MAX_RESPONSE_BODY_BYTES);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            request_error(
                "Не удалось прочитать ответ сервера",
                error,
                &prepared.secret_values,
                "response",
            )
        })?;
        let available = MAX_RESPONSE_BODY_BYTES.saturating_sub(bytes.len());
        if chunk.len() > available {
            bytes.extend_from_slice(&chunk[..available]);
            truncated = true;
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    let size_bytes = declared_size.unwrap_or(bytes.len());
    let (body, body_kind, is_json) = response_body(&content_type, &bytes, &prepared.secret_values);

    Ok(HttpResponse {
        request_id,
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        duration_ms,
        size_bytes,
        headers,
        body,
        is_json,
        content_type,
        body_kind,
        truncated,
    })
}

async fn send_prepared(
    client: &Client,
    method: &Method,
    url: &Url,
    headers: &HeaderMap,
    body: &BodyConfig,
    secrets: &[String],
) -> Result<reqwest::Response, HttpError> {
    let mut builder = client
        .request(method.clone(), url.clone())
        .headers(headers.clone());
    builder = match body {
        BodyConfig::None => builder,
        BodyConfig::Json { value } => builder
            .header(CONTENT_TYPE, "application/json")
            .body(value.clone()),
        BodyConfig::Graphql {
            query,
            variables,
            operation_name,
        } => {
            let variables = serde_json::from_str::<serde_json::Value>(variables)
                .unwrap_or_else(|_| serde_json::json!({}));
            let mut payload = serde_json::Map::from_iter([
                (
                    "query".to_string(),
                    serde_json::Value::String(query.clone()),
                ),
                ("variables".to_string(), variables),
            ]);
            if !operation_name.trim().is_empty() {
                payload.insert(
                    "operationName".to_string(),
                    serde_json::Value::String(operation_name.clone()),
                );
            }
            builder.json(&serde_json::Value::Object(payload))
        }
        BodyConfig::Raw {
            value,
            content_type,
        } => {
            if content_type.trim().is_empty() {
                builder.body(value.clone())
            } else {
                builder
                    .header(CONTENT_TYPE, content_type)
                    .body(value.clone())
            }
        }
        BodyConfig::FormUrlencoded { fields } => {
            let form = fields
                .iter()
                .map(|field| (field.name.clone(), field.value.clone()))
                .collect::<Vec<_>>();
            builder.form(&form)
        }
        BodyConfig::Multipart { fields } => {
            let mut form = reqwest::multipart::Form::new();
            for field in fields {
                match field {
                    crate::models::MultipartField::Text { name, value, .. } => {
                        form = form.text(name.clone(), value.clone());
                    }
                    crate::models::MultipartField::File {
                        name,
                        path,
                        content_type,
                        ..
                    } => {
                        let bytes = fs::read(path).map_err(|error| HttpError {
                            message: format!("Не удалось прочитать файл для поля {name}"),
                            details: Some(redact_text(&error.to_string(), secrets)),
                            error_type: "multipart".to_string(),
                        })?;
                        let file_name = std::path::Path::new(path)
                            .file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or("file")
                            .to_string();
                        let mut part = reqwest::multipart::Part::bytes(bytes).file_name(file_name);
                        if !content_type.trim().is_empty() {
                            part = part.mime_str(content_type).map_err(|error| HttpError {
                                message: format!("Некорректный Content-Type для поля {name}"),
                                details: Some(redact_text(&error.to_string(), secrets)),
                                error_type: "multipart".to_string(),
                            })?;
                        }
                        form = form.part(name.clone(), part);
                    }
                }
            }
            builder.multipart(form)
        }
    };
    let host = url.host_str().unwrap_or("сервер");
    builder.send().await.map_err(|error| {
        let (message, error_type) = if error.is_connect() {
            (format!("Не удалось подключиться к {host}"), "connection")
        } else if error.is_timeout() {
            (
                format!("Сервер {host} не ответил за отведённое время"),
                "timeout",
            )
        } else {
            ("Не удалось выполнить HTTP-запрос".to_string(), "request")
        };
        request_error(&message, error, secrets, error_type)
    })
}

fn store_response_cookies(cookie_jar: Option<&CookieJar>, url: &Url, response: &reqwest::Response) {
    if let Some(jar) = cookie_jar {
        for header in response.headers().get_all(reqwest::header::SET_COOKIE) {
            if let Ok(value) = header.to_str() {
                jar.store(url, value);
            }
        }
    }
}

fn is_redirect_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 301 | 302 | 303 | 307 | 308)
}

fn redirect_switches_to_get(status: reqwest::StatusCode, method: &Method) -> bool {
    (status == reqwest::StatusCode::SEE_OTHER && *method != Method::HEAD)
        || matches!(status.as_u16(), 301 | 302) && *method == Method::POST
}

fn remove_sensitive_headers(headers: &mut HeaderMap) {
    let names = headers
        .iter()
        .filter(|(name, value)| value.is_sensitive() || is_sensitive_header(name.as_str()))
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    for name in names {
        headers.remove(name);
    }
}

fn redirect_error(message: &str) -> HttpError {
    HttpError {
        message: message.to_string(),
        details: None,
        error_type: "redirect".to_string(),
    }
}

fn redirect_guard_error(message: String) -> HttpError {
    HttpError {
        message,
        details: None,
        error_type: "production_guard".to_string(),
    }
}

fn response_body(content_type: &str, bytes: &[u8], secrets: &[String]) -> (String, String, bool) {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let textual = media_type.starts_with("text/")
        || media_type.contains("json")
        || media_type.contains("xml")
        || media_type.contains("javascript")
        || media_type == "application/x-www-form-urlencoded"
        || (media_type == "application/octet-stream" && std::str::from_utf8(bytes).is_ok());
    if textual {
        let raw = String::from_utf8_lossy(bytes).into_owned();
        let body = redact_text(&raw, secrets);
        let is_json = serde_json::from_str::<serde_json::Value>(&body).is_ok();
        let kind = if is_json {
            "json"
        } else if media_type.contains("html") {
            "html"
        } else {
            "text"
        };
        (body, kind.to_string(), is_json)
    } else {
        let kind = if media_type.starts_with("image/") {
            "image"
        } else {
            "binary"
        };
        (STANDARD.encode(bytes), kind.to_string(), false)
    }
}

fn digest_authorization(
    challenge: &str,
    method: &Method,
    url: &Url,
    username: &str,
    password: &str,
) -> Result<String, HttpError> {
    digest_authorization_with_cnonce(
        challenge,
        method,
        url,
        username,
        password,
        &Uuid::new_v4().simple().to_string(),
    )
}

fn digest_authorization_with_cnonce(
    challenge: &str,
    method: &Method,
    url: &Url,
    username: &str,
    password: &str,
    cnonce: &str,
) -> Result<String, HttpError> {
    let (_, parameters) = challenge
        .split_once(' ')
        .ok_or_else(|| auth_error("Digest challenge не содержит параметры"))?;
    let values = parse_auth_parameters(parameters);
    let realm = values
        .get("realm")
        .ok_or_else(|| auth_error("Digest challenge не содержит realm"))?;
    let nonce = values
        .get("nonce")
        .ok_or_else(|| auth_error("Digest challenge не содержит nonce"))?;
    let algorithm = values.get("algorithm").map_or("MD5", String::as_str);
    let qop = values.get("qop").and_then(|value| {
        value
            .split(',')
            .map(str::trim)
            .find(|value| value.eq_ignore_ascii_case("auth"))
    });
    if values.contains_key("qop") && qop.is_none() {
        return Err(auth_error("Digest qop поддерживает только auth"));
    }
    let mut uri = if url.path().is_empty() {
        "/".to_string()
    } else {
        url.path().to_string()
    };
    if let Some(query) = url.query() {
        uri.push('?');
        uri.push_str(query);
    }
    let nc = "00000001";
    let base_ha1 = digest_hash(algorithm, &format!("{username}:{realm}:{password}"))?;
    let ha1 = if algorithm.to_ascii_lowercase().ends_with("-sess") {
        digest_hash(algorithm, &format!("{base_ha1}:{nonce}:{cnonce}"))?
    } else {
        base_ha1
    };
    let ha2 = digest_hash(algorithm, &format!("{}:{uri}", method.as_str()))?;
    let response = if let Some(qop) = qop {
        digest_hash(
            algorithm,
            &format!("{ha1}:{nonce}:{nc}:{cnonce}:{qop}:{ha2}"),
        )?
    } else {
        digest_hash(algorithm, &format!("{ha1}:{nonce}:{ha2}"))?
    };
    let mut parts = vec![
        format!("username=\"{}\"", quote_auth(username)),
        format!("realm=\"{}\"", quote_auth(realm)),
        format!("nonce=\"{}\"", quote_auth(nonce)),
        format!("uri=\"{}\"", quote_auth(&uri)),
        format!("response=\"{response}\""),
        format!("algorithm={algorithm}"),
    ];
    if let Some(opaque) = values.get("opaque") {
        parts.push(format!("opaque=\"{}\"", quote_auth(opaque)));
    }
    if let Some(qop) = qop {
        parts.extend([
            format!("qop={qop}"),
            format!("nc={nc}"),
            format!("cnonce=\"{cnonce}\""),
        ]);
    }
    Ok(format!("Digest {}", parts.join(", ")))
}

fn parse_auth_parameters(input: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    let mut items = Vec::new();
    for (index, character) in input.char_indices() {
        match character {
            '\\' if quoted && !escaped => escaped = true,
            '"' if !escaped => quoted = !quoted,
            ',' if !quoted => {
                items.push(&input[start..index]);
                start = index + 1;
            }
            _ => escaped = false,
        }
    }
    items.push(&input[start..]);
    for item in items {
        if let Some((name, value)) = item.trim().split_once('=') {
            result.insert(
                name.trim().to_ascii_lowercase(),
                value.trim().trim_matches('"').replace("\\\"", "\""),
            );
        }
    }
    result
}

fn quote_auth(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn digest_hash(algorithm: &str, value: &str) -> Result<String, HttpError> {
    match algorithm.to_ascii_uppercase().as_str() {
        "MD5" | "MD5-SESS" => Ok(hex(&Md5::digest(value.as_bytes()))),
        "SHA-256" | "SHA-256-SESS" => Ok(hex(&Sha256::digest(value.as_bytes()))),
        _ => Err(auth_error(&format!(
            "Digest algorithm {algorithm} не поддерживается"
        ))),
    }
}

fn sign_aws_request(prepared: &mut PreparedRequest, now: OffsetDateTime) -> Result<(), HttpError> {
    let PreparedAuth::AwsSigV4 {
        access_key,
        secret_key,
        session_token,
        region,
        service,
    } = &prepared.auth
    else {
        return Ok(());
    };
    if matches!(prepared.body, BodyConfig::Multipart { .. }) {
        return Err(auth_error("AWS SigV4 пока не поддерживает multipart body"));
    }
    if access_key.is_empty() || secret_key.is_empty() || region.is_empty() || service.is_empty() {
        return Err(auth_error(
            "Для AWS SigV4 нужны access key, secret key, region и service",
        ));
    }
    let date_format = format_description!("[year][month][day]T[hour][minute][second]Z");
    let amz_date = now
        .format(&date_format)
        .map_err(|_| auth_error("Не удалось сформировать дату AWS SigV4"))?;
    let short_date = &amz_date[..8];
    let payload_hash = hex(&Sha256::digest(request_body_bytes(&prepared.body)?));
    prepared.headers.insert(
        HeaderName::from_static("x-amz-date"),
        HeaderValue::from_str(&amz_date).map_err(|_| auth_error("Некорректная дата AWS SigV4"))?,
    );
    prepared.headers.insert(
        HeaderName::from_static("x-amz-content-sha256"),
        HeaderValue::from_str(&payload_hash)
            .map_err(|_| auth_error("Некорректный хеш AWS SigV4"))?,
    );
    if !session_token.is_empty() {
        let mut value = HeaderValue::from_str(session_token)
            .map_err(|_| auth_error("Некорректный AWS session token"))?;
        value.set_sensitive(true);
        prepared
            .headers
            .insert(HeaderName::from_static("x-amz-security-token"), value);
    }

    let mut signed_headers = vec!["host", "x-amz-content-sha256", "x-amz-date"];
    if !session_token.is_empty() {
        signed_headers.push("x-amz-security-token");
    }
    signed_headers.sort_unstable();
    let host = canonical_host(&prepared.url);
    let canonical_headers = signed_headers
        .iter()
        .map(|name| {
            let value = if *name == "host" {
                host.as_str()
            } else {
                prepared
                    .headers
                    .get(*name)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("")
            };
            format!("{name}:{}\n", normalize_header(value))
        })
        .collect::<String>();
    let signed_headers_value = signed_headers.join(";");
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        prepared.method.as_str(),
        canonical_uri(&prepared.url),
        canonical_query(&prepared.url),
        canonical_headers,
        signed_headers_value,
        payload_hash
    );
    let scope = format!("{short_date}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        hex(&Sha256::digest(canonical_request.as_bytes()))
    );
    let date_key = hmac_sha256(
        format!("AWS4{secret_key}").as_bytes(),
        short_date.as_bytes(),
    )?;
    let region_key = hmac_sha256(&date_key, region.as_bytes())?;
    let service_key = hmac_sha256(&region_key, service.as_bytes())?;
    let signing_key = hmac_sha256(&service_key, b"aws4_request")?;
    let signature = hex(&hmac_sha256(&signing_key, string_to_sign.as_bytes())?);
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers_value}, Signature={signature}"
    );
    let mut authorization = HeaderValue::from_str(&authorization)
        .map_err(|_| auth_error("Некорректная подпись AWS SigV4"))?;
    authorization.set_sensitive(true);
    prepared.headers.insert(AUTHORIZATION, authorization);
    Ok(())
}

fn request_body_bytes(body: &BodyConfig) -> Result<Vec<u8>, HttpError> {
    match body {
        BodyConfig::None => Ok(Vec::new()),
        BodyConfig::Json { value } | BodyConfig::Raw { value, .. } => Ok(value.as_bytes().to_vec()),
        BodyConfig::Graphql {
            query,
            variables,
            operation_name,
        } => {
            let variables = serde_json::from_str::<serde_json::Value>(variables)
                .map_err(|error| auth_error(&format!("Некорректные GraphQL variables: {error}")))?;
            let mut payload = serde_json::Map::from_iter([
                (
                    "query".to_string(),
                    serde_json::Value::String(query.clone()),
                ),
                ("variables".to_string(), variables),
            ]);
            if !operation_name.trim().is_empty() {
                payload.insert(
                    "operationName".to_string(),
                    serde_json::Value::String(operation_name.clone()),
                );
            }
            Ok(serde_json::Value::Object(payload).to_string().into_bytes())
        }
        BodyConfig::FormUrlencoded { fields } => {
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            for field in fields {
                serializer.append_pair(&field.name, &field.value);
            }
            Ok(serializer.finish().into_bytes())
        }
        BodyConfig::Multipart { .. } => {
            Err(auth_error("AWS SigV4 пока не поддерживает multipart body"))
        }
    }
}

fn canonical_host(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    }
}

fn canonical_uri(url: &Url) -> String {
    let path = if url.path().is_empty() {
        "/"
    } else {
        url.path()
    };
    path.split('/')
        .map(|part| aws_encode(&percent_decode(part), true))
        .collect::<Vec<_>>()
        .join("/")
}

fn canonical_query(url: &Url) -> String {
    let mut pairs = url
        .query_pairs()
        .map(|(name, value)| {
            (
                aws_encode(name.as_bytes(), true),
                aws_encode(value.as_bytes(), true),
            )
        })
        .collect::<Vec<_>>();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_decode(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2]))
        {
            result.push((high << 4) | low);
            index += 3;
        } else {
            result.push(bytes[index]);
            index += 1;
        }
    }
    result
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn aws_encode(value: &[u8], encode_slash: bool) -> String {
    value.iter().fold(String::new(), |mut output, byte| {
        if byte.is_ascii_alphanumeric()
            || matches!(*byte, b'-' | b'_' | b'.' | b'~')
            || (*byte == b'/' && !encode_slash)
        {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
        output
    })
}

fn normalize_header(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hmac_sha256(key: &[u8], value: &[u8]) -> Result<Vec<u8>, HttpError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key)
        .map_err(|_| auth_error("Не удалось создать HMAC для AWS SigV4"))?;
    mac.update(value);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn auth_error(message: &str) -> HttpError {
    HttpError {
        message: message.to_string(),
        details: None,
        error_type: "auth".to_string(),
    }
}

#[derive(Debug, thiserror::Error)]
enum PrepareError {
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    #[error("HTTP-метод {0} не поддерживается")]
    InvalidMethod(String),
    #[error("Укажи корректный URL с http:// или https://")]
    InvalidUrl,
    #[error("Некорректное имя заголовка: {0}")]
    InvalidHeaderName(String),
    #[error("Некорректное значение заголовка: {0}")]
    InvalidHeaderValue(String),
    #[error("Тело запроса содержит неправильный JSON: {0}")]
    InvalidJson(String),
    #[error("Некорректный GraphQL body: {0}")]
    InvalidGraphql(String),
    #[error("Не удалось прочитать {0}: {1}")]
    ReadTlsFile(&'static str, String),
    #[error("Для mTLS нужно выбрать и сертификат, и приватный ключ")]
    IncompleteIdentity,
    #[error("{0} нужно хранить в Secret Vault и указывать как {{{{secret:NAME}}}}")]
    UnsafeCredential(&'static str),
}

fn resolve_error(error: PrepareError) -> HttpError {
    HttpError {
        message: error.to_string(),
        details: None,
        error_type: "validation".to_string(),
    }
}

fn request_error(
    message: &str,
    error: reqwest::Error,
    secrets: &[String],
    error_type: &str,
) -> HttpError {
    HttpError {
        message: message.to_string(),
        details: Some(redact_text(&error.to_string(), secrets)),
        error_type: error_type.to_string(),
    }
}

#[cfg(test)]
fn unavailable_secret(name: &str) -> Result<String, ResolveError> {
    Err(ResolveError::MissingSecret(name.to_string()))
}

#[cfg(test)]
mod tests {
    use std::io;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        time::sleep,
    };

    use super::*;

    async fn server(
        response: &'static str,
        delay: Option<Duration>,
    ) -> io::Result<(String, tokio::task::JoinHandle<String>)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0; 8192];
            let read = socket.read(&mut buffer).await.unwrap();
            if let Some(delay) = delay {
                sleep(delay).await;
            }
            socket.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&buffer[..read]).into_owned()
        });
        Ok((format!("http://{address}"), handle))
    }

    #[tokio::test]
    async fn sends_get_with_headers_and_query() {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"ok\":true}";
        let (url, received) = server(response, None).await.unwrap();
        let mut request = RequestFile {
            url,
            ..RequestFile::default()
        };
        request.query.push(crate::models::KeyValue {
            name: "page".to_string(),
            value: "2".to_string(),
            enabled: true,
        });
        request
            .headers
            .insert("X-Test".to_string(), "yes".to_string());
        let result = send(&request, None, &mut unavailable_secret).await.unwrap();
        let raw = received.await.unwrap();
        assert_eq!(result.status, 200);
        assert!(result.is_json);
        assert!(raw.starts_with("GET /?page=2 HTTP/1.1"));
        assert!(raw.to_ascii_lowercase().contains("x-test: yes"));
    }

    #[tokio::test]
    async fn sends_post_json() {
        let response = "HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n";
        let (url, received) = server(response, None).await.unwrap();
        let request = RequestFile {
            method: "POST".to_string(),
            url,
            body: BodyConfig::Json {
                value: "{\"name\":\"Ada\"}".to_string(),
            },
            ..RequestFile::default()
        };
        let result = send(&request, None, &mut unavailable_secret).await.unwrap();
        let raw = received.await.unwrap();
        assert_eq!(result.status, 201);
        assert!(
            raw.to_ascii_lowercase()
                .contains("content-type: application/json")
        );
        assert!(raw.ends_with("{\"name\":\"Ada\"}"));
    }

    #[tokio::test]
    async fn sends_graphql_query_variables_and_operation_name() {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 19\r\n\r\n{\"data\":{\"user\":1}}";
        let (url, received) = server(response, None).await.unwrap();
        let request = RequestFile {
            method: "POST".to_string(),
            url,
            body: BodyConfig::Graphql {
                query: "query GetUser($id: ID!) { user(id: $id) { id } }".to_string(),
                variables: "{\"id\":\"42\"}".to_string(),
                operation_name: "GetUser".to_string(),
            },
            ..RequestFile::default()
        };
        let result = send(&request, None, &mut unavailable_secret).await.unwrap();
        let raw = received.await.unwrap();
        assert_eq!(result.status, 200);
        assert!(raw.contains("\"operationName\":\"GetUser\""));
        assert!(raw.contains("\"variables\":{\"id\":\"42\"}"));
        assert!(raw.contains("query GetUser"));
    }

    #[tokio::test]
    async fn strips_credentials_when_redirect_changes_port() {
        const TEST_SECRET: &str = "REQVAULT_REDIRECT_SECRET_DO_NOT_LEAK_123456";
        let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target_address = target_listener.local_addr().unwrap();
        let target_received = tokio::spawn(async move {
            let (mut socket, _) = target_listener.accept().await.unwrap();
            let mut buffer = vec![0; 8192];
            let read = socket.read(&mut buffer).await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
            String::from_utf8_lossy(&buffer[..read]).into_owned()
        });

        let source_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let source_address = source_listener.local_addr().unwrap();
        let source_received = tokio::spawn(async move {
            let (mut socket, _) = source_listener.accept().await.unwrap();
            let mut buffer = vec![0; 8192];
            let read = socket.read(&mut buffer).await.unwrap();
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{target_address}/final\r\nContent-Length: 0\r\n\r\n"
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&buffer[..read]).into_owned()
        });

        let mut request = RequestFile {
            url: format!("http://{source_address}/start"),
            auth: AuthConfig::Bearer {
                token: "{{secret:API_TOKEN}}".to_string(),
            },
            ..RequestFile::default()
        };
        request.headers.insert(
            "X-Customer-Token".to_string(),
            "{{secret:API_TOKEN}}".to_string(),
        );
        request
            .headers
            .insert("Cookie".to_string(), "session=private".to_string());
        request
            .headers
            .insert("X-Public-Trace".to_string(), "trace-42".to_string());
        let result = send(&request, None, &mut |_| Ok(TEST_SECRET.to_string()))
            .await
            .unwrap();
        let source = source_received.await.unwrap().to_ascii_lowercase();
        let target = target_received.await.unwrap().to_ascii_lowercase();
        assert_eq!(result.status, 200);
        assert!(source.contains("authorization: bearer"));
        assert!(source.contains("x-customer-token:"));
        assert!(source.contains("cookie: session=private"));
        assert!(!target.contains("authorization:"));
        assert!(!target.contains("x-customer-token:"));
        assert!(!target.contains("cookie:"));
        assert!(!target.contains(&TEST_SECRET.to_ascii_lowercase()));
        assert!(target.contains("x-public-trace: trace-42"));
    }

    #[tokio::test]
    async fn returns_http_error_status_as_response() {
        for status in ["404 Not Found", "500 Internal Server Error"] {
            let response = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\n\r\n");
            let leaked: &'static str = Box::leak(response.into_boxed_str());
            let (url, _) = server(leaked, None).await.unwrap();
            let request = RequestFile {
                url,
                ..RequestFile::default()
            };
            let result = send(&request, None, &mut unavailable_secret).await.unwrap();
            assert!(result.status == 404 || result.status == 500);
        }
    }

    #[tokio::test]
    async fn handles_timeout_connection_refused_and_invalid_url() {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let (url, _) = server(response, Some(Duration::from_millis(80)))
            .await
            .unwrap();
        let timeout_request = RequestFile {
            url,
            timeout_ms: 10,
            ..RequestFile::default()
        };
        assert_eq!(
            send(&timeout_request, None, &mut unavailable_secret)
                .await
                .unwrap_err()
                .error_type,
            "timeout"
        );

        let unused_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let unused_address = unused_listener.local_addr().unwrap();
        drop(unused_listener);
        let refused = RequestFile {
            url: format!("http://{unused_address}"),
            timeout_ms: 500,
            ..RequestFile::default()
        };
        let refused_error = send(&refused, None, &mut unavailable_secret)
            .await
            .unwrap_err();
        assert!(matches!(
            refused_error.error_type.as_str(),
            "connection" | "timeout"
        ));
        assert!(refused_error.message.contains("127.0.0.1"));

        let invalid = RequestFile {
            url: "not a url".to_string(),
            ..RequestFile::default()
        };
        assert_eq!(
            send(&invalid, None, &mut unavailable_secret)
                .await
                .unwrap_err()
                .error_type,
            "validation"
        );
    }

    #[tokio::test]
    async fn redacts_secret_from_response_and_error() {
        const TEST_SECRET: &str = "REQVAULT_TEST_SECRET_DO_NOT_LEAK_123456";
        let response = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{TEST_SECRET}",
                TEST_SECRET.len()
            )
            .into_boxed_str(),
        );
        let (url, _) = server(response, None).await.unwrap();
        let mut request = RequestFile {
            url,
            ..RequestFile::default()
        };
        request.headers.insert(
            "X-Test-Token".to_string(),
            "{{secret:API_TOKEN}}".to_string(),
        );
        let mut resolve_secret = |_: &str| Ok(TEST_SECRET.to_string());
        let result = send(&request, None, &mut resolve_secret).await.unwrap();
        assert!(!result.body.contains(TEST_SECRET));

        let unused_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let unused_address = unused_listener.local_addr().unwrap();
        drop(unused_listener);
        let mut request = RequestFile {
            url: format!("http://{unused_address}"),
            timeout_ms: 500,
            ..RequestFile::default()
        };
        request.query.push(crate::models::KeyValue {
            name: "token".to_string(),
            value: "{{secret:API_TOKEN}}".to_string(),
            enabled: true,
        });
        let error = send(&request, None, &mut resolve_secret).await.unwrap_err();
        let visible_error = format!("{} {:?}", error.message, error.details);
        assert!(visible_error.contains("***REDACTED***"));
        assert!(!visible_error.contains(TEST_SECRET));
    }

    #[tokio::test]
    async fn sends_multipart_text_and_file() {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let (url, received) = server(response, None).await.unwrap();
        let file_path =
            std::env::temp_dir().join(format!("reqvault-upload-{}.txt", Uuid::new_v4()));
        fs::write(&file_path, "multipart-test-file").unwrap();
        let request = RequestFile {
            method: "POST".to_string(),
            url,
            body: BodyConfig::Multipart {
                fields: vec![
                    crate::models::MultipartField::Text {
                        name: "description".to_string(),
                        value: "test upload".to_string(),
                        enabled: true,
                    },
                    crate::models::MultipartField::File {
                        name: "file".to_string(),
                        path: file_path.to_string_lossy().into_owned(),
                        content_type: "text/plain".to_string(),
                        enabled: true,
                    },
                ],
            },
            ..RequestFile::default()
        };
        let result = send(&request, None, &mut unavailable_secret).await.unwrap();
        let raw = received.await.unwrap();
        assert_eq!(result.status, 200);
        assert!(raw.contains("name=\"description\""));
        assert!(raw.contains("test upload"));
        assert!(raw.contains("name=\"file\""));
        assert!(raw.contains("multipart-test-file"));
        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn builds_rfc_digest_authorization() {
        let authorization = digest_authorization_with_cnonce(
            "Digest realm=\"testrealm@host.com\", qop=\"auth,auth-int\", nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", opaque=\"5ccc069c403ebaf9f0171e9517f40e41\"",
            &Method::GET,
            &Url::parse("http://www.example.com/dir/index.html").unwrap(),
            "Mufasa",
            "Circle Of Life",
            "0a4f113b",
        )
        .unwrap();
        assert!(authorization.starts_with("Digest username=\"Mufasa\""));
        assert!(authorization.contains("response=\"6629fae49393a05397450978507c4ef1\""));
        assert!(authorization.contains("qop=auth"));
        assert!(authorization.contains("nc=00000001"));
    }

    #[test]
    fn signs_aws_documentation_request() {
        let request = RequestFile {
            url: "https://iam.amazonaws.com/?Action=ListUsers&Version=2010-05-08".to_string(),
            auth: AuthConfig::AwsSigV4 {
                access_key: "{{secret:AWS_ACCESS_KEY}}".to_string(),
                secret_key: "{{secret:AWS_SECRET_KEY}}".to_string(),
                session_token: String::new(),
                region: "us-east-1".to_string(),
                service: "iam".to_string(),
            },
            ..RequestFile::default()
        };
        let mut resolver = |name: &str| match name {
            "AWS_ACCESS_KEY" => Ok("AKIDEXAMPLE".to_string()),
            "AWS_SECRET_KEY" => Ok("wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".to_string()),
            _ => Err(ResolveError::MissingSecret(name.to_string())),
        };
        let mut prepared = prepare(&request, None, &mut resolver).unwrap();
        sign_aws_request(
            &mut prepared,
            time::macros::datetime!(2015-08-30 12:36:00 UTC),
        )
        .unwrap();
        assert_eq!(prepared.headers["x-amz-date"], "20150830T123600Z");
        assert_eq!(
            prepared.headers["x-amz-content-sha256"],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let authorization = prepared.headers[AUTHORIZATION].to_str().unwrap();
        assert!(
            authorization.ends_with(
                "Signature=65f031d93b4631aedf16a8f7f830cdc8ce2bc5276c307b5a2cc2143d4b68e323"
            ),
            "{authorization}"
        );
    }

    #[tokio::test]
    async fn retries_digest_challenge() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let received = tokio::spawn(async move {
            let (mut first, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0; 4096];
            let _ = first.read(&mut buffer).await.unwrap();
            first
                .write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Digest realm=\"ReqVault\", nonce=\"test-nonce\", algorithm=SHA-256, qop=\"auth\"\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
            let (mut second, _) = listener.accept().await.unwrap();
            let read = second.read(&mut buffer).await.unwrap();
            second
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
            String::from_utf8_lossy(&buffer[..read]).into_owned()
        });
        let request = RequestFile {
            url: format!("http://{address}/private"),
            auth: AuthConfig::Digest {
                username: "pavel".to_string(),
                password: "{{secret:DIGEST_PASSWORD}}".to_string(),
            },
            ..RequestFile::default()
        };
        let mut resolver = |name: &str| match name {
            "DIGEST_PASSWORD" => Ok("secret".to_string()),
            _ => Err(ResolveError::MissingSecret(name.to_string())),
        };
        let response = send(&request, None, &mut resolver).await.unwrap();
        let raw = received.await.unwrap();
        assert_eq!(response.status, 200);
        assert!(raw.to_ascii_lowercase().contains("authorization: digest "));
        assert!(raw.contains("algorithm=SHA-256"));
    }

    #[tokio::test]
    async fn stores_and_sends_session_cookie() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let received = tokio::spawn(async move {
            let (mut first, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0; 4096];
            let _ = first.read(&mut buffer).await.unwrap();
            first
                .write_all(b"HTTP/1.1 200 OK\r\nSet-Cookie: session=private; Path=/; HttpOnly; SameSite=Lax\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
            let (mut second, _) = listener.accept().await.unwrap();
            let read = second.read(&mut buffer).await.unwrap();
            second
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
            String::from_utf8_lossy(&buffer[..read]).into_owned()
        });
        let jar = CookieJar::default();
        let request = RequestFile {
            url: format!("http://{address}/account"),
            ..RequestFile::default()
        };
        send_with_session(&request, None, &mut unavailable_secret, Some(&jar), None)
            .await
            .unwrap();
        send_with_session(&request, None, &mut unavailable_secret, Some(&jar), None)
            .await
            .unwrap();
        let raw = received.await.unwrap();
        assert!(raw.to_ascii_lowercase().contains("cookie: session=private"));
        assert_eq!(
            jar.request_header(&Url::parse(&request.url).unwrap())
                .as_deref(),
            Some("session=private")
        );
    }

    #[test]
    fn classifies_text_image_and_binary_response_bodies() {
        let (body, kind, is_json) =
            response_body("application/json; charset=utf-8", br#"{"ok":true}"#, &[]);
        assert_eq!(body, r#"{"ok":true}"#);
        assert_eq!(kind, "json");
        assert!(is_json);

        let bytes = [0_u8, 159, 146, 150];
        let (body, kind, is_json) = response_body("image/png", &bytes, &[]);
        assert_eq!(body, "AJ+Slg==");
        assert_eq!(kind, "image");
        assert!(!is_json);

        let (_, kind, _) = response_body("application/pdf", &bytes, &[]);
        assert_eq!(kind, "binary");
    }

    #[test]
    fn validates_proxy_and_tls_files() {
        let mut request = RequestFile {
            url: "https://api.example.test".to_string(),
            ..RequestFile::default()
        };
        request.transport.proxy = ProxyConfig::Custom {
            url: "http://127.0.0.1:8080".to_string(),
            username: String::new(),
            password: String::new(),
        };
        assert!(prepare(&request, None, &mut unavailable_secret).is_ok());

        request.transport.client_certificate_path = "certificate.pem".to_string();
        assert!(matches!(
            prepare(&request, None, &mut unavailable_secret),
            Err(PrepareError::IncompleteIdentity)
        ));
    }
}
