use url::Url;

use crate::{
    models::{EnvironmentFile, ProductionGuard, RequestFile},
    variables::resolve_variables,
};

pub fn validate(
    request: &RequestFile,
    environment: Option<&EnvironmentFile>,
    guard: &ProductionGuard,
) -> Result<(), String> {
    if !guard.enabled {
        return Ok(());
    }

    let method = request.method.trim().to_uppercase();
    if guard
        .blocked_methods
        .iter()
        .any(|blocked| blocked.eq_ignore_ascii_case(&method))
    {
        return Err(format!(
            "Production Guard блокирует метод {method} в этом workspace"
        ));
    }

    if guard.block_secrets_in_url && request.url.contains("{{secret:") {
        return Err("Production Guard запрещает ссылки на секреты в URL".to_string());
    }

    let variables = environment
        .map(|item| &item.variables)
        .cloned()
        .unwrap_or_default();
    let resolved = resolve_variables(&request.url, &variables)
        .map_err(|error| format!("Production Guard не смог проверить URL: {error}"))?;
    let url = Url::parse(&resolved)
        .map_err(|_| "Production Guard не смог разобрать URL запроса".to_string())?;

    if guard.require_https && url.scheme() != "https" {
        return Err("Production Guard разрешает только HTTPS".to_string());
    }

    if !guard.allowed_hosts.is_empty() {
        let host = url
            .host_str()
            .unwrap_or_default()
            .trim_end_matches('.')
            .to_lowercase();
        let allowed = guard
            .allowed_hosts
            .iter()
            .any(|pattern| host_matches(&host, pattern));
        if !allowed {
            return Err(format!(
                "Production Guard блокирует хост {host}. Добавь его в список разрешённых."
            ));
        }
    }

    Ok(())
}

fn host_matches(host: &str, pattern: &str) -> bool {
    let pattern = pattern.trim().trim_end_matches('.').to_lowercase();
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host != suffix && host.ends_with(&format!(".{suffix}"))
    } else {
        host == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_guard() -> ProductionGuard {
        ProductionGuard {
            enabled: true,
            allowed_hosts: vec!["api.example.test".to_string(), "*.service.test".to_string()],
            ..ProductionGuard::default()
        }
    }

    #[test]
    fn blocks_unsafe_scheme_host_and_method() {
        let guard = enabled_guard();
        let mut request = RequestFile {
            url: "http://api.example.test/users".to_string(),
            ..RequestFile::default()
        };
        assert!(validate(&request, None, &guard).is_err());

        request.url = "https://unknown.test/users".to_string();
        assert!(validate(&request, None, &guard).is_err());

        request.url = "https://api.example.test/users".to_string();
        request.method = "DELETE".to_string();
        assert!(validate(&request, None, &guard).is_err());
    }

    #[test]
    fn accepts_exact_and_wildcard_hosts() {
        let guard = enabled_guard();
        for url in [
            "https://api.example.test/users",
            "https://v2.service.test/users",
        ] {
            let request = RequestFile {
                url: url.to_string(),
                ..RequestFile::default()
            };
            assert!(validate(&request, None, &guard).is_ok());
        }
    }
}
