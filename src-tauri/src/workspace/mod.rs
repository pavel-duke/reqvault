use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use thiserror::Error;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::models::{
    EnvironmentFile, EnvironmentSummary, FORMAT_VERSION, RequestFile, RequestSummary,
    WorkspaceConfig, WorkspaceSnapshot,
};
use crate::{
    fs_utils::atomic_write,
    models::{AuthConfig, ProxyConfig},
    variables::is_exact_secret_reference,
};

pub mod reliability;

const CONFIG_FILE: &str = "reqvault.yaml";
const MAX_BUNDLE_SIZE: u64 = 20 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceBundle {
    bundle_version: u32,
    name: String,
    production_guard: crate::models::ProductionGuard,
    requests: Vec<RequestSummary>,
    environments: Vec<EnvironmentSummary>,
}

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("Папка workspace не найдена")]
    MissingDirectory,
    #[error("Выбранный путь не является папкой")]
    NotDirectory,
    #[error("В этой папке уже есть reqvault.yaml")]
    AlreadyExists,
    #[error("Для импорта выбери пустую папку")]
    TargetNotEmpty,
    #[error("В папке нет reqvault.yaml")]
    MissingConfig,
    #[error("Формат workspace не поддерживается: {0}")]
    UnsupportedFormat(u32),
    #[error("Недопустимый путь внутри workspace")]
    InvalidRelativePath,
    #[error("Не удалось прочитать {path}: {message}")]
    Read { path: String, message: String },
    #[error("Не удалось записать {path}: {message}")]
    Write { path: String, message: String },
    #[error("Не удалось разобрать YAML в {path}: {message}")]
    InvalidYaml { path: String, message: String },
    #[error("Не удалось разобрать ReqVault bundle в {path}: {message}")]
    InvalidBundle { path: String, message: String },
    #[error(
        "{0} нельзя сохранять открытым текстом. Добавь значение в Secret Vault и используй ссылку {{{{secret:NAME}}}}"
    )]
    UnsafeCredential(&'static str),
}

pub fn create(path: &Path, name: Option<String>) -> Result<WorkspaceSnapshot, WorkspaceError> {
    if !path.exists() {
        return Err(WorkspaceError::MissingDirectory);
    }
    if !path.is_dir() {
        return Err(WorkspaceError::NotDirectory);
    }
    if path.join(CONFIG_FILE).exists() {
        return Err(WorkspaceError::AlreadyExists);
    }

    fs::create_dir_all(path.join("requests")).map_err(|error| write_error(path, error))?;
    fs::create_dir_all(path.join("environments")).map_err(|error| write_error(path, error))?;

    let fallback_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("ReqVault workspace");
    let config = WorkspaceConfig {
        format_version: FORMAT_VERSION,
        id: Uuid::new_v4().to_string(),
        name: name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| fallback_name.to_string()),
        production_guard: Default::default(),
    };

    write_yaml(&path.join(CONFIG_FILE), &config)?;
    write_yaml(
        &path.join("environments").join("local.yaml"),
        &EnvironmentFile::default(),
    )?;

    open(path)
}

pub fn open(path: &Path) -> Result<WorkspaceSnapshot, WorkspaceError> {
    if !path.exists() {
        return Err(WorkspaceError::MissingDirectory);
    }
    if !path.is_dir() {
        return Err(WorkspaceError::NotDirectory);
    }

    let config = load_config(path)?;

    let mut requests = Vec::new();
    collect_yaml_files(path, &path.join("requests"), &mut |file, relative| {
        let request: RequestFile = read_yaml(file)?;
        ensure_format(request.format_version)?;
        requests.push(RequestSummary {
            relative_path: path_to_slashes(relative),
            request,
        });
        Ok(())
    })?;
    requests.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let mut environments = Vec::new();
    collect_yaml_files(path, &path.join("environments"), &mut |file, relative| {
        let environment: EnvironmentFile = read_yaml(file)?;
        ensure_format(environment.format_version)?;
        environments.push(EnvironmentSummary {
            relative_path: path_to_slashes(relative),
            environment,
        });
        Ok(())
    })?;
    environments.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let root_path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();

    Ok(WorkspaceSnapshot {
        root_path,
        config,
        requests,
        environments,
    })
}

pub fn load_config(path: &Path) -> Result<WorkspaceConfig, WorkspaceError> {
    let config_path = path.join(CONFIG_FILE);
    if !config_path.is_file() {
        return Err(WorkspaceError::MissingConfig);
    }
    let config: WorkspaceConfig = read_yaml(&config_path)?;
    ensure_format(config.format_version)?;
    Ok(config)
}

pub fn save_config(
    root: &Path,
    config: &WorkspaceConfig,
) -> Result<WorkspaceConfig, WorkspaceError> {
    ensure_workspace(root)?;
    ensure_format(config.format_version)?;
    let current: WorkspaceConfig = read_yaml(&root.join(CONFIG_FILE))?;
    if current.id != config.id {
        return Err(WorkspaceError::InvalidRelativePath);
    }
    let mut normalized = config.clone();
    normalized.production_guard.allowed_hosts = normalized
        .production_guard
        .allowed_hosts
        .into_iter()
        .map(|host| host.trim().to_lowercase())
        .filter(|host| !host.is_empty())
        .collect();
    normalized.production_guard.allowed_hosts.sort();
    normalized.production_guard.allowed_hosts.dedup();
    normalized.production_guard.blocked_methods = normalized
        .production_guard
        .blocked_methods
        .into_iter()
        .map(|method| method.trim().to_uppercase())
        .filter(|method| !method.is_empty())
        .collect();
    normalized.production_guard.blocked_methods.sort();
    normalized.production_guard.blocked_methods.dedup();
    write_yaml(&root.join(CONFIG_FILE), &normalized)?;
    Ok(normalized)
}

pub fn export_bundle(root: &Path, destination: &Path) -> Result<(), WorkspaceError> {
    let snapshot = open(root)?;
    let bundle = WorkspaceBundle {
        bundle_version: 1,
        name: snapshot.config.name,
        production_guard: snapshot.config.production_guard,
        requests: snapshot.requests,
        environments: snapshot.environments,
    };
    let content = serde_json::to_string_pretty(&bundle).map_err(|error| WorkspaceError::Write {
        path: destination.display().to_string(),
        message: error.to_string(),
    })?;
    atomic_write(destination, content.as_bytes()).map_err(|error| write_error(destination, error))
}

pub fn import_bundle(source: &Path, target: &Path) -> Result<WorkspaceSnapshot, WorkspaceError> {
    let metadata = fs::metadata(source).map_err(|error| read_error(source, error))?;
    if metadata.len() > MAX_BUNDLE_SIZE {
        return Err(WorkspaceError::Read {
            path: source.display().to_string(),
            message: "bundle больше 20 МБ".to_string(),
        });
    }
    let content = fs::read_to_string(source).map_err(|error| read_error(source, error))?;
    let bundle: WorkspaceBundle =
        serde_json::from_str(&content).map_err(|error| WorkspaceError::InvalidBundle {
            path: source.display().to_string(),
            message: error.to_string(),
        })?;
    if bundle.bundle_version != 1 {
        return Err(WorkspaceError::UnsupportedFormat(bundle.bundle_version));
    }
    if fs::read_dir(target)
        .map_err(|error| read_error(target, error))?
        .next()
        .is_some()
    {
        return Err(WorkspaceError::TargetNotEmpty);
    }

    let created = create(target, Some(bundle.name))?;
    let mut config = created.config;
    config.production_guard = bundle.production_guard;
    save_config(target, &config)?;
    for item in bundle.requests {
        save_request(target, Some(&item.relative_path), None, &item.request)?;
    }
    for item in bundle.environments {
        save_environment(target, Some(&item.relative_path), &item.environment)?;
    }
    open(target)
}

pub fn save_request(
    root: &Path,
    relative_path: Option<&str>,
    collection: Option<&str>,
    request: &RequestFile,
) -> Result<RequestSummary, WorkspaceError> {
    ensure_workspace(root)?;
    ensure_format(request.format_version)?;
    validate_credentials(request)?;

    let relative = match relative_path {
        Some(value) => validate_relative_file(value, "requests")?,
        None => unique_request_path(root, collection, &request.name),
    };
    let full_path = root.join(&relative);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|error| write_error(parent, error))?;
    }
    write_yaml(&full_path, request)?;

    Ok(RequestSummary {
        relative_path: path_to_slashes(&relative),
        request: request.clone(),
    })
}

fn validate_credentials(request: &RequestFile) -> Result<(), WorkspaceError> {
    let require_reference = |value: &str, label: &'static str| {
        if value.trim().is_empty() || is_exact_secret_reference(value) {
            Ok(())
        } else {
            Err(WorkspaceError::UnsafeCredential(label))
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

pub fn delete_request(root: &Path, relative_path: &str) -> Result<(), WorkspaceError> {
    ensure_workspace(root)?;
    let relative = validate_relative_file(relative_path, "requests")?;
    let full_path = root.join(relative);
    if full_path.exists() {
        fs::remove_file(&full_path).map_err(|error| write_error(&full_path, error))?;
    }
    Ok(())
}

pub fn save_environment(
    root: &Path,
    relative_path: Option<&str>,
    environment: &EnvironmentFile,
) -> Result<EnvironmentSummary, WorkspaceError> {
    ensure_workspace(root)?;
    ensure_format(environment.format_version)?;
    let relative = match relative_path {
        Some(value) => validate_relative_file(value, "environments")?,
        None => PathBuf::from("environments").join(format!(
            "{}.yaml",
            safe_name(&environment.name, "environment")
        )),
    };
    let full_path = root.join(&relative);
    write_yaml(&full_path, environment)?;

    Ok(EnvironmentSummary {
        relative_path: path_to_slashes(&relative),
        environment: environment.clone(),
    })
}

pub fn delete_environment(root: &Path, relative_path: &str) -> Result<(), WorkspaceError> {
    ensure_workspace(root)?;
    let relative = validate_relative_file(relative_path, "environments")?;
    let full_path = root.join(relative);
    if full_path.exists() {
        fs::remove_file(&full_path).map_err(|error| write_error(&full_path, error))?;
    }
    Ok(())
}

fn ensure_workspace(root: &Path) -> Result<(), WorkspaceError> {
    if !root.is_dir() || !root.join(CONFIG_FILE).is_file() {
        return Err(WorkspaceError::MissingConfig);
    }
    Ok(())
}

fn ensure_format(version: u32) -> Result<(), WorkspaceError> {
    if version != FORMAT_VERSION {
        return Err(WorkspaceError::UnsupportedFormat(version));
    }
    Ok(())
}

fn unique_request_path(root: &Path, collection: Option<&str>, name: &str) -> PathBuf {
    let folder = safe_name(collection.unwrap_or("Общее"), "common");
    let stem = safe_name(name, "request");
    let base = PathBuf::from("requests").join(folder);
    let mut candidate = base.join(format!("{stem}.yaml"));
    let mut suffix = 2;
    while root.join(&candidate).exists() {
        candidate = base.join(format!("{stem}-{suffix}.yaml"));
        suffix += 1;
    }
    candidate
}

fn safe_name(value: &str, fallback: &str) -> String {
    let mut result = String::new();
    let mut last_was_separator = false;
    for character in value.trim().chars() {
        if character.is_alphanumeric() {
            result.extend(character.to_lowercase());
            last_was_separator = false;
        } else if !last_was_separator && !result.is_empty() {
            result.push('-');
            last_was_separator = true;
        }
    }
    let trimmed = result.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_relative_file(value: &str, prefix: &str) -> Result<PathBuf, WorkspaceError> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.extension().and_then(|value| value.to_str()) != Some("yaml")
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || path.components().next() != Some(Component::Normal(prefix.as_ref()))
    {
        return Err(WorkspaceError::InvalidRelativePath);
    }
    Ok(path.to_path_buf())
}

fn collect_yaml_files<F>(
    root: &Path,
    directory: &Path,
    callback: &mut F,
) -> Result<(), WorkspaceError>
where
    F: FnMut(&Path, &Path) -> Result<(), WorkspaceError>,
{
    if !directory.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(directory).map_err(|error| read_error(directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| read_error(directory, error))?;
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files(root, &path, callback)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| WorkspaceError::InvalidRelativePath)?;
            callback(&path, relative)?;
        }
    }
    Ok(())
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, WorkspaceError> {
    let content = fs::read_to_string(path).map_err(|error| read_error(path, error))?;
    serde_yaml::from_str(&content).map_err(|error| WorkspaceError::InvalidYaml {
        path: path.display().to_string(),
        message: error.to_string(),
    })
}

fn write_yaml<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), WorkspaceError> {
    let content = serde_yaml::to_string(value).map_err(|error| WorkspaceError::Write {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    atomic_write(path, content.as_bytes()).map_err(|error| write_error(path, error))
}

fn path_to_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn read_error(path: &Path, error: std::io::Error) -> WorkspaceError {
    WorkspaceError::Read {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

fn write_error(path: &Path, error: std::io::Error) -> WorkspaceError {
    WorkspaceError::Write {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn temp_workspace() -> PathBuf {
        let path = std::env::temp_dir().join(format!("reqvault-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn creates_and_reopens_workspace() {
        let path = temp_workspace();
        let created = create(&path, Some("Тестовый API".to_string())).unwrap();
        assert_eq!(created.config.name, "Тестовый API");
        assert_eq!(created.environments.len(), 1);

        let mut request = RequestFile {
            name: "Получить пользователя".to_string(),
            url: "{{BASE_URL}}/users".to_string(),
            ..RequestFile::default()
        };
        request.headers.insert(
            "Authorization".to_string(),
            "Bearer {{secret:API_TOKEN}}".to_string(),
        );
        save_request(&path, None, Some("Users"), &request).unwrap();

        let reopened = open(&path).unwrap();
        assert_eq!(reopened.requests.len(), 1);
        assert_eq!(reopened.requests[0].request, request);
        assert!(reopened.requests[0].relative_path.starts_with("requests/"));

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn saves_environment_as_yaml() {
        let path = temp_workspace();
        create(&path, None).unwrap();
        let environment = EnvironmentFile {
            format_version: FORMAT_VERSION,
            name: "testing".to_string(),
            variables: BTreeMap::from([(
                "BASE_URL".to_string(),
                "https://api.example.test".to_string(),
            )]),
        };
        let saved = save_environment(&path, None, &environment).unwrap();
        assert_eq!(saved.relative_path, "environments/testing.yaml");
        assert_eq!(open(&path).unwrap().environments.len(), 2);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn rejects_paths_outside_workspace() {
        assert!(validate_relative_file("../token.yaml", "requests").is_err());
        assert!(validate_relative_file("environments/local.yaml", "requests").is_err());
        assert!(validate_relative_file("requests/item.json", "requests").is_err());
    }

    #[test]
    fn rejects_invalid_workspace() {
        let path = temp_workspace();
        let error = open(&path).unwrap_err();
        assert!(matches!(error, WorkspaceError::MissingConfig));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn never_writes_secret_values_to_yaml() {
        const TEST_SECRET: &str = "REQVAULT_TEST_SECRET_DO_NOT_LEAK_123456";
        let path = temp_workspace();
        create(&path, None).unwrap();
        let request = RequestFile {
            url: "https://api.example.test".to_string(),
            auth: crate::models::AuthConfig::Bearer {
                token: "{{secret:API_TOKEN}}".to_string(),
            },
            ..RequestFile::default()
        };
        save_request(&path, None, None, &request).unwrap();

        let yaml = fs::read_to_string(
            path.join("requests")
                .join("общее")
                .join("новый-запрос.yaml"),
        )
        .unwrap();
        assert!(yaml.contains("{{secret:API_TOKEN}}"));
        assert!(!yaml.contains(TEST_SECRET));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn rejects_plain_credentials() {
        let path = temp_workspace();
        create(&path, None).unwrap();
        let request = RequestFile {
            auth: crate::models::AuthConfig::Bearer {
                token: "plain-token".to_string(),
            },
            ..RequestFile::default()
        };
        assert!(matches!(
            save_request(&path, None, None, &request),
            Err(WorkspaceError::UnsafeCredential(_))
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn exports_and_imports_portable_bundle_with_new_workspace_id() {
        let source = temp_workspace();
        let target = temp_workspace();
        let bundle = std::env::temp_dir().join(format!("reqvault-{}.json", Uuid::new_v4()));
        let created = create(&source, Some("Portable API".to_string())).unwrap();
        let request = RequestFile {
            name: "Health".to_string(),
            url: "https://api.example.test/health".to_string(),
            ..RequestFile::default()
        };
        save_request(&source, None, Some("System"), &request).unwrap();
        export_bundle(&source, &bundle).unwrap();
        let exported = fs::read_to_string(&bundle).unwrap();
        assert!(!exported.contains(&created.config.id));

        let imported = import_bundle(&bundle, &target).unwrap();
        assert_eq!(imported.config.name, "Portable API");
        assert_ne!(imported.config.id, created.config.id);
        assert_eq!(imported.requests.len(), 1);

        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(target).unwrap();
        fs::remove_file(bundle).unwrap();
    }

    #[test]
    fn opens_large_workspace_with_stable_order() {
        let path = temp_workspace();
        create(&path, Some("Large API".to_string())).unwrap();
        let collection = path.join("requests").join("generated");
        fs::create_dir_all(&collection).unwrap();
        for index in (0..1_000).rev() {
            let request = RequestFile {
                name: format!("Request {index:04}"),
                url: format!("https://api.example.test/items/{index}"),
                ..RequestFile::default()
            };
            write_yaml(
                &collection.join(format!("request-{index:04}.yaml")),
                &request,
            )
            .unwrap();
        }

        let snapshot = open(&path).unwrap();
        assert_eq!(snapshot.requests.len(), 1_000);
        assert_eq!(
            snapshot.requests.first().unwrap().request.name,
            "Request 0000"
        );
        assert_eq!(
            snapshot.requests.last().unwrap().request.name,
            "Request 0999"
        );
        assert!(
            snapshot
                .requests
                .windows(2)
                .all(|items| items[0].relative_path < items[1].relative_path)
        );
        fs::remove_dir_all(path).unwrap();
    }
}
