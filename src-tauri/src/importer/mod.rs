use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value};

use crate::{
    models::{
        AuthConfig, BodyConfig, EnvironmentFile, ImportResult, KeyValue, MultipartField,
        ProxyConfig, RequestFile,
    },
    redaction::is_sensitive_header,
    workspace,
};

const MAX_IMPORT_SIZE: u64 = 10 * 1024 * 1024;

struct ParsedImport {
    source: &'static str,
    requests: Vec<(String, RequestFile)>,
    environment: Option<EnvironmentFile>,
    warnings: Vec<String>,
}

pub fn import_file(workspace_path: &Path, file_path: &Path) -> Result<ImportResult, String> {
    let metadata = fs::metadata(file_path).map_err(|_| "Файл импорта не найден".to_string())?;
    if metadata.len() > MAX_IMPORT_SIZE {
        return Err("Файл импорта больше 10 МБ".to_string());
    }
    let content = fs::read_to_string(file_path)
        .map_err(|_| "Не удалось прочитать файл импорта как UTF-8".to_string())?;
    let value: Value = serde_yaml::from_str(&content)
        .map_err(|error| format!("Не удалось разобрать JSON/YAML: {error}"))?;
    let parsed = if is_postman(&value) {
        parse_postman(&value)?
    } else if value.get("openapi").is_some() || value.get("swagger").is_some() {
        let (resolved, ref_warnings) = resolve_openapi_refs(&value, file_path);
        let mut parsed = parse_openapi(&resolved)?;
        parsed.warnings.extend(ref_warnings);
        parsed
    } else {
        return Err("Поддерживаются Postman Collection 2.x, OpenAPI 3.x и Swagger 2.0".to_string());
    };

    let mut imported_requests = 0;
    for (collection, request) in parsed.requests {
        workspace::save_request(workspace_path, None, Some(&collection), &request)
            .map_err(|error| error.to_string())?;
        imported_requests += 1;
    }
    let imported_environments = if let Some(environment) = parsed.environment {
        workspace::save_environment(workspace_path, None, &environment)
            .map_err(|error| error.to_string())?;
        1
    } else {
        0
    };
    let snapshot = workspace::open(workspace_path).map_err(|error| error.to_string())?;
    Ok(ImportResult {
        source: parsed.source.to_string(),
        imported_requests,
        imported_environments,
        warnings: parsed.warnings,
        workspace: snapshot,
    })
}

fn resolve_openapi_refs(root: &Value, file_path: &Path) -> (Value, Vec<String>) {
    let mut warnings = Vec::new();
    let import_root = file_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."));
    let mut stack = HashSet::new();
    let resolved = resolve_openapi_node(
        root,
        root,
        file_path,
        &import_root,
        &mut stack,
        &mut warnings,
        0,
    );
    (resolved, warnings)
}

fn resolve_openapi_node(
    value: &Value,
    document_root: &Value,
    document_path: &Path,
    import_root: &Path,
    stack: &mut HashSet<String>,
    warnings: &mut Vec<String>,
    depth: usize,
) -> Value {
    if depth > 32 {
        warnings.push("Достигнут предел вложенности OpenAPI $ref".to_string());
        return value.clone();
    }
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| {
                    resolve_openapi_node(
                        item,
                        document_root,
                        document_path,
                        import_root,
                        stack,
                        warnings,
                        depth + 1,
                    )
                })
                .collect(),
        ),
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                let key = format!("{}#{reference}", document_path.display());
                if !stack.insert(key.clone()) {
                    warnings.push(format!(
                        "Циклический OpenAPI $ref оставлен без раскрытия: {reference}"
                    ));
                    return value.clone();
                }
                let resolved = resolve_openapi_reference(
                    reference,
                    document_root,
                    document_path,
                    import_root,
                    stack,
                    warnings,
                    depth + 1,
                );
                stack.remove(&key);
                if let Some(mut target) = resolved {
                    if let Value::Object(target_object) = &mut target {
                        for (name, sibling) in object.iter().filter(|(name, _)| *name != "$ref") {
                            target_object.insert(
                                name.clone(),
                                resolve_openapi_node(
                                    sibling,
                                    document_root,
                                    document_path,
                                    import_root,
                                    stack,
                                    warnings,
                                    depth + 1,
                                ),
                            );
                        }
                    }
                    target
                } else {
                    value.clone()
                }
            } else {
                Value::Object(
                    object
                        .iter()
                        .map(|(name, child)| {
                            (
                                name.clone(),
                                resolve_openapi_node(
                                    child,
                                    document_root,
                                    document_path,
                                    import_root,
                                    stack,
                                    warnings,
                                    depth + 1,
                                ),
                            )
                        })
                        .collect(),
                )
            }
        }
        _ => value.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_openapi_reference(
    reference: &str,
    document_root: &Value,
    document_path: &Path,
    import_root: &Path,
    stack: &mut HashSet<String>,
    warnings: &mut Vec<String>,
    depth: usize,
) -> Option<Value> {
    if let Some(fragment) = reference.strip_prefix('#') {
        let target = document_root.pointer(fragment);
        if target.is_none() {
            warnings.push(format!("OpenAPI $ref не найден: {reference}"));
        }
        return target.map(|value| {
            resolve_openapi_node(
                value,
                document_root,
                document_path,
                import_root,
                stack,
                warnings,
                depth,
            )
        });
    }
    if reference.starts_with("http://") || reference.starts_with("https://") {
        warnings.push(format!(
            "Удалённый OpenAPI $ref не загружен автоматически: {reference}"
        ));
        return None;
    }

    let (relative, fragment) = reference.split_once('#').unwrap_or((reference, ""));
    let candidate = document_path.parent().unwrap_or(import_root).join(relative);
    let canonical = match candidate.canonicalize() {
        Ok(path) if path.starts_with(import_root) => path,
        _ => {
            warnings.push(format!(
                "Внешний OpenAPI $ref выходит за каталог импорта или не найден: {reference}"
            ));
            return None;
        }
    };
    match fs::metadata(&canonical) {
        Ok(value) if value.len() <= MAX_IMPORT_SIZE => {}
        _ => {
            warnings.push(format!("Внешний OpenAPI $ref слишком большой: {reference}"));
            return None;
        }
    }
    let content = match fs::read_to_string(&canonical) {
        Ok(value) => value,
        Err(_) => {
            warnings.push(format!("Не удалось прочитать OpenAPI $ref: {reference}"));
            return None;
        }
    };
    let external: Value = match serde_yaml::from_str(&content) {
        Ok(value) => value,
        Err(_) => {
            warnings.push(format!("Не удалось разобрать OpenAPI $ref: {reference}"));
            return None;
        }
    };
    let target = if fragment.is_empty() {
        Some(&external)
    } else {
        external.pointer(fragment)
    };
    if target.is_none() {
        warnings.push(format!("Фрагмент OpenAPI $ref не найден: {reference}"));
    }
    target.map(|value| {
        resolve_openapi_node(
            value,
            &external,
            &canonical,
            import_root,
            stack,
            warnings,
            depth,
        )
    })
}

pub fn import_curl(workspace_path: &Path, command: &str) -> Result<ImportResult, String> {
    if command.len() > 1024 * 1024 {
        return Err("Команда cURL больше 1 МБ".to_string());
    }
    let (request, warnings) = parse_curl(command)?;
    workspace::save_request(workspace_path, None, Some("cURL import"), &request)
        .map_err(|error| error.to_string())?;
    let snapshot = workspace::open(workspace_path).map_err(|error| error.to_string())?;
    Ok(ImportResult {
        source: "cURL".to_string(),
        imported_requests: 1,
        imported_environments: 0,
        warnings,
        workspace: snapshot,
    })
}

fn parse_curl(command: &str) -> Result<(RequestFile, Vec<String>), String> {
    let tokens = split_command(command)?;
    let Some(first) = tokens.first() else {
        return Err("Вставь команду cURL".to_string());
    };
    let executable = first.replace('\\', "/").to_lowercase();
    if !executable.ends_with("curl") && !executable.ends_with("curl.exe") {
        return Err("Команда должна начинаться с curl".to_string());
    }

    let mut request = RequestFile {
        name: "Импортированный cURL".to_string(),
        ..RequestFile::default()
    };
    let mut warnings = Vec::new();
    let mut data: Option<String> = None;
    let mut form_fields = Vec::new();
    let mut content_type = String::new();
    let mut index = 1;
    while index < tokens.len() {
        let token = &tokens[index];
        let mut take_value = |label: &str| -> Result<String, String> {
            index += 1;
            tokens
                .get(index)
                .cloned()
                .ok_or_else(|| format!("После {label} ожидается значение"))
        };
        match token.as_str() {
            "-X" | "--request" => request.method = take_value(token)?.to_uppercase(),
            "--url" => request.url = take_value(token)?,
            "-H" | "--header" => {
                let raw = take_value(token)?;
                import_curl_header(&raw, &mut request, &mut content_type, &mut warnings)?;
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-urlencode" => {
                data = Some(take_value(token)?);
                if request.method == "GET" {
                    request.method = "POST".to_string();
                }
            }
            "-F" | "--form" | "--form-string" => {
                let raw = take_value(token)?;
                form_fields.push(curl_form_field(&raw)?);
                if request.method == "GET" {
                    request.method = "POST".to_string();
                }
            }
            "-u" | "--user" => {
                let raw = take_value(token)?;
                let (username, had_password) = raw
                    .split_once(':')
                    .map(|(user, password)| (user.to_string(), !password.is_empty()))
                    .unwrap_or((raw, false));
                request.auth = AuthConfig::Basic {
                    username,
                    password: "{{secret:CURL_PASSWORD}}".to_string(),
                };
                if had_password {
                    warnings.push(
                        "Пароль Basic Auth удалён. Добавь CURL_PASSWORD в Secret Vault."
                            .to_string(),
                    );
                }
            }
            "--oauth2-bearer" => {
                let value = take_value(token)?;
                request.auth = AuthConfig::Bearer {
                    token: if value.starts_with("{{secret:") {
                        value
                    } else {
                        warnings.push(
                            "OAuth bearer token удалён. Добавь CURL_BEARER_TOKEN в Secret Vault."
                                .to_string(),
                        );
                        "{{secret:CURL_BEARER_TOKEN}}".to_string()
                    },
                };
            }
            "-A" | "--user-agent" => {
                request
                    .headers
                    .insert("User-Agent".to_string(), take_value(token)?);
            }
            "-e" | "--referer" => {
                request
                    .headers
                    .insert("Referer".to_string(), take_value(token)?);
            }
            "-b" | "--cookie" => {
                let raw = take_value(token)?;
                request.headers.insert(
                    "Cookie".to_string(),
                    if raw.starts_with("{{secret:") {
                        raw
                    } else {
                        warnings
                            .push("Cookie удалён. Добавь CURL_COOKIE в Secret Vault.".to_string());
                        "{{secret:CURL_COOKIE}}".to_string()
                    },
                );
            }
            "--proxy" | "-x" => {
                let url = take_value(token)?;
                request.transport.proxy = ProxyConfig::Custom {
                    url,
                    username: String::new(),
                    password: String::new(),
                };
            }
            "--proxy-user" | "-U" => {
                let raw = take_value(token)?;
                let username = raw.split_once(':').map(|item| item.0).unwrap_or(&raw);
                let url = match &request.transport.proxy {
                    ProxyConfig::Custom { url, .. } => url.clone(),
                    _ => String::new(),
                };
                request.transport.proxy = ProxyConfig::Custom {
                    url,
                    username: username.to_string(),
                    password: "{{secret:CURL_PROXY_PASSWORD}}".to_string(),
                };
                warnings.push(
                    "Пароль proxy удалён. Добавь CURL_PROXY_PASSWORD в Secret Vault.".to_string(),
                );
            }
            "--cacert" => request.transport.custom_ca_path = take_value(token)?,
            "--cert" => request.transport.client_certificate_path = take_value(token)?,
            "--key" => request.transport.client_key_path = take_value(token)?,
            "--max-time" => {
                let seconds = take_value(token)?
                    .parse::<f64>()
                    .map_err(|_| "Значение --max-time должно быть числом секунд".to_string())?;
                request.timeout_ms = (seconds * 1000.0).round().clamp(1.0, 600_000.0) as u64;
            }
            "-L" | "--location" => request.follow_redirects = true,
            "--no-location" => request.follow_redirects = false,
            "--compressed" | "--silent" | "-s" | "--show-error" | "-S" => {}
            value if value.starts_with("-X") && value.len() > 2 => {
                request.method = value[2..].to_uppercase();
            }
            value if value.starts_with('-') => {
                warnings.push(format!("Параметр {value} не перенесён"));
            }
            value => request.url = value.to_string(),
        }
        index += 1;
    }

    if request.url.trim().is_empty() {
        return Err("Команда cURL не содержит URL".to_string());
    }
    if let Ok(url) = url::Url::parse(&request.url)
        && let Some(host) = url.host_str()
    {
        request.name = format!("{} {}", request.method, host);
    }
    request.body = if !form_fields.is_empty() {
        BodyConfig::Multipart {
            fields: form_fields,
        }
    } else if let Some(value) = data {
        if content_type.to_lowercase().contains("application/json")
            || serde_json::from_str::<Value>(&value).is_ok()
        {
            BodyConfig::Json { value }
        } else {
            BodyConfig::Raw {
                value,
                content_type,
            }
        }
    } else {
        BodyConfig::None
    };

    Ok((request, warnings))
}

fn import_curl_header(
    raw: &str,
    request: &mut RequestFile,
    content_type: &mut String,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let (name, value) = raw
        .split_once(':')
        .ok_or_else(|| format!("Некорректный заголовок cURL: {raw}"))?;
    let name = name.trim();
    let value = value.trim();
    if name.eq_ignore_ascii_case("content-type") {
        *content_type = value.to_string();
    }
    if name.eq_ignore_ascii_case("authorization") {
        if value.to_lowercase().starts_with("bearer ") {
            request.auth = AuthConfig::Bearer {
                token: "{{secret:CURL_BEARER_TOKEN}}".to_string(),
            };
        } else if value.to_lowercase().starts_with("basic ") {
            request.auth = AuthConfig::Basic {
                username: String::new(),
                password: "{{secret:CURL_PASSWORD}}".to_string(),
            };
        } else {
            request.headers.insert(
                name.to_string(),
                "{{secret:CURL_AUTHORIZATION}}".to_string(),
            );
        }
        warnings
            .push("Значение Authorization удалено. Добавь credential в Secret Vault.".to_string());
    } else if is_sensitive_header(name) {
        let secret_name = name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>();
        request.headers.insert(
            name.to_string(),
            format!("{{{{secret:CURL_{secret_name}}}}}"),
        );
        warnings.push(format!(
            "Значение заголовка {name} удалено. Добавь его в Secret Vault."
        ));
    } else {
        request.headers.insert(name.to_string(), value.to_string());
    }
    Ok(())
}

fn curl_form_field(raw: &str) -> Result<MultipartField, String> {
    let (name, raw_value) = raw
        .split_once('=')
        .ok_or_else(|| format!("Некорректное multipart-поле: {raw}"))?;
    if name.trim().is_empty() {
        return Err("Имя multipart-поля не может быть пустым".to_string());
    }
    if let Some(file) = raw_value.strip_prefix('@') {
        let mut parts = file.split(';');
        let path = parts.next().unwrap_or_default().to_string();
        let content_type = parts
            .find_map(|part| part.strip_prefix("type="))
            .unwrap_or_default()
            .to_string();
        Ok(MultipartField::File {
            name: name.to_string(),
            path,
            content_type,
            enabled: true,
        })
    } else {
        Ok(MultipartField::Text {
            name: name.to_string(),
            value: raw_value.to_string(),
            enabled: true,
        })
    }
}

fn split_command(command: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let normalized = command
        .replace("\\\r\n", " ")
        .replace("\\\n", " ")
        .replace("`\r\n", " ")
        .replace("`\n", " ")
        .replace("^\r\n", " ")
        .replace("^\n", " ");
    let mut characters = normalized.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\\' && quote != Some('\'') {
            let next = characters.peek().copied();
            let escapes_next = match quote {
                Some('"') => matches!(next, Some('"' | '\\')),
                None => next.is_some_and(|value| {
                    value.is_whitespace() || matches!(value, '\'' | '"' | '\\')
                }),
                _ => false,
            };
            if escapes_next {
                if let Some(value) = characters.next() {
                    current.push(value);
                }
            } else {
                current.push(character);
            }
            continue;
        }
        if let Some(active) = quote {
            if character == active {
                quote = None;
            } else {
                current.push(character);
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
        } else if character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if quote.is_some() {
        return Err("В команде cURL не закрыта кавычка".to_string());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn is_postman(value: &Value) -> bool {
    value.get("item").and_then(Value::as_array).is_some()
        || value
            .pointer("/info/schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema.contains("getpostman.com"))
}

fn parse_postman(root: &Value) -> Result<ParsedImport, String> {
    let mut requests = Vec::new();
    let mut warnings = Vec::new();
    let collection = root
        .pointer("/info/name")
        .and_then(Value::as_str)
        .unwrap_or("Postman import");
    let inherited_auth = root.get("auth");
    walk_postman_items(
        root.get("item")
            .and_then(Value::as_array)
            .ok_or_else(|| "Postman collection не содержит список item".to_string())?,
        collection,
        inherited_auth,
        &mut requests,
        &mut warnings,
    );

    let variables = root
        .get("variable")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let name = item.get("key").and_then(Value::as_str)?.trim();
                    if name.is_empty() {
                        return None;
                    }
                    if item.get("type").and_then(Value::as_str) == Some("secret") {
                        warnings.push(format!(
                            "Значение секретной переменной {name} не импортировано. Добавь его в Secret Vault."
                        ));
                        return None;
                    }
                    Some((
                        name.to_string(),
                        scalar_string(item.get("value").unwrap_or(&Value::Null)),
                    ))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let environment = (!variables.is_empty()).then_some(EnvironmentFile {
        name: "postman".to_string(),
        variables,
        ..EnvironmentFile::default()
    });

    Ok(ParsedImport {
        source: "Postman",
        requests,
        environment,
        warnings,
    })
}

fn walk_postman_items(
    items: &[Value],
    collection: &str,
    inherited_auth: Option<&Value>,
    output: &mut Vec<(String, RequestFile)>,
    warnings: &mut Vec<String>,
) {
    for item in items {
        let name = item.get("name").and_then(Value::as_str).unwrap_or("Запрос");
        if let Some(children) = item.get("item").and_then(Value::as_array) {
            let nested = format!("{collection} - {name}");
            walk_postman_items(
                children,
                &nested,
                item.get("auth").or(inherited_auth),
                output,
                warnings,
            );
            continue;
        }
        let Some(request) = item.get("request") else {
            continue;
        };
        output.push((
            collection.to_string(),
            postman_request(
                name,
                request,
                request.get("auth").or(inherited_auth),
                warnings,
            ),
        ));
    }
}

fn postman_request(
    name: &str,
    value: &Value,
    auth: Option<&Value>,
    warnings: &mut Vec<String>,
) -> RequestFile {
    let url_value = value.get("url").unwrap_or(&Value::Null);
    let raw_url = url_value
        .as_str()
        .or_else(|| url_value.get("raw").and_then(Value::as_str))
        .unwrap_or("");
    let query = url_value
        .get("query")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(KeyValue {
                        name: item.get("key")?.as_str()?.to_string(),
                        value: scalar_string(item.get("value").unwrap_or(&Value::Null)),
                        enabled: !item
                            .get("disabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let url = if query.is_empty() {
        raw_url.to_string()
    } else {
        raw_url.split('?').next().unwrap_or(raw_url).to_string()
    };
    let mut headers = BTreeMap::new();
    for header in value
        .get("header")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if header.get("disabled").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let Some(key) = header.get("key").and_then(Value::as_str) else {
            continue;
        };
        let raw = scalar_string(header.get("value").unwrap_or(&Value::Null));
        let saved = if is_sensitive_header(key) {
            warnings.push(format!(
                "Значение заголовка {key} в запросе {name} не импортировано как открытый текст."
            ));
            "{{secret:IMPORTED_CREDENTIAL}}".to_string()
        } else {
            raw
        };
        headers.insert(key.to_string(), saved);
    }

    RequestFile {
        name: name.to_string(),
        method: value
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("GET")
            .to_uppercase(),
        url,
        headers,
        query,
        auth: postman_auth(auth, name, warnings),
        body: postman_body(value.get("body")),
        ..RequestFile::default()
    }
}

fn postman_auth(
    auth: Option<&Value>,
    request_name: &str,
    warnings: &mut Vec<String>,
) -> AuthConfig {
    let Some(auth) = auth else {
        return AuthConfig::None;
    };
    let kind = auth.get("type").and_then(Value::as_str).unwrap_or("noauth");
    let values = auth
        .get(kind)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some((
                        item.get("key")?.as_str()?.to_string(),
                        scalar_string(item.get("value").unwrap_or(&Value::Null)),
                    ))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let protected = |label: &str, fallback: &str, warnings: &mut Vec<String>| {
        let value = values.get(label).cloned().unwrap_or_default();
        if value.starts_with("{{secret:") {
            value
        } else {
            if !value.is_empty() {
                warnings.push(format!(
                    "Credential из запроса {request_name} отброшен. Сохрани его в Secret Vault."
                ));
            }
            format!("{{{{secret:{fallback}}}}}")
        }
    };
    match kind {
        "bearer" => AuthConfig::Bearer {
            token: protected("token", "IMPORTED_TOKEN", warnings),
        },
        "basic" => AuthConfig::Basic {
            username: values.get("username").cloned().unwrap_or_default(),
            password: protected("password", "IMPORTED_PASSWORD", warnings),
        },
        "apikey" => {
            let key = values
                .get("key")
                .cloned()
                .unwrap_or_else(|| "X-API-Key".to_string());
            let value = protected("value", "IMPORTED_API_KEY", warnings);
            if values.get("in").is_some_and(|value| value == "query") {
                AuthConfig::ApiKeyQuery { name: key, value }
            } else {
                AuthConfig::ApiKeyHeader { name: key, value }
            }
        }
        "oauth2" => AuthConfig::OAuth2 {
            grant_type: "authorization_code_pkce".to_string(),
            authorization_url: values.get("authUrl").cloned().unwrap_or_default(),
            token_url: values.get("accessTokenUrl").cloned().unwrap_or_default(),
            client_id: values.get("clientId").cloned().unwrap_or_default(),
            client_secret: "{{secret:OAUTH_CLIENT_SECRET}}".to_string(),
            scopes: values.get("scope").cloned().unwrap_or_default(),
            access_token: "{{secret:OAUTH_ACCESS_TOKEN}}".to_string(),
            refresh_token: "{{secret:OAUTH_REFRESH_TOKEN}}".to_string(),
        },
        _ => AuthConfig::None,
    }
}

fn postman_body(body: Option<&Value>) -> BodyConfig {
    let Some(body) = body else {
        return BodyConfig::None;
    };
    match body.get("mode").and_then(Value::as_str).unwrap_or("") {
        "raw" => {
            let value = body
                .get("raw")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if body
                .pointer("/options/raw/language")
                .and_then(Value::as_str)
                == Some("json")
            {
                BodyConfig::Json { value }
            } else {
                BodyConfig::Raw {
                    value,
                    content_type: "text/plain".to_string(),
                }
            }
        }
        "urlencoded" => BodyConfig::FormUrlencoded {
            fields: postman_key_values(body.get("urlencoded")),
        },
        "formdata" => BodyConfig::Multipart {
            fields: body
                .get("formdata")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| {
                    let name = item.get("key")?.as_str()?.to_string();
                    let enabled = item.get("disabled").and_then(Value::as_bool) != Some(true);
                    if item.get("type").and_then(Value::as_str) == Some("file") {
                        Some(MultipartField::File {
                            name,
                            path: item
                                .get("src")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            content_type: item
                                .get("contentType")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            enabled,
                        })
                    } else {
                        Some(MultipartField::Text {
                            name,
                            value: scalar_string(item.get("value").unwrap_or(&Value::Null)),
                            enabled,
                        })
                    }
                })
                .collect(),
        },
        _ => BodyConfig::None,
    }
}

fn postman_key_values(value: Option<&Value>) -> Vec<KeyValue> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(KeyValue {
                name: item.get("key")?.as_str()?.to_string(),
                value: scalar_string(item.get("value").unwrap_or(&Value::Null)),
                enabled: item.get("disabled").and_then(Value::as_bool) != Some(true),
            })
        })
        .collect()
}

fn parse_openapi(root: &Value) -> Result<ParsedImport, String> {
    let title = root
        .pointer("/info/title")
        .and_then(Value::as_str)
        .unwrap_or("OpenAPI import");
    let mut variables = BTreeMap::new();
    let base_url = openapi_base_url(root, &mut variables);
    let schemes = root
        .pointer("/components/securitySchemes")
        .or_else(|| root.pointer("/securityDefinitions"));
    let mut requests = Vec::new();
    let paths = root
        .get("paths")
        .and_then(Value::as_object)
        .ok_or_else(|| "OpenAPI не содержит paths".to_string())?;
    for (path, path_item) in paths {
        let Some(operations) = path_item.as_object() else {
            continue;
        };
        for (method, operation) in operations {
            if !["get", "post", "put", "patch", "delete", "head", "options"]
                .contains(&method.as_str())
            {
                continue;
            }
            let name = operation
                .get("summary")
                .or_else(|| operation.get("operationId"))
                .and_then(Value::as_str)
                .unwrap_or(path);
            let mut request = RequestFile {
                name: name.to_string(),
                method: method.to_uppercase(),
                url: format!("{}{}", base_url.trim_end_matches('/'), openapi_path(path)),
                auth: openapi_auth(root, operation, schemes),
                ..RequestFile::default()
            };
            let parameters = path_item
                .get("parameters")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .chain(
                    operation
                        .get("parameters")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten(),
                );
            for parameter in parameters {
                let Some(parameter_name) = parameter.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let value = parameter
                    .get("example")
                    .or_else(|| parameter.pointer("/schema/example"))
                    .or_else(|| parameter.pointer("/schema/default"))
                    .map(scalar_string)
                    .unwrap_or_default();
                match parameter.get("in").and_then(Value::as_str) {
                    Some("query") => request.query.push(KeyValue {
                        name: parameter_name.to_string(),
                        value,
                        enabled: true,
                    }),
                    Some("header") => {
                        request.headers.insert(parameter_name.to_string(), value);
                    }
                    _ => {}
                }
            }
            request.body = openapi_body(operation);
            requests.push((title.to_string(), request));
        }
    }
    let environment = (!variables.is_empty()).then_some(EnvironmentFile {
        name: "openapi".to_string(),
        variables,
        ..EnvironmentFile::default()
    });
    Ok(ParsedImport {
        source: "OpenAPI",
        requests,
        environment,
        warnings: Vec::new(),
    })
}

fn openapi_base_url(root: &Value, variables: &mut BTreeMap<String, String>) -> String {
    if let Some(server) = root.pointer("/servers/0") {
        let mut url = server
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some(items) = server.get("variables").and_then(Value::as_object) {
            for (name, config) in items {
                let key = name.to_uppercase();
                variables.insert(
                    key.clone(),
                    scalar_string(config.get("default").unwrap_or(&Value::Null)),
                );
                url = url.replace(&format!("{{{name}}}"), &format!("{{{{{key}}}}}"));
            }
        }
        return url;
    }
    let scheme = root
        .pointer("/schemes/0")
        .and_then(Value::as_str)
        .unwrap_or("https");
    let host = root
        .get("host")
        .and_then(Value::as_str)
        .unwrap_or("api.example.test");
    let base = root.get("basePath").and_then(Value::as_str).unwrap_or("");
    format!("{scheme}://{host}{base}")
}

fn openapi_path(path: &str) -> String {
    let mut result = String::new();
    let mut chars = path.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '{' {
            let mut name = String::new();
            while chars.peek().is_some_and(|item| *item != '}') {
                name.push(chars.next().unwrap_or_default());
            }
            let _ = chars.next();
            result.push_str(&format!("{{{{{}}}}}", name.to_uppercase()));
        } else {
            result.push(character);
        }
    }
    result
}

fn openapi_auth(root: &Value, operation: &Value, schemes: Option<&Value>) -> AuthConfig {
    let security = operation.get("security").or_else(|| root.get("security"));
    let scheme_name = security
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_object)
        .and_then(|item| item.keys().next());
    let Some(config) = scheme_name.and_then(|name| schemes?.get(name)) else {
        return AuthConfig::None;
    };
    match config.get("type").and_then(Value::as_str).unwrap_or("") {
        "apiKey" => {
            let name = config
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("X-API-Key")
                .to_string();
            let value = "{{secret:OPENAPI_API_KEY}}".to_string();
            if config.get("in").and_then(Value::as_str) == Some("query") {
                AuthConfig::ApiKeyQuery { name, value }
            } else {
                AuthConfig::ApiKeyHeader { name, value }
            }
        }
        "oauth2" => {
            let flow = config
                .pointer("/flows/authorizationCode")
                .or_else(|| config.pointer("/flows/clientCredentials"))
                .unwrap_or(config);
            let client_credentials = config.pointer("/flows/clientCredentials").is_some();
            let scopes = flow
                .get("scopes")
                .and_then(Value::as_object)
                .map(|items| items.keys().cloned().collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            AuthConfig::OAuth2 {
                grant_type: if client_credentials {
                    "client_credentials"
                } else {
                    "authorization_code_pkce"
                }
                .to_string(),
                authorization_url: flow
                    .get("authorizationUrl")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                token_url: flow
                    .get("tokenUrl")
                    .or_else(|| flow.get("tokenUrl"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                client_id: String::new(),
                client_secret: "{{secret:OAUTH_CLIENT_SECRET}}".to_string(),
                scopes,
                access_token: "{{secret:OAUTH_ACCESS_TOKEN}}".to_string(),
                refresh_token: "{{secret:OAUTH_REFRESH_TOKEN}}".to_string(),
            }
        }
        "basic" => AuthConfig::Basic {
            username: String::new(),
            password: "{{secret:OPENAPI_PASSWORD}}".to_string(),
        },
        "http" if config.get("scheme").and_then(Value::as_str) == Some("basic") => {
            AuthConfig::Basic {
                username: String::new(),
                password: "{{secret:OPENAPI_PASSWORD}}".to_string(),
            }
        }
        "http" if config.get("scheme").and_then(Value::as_str) == Some("bearer") => {
            AuthConfig::Bearer {
                token: "{{secret:OPENAPI_TOKEN}}".to_string(),
            }
        }
        _ => AuthConfig::None,
    }
}

fn openapi_body(operation: &Value) -> BodyConfig {
    if let Some(content) = operation
        .pointer("/requestBody/content")
        .and_then(Value::as_object)
    {
        if let Some(config) = content.get("application/json") {
            let value = config
                .get("example")
                .cloned()
                .or_else(|| config.get("schema").map(schema_example))
                .unwrap_or_else(|| Value::Object(Map::new()));
            return BodyConfig::Json {
                value: serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()),
            };
        }
        if let Some(config) = content.get("application/x-www-form-urlencoded") {
            return BodyConfig::FormUrlencoded {
                fields: schema_fields(config.get("schema")),
            };
        }
        if let Some(config) = content.get("multipart/form-data") {
            return BodyConfig::Multipart {
                fields: schema_multipart_fields(config.get("schema")),
            };
        }
    }
    if let Some(parameters) = operation.get("parameters").and_then(Value::as_array)
        && let Some(body) = parameters
            .iter()
            .find(|item| item.get("in").and_then(Value::as_str) == Some("body"))
    {
        let value = schema_example(body.get("schema").unwrap_or(&Value::Null));
        return BodyConfig::Json {
            value: serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()),
        };
    }
    BodyConfig::None
}

fn schema_fields(schema: Option<&Value>) -> Vec<KeyValue> {
    schema
        .and_then(|value| value.get("properties"))
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(name, property)| KeyValue {
            name: name.clone(),
            value: property
                .get("example")
                .or_else(|| property.get("default"))
                .map(scalar_string)
                .unwrap_or_default(),
            enabled: true,
        })
        .collect()
}

fn schema_multipart_fields(schema: Option<&Value>) -> Vec<MultipartField> {
    schema
        .and_then(|value| value.get("properties"))
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(name, property)| {
            if property.get("format").and_then(Value::as_str) == Some("binary") {
                MultipartField::File {
                    name: name.clone(),
                    path: String::new(),
                    content_type: String::new(),
                    enabled: true,
                }
            } else {
                MultipartField::Text {
                    name: name.clone(),
                    value: property
                        .get("example")
                        .or_else(|| property.get("default"))
                        .map(scalar_string)
                        .unwrap_or_default(),
                    enabled: true,
                }
            }
        })
        .collect()
}

fn schema_example(schema: &Value) -> Value {
    if let Some(example) = schema.get("example") {
        return example.clone();
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("object") | None => Value::Object(
            schema
                .get("properties")
                .and_then(Value::as_object)
                .map(|items| {
                    items
                        .iter()
                        .map(|(name, value)| (name.clone(), schema_example(value)))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        Some("array") => Value::Array(vec![schema_example(
            schema.get("items").unwrap_or(&Value::Null),
        )]),
        Some("integer" | "number") => Value::from(0),
        Some("boolean") => Value::Bool(false),
        _ => Value::String(String::new()),
    }
}

fn scalar_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "REQVAULT_TEST_SECRET_DO_NOT_LEAK_123456";

    #[test]
    fn imports_postman_without_plain_credentials() {
        let source = serde_json::json!({
            "info": { "name": "Users", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
            "item": [{
                "name": "Get user",
                "request": {
                    "method": "GET",
                    "url": { "raw": "https://api.example.test/users?page=2", "query": [{ "key": "page", "value": "2" }] },
                    "auth": { "type": "bearer", "bearer": [{ "key": "token", "value": TEST_SECRET }] }
                }
            }]
        });
        let parsed = parse_postman(&source).unwrap();
        assert_eq!(parsed.requests.len(), 1);
        let request = &parsed.requests[0].1;
        assert_eq!(request.url, "https://api.example.test/users");
        assert_eq!(request.query[0].value, "2");
        let yaml = serde_yaml::to_string(request).unwrap();
        assert!(yaml.contains("{{secret:IMPORTED_TOKEN}}"));
        assert!(!yaml.contains(TEST_SECRET));
    }

    #[test]
    fn imports_openapi_json_and_yaml() {
        for source in [
            r#"{"openapi":"3.0.3","info":{"title":"Pets"},"servers":[{"url":"https://api.example.test"}],"paths":{"/pets/{id}":{"get":{"summary":"Get pet","parameters":[{"in":"query","name":"expand","example":"owner"}]}}}}"#,
            r#"openapi: 3.0.3
info:
  title: Pets
servers:
  - url: https://api.example.test
paths:
  /pets/{id}:
    get:
      summary: Get pet
"#,
        ] {
            let value: Value = serde_yaml::from_str(source).unwrap();
            let parsed = parse_openapi(&value).unwrap();
            assert_eq!(parsed.requests.len(), 1);
            assert_eq!(
                parsed.requests[0].1.url,
                "https://api.example.test/pets/{{ID}}"
            );
        }
    }

    #[test]
    fn imports_curl_without_credentials() {
        let command = format!(
            "curl -X POST 'https://api.example.test/users' -H 'Authorization: Bearer {TEST_SECRET}' -H 'Content-Type: application/json' -d '{{\"name\":\"Pavel\"}}'"
        );
        let (request, warnings) = parse_curl(&command).unwrap();
        assert_eq!(request.method, "POST");
        assert!(matches!(request.body, BodyConfig::Json { .. }));
        assert!(matches!(request.auth, AuthConfig::Bearer { .. }));
        let yaml = serde_yaml::to_string(&request).unwrap();
        assert!(yaml.contains("{{secret:CURL_BEARER_TOKEN}}"));
        assert!(!yaml.contains(TEST_SECRET));
        assert!(!warnings.is_empty());
    }

    #[test]
    fn resolves_local_external_openapi_refs() {
        let root = std::env::temp_dir().join(format!("reqvault-openapi-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("schemas")).unwrap();
        let main_path = root.join("openapi.yaml");
        fs::write(
            &main_path,
            r#"openapi: 3.0.3
info: { title: Users }
servers: [{ url: https://api.example.test }]
paths:
  /users:
    post:
      requestBody:
        content:
          application/json:
            schema:
              $ref: schemas/user.yaml#/User
"#,
        )
        .unwrap();
        fs::write(
            root.join("schemas/user.yaml"),
            r#"User:
  type: object
  properties:
    name:
      type: string
      example: Pavel
"#,
        )
        .unwrap();
        let value: Value = serde_yaml::from_str(&fs::read_to_string(&main_path).unwrap()).unwrap();
        let (resolved, warnings) = resolve_openapi_refs(&value, &main_path);
        let parsed = parse_openapi(&resolved).unwrap();
        assert!(warnings.is_empty());
        assert!(matches!(
            &parsed.requests[0].1.body,
            BodyConfig::Json { value } if value.contains("Pavel")
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
