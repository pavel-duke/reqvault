use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

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

    validate_target(&url, guard)
}

pub fn validate_redirect(from: &Url, target: &Url, guard: &ProductionGuard) -> Result<(), String> {
    if !guard.enabled {
        return Ok(());
    }
    validate_target(target, guard)?;
    if guard.block_cross_origin_redirects
        && !same_origin(from, target)
        && !is_explicitly_allowed(target, guard)
    {
        let host = normalized_host(target);
        return Err(format!(
            "Production Guard заблокировал междоменный редирект на {host}. Добавь этот хост в разрешённые, если переход ожидаем."
        ));
    }
    Ok(())
}

pub async fn validate_resolved_target(url: &Url, guard: &ProductionGuard) -> Result<(), String> {
    if !guard.enabled || !guard.block_private_networks || is_explicitly_allowed(url, guard) {
        return Ok(());
    }
    let host = normalized_host(url);
    if host.parse::<IpAddr>().is_ok() || host.is_empty() {
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(80);
    let addresses = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| "Production Guard не смог безопасно проверить адрес сервера".to_string())?;
    if addresses
        .into_iter()
        .any(|address| ip_risk(address.ip()).is_some())
    {
        return Err(format!(
            "Production Guard блокирует {host}: DNS указывает на локальный или служебный адрес. Добавь хост в разрешённые, если это ожидаемо."
        ));
    }
    Ok(())
}

pub fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme().eq_ignore_ascii_case(right.scheme())
        && normalized_host(left) == normalized_host(right)
        && left.port_or_known_default() == right.port_or_known_default()
}

pub fn network_risk(url: &Url) -> Option<&'static str> {
    let host = normalized_host(url);
    if host == "localhost" || host.ends_with(".localhost") {
        return Some("локальный hostname");
    }
    if is_metadata_hostname(&host) {
        return Some("служебный адрес облачной инфраструктуры");
    }
    host.parse::<IpAddr>().ok().and_then(ip_risk)
}

fn validate_target(url: &Url, guard: &ProductionGuard) -> Result<(), String> {
    if guard.require_https && url.scheme() != "https" {
        return Err("Production Guard разрешает только HTTPS".to_string());
    }
    let host = normalized_host(url);
    if !guard.allowed_hosts.is_empty() && !is_explicitly_allowed(url, guard) {
        return Err(format!(
            "Production Guard блокирует хост {host}. Добавь его в список разрешённых."
        ));
    }
    if guard.block_private_networks
        && !is_explicitly_allowed(url, guard)
        && let Some(risk) = network_risk(url)
    {
        return Err(format!(
            "Production Guard блокирует {host}: {risk}. Добавь хост в разрешённые, если это локальный API."
        ));
    }
    Ok(())
}

fn is_explicitly_allowed(url: &Url, guard: &ProductionGuard) -> bool {
    let host = normalized_host(url);
    guard
        .allowed_hosts
        .iter()
        .any(|pattern| host_matches(&host, pattern))
}

fn normalized_host(url: &Url) -> String {
    url.host_str()
        .unwrap_or_default()
        .trim_end_matches('.')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_lowercase()
}

fn host_matches(host: &str, pattern: &str) -> bool {
    let pattern = pattern.trim().trim_end_matches('.').to_lowercase();
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host != suffix && host.ends_with(&format!(".{suffix}"))
    } else {
        host == pattern
    }
}

fn is_metadata_hostname(host: &str) -> bool {
    matches!(
        host,
        "metadata.google.internal"
            | "metadata.google"
            | "instance-data"
            | "metadata.azure.internal"
    )
}

fn ip_risk(ip: IpAddr) -> Option<&'static str> {
    match ip {
        IpAddr::V4(ip) => ipv4_risk(ip),
        IpAddr::V6(ip) => ipv6_risk(ip),
    }
}

fn ipv4_risk(ip: Ipv4Addr) -> Option<&'static str> {
    let octets = ip.octets();
    if ip == Ipv4Addr::new(169, 254, 169, 254)
        || ip == Ipv4Addr::new(169, 254, 170, 2)
        || ip == Ipv4Addr::new(100, 100, 100, 200)
    {
        Some("служебный адрес cloud metadata")
    } else if ip.is_loopback() {
        Some("loopback-адрес")
    } else if ip.is_private() || octets[0] == 100 && (64..=127).contains(&octets[1]) {
        Some("приватный адрес")
    } else if ip.is_link_local() {
        Some("link-local адрес")
    } else if ip.is_unspecified() || ip.is_broadcast() {
        Some("служебный адрес")
    } else {
        None
    }
}

fn ipv6_risk(ip: Ipv6Addr) -> Option<&'static str> {
    let segments = ip.segments();
    if ip.is_loopback() {
        Some("loopback-адрес")
    } else if ip.is_unspecified() {
        Some("служебный адрес")
    } else if segments[0] & 0xfe00 == 0xfc00 {
        Some("приватный IPv6-адрес")
    } else if segments[0] & 0xffc0 == 0xfe80 {
        Some("link-local IPv6-адрес")
    } else {
        ip.to_ipv4_mapped().and_then(ipv4_risk)
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

    #[test]
    fn blocks_local_private_link_local_and_metadata_targets() {
        let mut guard = enabled_guard();
        guard.allowed_hosts.clear();
        for value in [
            "https://localhost/api",
            "https://127.0.0.1/api",
            "https://10.0.0.2/api",
            "https://169.254.169.254/latest/meta-data",
            "https://[::1]/api",
            "https://[fe80::1]/api",
            "https://metadata.google.internal/computeMetadata/v1/",
        ] {
            let request = RequestFile {
                url: value.to_string(),
                ..RequestFile::default()
            };
            assert!(validate(&request, None, &guard).is_err(), "{value}");
        }
    }

    #[test]
    fn allows_local_development_when_guard_is_off_or_host_is_explicit() {
        let request = RequestFile {
            url: "http://127.0.0.1:4318/api".to_string(),
            ..RequestFile::default()
        };
        assert!(validate(&request, None, &ProductionGuard::default()).is_ok());

        let guard = ProductionGuard {
            enabled: true,
            require_https: false,
            allowed_hosts: vec!["127.0.0.1".to_string()],
            ..ProductionGuard::default()
        };
        assert!(validate(&request, None, &guard).is_ok());
    }

    #[test]
    fn compares_scheme_host_and_effective_port_for_redirects() {
        let base = Url::parse("https://api.example.test/users").unwrap();
        assert!(same_origin(
            &base,
            &Url::parse("https://api.example.test:443/next").unwrap()
        ));
        assert!(!same_origin(
            &base,
            &Url::parse("http://api.example.test/next").unwrap()
        ));
        assert!(!same_origin(
            &Url::parse("http://api.example.test/start").unwrap(),
            &base
        ));
        assert!(!same_origin(
            &base,
            &Url::parse("https://other.example.test/next").unwrap()
        ));
        assert!(!same_origin(
            &base,
            &Url::parse("https://api.example.test:8443/next").unwrap()
        ));
    }

    #[test]
    fn redirect_requires_explicit_target_in_production_guard() {
        let from = Url::parse("https://api.example.test/start").unwrap();
        let target = Url::parse("https://login.example.test/next").unwrap();
        let mut guard = enabled_guard();
        assert!(validate_redirect(&from, &target, &guard).is_err());
        guard.allowed_hosts.push("login.example.test".to_string());
        assert!(validate_redirect(&from, &target, &guard).is_ok());
    }
}
