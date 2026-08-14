use std::{path::Path, time::Instant};

use serde_json::Value;

use crate::{
    guard, http,
    models::{
        AssertionResult, AuthConfig, CollectionRunOptions, CollectionRunReport, EnvironmentFile,
        HttpResponse, RequestFile, RequestRunResult, ResponseAssertion,
    },
    oauth,
    secrets::{self, KeyringBackend},
    session::CookieJar,
    variables::ResolveError,
    workspace,
};

pub async fn run_workspace(
    workspace_path: &Path,
    options: &CollectionRunOptions,
) -> Result<CollectionRunReport, String> {
    let snapshot = workspace::open(workspace_path).map_err(|error| error.to_string())?;
    let environment = select_environment(&snapshot.environments, options.environment.as_deref())?;
    let backend = KeyringBackend::new(&snapshot.config.id).map_err(|error| error.to_string())?;
    let started_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let started = Instant::now();
    let mut results = Vec::new();
    let cookie_jar = CookieJar::default();

    for summary in snapshot.requests.iter().filter(|summary| {
        options.collection.as_deref().is_none_or(|collection| {
            collection_name(&summary.relative_path).eq_ignore_ascii_case(collection)
        })
    }) {
        let result = run_request(
            &summary.relative_path,
            &summary.request,
            environment,
            &snapshot.config.production_guard,
            &backend,
            &cookie_jar,
        )
        .await;
        let failed = !result.passed;
        results.push(result);
        if failed && options.stop_on_failure {
            break;
        }
    }
    if results.is_empty() {
        return Err("В выбранной коллекции нет запросов".to_string());
    }
    let passed = results.iter().filter(|result| result.passed).count();
    let total = results.len();
    Ok(CollectionRunReport {
        started_at_ms,
        duration_ms: started.elapsed().as_millis(),
        total,
        passed,
        failed: total - passed,
        results,
    })
}

async fn run_request(
    relative_path: &str,
    request: &RequestFile,
    environment: Option<&EnvironmentFile>,
    production_guard: &crate::models::ProductionGuard,
    backend: &KeyringBackend,
    cookie_jar: &CookieJar,
) -> RequestRunResult {
    if let Err(error) = guard::validate(request, environment, production_guard) {
        return failed_result(relative_path, request, error);
    }
    let send = || async {
        http::send_with_session(
            request,
            environment,
            &mut |name| {
                secrets::get(backend, name).map_err(|error| match error {
                    secrets::SecretError::NotFound(name) => ResolveError::MissingSecret(name),
                    _ => ResolveError::SecretStorage,
                })
            },
            Some(cookie_jar),
            Some(production_guard),
        )
        .await
    };
    let mut response = match send().await {
        Ok(response) => response,
        Err(error) => return failed_result(relative_path, request, error.message),
    };
    if response.status == 401
        && matches!(&request.auth, AuthConfig::OAuth2 { .. })
        && oauth::refresh(&request.auth, environment, backend)
            .await
            .is_ok()
    {
        response = match send().await {
            Ok(response) => response,
            Err(error) => return failed_result(relative_path, request, error.message),
        };
    }

    let assertions = evaluate(&response, &request.tests);
    let passed = response.status < 400 && assertions.iter().all(|result| result.passed);
    RequestRunResult {
        relative_path: relative_path.to_string(),
        request_name: request.name.clone(),
        method: request.method.clone(),
        status: Some(response.status),
        duration_ms: Some(response.duration_ms),
        passed,
        assertions,
        error: (response.status >= 400)
            .then(|| format!("HTTP {} {}", response.status, response.status_text.trim())),
    }
}

fn failed_result(relative_path: &str, request: &RequestFile, error: String) -> RequestRunResult {
    RequestRunResult {
        relative_path: relative_path.to_string(),
        request_name: request.name.clone(),
        method: request.method.clone(),
        status: None,
        duration_ms: None,
        passed: false,
        assertions: Vec::new(),
        error: Some(error),
    }
}

fn select_environment<'a>(
    environments: &'a [crate::models::EnvironmentSummary],
    selected: Option<&str>,
) -> Result<Option<&'a EnvironmentFile>, String> {
    let Some(selected) = selected.filter(|value| !value.trim().is_empty()) else {
        return Ok(environments.first().map(|item| &item.environment));
    };
    environments
        .iter()
        .find(|item| {
            item.relative_path.eq_ignore_ascii_case(selected)
                || item.environment.name.eq_ignore_ascii_case(selected)
        })
        .map(|item| Some(&item.environment))
        .ok_or_else(|| format!("Окружение {selected} не найдено"))
}

fn collection_name(relative_path: &str) -> &str {
    relative_path.split('/').nth(1).unwrap_or("Общее")
}

pub fn evaluate(response: &HttpResponse, assertions: &[ResponseAssertion]) -> Vec<AssertionResult> {
    assertions
        .iter()
        .filter(|assertion| assertion.enabled())
        .map(|assertion| evaluate_one(response, assertion))
        .collect()
}

fn evaluate_one(response: &HttpResponse, assertion: &ResponseAssertion) -> AssertionResult {
    match assertion {
        ResponseAssertion::Status { expected, .. } => assertion_result(
            response.status == *expected,
            "HTTP status".to_string(),
            expected.to_string(),
            response.status.to_string(),
        ),
        ResponseAssertion::Header {
            name,
            operator,
            expected,
            ..
        } => {
            let actual = response
                .headers
                .iter()
                .find(|header| header.name.eq_ignore_ascii_case(name))
                .map(|header| header.value.as_str());
            let passed = compare_optional(actual, operator, expected);
            assertion_result(
                passed,
                format!("Header {name}"),
                expectation(operator, expected),
                actual.unwrap_or("<отсутствует>").to_string(),
            )
        }
        ResponseAssertion::JsonPath {
            path,
            operator,
            expected,
            ..
        } => {
            let json = serde_json::from_str::<Value>(&response.body).ok();
            let actual_value = json.as_ref().and_then(|value| json_path(value, path));
            let actual = actual_value.map(json_string);
            let passed = match operator.as_str() {
                "exists" => actual_value.is_some(),
                "equals" => actual_value.is_some_and(|value| json_equals(value, expected)),
                "contains" => actual
                    .as_deref()
                    .is_some_and(|value| value.contains(expected)),
                _ => false,
            };
            assertion_result(
                passed,
                format!("JSON {path}"),
                expectation(operator, expected),
                actual.unwrap_or_else(|| "<отсутствует>".to_string()),
            )
        }
        ResponseAssertion::BodyContains { expected, .. } => assertion_result(
            response.body.contains(expected),
            "Тело ответа".to_string(),
            format!("содержит {expected}"),
            truncate(&response.body, 160),
        ),
        ResponseAssertion::ResponseTime { max_ms, .. } => assertion_result(
            response.duration_ms <= u128::from(*max_ms),
            "Время ответа".to_string(),
            format!("≤ {max_ms} мс"),
            format!("{} мс", response.duration_ms),
        ),
    }
}

fn compare_optional(actual: Option<&str>, operator: &str, expected: &str) -> bool {
    match operator {
        "exists" => actual.is_some(),
        "equals" => actual == Some(expected),
        "contains" => actual.is_some_and(|value| value.contains(expected)),
        _ => false,
    }
}

fn expectation(operator: &str, expected: &str) -> String {
    match operator {
        "exists" => "существует".to_string(),
        "equals" => format!("равно {expected}"),
        "contains" => format!("содержит {expected}"),
        _ => format!("{operator} {expected}"),
    }
}

fn json_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() || path == "$" {
        return Some(root);
    }
    if let Some(pointer) = path.strip_prefix('/') {
        return root.pointer(&format!("/{pointer}"));
    }
    let normalized = path.trim_start_matches("$.");
    normalized.split('.').try_fold(root, |value, part| {
        if let Ok(index) = part.parse::<usize>() {
            value.get(index)
        } else {
            value.get(part)
        }
    })
}

fn json_equals(actual: &Value, expected: &str) -> bool {
    serde_json::from_str::<Value>(expected)
        .map(|expected| &expected == actual)
        .unwrap_or_else(|_| json_string(actual) == expected)
}

fn json_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn assertion_result(
    passed: bool,
    label: String,
    expected: String,
    actual: String,
) -> AssertionResult {
    AssertionResult {
        passed,
        label,
        expected,
        actual,
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        format!("{}…", value.chars().take(max).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    use uuid::Uuid;

    use crate::models::ResponseHeader;

    use super::*;

    fn response() -> HttpResponse {
        HttpResponse {
            request_id: "test".to_string(),
            status: 200,
            status_text: "OK".to_string(),
            duration_ms: 42,
            size_bytes: 24,
            headers: vec![ResponseHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            }],
            body: r#"{"user":{"id":42,"name":"Pavel"}}"#.to_string(),
            is_json: true,
            content_type: "application/json".to_string(),
            body_kind: "json".to_string(),
            truncated: false,
        }
    }

    #[test]
    fn evaluates_status_headers_json_body_and_time() {
        let assertions = vec![
            ResponseAssertion::Status {
                expected: 200,
                enabled: true,
            },
            ResponseAssertion::Header {
                name: "content-type".to_string(),
                operator: "contains".to_string(),
                expected: "json".to_string(),
                enabled: true,
            },
            ResponseAssertion::JsonPath {
                path: "$.user.id".to_string(),
                operator: "equals".to_string(),
                expected: "42".to_string(),
                enabled: true,
            },
            ResponseAssertion::BodyContains {
                expected: "Pavel".to_string(),
                enabled: true,
            },
            ResponseAssertion::ResponseTime {
                max_ms: 100,
                enabled: true,
            },
        ];
        assert!(
            evaluate(&response(), &assertions)
                .iter()
                .all(|item| item.passed)
        );
    }

    #[test]
    fn reports_failed_assertion_without_response_body_dump() {
        let results = evaluate(
            &response(),
            &[ResponseAssertion::Status {
                expected: 204,
                enabled: true,
            }],
        );
        assert!(!results[0].passed);
        assert_eq!(results[0].actual, "200");
    }

    #[tokio::test]
    async fn runs_saved_collection_and_reports_success() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = socket.read(&mut buffer).await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .await
                .unwrap();
        });
        let root = std::env::temp_dir().join(format!("reqvault-runner-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        workspace::create(&root, Some("Runner".to_string())).unwrap();
        let request = RequestFile {
            name: "Health".to_string(),
            url: format!("http://{address}/health"),
            tests: vec![ResponseAssertion::Status {
                expected: 200,
                enabled: true,
            }],
            ..RequestFile::default()
        };
        workspace::save_request(&root, None, Some("system"), &request).unwrap();

        let report = run_workspace(&root, &CollectionRunOptions::default())
            .await
            .unwrap();
        server.await.unwrap();
        assert_eq!(report.total, 1);
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 0);
        fs::remove_dir_all(root).unwrap();
    }
}
