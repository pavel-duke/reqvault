use std::collections::BTreeSet;

use regex::Regex;
use thiserror::Error;

const INDEX_NAME: &str = "__reqvault_secret_names__";

pub trait SecretBackend {
    fn get(&self, name: &str) -> Result<Option<String>, String>;
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
}

pub struct KeyringBackend {
    service: String,
}

impl KeyringBackend {
    pub fn new(workspace_id: &str) -> Result<Self, SecretError> {
        validate_workspace_id(workspace_id)?;
        Ok(Self {
            service: format!("io.github.pavel-duke.reqvault.{workspace_id}"),
        })
    }

    fn entry(&self, name: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(&self.service, name).map_err(|error| error.to_string())
    }
}

impl SecretBackend for KeyringBackend {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        match self.entry(name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        self.entry(name)?
            .set_password(value)
            .map_err(|error| error.to_string())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        match self.entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretError {
    #[error(
        "Имя секрета должно начинаться с буквы и содержать только буквы, цифры, точку, дефис или подчёркивание"
    )]
    InvalidName,
    #[error("Некорректный идентификатор workspace")]
    InvalidWorkspace,
    #[error("Значение секрета не может быть пустым")]
    EmptyValue,
    #[error("Секрет {0} не найден в системном хранилище")]
    NotFound(String),
    #[error("Системное хранилище секретов недоступно")]
    StorageUnavailable,
    #[error("Не удалось прочитать список секретов")]
    InvalidIndex,
}

pub fn list(backend: &impl SecretBackend) -> Result<Vec<String>, SecretError> {
    let Some(index) = backend
        .get(INDEX_NAME)
        .map_err(|_| SecretError::StorageUnavailable)?
    else {
        return Ok(Vec::new());
    };
    let names: Vec<String> = serde_json::from_str(&index).map_err(|_| SecretError::InvalidIndex)?;
    let mut unique = names
        .into_iter()
        .filter(|name| valid_name(name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    unique.sort();
    Ok(unique)
}

pub fn save(
    backend: &impl SecretBackend,
    name: &str,
    value: &str,
) -> Result<Vec<String>, SecretError> {
    let name = normalize_name(name)?;
    if value.is_empty() {
        return Err(SecretError::EmptyValue);
    }
    let mut names = list(backend)?;
    let is_new = !names.contains(&name);
    backend
        .set(&name, value)
        .map_err(|_| SecretError::StorageUnavailable)?;
    if is_new {
        names.push(name.clone());
        names.sort();
        if let Err(error) = write_index(backend, &names) {
            let _ = backend.delete(&name);
            return Err(error);
        }
    }
    Ok(names)
}

pub fn delete(backend: &impl SecretBackend, name: &str) -> Result<Vec<String>, SecretError> {
    let name = normalize_name(name)?;
    backend
        .delete(&name)
        .map_err(|_| SecretError::StorageUnavailable)?;
    let mut names = list(backend)?;
    names.retain(|saved| saved != &name);
    write_index(backend, &names)?;
    Ok(names)
}

pub fn get(backend: &impl SecretBackend, name: &str) -> Result<String, SecretError> {
    let name = normalize_name(name)?;
    backend
        .get(&name)
        .map_err(|_| SecretError::StorageUnavailable)?
        .ok_or(SecretError::NotFound(name))
}

fn write_index(backend: &impl SecretBackend, names: &[String]) -> Result<(), SecretError> {
    let value = serde_json::to_string(names).map_err(|_| SecretError::InvalidIndex)?;
    backend
        .set(INDEX_NAME, &value)
        .map_err(|_| SecretError::StorageUnavailable)
}

fn normalize_name(name: &str) -> Result<String, SecretError> {
    let normalized = name.trim().to_uppercase();
    if valid_name(&normalized) {
        Ok(normalized)
    } else {
        Err(SecretError::InvalidName)
    }
}

fn valid_name(name: &str) -> bool {
    static NAME_PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    NAME_PATTERN
        .get_or_init(|| Regex::new(r"^[A-Za-z][A-Za-z0-9_.-]{0,127}$").unwrap())
        .is_match(name)
}

fn validate_workspace_id(workspace_id: &str) -> Result<(), SecretError> {
    uuid::Uuid::parse_str(workspace_id)
        .map(|_| ())
        .map_err(|_| SecretError::InvalidWorkspace)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeMap};

    use super::*;

    #[derive(Default)]
    struct MemoryBackend {
        values: RefCell<BTreeMap<String, String>>,
    }

    impl SecretBackend for MemoryBackend {
        fn get(&self, name: &str) -> Result<Option<String>, String> {
            Ok(self.values.borrow().get(name).cloned())
        }

        fn set(&self, name: &str, value: &str) -> Result<(), String> {
            self.values
                .borrow_mut()
                .insert(name.to_string(), value.to_string());
            Ok(())
        }

        fn delete(&self, name: &str) -> Result<(), String> {
            self.values.borrow_mut().remove(name);
            Ok(())
        }
    }

    #[test]
    fn stores_replaces_lists_and_deletes_secret() {
        let backend = MemoryBackend::default();
        assert_eq!(save(&backend, "api_token", "first").unwrap(), ["API_TOKEN"]);
        assert_eq!(get(&backend, "API_TOKEN").unwrap(), "first");
        assert_eq!(
            save(&backend, "API_TOKEN", "second").unwrap(),
            ["API_TOKEN"]
        );
        assert_eq!(get(&backend, "api_token").unwrap(), "second");
        assert!(delete(&backend, "API_TOKEN").unwrap().is_empty());
        assert_eq!(
            get(&backend, "API_TOKEN"),
            Err(SecretError::NotFound("API_TOKEN".to_string()))
        );
    }

    #[test]
    fn rejects_invalid_names_and_empty_values() {
        let backend = MemoryBackend::default();
        assert_eq!(
            save(&backend, "../../token", "value"),
            Err(SecretError::InvalidName)
        );
        assert_eq!(save(&backend, "TOKEN", ""), Err(SecretError::EmptyValue));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn uses_windows_credential_manager() {
        const TEST_SECRET: &str = "REQVAULT_TEST_SECRET_DO_NOT_LEAK_123456";
        let workspace_id = uuid::Uuid::new_v4().to_string();
        let backend = KeyringBackend::new(&workspace_id).unwrap();

        let result = (|| {
            save(&backend, "REQVAULT_KEYRING_TEST", TEST_SECRET)?;
            let loaded = get(&backend, "REQVAULT_KEYRING_TEST")?;
            if loaded == TEST_SECRET {
                Ok(())
            } else {
                Err(SecretError::StorageUnavailable)
            }
        })();

        let _ = backend.delete("REQVAULT_KEYRING_TEST");
        let _ = backend.delete(INDEX_NAME);
        assert!(result.is_ok());
    }
}
