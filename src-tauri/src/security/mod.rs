use std::collections::BTreeMap;

use url::Url;

use crate::{
    models::{AuthConfig, BodyConfig, EnvironmentFile, RequestFile, SecurityReport},
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
        AuthConfig::Basic { username, password } => {
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
        AuthConfig::ApiKeyHeader { name, .. } => parts.push(format!(
            "  -H {}",
            shell_quote(&format!("{}: ***REDACTED***", safe(name)?))
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
        BodyConfig::Raw {
            value,
            content_type,
        } => vec![value, content_type],
        BodyConfig::FormUrlencoded { fields } => fields
            .iter()
            .filter(|field| field.enabled)
            .flat_map(|field| [field.name.as_str(), field.value.as_str()])
            .collect(),
    }
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
}
