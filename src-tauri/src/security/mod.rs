use std::collections::BTreeMap;

use url::Url;

use crate::{
    models::{
        AuthConfig, BodyConfig, EnvironmentFile, MultipartField, ProxyConfig, RequestFile,
        SecurityReport,
    },
    redaction::{is_sensitive_header, redact_header},
    variables::{redact_secret_references, resolve_variables, secret_names},
};

const ALLOWED_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

pub fn analyze(request: &RequestFile, environment: Option<&EnvironmentFile>) -> SecurityReport {
    let variables = variables(environment);
    let resolved_url =
        resolve_variables(&request.url, &variables).unwrap_or_else(|_| request.url.clone());
    let safe_url = redact_secret_references(&resolved_url);
    let parsed = Url::parse(&safe_url).ok();
    let https = parsed
        .as_ref()
        .map(|url| url.scheme() == "https")
        .unwrap_or(false);
    let host = parsed
        .as_ref()
        .and_then(Url::host_str)
        .unwrap_or("—")
        .to_string();

    let mut in_headers = request
        .headers
        .iter()
        .map(|(name, value)| reference_count(name) + reference_count(value))
        .sum::<usize>();
    let mut in_query = request
        .query
        .iter()
        .filter(|item| item.enabled)
        .map(|item| reference_count(&item.name) + reference_count(&item.value))
        .sum::<usize>();
    if let Some(query) = request.url.split_once('?').map(|(_, query)| query) {
        in_query += reference_count(query);
    }

    let auth_count = match &request.auth {
        AuthConfig::None => 0,
        AuthConfig::Bearer { token } => {
            let count = reference_count(token);
            in_headers += count;
            count
        }
        AuthConfig::Basic { username, password } | AuthConfig::Digest { username, password } => {
            let count = reference_count(username) + reference_count(password);
            in_headers += count;
            count
        }
        AuthConfig::ApiKeyHeader { name, value } => {
            let count = reference_count(name) + reference_count(value);
            in_headers += count;
            count
        }
        AuthConfig::ApiKeyQuery { name, value } => {
            let count = reference_count(name) + reference_count(value);
            in_query += count;
            count
        }
        AuthConfig::OAuth2 {
            access_token,
            client_secret,
            refresh_token,
            ..
        } => {
            let count = reference_count(access_token)
                + reference_count(client_secret)
                + reference_count(refresh_token);
            in_headers += reference_count(access_token);
            count
        }
        AuthConfig::AwsSigV4 {
            access_key,
            secret_key,
            session_token,
            ..
        } => {
            let count = reference_count(access_key)
                + reference_count(secret_key)
                + reference_count(session_token);
            in_headers += count;
            count
        }
    };
    let direct_url_count = request
        .url
        .split_once('?')
        .map(|(path, _)| reference_count(path))
        .unwrap_or_else(|| reference_count(&request.url));
    let body_count = body_strings(&request.body)
        .iter()
        .map(|value| reference_count(value))
        .sum::<usize>();
    let headers_count = request
        .headers
        .iter()
        .map(|(name, value)| reference_count(name) + reference_count(value))
        .sum::<usize>();
    let query_count = request
        .query
        .iter()
        .filter(|item| item.enabled)
        .map(|item| reference_count(&item.name) + reference_count(&item.value))
        .sum::<usize>()
        + request
            .url
            .split_once('?')
            .map(|(_, query)| reference_count(query))
            .unwrap_or(0);
    let secrets = direct_url_count + headers_count + query_count + auth_count + body_count;

    let mut warnings = Vec::new();
    if in_query > 0 {
        warnings.push(
            "Секрет используется в URL. Он может попасть в логи сервера или прокси.".to_string(),
        );
    }
    let local_http = host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1";
    if parsed.is_some() && !https && !local_http {
        warnings.push(
            "Запрос использует HTTP. Данные передаются без защиты транспортного уровня."
                .to_string(),
        );
    }

    SecurityReport {
        https,
        host,
        secrets,
        in_headers,
        in_query,
        warnings,
    }
}

pub fn curl(
    request: &RequestFile,
    environment: Option<&EnvironmentFile>,
) -> Result<String, String> {
    let method = request.method.trim().to_uppercase();
    if !ALLOWED_METHODS.contains(&method.as_str()) {
        return Err(format!("HTTP-метод {method} не поддерживается"));
    }

    let variables = variables(environment);
    let safe = |value: &str| {
        resolve_variables(value, &variables)
            .map(|resolved| redact_secret_references(&resolved))
            .map_err(|error| error.to_string())
    };

    let resolved_url = safe(&request.url)?;
    let mut url = Url::parse(&resolved_url)
        .map_err(|_| "Укажи корректный URL с http:// или https://".to_string())?;
    {
        let mut query = url.query_pairs_mut();
        for item in request
            .query
            .iter()
            .filter(|item| item.enabled && !item.name.is_empty())
        {
            query.append_pair(&safe(&item.name)?, &safe(&item.value)?);
        }
        if let AuthConfig::ApiKeyQuery { name, .. } = &request.auth {
            query.append_pair(&safe(name)?, "***REDACTED***");
        }
    }

    let mut parts = vec![format!("curl -X {} {}", method, shell_quote(url.as_str()))];
    for (name, value) in &request.headers {
        let safe_name = safe(name)?;
        let safe_value = if is_sensitive_header(&safe_name) {
            redact_header(&safe_name, value, &[])
        } else {
            safe(value)?
        };
        parts.push(format!(
            "  -H {}",
            shell_quote(&format!("{safe_name}: {safe_value}"))
        ));
    }

    match &request.auth {
        AuthConfig::None | AuthConfig::ApiKeyQuery { .. } => {}
        AuthConfig::Bearer { .. } => parts.push(format!(
            "  -H {}",
            shell_quote("Authorization: Bearer ***REDACTED***")
        )),
        AuthConfig::Basic { .. } => parts.push(format!("  -u {}", shell_quote("***REDACTED***"))),
        AuthConfig::Digest { username, .. } => {
            parts.push("  --digest".to_string());
            parts.push(format!(
                "  -u {}",
                shell_quote(&format!("{}:***REDACTED***", safe(username)?))
            ));
        }
        AuthConfig::ApiKeyHeader { name, .. } => parts.push(format!(
            "  -H {}",
            shell_quote(&format!("{}: ***REDACTED***", safe(name)?))
        )),
        AuthConfig::OAuth2 { .. } => parts.push(format!(
            "  -H {}",
            shell_quote("Authorization: Bearer ***REDACTED***")
        )),
        AuthConfig::AwsSigV4 {
            region, service, ..
        } => parts.push(format!(
            "  --aws-sigv4 {}",
            shell_quote(&format!("aws:amz:{}:{}", safe(region)?, safe(service)?))
        )),
    }

    match &request.body {
        BodyConfig::None => {}
        BodyConfig::Json { value } => {
            if !request
                .headers
                .keys()
                .any(|name| name.eq_ignore_ascii_case("content-type"))
            {
                parts.push(format!(
                    "  -H {}",
                    shell_quote("Content-Type: application/json")
                ));
            }
            parts.push(format!("  --data-raw {}", shell_quote(&safe(value)?)));
        }
        BodyConfig::Graphql {
            query,
            variables,
            operation_name,
        } => {
            if !request
                .headers
                .keys()
                .any(|name| name.eq_ignore_ascii_case("content-type"))
            {
                parts.push(format!(
                    "  -H {}",
                    shell_quote("Content-Type: application/json")
                ));
            }
            let variables = safe(variables)?;
            let variables = serde_json::from_str::<serde_json::Value>(&variables)
                .unwrap_or_else(|_| serde_json::json!({}));
            let mut payload = serde_json::Map::from_iter([
                ("query".to_string(), serde_json::Value::String(safe(query)?)),
                ("variables".to_string(), variables),
            ]);
            if !operation_name.trim().is_empty() {
                payload.insert(
                    "operationName".to_string(),
                    serde_json::Value::String(safe(operation_name)?),
                );
            }
            let body = serde_json::to_string(&serde_json::Value::Object(payload))
                .map_err(|_| "Не удалось подготовить GraphQL cURL".to_string())?;
            parts.push(format!("  --data-raw {}", shell_quote(&body)));
        }
        BodyConfig::Raw {
            value,
            content_type,
        } => {
            if !content_type.is_empty() {
                parts.push(format!(
                    "  -H {}",
                    shell_quote(&format!("Content-Type: {}", safe(content_type)?))
                ));
            }
            parts.push(format!("  --data-raw {}", shell_quote(&safe(value)?)));
        }
        BodyConfig::FormUrlencoded { fields } => {
            for field in fields
                .iter()
                .filter(|field| field.enabled && !field.name.is_empty())
            {
                parts.push(format!(
                    "  --data-urlencode {}",
                    shell_quote(&format!("{}={}", safe(&field.name)?, safe(&field.value)?))
                ));
            }
        }
        BodyConfig::Multipart { fields } => {
            for field in fields {
                match field {
                    MultipartField::Text {
                        name,
                        value,
                        enabled: true,
                    } if !name.is_empty() => parts.push(format!(
                        "  -F {}",
                        shell_quote(&format!("{}={}", safe(name)?, safe(value)?))
                    )),
                    MultipartField::File {
                        name,
                        path,
                        content_type,
                        enabled: true,
                    } if !name.is_empty() => {
                        let suffix = if content_type.is_empty() {
                            String::new()
                        } else {
                            format!(";type={}", safe(content_type)?)
                        };
                        parts.push(format!(
                            "  -F {}",
                            shell_quote(&format!("{}=@{}{}", safe(name)?, safe(path)?, suffix))
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    match &request.transport.proxy {
        ProxyConfig::None => parts.push("  --noproxy '*'".to_string()),
        ProxyConfig::System => {}
        ProxyConfig::Custom {
            url,
            username,
            password: _,
        } => {
            let safe_url = safe_proxy_url(&safe(url)?)?;
            parts.push(format!("  --proxy {}", shell_quote(&safe_url)));
            if !username.is_empty() {
                parts.push(format!(
                    "  --proxy-user {}",
                    shell_quote(&format!("{}:***REDACTED***", safe(username)?))
                ));
            }
        }
    }
    if !request.transport.custom_ca_path.is_empty() {
        parts.push(format!(
            "  --cacert {}",
            shell_quote(&safe(&request.transport.custom_ca_path)?)
        ));
    }
    if !request.transport.client_certificate_path.is_empty() {
        parts.push(format!(
            "  --cert {}",
            shell_quote(&safe(&request.transport.client_certificate_path)?)
        ));
    }
    if !request.transport.client_key_path.is_empty() {
        parts.push(format!(
            "  --key {}",
            shell_quote(&safe(&request.transport.client_key_path)?)
        ));
    }

    Ok(parts.join(" \\\n"))
}

fn variables(environment: Option<&EnvironmentFile>) -> BTreeMap<String, String> {
    environment
        .map(|environment| environment.variables.clone())
        .unwrap_or_default()
}

fn reference_count(value: &str) -> usize {
    secret_names(value).len()
}

fn body_strings(body: &BodyConfig) -> Vec<&str> {
    match body {
        BodyConfig::None => Vec::new(),
        BodyConfig::Json { value } => vec![value],
        BodyConfig::Graphql {
            query,
            variables,
            operation_name,
        } => vec![query, variables, operation_name],
        BodyConfig::Raw {
            value,
            content_type,
        } => vec![value, content_type],
        BodyConfig::FormUrlencoded { fields } => fields
            .iter()
            .filter(|field| field.enabled)
            .flat_map(|field| [field.name.as_str(), field.value.as_str()])
            .collect(),
        BodyConfig::Multipart { fields } => fields
            .iter()
            .filter_map(|field| match field {
                MultipartField::Text {
                    name,
                    value,
                    enabled: true,
                } => Some(vec![name.as_str(), value.as_str()]),
                MultipartField::File {
                    name,
                    path,
                    content_type,
                    enabled: true,
                } => Some(vec![name.as_str(), path.as_str(), content_type.as_str()]),
                _ => None,
            })
            .flatten()
            .collect(),
    }
}

fn safe_proxy_url(value: &str) -> Result<String, String> {
    let mut url = Url::parse(value).map_err(|_| "Укажи корректный URL proxy".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" && url.scheme() != "socks5" {
        return Err("Proxy должен использовать http://, https:// или socks5://".to_string());
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    Ok(url.to_string())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "REQVAULT_TEST_SECRET_DO_NOT_LEAK_123456";

    #[test]
    fn reports_query_secrets_and_insecure_http() {
        let mut request = RequestFile {
            url: "http://api.example.test/users".to_string(),
            ..RequestFile::default()
        };
        request.query.push(crate::models::KeyValue {
            name: "token".to_string(),
            value: "{{secret:API_TOKEN}}".to_string(),
            enabled: true,
        });
        request.auth = AuthConfig::Bearer {
            token: "{{secret:SECOND}}".to_string(),
        };
        let report = analyze(&request, None);
        assert!(!report.https);
        assert_eq!(report.host, "api.example.test");
        assert_eq!(report.secrets, 2);
        assert_eq!(report.in_headers, 1);
        assert_eq!(report.in_query, 1);
        assert_eq!(report.warnings.len(), 2);
    }

    #[test]
    fn allows_plain_http_for_localhost() {
        let request = RequestFile {
            url: "http://127.0.0.1:3000".to_string(),
            ..RequestFile::default()
        };
        assert!(analyze(&request, None).warnings.is_empty());
    }

    #[test]
    fn generated_curl_never_contains_secret() {
        let mut request = RequestFile {
            url: "https://api.example.test/users".to_string(),
            auth: AuthConfig::Bearer {
                token: "{{secret:API_TOKEN}}".to_string(),
            },
            ..RequestFile::default()
        };
        request
            .headers
            .insert("X-Trace".to_string(), "{{secret:TRACE_TOKEN}}".to_string());
        let output = curl(&request, None).unwrap();
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("{{secret:"));
        assert!(!output.contains(TEST_SECRET));
    }

    #[test]
    fn rejects_unsafe_method_in_curl() {
        let request = RequestFile {
            method: "GET; echo compromised".to_string(),
            url: "https://api.example.test/users".to_string(),
            ..RequestFile::default()
        };

        assert!(curl(&request, None).is_err());
    }

    #[test]
    fn graphql_curl_keeps_payload_and_redacts_secret_references() {
        let request = RequestFile {
            method: "POST".to_string(),
            url: "https://api.example.test/graphql".to_string(),
            body: BodyConfig::Graphql {
                query: "query User { user { id } }".to_string(),
                variables: "{\"token\":\"{{secret:GRAPHQL_TOKEN}}\"}".to_string(),
                operation_name: "User".to_string(),
            },
            ..RequestFile::default()
        };
        let output = curl(&request, None).unwrap();
        assert!(output.contains("operationName"));
        assert!(output.contains("***REDACTED***"));
        assert!(!output.contains("GRAPHQL_TOKEN"));
    }
}
