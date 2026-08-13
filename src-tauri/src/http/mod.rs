use std::{fs, time::Duration};

use reqwest::{
    Certificate, Client, Identity, Method, Proxy,
    header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
};
use url::Url;
use uuid::Uuid;

use crate::{
    models::{
        AuthConfig, BodyConfig, EnvironmentFile, HttpError, HttpResponse, ProxyConfig, RequestFile,
        ResponseHeader,
    },
    redaction::{redact_header, redact_text},
    variables::{ResolveError, is_exact_secret_reference, resolve_secrets, resolve_variables},
};

const ALLOWED_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

#[derive(Debug, Clone)]
struct PreparedRequest {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: BodyConfig,
    timeout: Duration,
    follow_redirects: bool,
    secret_values: Vec<String>,
    transport: PreparedTransport,
}

#[derive(Debug, Clone)]
struct PreparedTransport {
    proxy: PreparedProxy,
    custom_ca: Option<Vec<u8>>,
    identity: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
enum PreparedProxy {
    None,
    System,
    Custom {
        url: String,
        username: String,
        password: String,
    },
}

pub async fn send<F>(
    request: &RequestFile,
    environment: Option<&EnvironmentFile>,
    resolve_secret: &mut F,
) -> Result<HttpResponse, HttpError>
where
    F: FnMut(&str) -> Result<String, ResolveError>,
{
    let prepared = prepare(request, environment, resolve_secret).map_err(resolve_error)?;
    execute(prepared).await
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
    let mut secret_values = Vec::new();
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
        let resolved_name = resolve(name)?;
        let resolved_value = resolve(value)?;
        let header_name = HeaderName::from_bytes(resolved_name.as_bytes())
            .map_err(|_| PrepareError::InvalidHeaderName(resolved_name.clone()))?;
        let header_value = HeaderValue::from_str(&resolved_value)
            .map_err(|_| PrepareError::InvalidHeaderValue(resolved_name.clone()))?;
        headers.insert(header_name, header_value);
    }

    match &request.auth {
        AuthConfig::None | AuthConfig::ApiKeyQuery { .. } => {}
        AuthConfig::Bearer { token } => {
            let value = HeaderValue::from_str(&format!("Bearer {}", resolve(token)?))
                .map_err(|_| PrepareError::InvalidHeaderValue("Authorization".to_string()))?;
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }
        AuthConfig::Basic { username, password } => {
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            let credentials = format!("{}:{}", resolve(username)?, resolve(password)?);
            let value =
                HeaderValue::from_str(&format!("Basic {}", STANDARD.encode(credentials)))
                    .map_err(|_| PrepareError::InvalidHeaderValue("Authorization".to_string()))?;
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }
        AuthConfig::ApiKeyHeader { name, value } => {
            let resolved_name = resolve(name)?;
            let header_name = HeaderName::from_bytes(resolved_name.as_bytes())
                .map_err(|_| PrepareError::InvalidHeaderName(resolved_name.clone()))?;
            let header_value = HeaderValue::from_str(&resolve(value)?)
                .map_err(|_| PrepareError::InvalidHeaderValue(resolved_name))?;
            headers.insert(header_name, header_value);
        }
        AuthConfig::OAuth2 { access_token, .. } => {
            let value = HeaderValue::from_str(&format!("Bearer {}", resolve(access_token)?))
                .map_err(|_| PrepareError::InvalidHeaderValue("Authorization".to_string()))?;
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }
    }

    let body = match &request.body {
        BodyConfig::None => BodyConfig::None,
        BodyConfig::Json { value } => {
            let value = resolve(value)?;
            serde_json::from_str::<serde_json::Value>(&value)
                .map_err(|error| PrepareError::InvalidJson(error.to_string()))?;
            BodyConfig::Json { value }
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

async fn execute(prepared: PreparedRequest) -> Result<HttpResponse, HttpError> {
    let request_id = Uuid::new_v4().to_string();
    let host = prepared.url.host_str().unwrap_or("сервер").to_string();
    let mut client_builder = Client::builder()
        .redirect(if prepared.follow_redirects {
            Policy::limited(10)
        } else {
            Policy::none()
        })
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

    let mut builder = client
        .request(prepared.method, prepared.url)
        .headers(prepared.headers);
    builder = match &prepared.body {
        BodyConfig::None => builder,
        BodyConfig::Json { value } => builder
            .header(CONTENT_TYPE, "application/json")
            .body(value.clone()),
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
                            details: Some(redact_text(&error.to_string(), &prepared.secret_values)),
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
                                details: Some(redact_text(
                                    &error.to_string(),
                                    &prepared.secret_values,
                                )),
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

    let started = std::time::Instant::now();
    let response = builder.send().await.map_err(|error| {
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
        request_error(&message, error, &prepared.secret_values, error_type)
    })?;
    let duration_ms = started.elapsed().as_millis();
    let status = response.status();
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
    let bytes = response.bytes().await.map_err(|error| {
        request_error(
            "Не удалось прочитать ответ сервера",
            error,
            &prepared.secret_values,
            "response",
        )
    })?;
    let size_bytes = bytes.len();
    let raw_body = String::from_utf8_lossy(&bytes).into_owned();
    let body = redact_text(&raw_body, &prepared.secret_values);
    let is_json = serde_json::from_str::<serde_json::Value>(&body).is_ok();

    Ok(HttpResponse {
        request_id,
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        duration_ms,
        size_bytes,
        headers,
        body,
        is_json,
    })
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
