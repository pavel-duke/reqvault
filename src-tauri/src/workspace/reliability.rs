use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    fs_utils::atomic_write,
    models::{
        BodyConfig, DiagnosticIssue, EnvironmentFile, FORMAT_VERSION, MigrationPlan,
        MultipartField, RequestFile, WorkspaceDiagnostics,
    },
    variables::{secret_names, variable_names},
};

use super::CONFIG_FILE;

pub fn fingerprint(root: &Path) -> Result<String, String> {
    let files = relevant_yaml_files(root)?;
    let mut digest = Sha256::new();
    for (relative, full) in files {
        digest.update(relative.to_string_lossy().as_bytes());
        let content = fs::read(&full).map_err(|error| format!("{}: {error}", full.display()))?;
        digest.update((content.len() as u64).to_le_bytes());
        digest.update(content);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn diagnose(root: &Path, available_secrets: &[String]) -> Result<WorkspaceDiagnostics, String> {
    if !root.is_dir() {
        return Err("Папка workspace не найдена".to_string());
    }
    let files = relevant_yaml_files(root)?;
    let mut issues = Vec::new();
    if !root.join("requests").is_dir() {
        issue(
            &mut issues,
            "warning",
            "missing_requests_directory",
            "requests",
            "Папка requests отсутствует",
            "Создайте папку requests или сохраните новый запрос",
        );
    }
    if !root.join("environments").is_dir() {
        issue(
            &mut issues,
            "warning",
            "missing_environments_directory",
            "environments",
            "Папка environments отсутствует",
            "Создайте хотя бы одно окружение",
        );
    }

    let available = available_secrets
        .iter()
        .map(|name| name.to_ascii_uppercase())
        .collect::<BTreeSet<_>>();
    let mut used_secrets = BTreeSet::new();
    let mut known_variables = BTreeSet::new();
    let mut requests = Vec::new();
    let mut environment_count = 0;

    for (relative, full) in &files {
        let path = slashes(relative);
        let content = match fs::read_to_string(full) {
            Ok(content) => content,
            Err(error) => {
                issue(
                    &mut issues,
                    "error",
                    "unreadable_file",
                    &path,
                    &format!("Файл не читается: {error}"),
                    "Проверьте права доступа и состояние диска",
                );
                continue;
            }
        };
        let raw: serde_yaml::Value = match serde_yaml::from_str(&content) {
            Ok(value) => value,
            Err(error) => {
                issue(
                    &mut issues,
                    "error",
                    "invalid_yaml",
                    &path,
                    &format!("Некорректный YAML: {error}"),
                    "Исправьте синтаксис YAML и повторите диагностику",
                );
                continue;
            }
        };
        match explicit_format_version(&raw) {
            Some(version) if version > FORMAT_VERSION => issue(
                &mut issues,
                "error",
                "newer_format",
                &path,
                &format!("Формат {version} новее поддерживаемого {FORMAT_VERSION}"),
                "Обновите ReqVault до совместимой версии",
            ),
            Some(version) if version < FORMAT_VERSION => issue(
                &mut issues,
                "warning",
                "migration_required",
                &path,
                &format!("Формат {version} требует миграции"),
                "Просмотрите план миграции и создайте резервную копию",
            ),
            None => issue(
                &mut issues,
                "warning",
                "missing_format_version",
                &path,
                "Не указан format_version",
                "Выполните миграцию workspace",
            ),
            _ => {}
        }
        for name in secret_names(&content) {
            used_secrets.insert(name.to_ascii_uppercase());
        }
        if relative == Path::new(CONFIG_FILE) {
            match serde_yaml::from_str::<crate::models::WorkspaceConfig>(&content) {
                Ok(config) => {
                    if Uuid::parse_str(&config.id).is_err() {
                        issue(
                            &mut issues,
                            "error",
                            "invalid_workspace_id",
                            &path,
                            "Некорректный UUID workspace",
                            "Создайте новый UUID и не используйте ID другого workspace",
                        );
                    }
                    if config.name.trim().is_empty() {
                        issue(
                            &mut issues,
                            "warning",
                            "empty_workspace_name",
                            &path,
                            "Имя workspace пустое",
                            "Укажите понятное имя проекта",
                        );
                    }
                }
                Err(error) => issue(
                    &mut issues,
                    "error",
                    "invalid_config",
                    &path,
                    &format!("Некорректная конфигурация: {error}"),
                    "Исправьте обязательные поля reqvault.yaml",
                ),
            }
        } else if relative.starts_with("environments") {
            match serde_yaml::from_str::<EnvironmentFile>(&content) {
                Ok(environment) => {
                    environment_count += 1;
                    known_variables.extend(environment.variables.keys().cloned());
                }
                Err(error) => issue(
                    &mut issues,
                    "error",
                    "invalid_environment",
                    &path,
                    &format!("Некорректное окружение: {error}"),
                    "Исправьте структуру environment YAML",
                ),
            }
        } else if relative.starts_with("requests") {
            match serde_yaml::from_str::<RequestFile>(&content) {
                Ok(request) => requests.push((path, request, content)),
                Err(error) => issue(
                    &mut issues,
                    "error",
                    "invalid_request",
                    &path,
                    &format!("Некорректный запрос: {error}"),
                    "Исправьте структуру request YAML",
                ),
            }
        }
    }

    let mut names = BTreeMap::<String, Vec<String>>::new();
    let request_count = requests.len();
    for (path, request, content) in requests {
        names
            .entry(request.name.to_lowercase())
            .or_default()
            .push(path.clone());
        if request.url.trim().is_empty() {
            issue(
                &mut issues,
                "warning",
                "empty_url",
                &path,
                "URL запроса пуст",
                "Укажите URL перед запуском запроса",
            );
        }
        if let Err(error) = super::validate_credentials(&request) {
            issue(
                &mut issues,
                "error",
                "unsafe_credential",
                &path,
                &error.to_string(),
                "Перенесите credential в Secret Vault",
            );
        }
        for variable in variable_names(&content) {
            if !known_variables.contains(&variable) {
                issue(
                    &mut issues,
                    "warning",
                    "missing_variable",
                    &path,
                    &format!("Переменная {variable} не найдена ни в одном окружении"),
                    "Добавьте переменную в environment или исправьте имя",
                );
            }
        }
        check_request_files(root, &path, &request, &mut issues);
    }
    for (name, paths) in names.into_iter().filter(|(_, paths)| paths.len() > 1) {
        issue(
            &mut issues,
            "warning",
            "duplicate_request_name",
            &paths.join(", "),
            &format!("Имя запроса «{name}» повторяется {} раз", paths.len()),
            "Используйте уникальные имена для удобного поиска и отчётов",
        );
    }
    for missing in used_secrets.difference(&available) {
        issue(
            &mut issues,
            "warning",
            "missing_secret",
            "Secret Vault",
            &format!("Секрет {missing} используется, но не сохранён"),
            "Добавьте секрет через окно Secret Vault",
        );
    }
    for unused in available.difference(&used_secrets) {
        issue(
            &mut issues,
            "info",
            "unused_secret",
            "Secret Vault",
            &format!("Секрет {unused} сейчас не используется"),
            "Удалите его, если он больше не нужен",
        );
    }
    if environment_count == 0 {
        issue(
            &mut issues,
            "warning",
            "no_environments",
            "environments",
            "Не найдено ни одного корректного окружения",
            "Создайте local или testing environment",
        );
    }
    issues.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then((&left.path, &left.code).cmp(&(&right.path, &right.code)))
    });
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .count();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == "warning")
        .count();
    Ok(WorkspaceDiagnostics {
        checked_at_ms: now_ms(),
        fingerprint: fingerprint(root)?,
        files: files.len(),
        requests: request_count,
        environments: environment_count,
        errors,
        warnings,
        issues,
        migration: migration_plan(root)?,
    })
}

pub fn migration_plan(root: &Path) -> Result<MigrationPlan, String> {
    let files = relevant_yaml_files(root)?;
    if !root.join(CONFIG_FILE).is_file() {
        return Err("В папке нет reqvault.yaml".to_string());
    }
    let mut migrate = Vec::new();
    let mut warnings = Vec::new();
    let mut current = FORMAT_VERSION;
    for (relative, full) in files {
        let content =
            fs::read_to_string(&full).map_err(|error| format!("{}: {error}", full.display()))?;
        let raw: serde_yaml::Value = match serde_yaml::from_str(&content) {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!(
                    "{}: некорректный YAML ({error})",
                    slashes(&relative)
                ));
                continue;
            }
        };
        let version = explicit_format_version(&raw).unwrap_or(0);
        if version > FORMAT_VERSION {
            return Err(format!(
                "{} использует более новый формат {version}",
                slashes(&relative)
            ));
        }
        current = current.min(version);
        if version < FORMAT_VERSION {
            migrate.push(slashes(&relative));
        }
    }
    Ok(MigrationPlan {
        required: !migrate.is_empty(),
        current_version: current,
        target_version: FORMAT_VERSION,
        changes: if migrate.is_empty() {
            Vec::new()
        } else {
            vec![
                "Добавить явный format_version: 1 без изменения содержимого запросов и окружений"
                    .to_string(),
            ]
        },
        files: migrate,
        warnings,
    })
}

pub fn apply_migration(root: &Path) -> Result<Option<String>, String> {
    let plan = migration_plan(root)?;
    if !plan.warnings.is_empty() {
        return Err("Сначала исправьте ошибки YAML из плана миграции".to_string());
    }
    if !plan.required {
        return Ok(None);
    }
    let backup_id = format!("{}-{}", now_ms(), Uuid::new_v4());
    let backup_root = root.join(".reqvault").join("backups").join(&backup_id);
    for relative in &plan.files {
        let source = root.join(relative);
        let destination = backup_root.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Не удалось создать backup: {error}"))?;
        }
        fs::copy(&source, &destination)
            .map_err(|error| format!("Не удалось сохранить {}: {error}", source.display()))?;
    }
    let result = (|| {
        for relative in &plan.files {
            let path = root.join(relative);
            let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            let mut raw: serde_yaml::Value =
                serde_yaml::from_str(&content).map_err(|error| error.to_string())?;
            let mapping = raw
                .as_mapping_mut()
                .ok_or_else(|| format!("{} не является YAML-объектом", path.display()))?;
            mapping.insert(
                serde_yaml::Value::String("format_version".to_string()),
                serde_yaml::Value::Number(FORMAT_VERSION.into()),
            );
            let migrated = serde_yaml::to_string(&raw).map_err(|error| error.to_string())?;
            atomic_write(&path, migrated.as_bytes()).map_err(|error| error.to_string())?;
        }
        Ok::<(), String>(())
    })();
    if let Err(error) = result {
        let _ = restore_backup(root, &backup_id);
        return Err(format!(
            "Миграция отменена, исходные файлы восстановлены: {error}"
        ));
    }
    Ok(Some(backup_id))
}

pub fn restore_backup(root: &Path, backup_id: &str) -> Result<(), String> {
    validate_backup_id(backup_id)?;
    let backup_root = root.join(".reqvault").join("backups").join(backup_id);
    if !backup_root.is_dir() {
        return Err("Резервная копия не найдена".to_string());
    }
    for (relative, source) in recursive_files(&backup_root)? {
        let destination = root.join(&relative);
        let content = fs::read(&source).map_err(|error| error.to_string())?;
        atomic_write(&destination, &content).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn check_request_files(
    root: &Path,
    request_path: &str,
    request: &RequestFile,
    issues: &mut Vec<DiagnosticIssue>,
) {
    for (label, value) in [
        ("custom CA", request.transport.custom_ca_path.as_str()),
        (
            "client certificate",
            request.transport.client_certificate_path.as_str(),
        ),
        ("client key", request.transport.client_key_path.as_str()),
    ] {
        check_file(root, request_path, label, value, issues);
    }
    if let BodyConfig::Multipart { fields } = &request.body {
        for field in fields {
            if let MultipartField::File {
                name,
                path,
                enabled,
                ..
            } = field
                && *enabled
            {
                check_file(
                    root,
                    request_path,
                    &format!("multipart {name}"),
                    path,
                    issues,
                );
            }
        }
    }
}

fn check_file(
    root: &Path,
    request_path: &str,
    label: &str,
    value: &str,
    issues: &mut Vec<DiagnosticIssue>,
) {
    if value.trim().is_empty() || value.contains("{{") {
        return;
    }
    let path = Path::new(value);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    if !resolved.is_file() {
        issue(
            issues,
            "warning",
            "missing_external_file",
            request_path,
            &format!("Не найден файл для {label}: {value}"),
            "Исправьте путь или добавьте файл",
        );
    }
}

fn explicit_format_version(value: &serde_yaml::Value) -> Option<u32> {
    value
        .as_mapping()?
        .get(serde_yaml::Value::String("format_version".to_string()))?
        .as_u64()
        .and_then(|version| u32::try_from(version).ok())
}

fn relevant_yaml_files(root: &Path) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut files = Vec::new();
    let config = root.join(CONFIG_FILE);
    if config.is_file() {
        files.push((PathBuf::from(CONFIG_FILE), config));
    }
    for directory in ["requests", "environments"] {
        let base = root.join(directory);
        if base.is_dir() {
            for (relative, full) in recursive_files(&base)? {
                if full.extension().and_then(|value| value.to_str()) == Some("yaml") {
                    files.push((PathBuf::from(directory).join(relative), full));
                }
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn recursive_files(root: &Path) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut result = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| "Некорректный путь backup".to_string())?
                    .to_path_buf();
                result.push((relative, path));
            }
        }
    }
    result.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(result)
}

fn validate_backup_id(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.components().count() != 1
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("Некорректный идентификатор backup".to_string());
    }
    Ok(())
}

fn issue(
    issues: &mut Vec<DiagnosticIssue>,
    severity: &str,
    code: &str,
    path: &str,
    message: &str,
    remediation: &str,
) {
    issues.push(DiagnosticIssue {
        severity: severity.to_string(),
        code: code.to_string(),
        path: path.to_string(),
        message: message.to_string(),
        remediation: remediation.to_string(),
    });
}

fn severity_rank(value: &str) -> u8 {
    match value {
        "error" => 0,
        "warning" => 1,
        _ => 2,
    }
}

fn slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("reqvault-reliability-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("requests")).unwrap();
        fs::create_dir_all(root.join("environments")).unwrap();
        root
    }

    #[test]
    fn reports_invalid_yaml_missing_secret_variable_and_file() {
        let root = temp_root();
        fs::write(
            root.join(CONFIG_FILE),
            format!("format_version: 1\nid: {}\nname: Test\n", Uuid::new_v4()),
        )
        .unwrap();
        fs::write(
            root.join("environments/local.yaml"),
            "format_version: 1\nname: local\nvariables: {}\n",
        )
        .unwrap();
        fs::write(root.join("requests/broken.yaml"), "name: [broken").unwrap();
        fs::write(root.join("requests/users.yaml"), "format_version: 1\nname: Users\nmethod: GET\nurl: '{{MISSING}}/users'\nauth:\n  type: bearer\n  token: '{{secret:TOKEN}}'\ntransport:\n  custom_ca_path: missing.pem\n").unwrap();
        let report = diagnose(&root, &[]).unwrap();
        assert!(report.errors >= 1);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_yaml")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_secret")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_variable")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_external_file")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn previews_migrates_and_restores_legacy_files() {
        let root = temp_root();
        fs::write(
            root.join(CONFIG_FILE),
            format!("id: {}\nname: Legacy\n", Uuid::new_v4()),
        )
        .unwrap();
        fs::write(
            root.join("environments/local.yaml"),
            "name: local\nvariables: {}\n",
        )
        .unwrap();
        let original = fs::read_to_string(root.join(CONFIG_FILE)).unwrap();
        let plan = migration_plan(&root).unwrap();
        assert!(plan.required);
        assert_eq!(plan.current_version, 0);
        let backup = apply_migration(&root).unwrap().unwrap();
        assert!(
            fs::read_to_string(root.join(CONFIG_FILE))
                .unwrap()
                .contains("format_version: 1")
        );
        restore_backup(&root, &backup).unwrap();
        assert_eq!(
            fs::read_to_string(root.join(CONFIG_FILE)).unwrap(),
            original
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fingerprint_changes_after_external_edit() {
        let root = temp_root();
        fs::write(
            root.join(CONFIG_FILE),
            format!("format_version: 1\nid: {}\nname: Test\n", Uuid::new_v4()),
        )
        .unwrap();
        let before = fingerprint(&root).unwrap();
        fs::write(
            root.join("requests/new.yaml"),
            "format_version: 1\nname: New\n",
        )
        .unwrap();
        assert_ne!(before, fingerprint(&root).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
