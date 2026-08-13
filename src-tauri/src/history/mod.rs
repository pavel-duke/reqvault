use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use uuid::Uuid;

use crate::{
    models::{HistoryEntry, HistorySettings, HistorySummary, HttpResponse, RequestFile},
    variables::redact_secret_references,
};

const SETTINGS_FILE: &str = "settings.json";

pub fn settings(root: &Path, workspace_id: &str) -> Result<HistorySettings, String> {
    let folder = workspace_folder(root, workspace_id)?;
    let path = folder.join(SETTINGS_FILE);
    if !path.exists() {
        return Ok(HistorySettings::default());
    }
    let content = fs::read_to_string(path)
        .map_err(|_| "Не удалось прочитать настройки истории".to_string())?;
    let mut value: HistorySettings =
        serde_json::from_str(&content).map_err(|_| "Настройки истории повреждены".to_string())?;
    value.max_entries = value.max_entries.clamp(1, 500);
    Ok(value)
}

pub fn set_settings(
    root: &Path,
    workspace_id: &str,
    mut value: HistorySettings,
) -> Result<HistorySettings, String> {
    value.max_entries = value.max_entries.clamp(1, 500);
    let folder = workspace_folder(root, workspace_id)?;
    fs::create_dir_all(&folder).map_err(|_| "Не удалось создать папку истории".to_string())?;
    write_json(&folder.join(SETTINGS_FILE), &value)?;
    enforce_limit(&folder, value.max_entries as usize)?;
    Ok(value)
}

pub fn record(
    root: &Path,
    workspace_id: &str,
    request: &RequestFile,
    response: &HttpResponse,
) -> Result<(), String> {
    let settings = settings(root, workspace_id)?;
    if !settings.enabled {
        return Ok(());
    }
    let folder = workspace_folder(root, workspace_id)?;
    fs::create_dir_all(&folder).map_err(|_| "Не удалось создать папку истории".to_string())?;
    let id = Uuid::new_v4().to_string();
    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let entry = HistoryEntry {
        summary: HistorySummary {
            id: id.clone(),
            created_at_ms,
            request_name: request.name.clone(),
            method: request.method.clone(),
            url: redact_secret_references(&request.url),
            status: response.status,
            duration_ms: response.duration_ms,
            size_bytes: response.size_bytes,
        },
        status_text: response.status_text.clone(),
        headers: response.headers.clone(),
        body: response.body.clone(),
        is_json: response.is_json,
        content_type: response.content_type.clone(),
        body_kind: response.body_kind.clone(),
        truncated: response.truncated,
    };
    write_json(&folder.join(format!("{created_at_ms}-{id}.json")), &entry)?;
    enforce_limit(&folder, settings.max_entries as usize)
}

pub fn list(root: &Path, workspace_id: &str) -> Result<Vec<HistorySummary>, String> {
    let folder = workspace_folder(root, workspace_id)?;
    if !folder.exists() {
        return Ok(Vec::new());
    }
    let mut entries = history_files(&folder)?
        .into_iter()
        .filter_map(|path| read_entry(&path).ok().map(|entry| entry.summary))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.created_at_ms));
    Ok(entries)
}

pub fn get(root: &Path, workspace_id: &str, id: &str) -> Result<HistoryEntry, String> {
    validate_entry_id(id)?;
    let folder = workspace_folder(root, workspace_id)?;
    history_files(&folder)?
        .into_iter()
        .find(|path| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .is_some_and(|stem| stem.ends_with(id))
        })
        .ok_or_else(|| "Запись истории не найдена".to_string())
        .and_then(|path| read_entry(&path))
}

pub fn delete(root: &Path, workspace_id: &str, id: &str) -> Result<(), String> {
    validate_entry_id(id)?;
    let folder = workspace_folder(root, workspace_id)?;
    if let Some(path) = history_files(&folder)?.into_iter().find(|path| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .is_some_and(|stem| stem.ends_with(id))
    }) {
        fs::remove_file(path).map_err(|_| "Не удалось удалить запись истории".to_string())?;
    }
    Ok(())
}

pub fn clear(root: &Path, workspace_id: &str) -> Result<(), String> {
    let folder = workspace_folder(root, workspace_id)?;
    for path in history_files(&folder)? {
        fs::remove_file(path).map_err(|_| "Не удалось очистить историю".to_string())?;
    }
    Ok(())
}

fn workspace_folder(root: &Path, workspace_id: &str) -> Result<PathBuf, String> {
    let id = Uuid::parse_str(workspace_id)
        .map_err(|_| "Некорректный идентификатор workspace".to_string())?;
    Ok(root.join("history").join(id.to_string()))
}

fn validate_entry_id(id: &str) -> Result<(), String> {
    Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| "Некорректный идентификатор записи".to_string())
}

fn history_files(folder: &Path) -> Result<Vec<PathBuf>, String> {
    if !folder.exists() {
        return Ok(Vec::new());
    }
    let files = fs::read_dir(folder)
        .map_err(|_| "Не удалось прочитать историю".to_string())?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("json")
                && path.file_name().and_then(|value| value.to_str()) != Some(SETTINGS_FILE)
        })
        .collect::<Vec<_>>();
    Ok(files)
}

fn read_entry(path: &Path) -> Result<HistoryEntry, String> {
    let content =
        fs::read_to_string(path).map_err(|_| "Не удалось прочитать запись истории".to_string())?;
    serde_json::from_str(&content).map_err(|_| "Запись истории повреждена".to_string())
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let content = serde_json::to_vec_pretty(value)
        .map_err(|_| "Не удалось подготовить данные истории".to_string())?;
    fs::write(&temporary, content).map_err(|_| "Не удалось записать историю".to_string())?;
    fs::rename(&temporary, path).map_err(|_| "Не удалось завершить запись истории".to_string())
}

fn enforce_limit(folder: &Path, limit: usize) -> Result<(), String> {
    let mut files = history_files(folder)?;
    files.sort();
    let remove_count = files.len().saturating_sub(limit);
    for path in files.into_iter().take(remove_count) {
        fs::remove_file(path).map_err(|_| "Не удалось применить лимит истории".to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!("reqvault-history-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn response(index: usize) -> HttpResponse {
        HttpResponse {
            request_id: Uuid::new_v4().to_string(),
            status: 200,
            status_text: "OK".to_string(),
            duration_ms: index as u128,
            size_bytes: 2,
            headers: Vec::new(),
            body: "{}".to_string(),
            is_json: true,
            content_type: "application/json".to_string(),
            body_kind: "json".to_string(),
            truncated: false,
        }
    }

    #[test]
    fn history_is_opt_in_and_respects_limit() {
        let root = temp_root();
        let workspace_id = Uuid::new_v4().to_string();
        let request = RequestFile {
            url: "https://api.example.test/{{secret:TOKEN}}".to_string(),
            ..RequestFile::default()
        };
        record(&root, &workspace_id, &request, &response(0)).unwrap();
        assert!(list(&root, &workspace_id).unwrap().is_empty());

        set_settings(
            &root,
            &workspace_id,
            HistorySettings {
                enabled: true,
                max_entries: 2,
            },
        )
        .unwrap();
        for index in 1..=3 {
            record(&root, &workspace_id, &request, &response(index)).unwrap();
        }
        let saved = list(&root, &workspace_id).unwrap();
        assert_eq!(saved.len(), 2);
        assert!(saved.iter().all(|item| !item.url.contains("{{secret:")));
        let loaded = get(&root, &workspace_id, &saved[0].id).unwrap();
        assert_eq!(loaded.summary.id, saved[0].id);
        clear(&root, &workspace_id).unwrap();
        assert!(list(&root, &workspace_id).unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
