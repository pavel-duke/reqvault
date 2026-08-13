use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use futures_util::{SinkExt, StreamExt};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tauri::ipc::Channel;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use url::Url;
use uuid::Uuid;

use crate::{
    models::{StreamConnectConfig, StreamEvent},
    redaction::redact_text,
    secrets::{self, KeyringBackend},
    variables::{ResolveError, resolve_secrets, resolve_variables},
    workspace,
};

#[derive(Default)]
pub struct StreamState {
    sessions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<StreamCommand>>>>,
}

enum StreamCommand {
    Send(String),
    Close,
}

struct PreparedStream {
    protocol: String,
    url: String,
    headers: HeaderMap,
    secret_values: Vec<String>,
}

pub fn connect(
    state: &StreamState,
    config: StreamConnectConfig,
    events: Channel<StreamEvent>,
) -> Result<String, String> {
    let prepared = prepare(config)?;
    let session_id = Uuid::new_v4().to_string();
    let (commands, receiver) = mpsc::unbounded_channel();
    state
        .sessions
        .lock()
        .map_err(|_| "Не удалось открыть потоковую сессию".to_string())?
        .insert(session_id.clone(), commands);

    let sessions = Arc::clone(&state.sessions);
    let task_session = session_id.clone();
    tauri::async_runtime::spawn(async move {
        let result = if prepared.protocol == "websocket" {
            run_websocket(&task_session, prepared, receiver, &events).await
        } else {
            run_sse(&task_session, prepared, receiver, &events).await
        };
        if let Err(error) = result {
            emit(&events, &task_session, "error", error);
        }
        emit(&events, &task_session, "closed", "Соединение закрыто");
        if let Ok(mut active) = sessions.lock() {
            active.remove(&task_session);
        }
    });

    Ok(session_id)
}

pub fn send(state: &StreamState, session_id: &str, message: String) -> Result<(), String> {
    if message.len() > 1024 * 1024 {
        return Err("WebSocket-сообщение больше 1 МБ".to_string());
    }
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "Потоковая сессия недоступна".to_string())?;
    sessions
        .get(session_id)
        .ok_or_else(|| "Потоковая сессия не найдена".to_string())?
        .send(StreamCommand::Send(message))
        .map_err(|_| "WebSocket-соединение уже закрыто".to_string())
}

pub fn disconnect(state: &StreamState, session_id: &str) -> Result<(), String> {
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "Потоковая сессия недоступна".to_string())?;
    sessions
        .get(session_id)
        .ok_or_else(|| "Потоковая сессия не найдена".to_string())?
        .send(StreamCommand::Close)
        .map_err(|_| "Соединение уже закрыто".to_string())
}

fn prepare(config: StreamConnectConfig) -> Result<PreparedStream, String> {
    if !matches!(config.protocol.as_str(), "websocket" | "sse") {
        return Err("Поддерживаются WebSocket и SSE".to_string());
    }
    let workspace_config = workspace::load_config(Path::new(&config.workspace_path))
        .map_err(|error| error.to_string())?;
    if workspace_config.id != config.workspace_id {
        return Err("Workspace не совпадает с идентификатором сессии".to_string());
    }
    let backend = KeyringBackend::new(&config.workspace_id).map_err(|error| error.to_string())?;
    let variables = config
        .environment
        .map(|item| item.variables)
        .unwrap_or_default();
    let mut secret_values = Vec::new();
    let mut resolve = |value: &str| -> Result<String, String> {
        let with_variables =
            resolve_variables(value, &variables).map_err(|error| error.to_string())?;
        resolve_secrets(
            &with_variables,
            &mut |name| {
                secrets::get(&backend, name).map_err(|error| match error {
                    secrets::SecretError::NotFound(name) => ResolveError::MissingSecret(name),
                    _ => ResolveError::SecretStorage,
                })
            },
            &mut secret_values,
        )
        .map_err(|error| error.to_string())
    };

    let url = resolve(&config.url)?;
    validate_url(&url, &config.protocol)?;
    let parsed = Url::parse(&url).map_err(|_| "Укажи корректный URL потока".to_string())?;
    let host = parsed.host_str().unwrap_or_default().to_lowercase();
    let guard = &workspace_config.production_guard;
    if guard.enabled {
        let secure = if config.protocol == "websocket" {
            parsed.scheme() == "wss"
        } else {
            parsed.scheme() == "https"
        };
        if guard.require_https && !secure {
            return Err("Production Guard требует защищённое потоковое соединение".to_string());
        }
        if guard.block_secrets_in_url && config.url.contains("{{secret:") {
            return Err("Production Guard запрещает ссылки на секреты в URL".to_string());
        }
        if !guard.allowed_hosts.is_empty()
            && !guard
                .allowed_hosts
                .iter()
                .any(|pattern| host_matches(&host, pattern))
        {
            return Err(format!("Production Guard блокирует хост {host}"));
        }
    }

    let mut headers = HeaderMap::new();
    for (name, value) in config.headers {
        let name = resolve(&name)?;
        let value = resolve(&value)?;
        headers.insert(
            HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| format!("Некорректный заголовок {name}"))?,
            HeaderValue::from_str(&value)
                .map_err(|_| format!("Некорректное значение заголовка {name}"))?,
        );
    }
    Ok(PreparedStream {
        protocol: config.protocol,
        url,
        headers,
        secret_values,
    })
}

async fn run_websocket(
    session_id: &str,
    prepared: PreparedStream,
    mut commands: mpsc::UnboundedReceiver<StreamCommand>,
    events: &Channel<StreamEvent>,
) -> Result<(), String> {
    let mut request = prepared
        .url
        .as_str()
        .into_client_request()
        .map_err(|_| "Не удалось подготовить WebSocket request".to_string())?;
    for (name, value) in &prepared.headers {
        request.headers_mut().insert(name, value.clone());
    }
    let (socket, response) = connect_async(request)
        .await
        .map_err(|error| redact_text(&error.to_string(), &prepared.secret_values))?;
    emit(
        events,
        session_id,
        "connected",
        format!("WebSocket подключён: HTTP {}", response.status()),
    );
    let (mut writer, mut reader) = socket.split();
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(StreamCommand::Send(value)) => {
                    writer.send(Message::Text(value.clone().into())).await
                        .map_err(|error| redact_text(&error.to_string(), &prepared.secret_values))?;
                    emit(events, session_id, "sent", redact_text(&value, &prepared.secret_values));
                }
                Some(StreamCommand::Close) | None => {
                    let _ = writer.send(Message::Close(None)).await;
                    break;
                }
            },
            message = reader.next() => match message {
                Some(Ok(Message::Text(value))) => emit(events, session_id, "message", redact_text(&value, &prepared.secret_values)),
                Some(Ok(Message::Binary(value))) => emit(events, session_id, "binary", format!("{} байт", value.len())),
                Some(Ok(Message::Ping(_))) => emit(events, session_id, "ping", "Ping"),
                Some(Ok(Message::Pong(_))) => emit(events, session_id, "pong", "Pong"),
                Some(Ok(Message::Close(frame))) => {
                    let description = frame.map(|value| format!("{} {}", value.code, value.reason)).unwrap_or_else(|| "Сервер закрыл соединение".to_string());
                    emit(events, session_id, "close_frame", description);
                    break;
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(redact_text(&error.to_string(), &prepared.secret_values)),
                None => break,
            }
        }
    }
    Ok(())
}

async fn run_sse(
    session_id: &str,
    prepared: PreparedStream,
    mut commands: mpsc::UnboundedReceiver<StreamCommand>,
    events: &Channel<StreamEvent>,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|_| "Не удалось подготовить SSE-клиент".to_string())?;
    let response = client
        .get(&prepared.url)
        .headers(prepared.headers)
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .send()
        .await
        .map_err(|error| redact_text(&error.to_string(), &prepared.secret_values))?;
    if !response.status().is_success() {
        return Err(format!("SSE endpoint вернул HTTP {}", response.status()));
    }
    emit(events, session_id, "connected", "SSE подключён");
    let mut bytes = response.bytes_stream();
    let mut buffer = String::new();
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(StreamCommand::Close) | None => break,
                Some(StreamCommand::Send(_)) => emit(events, session_id, "error", "SSE не поддерживает отправку сообщений"),
            },
            chunk = bytes.next() => match chunk {
                Some(Ok(chunk)) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));
                    drain_sse_events(&mut buffer, |kind, value| emit(events, session_id, kind, redact_text(&value, &prepared.secret_values)));
                }
                Some(Err(error)) => return Err(redact_text(&error.to_string(), &prepared.secret_values)),
                None => break,
            }
        }
    }
    Ok(())
}

fn drain_sse_events(buffer: &mut String, mut output: impl FnMut(&str, String)) {
    while let Some(index) = buffer.find("\n\n").or_else(|| buffer.find("\r\n\r\n")) {
        let separator = if buffer[index..].starts_with("\r\n\r\n") {
            4
        } else {
            2
        };
        let block = buffer[..index].replace("\r\n", "\n");
        buffer.drain(..index + separator);
        let mut event_name = "event".to_string();
        let mut data = Vec::new();
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("event:") {
                event_name = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("data:") {
                data.push(value.trim_start().to_string());
            }
        }
        if !data.is_empty() {
            output(&event_name, data.join("\n"));
        }
    }
}

fn validate_url(value: &str, protocol: &str) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| "Укажи корректный URL потока".to_string())?;
    let valid = if protocol == "websocket" {
        matches!(url.scheme(), "ws" | "wss")
    } else {
        matches!(url.scheme(), "http" | "https")
    };
    if valid {
        Ok(())
    } else if protocol == "websocket" {
        Err("WebSocket URL должен начинаться с ws:// или wss://".to_string())
    } else {
        Err("SSE URL должен начинаться с http:// или https://".to_string())
    }
}

fn host_matches(host: &str, pattern: &str) -> bool {
    let pattern = pattern.trim().trim_end_matches('.').to_lowercase();
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host != suffix && host.ends_with(&format!(".{suffix}"))
    } else {
        host == pattern
    }
}

fn emit(events: &Channel<StreamEvent>, session_id: &str, kind: &str, data: impl Into<String>) {
    let _ = events.send(StreamEvent {
        session_id: session_id.to_string(),
        kind: kind.to_string(),
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        data: data.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_multiline_sse_events() {
        let mut buffer =
            "event: update\ndata: first\ndata: second\n\ndata: next\n\npartial".to_string();
        let mut output = Vec::new();
        drain_sse_events(&mut buffer, |kind, value| {
            output.push((kind.to_string(), value));
        });
        assert_eq!(
            output[0],
            ("update".to_string(), "first\nsecond".to_string())
        );
        assert_eq!(output[1], ("event".to_string(), "next".to_string()));
        assert_eq!(buffer, "partial");
    }

    #[test]
    fn validates_protocol_specific_urls() {
        assert!(validate_url("wss://api.example.test/socket", "websocket").is_ok());
        assert!(validate_url("https://api.example.test/events", "sse").is_ok());
        assert!(validate_url("https://api.example.test/socket", "websocket").is_err());
    }
}
