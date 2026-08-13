use std::{collections::BTreeMap, sync::OnceLock};

use regex::{Captures, Regex};
use thiserror::Error;

fn variable_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"\{\{([A-Za-z][A-Za-z0-9_.-]*)\}\}").unwrap())
}

fn secret_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"\{\{secret:([A-Za-z][A-Za-z0-9_.-]*)\}\}").unwrap())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResolveError {
    #[error("Переменная {0} не задана в выбранном окружении")]
    MissingVariable(String),
    #[error("Секрет {0} не найден в системном хранилище")]
    MissingSecret(String),
    #[error("Системное хранилище секретов недоступно")]
    SecretStorage,
}

pub fn resolve_variables(
    input: &str,
    variables: &BTreeMap<String, String>,
) -> Result<String, ResolveError> {
    let mut missing = None;
    let result = variable_pattern().replace_all(input, |captures: &Captures<'_>| {
        let name = &captures[1];
        match variables.get(name) {
            Some(value) => value.clone(),
            None => {
                missing = Some(name.to_string());
                captures[0].to_string()
            }
        }
    });
    match missing {
        Some(name) => Err(ResolveError::MissingVariable(name)),
        None => Ok(result.into_owned()),
    }
}

pub fn resolve_secrets<F>(
    input: &str,
    resolve: &mut F,
    used_values: &mut Vec<String>,
) -> Result<String, ResolveError>
where
    F: FnMut(&str) -> Result<String, ResolveError>,
{
    let mut failure = None;
    let result = secret_pattern().replace_all(input, |captures: &Captures<'_>| {
        let name = &captures[1];
        match resolve(name) {
            Ok(value) => {
                if !used_values.contains(&value) {
                    used_values.push(value.clone());
                }
                value
            }
            Err(error) => {
                failure = Some(error);
                captures[0].to_string()
            }
        }
    });
    match failure {
        Some(error) => Err(error),
        None => Ok(result.into_owned()),
    }
}

#[cfg(test)]
fn secret_names(input: &str) -> Vec<String> {
    secret_pattern()
        .captures_iter(input)
        .map(|captures| captures[1].to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "REQVAULT_TEST_SECRET_DO_NOT_LEAK_123456";

    #[test]
    fn resolves_environment_variables() {
        let variables = BTreeMap::from([
            (
                "BASE_URL".to_string(),
                "https://api.example.test".to_string(),
            ),
            ("USER_ID".to_string(), "42".to_string()),
        ]);
        let resolved = resolve_variables("{{BASE_URL}}/users/{{USER_ID}}", &variables).unwrap();
        assert_eq!(resolved, "https://api.example.test/users/42");
    }

    #[test]
    fn reports_missing_variable() {
        let error = resolve_variables("{{BASE_URL}}/users", &BTreeMap::new()).unwrap_err();
        assert_eq!(error, ResolveError::MissingVariable("BASE_URL".to_string()));
    }

    #[test]
    fn parses_and_resolves_secret_references() {
        let input = "Bearer {{secret:API_TOKEN}} and {{secret:SECOND}}";
        assert_eq!(secret_names(input), vec!["API_TOKEN", "SECOND"]);
        let mut values = Vec::new();
        let resolved = resolve_secrets(
            input,
            &mut |name| {
                if name == "API_TOKEN" || name == "SECOND" {
                    Ok(TEST_SECRET.to_string())
                } else {
                    Err(ResolveError::MissingSecret(name.to_string()))
                }
            },
            &mut values,
        )
        .unwrap();
        assert!(resolved.contains(TEST_SECRET));
        assert_eq!(values, vec![TEST_SECRET]);
    }
}
