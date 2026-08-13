use std::{
    fs,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::{
    fs_utils::atomic_write,
    models::{HttpResponse, RequestFile},
    redaction::redact_header,
    variables::redact_secret_references,
};

pub fn export(
    destination: &Path,
    format: &str,
    request: &RequestFile,
    response: &HttpResponse,
) -> Result<(), String> {
    let bytes = match format {
        "body" => decoded_body(response)?,
        "http" => full_http_response(response)?,
        "har" => safe_har(request, response)?,
        _ => return Err("Неизвестный формат экспорта ответа".to_string()),
    };
    atomic_write(destination, &bytes).map_err(|_| "Не удалось сохранить экспорт ответа".to_string())
}

pub fn save_fixture(
    workspace: &Path,
    name: &str,
    response: &HttpResponse,
) -> Result<String, String> {
    if response.truncated {
        return Err("Нельзя сохранить fixture из обрезанного ответа".to_string());
    }
    let file_name = safe_file_name(name)?;
    let directory = workspace.join("fixtures");
    fs::create_dir_all(&directory).map_err(|_| "Не удалось создать папку fixtures".to_string())?;
    let path = directory.join(file_name);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(&path).map_err(|error| match error.kind() {
        std::io::ErrorKind::AlreadyExists => {
            "Fixture с таким именем уже существует. Выберите другое имя".to_string()
        }
        _ => "Не удалось создать fixture".to_string(),
    })?;
    std::io::Write::write_all(&mut file, &decoded_body(response)?)
        .map_err(|_| "Не удалось записать fixture".to_string())?;
    path.strip_prefix(workspace)
        .unwrap_or(&path)
        .to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| "Не удалось сформировать путь fixture".to_string())
}

fn full_http_response(response: &HttpResponse) -> Result<Vec<u8>, String> {
    let mut result =
        format!("HTTP/1.1 {} {}\r\n", response.status, response.status_text).into_bytes();
    for header in &response.headers {
        result.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    if response.truncated {
        result.extend_from_slice(b"X-ReqVault-Truncated: true\r\n");
    }
    result.extend_from_slice(b"\r\n");
    result.extend_from_slice(&decoded_body(response)?);
    Ok(result)
}

fn safe_har(request: &RequestFile, response: &HttpResponse) -> Result<Vec<u8>, String> {
    let request_headers = request
        .headers
        .iter()
        .map(|(name, value)| {
            serde_json::json!({
                "name": name,
                "value": redact_secret_references(&redact_header(name, value, &[])),
            })
        })
        .collect::<Vec<_>>();
    let response_headers = response
        .headers
        .iter()
        .map(|header| serde_json::json!({ "name": header.name, "value": header.value }))
        .collect::<Vec<_>>();
    let binary = matches!(response.body_kind.as_str(), "image" | "binary");
    let mut content = serde_json::json!({
        "size": response.size_bytes,
        "mimeType": response.content_type,
        "text": response.body,
    });
    if binary {
        content["encoding"] = serde_json::Value::String("base64".to_string());
    }
    if response.truncated {
        content["comment"] = serde_json::Value::String(
            "Body preview was truncated by ReqVault at 8 MiB".to_string(),
        );
    }
    let safe_url = safe_request_url(&request.url);
    let document = serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": { "name": "ReqVault", "version": env!("CARGO_PKG_VERSION") },
            "entries": [{
                "startedDateTime": "1970-01-01T00:00:00.000Z",
                "time": response.duration_ms,
                "request": {
                    "method": request.method,
                    "url": safe_url,
                    "httpVersion": "HTTP/1.1",
                    "headers": request_headers,
                    "queryString": [],
                    "cookies": [],
                    "postData": { "mimeType": "application/octet-stream", "text": "[omitted by ReqVault safe export]" },
                    "headersSize": -1,
                    "bodySize": -1
                },
                "response": {
                    "status": response.status,
                    "statusText": response.status_text,
                    "httpVersion": "HTTP/1.1",
                    "headers": response_headers,
                    "cookies": [],
                    "content": content,
                    "redirectURL": "",
                    "headersSize": -1,
                    "bodySize": response.size_bytes
                },
                "cache": {},
                "timings": { "send": 0, "wait": response.duration_ms, "receive": 0 }
            }]
        }
    });
    serde_json::to_vec_pretty(&document).map_err(|_| "Не удалось подготовить HAR".to_string())
}

fn safe_request_url(raw: &str) -> String {
    let redacted = redact_secret_references(raw);
    let Ok(mut url) = url::Url::parse(&redacted) else {
        return redacted;
    };
    let pairs = url
        .query_pairs()
        .map(|(name, value)| {
            let sensitive = ["token", "key", "secret", "password", "auth", "session"]
                .iter()
                .any(|marker| name.to_ascii_lowercase().contains(marker));
            (
                name.into_owned(),
                if sensitive {
                    "***REDACTED***".to_string()
                } else {
                    value.into_owned()
                },
            )
        })
        .collect::<Vec<_>>();
    if !pairs.is_empty() {
        url.set_query(None);
        let mut query = url.query_pairs_mut();
        for (name, value) in pairs {
            query.append_pair(&name, &value);
        }
    }
    url.into()
}

fn decoded_body(response: &HttpResponse) -> Result<Vec<u8>, String> {
    if matches!(response.body_kind.as_str(), "image" | "binary") {
        STANDARD
            .decode(&response.body)
            .map_err(|_| "Бинарное тело ответа повреждено".to_string())
    } else {
        Ok(response.body.as_bytes().to_vec())
    }
}

fn safe_file_name(name: &str) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Укажите имя fixture".to_string());
    }
    let requested = PathBuf::from(name.trim());
    if requested.components().count() != 1 {
        return Err("Fixture нужно сохранить одним именем без пути".to_string());
    }
    let sanitized = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if sanitized == "." || sanitized == ".." || sanitized.is_empty() {
        return Err("Некорректное имя fixture".to_string());
    }
    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ResponseHeader;
    use uuid::Uuid;

    fn response() -> HttpResponse {
        HttpResponse {
            request_id: "test".to_string(),
            status: 200,
            status_text: "OK".to_string(),
            duration_ms: 15,
            size_bytes: 11,
            headers: vec![ResponseHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            }],
            body: "{\"ok\":true}".to_string(),
            is_json: true,
            content_type: "application/json".to_string(),
            body_kind: "json".to_string(),
            truncated: false,
        }
    }

    #[test]
    fn har_does_not_export_credentials() {
        let mut request = RequestFile {
            url: "https://api.example.test/users?access_token=plain-credential".to_string(),
            ..RequestFile::default()
        };
        request.headers.insert(
            "Authorization".to_string(),
            "Bearer very-secret-value".to_string(),
        );
        let har = String::from_utf8(safe_har(&request, &response()).unwrap()).unwrap();
        assert!(!har.contains("{{secret:"));
        assert!(!har.contains("very-secret-value"));
        assert!(!har.contains("plain-credential"));
        assert!(har.contains("********"));
    }

    #[test]
    fn fixture_stays_inside_workspace_and_does_not_overwrite() {
        let root = std::env::temp_dir().join(format!("reqvault-fixture-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        assert!(save_fixture(&root, "../outside.json", &response()).is_err());
        assert_eq!(
            save_fixture(&root, "users.json", &response()).unwrap(),
            "fixtures/users.json"
        );
        assert!(save_fixture(&root, "users.json", &response()).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
