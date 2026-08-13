use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cookie::{Cookie, Expiration};
use url::Url;
use uuid::Uuid;

use crate::models::CookieSummary;

#[derive(Default)]
pub struct SessionState {
    workspaces: Mutex<HashMap<String, CookieJar>>,
}

#[derive(Debug, Clone)]
pub struct CookieJar {
    inner: Arc<Mutex<Vec<StoredCookie>>>,
}

impl Default for CookieJar {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[derive(Debug, Clone)]
struct StoredCookie {
    id: String,
    name: String,
    value: String,
    domain: String,
    host_only: bool,
    path: String,
    secure: bool,
    http_only: bool,
    expires_at: Option<i64>,
}

impl SessionState {
    pub fn jar(&self, workspace_id: &str) -> Result<CookieJar, String> {
        let mut workspaces = self
            .workspaces
            .lock()
            .map_err(|_| "Cookie jar недоступен".to_string())?;
        Ok(workspaces
            .entry(workspace_id.to_string())
            .or_default()
            .clone())
    }

    pub fn list(&self, workspace_id: &str) -> Result<Vec<CookieSummary>, String> {
        self.jar(workspace_id)?.list()
    }

    pub fn delete(&self, workspace_id: &str, cookie_id: &str) -> Result<(), String> {
        self.jar(workspace_id)?.delete(cookie_id)
    }

    pub fn clear(&self, workspace_id: &str) -> Result<(), String> {
        self.jar(workspace_id)?.clear()
    }

    pub fn drop_workspace(&self, workspace_id: &str) {
        if let Ok(mut workspaces) = self.workspaces.lock() {
            workspaces.remove(workspace_id);
        }
    }
}

impl CookieJar {
    pub fn request_header(&self, url: &Url) -> Option<String> {
        let host = url.host_str()?.trim_end_matches('.').to_lowercase();
        let path = if url.path().is_empty() {
            "/"
        } else {
            url.path()
        };
        let secure = url.scheme() == "https";
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let mut cookies = self.inner.lock().ok()?;
        cookies.retain(|cookie| cookie.expires_at.is_none_or(|expiry| expiry > now));
        let values = cookies
            .iter()
            .filter(|cookie| {
                (!cookie.secure || secure)
                    && domain_matches(&host, &cookie.domain, cookie.host_only)
                    && path_matches(path, &cookie.path)
            })
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>();
        (!values.is_empty()).then(|| values.join("; "))
    }

    pub fn store(&self, url: &Url, header: &str) {
        let Ok(parsed) = Cookie::parse(header.to_string()) else {
            return;
        };
        let Some(host) = url
            .host_str()
            .map(|value| value.trim_end_matches('.').to_lowercase())
        else {
            return;
        };
        let domain = parsed
            .domain()
            .map(|value| {
                value
                    .trim_start_matches('.')
                    .trim_end_matches('.')
                    .to_lowercase()
            })
            .unwrap_or_else(|| host.clone());
        if !domain_matches(&host, &domain, false) {
            return;
        }
        let host_only = parsed.domain().is_none();
        let path = parsed
            .path()
            .map(str::to_string)
            .unwrap_or_else(|| default_path(url.path()));
        let expires_at = if let Some(max_age) = parsed.max_age() {
            Some(time::OffsetDateTime::now_utc().unix_timestamp() + max_age.whole_seconds())
        } else {
            match parsed.expires() {
                Some(Expiration::DateTime(value)) => Some(value.unix_timestamp()),
                _ => None,
            }
        };
        let mut cookies = match self.inner.lock() {
            Ok(value) => value,
            Err(_) => return,
        };
        cookies.retain(|cookie| {
            !(cookie.name == parsed.name() && cookie.domain == domain && cookie.path == path)
        });
        if parsed.value().is_empty()
            || expires_at
                .is_some_and(|value| value <= time::OffsetDateTime::now_utc().unix_timestamp())
        {
            return;
        }
        cookies.push(StoredCookie {
            id: Uuid::new_v4().to_string(),
            name: parsed.name().to_string(),
            value: parsed.value().to_string(),
            domain,
            host_only,
            path,
            secure: parsed.secure().unwrap_or(false),
            http_only: parsed.http_only().unwrap_or(false),
            expires_at,
        });
    }

    fn list(&self) -> Result<Vec<CookieSummary>, String> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let mut cookies = self
            .inner
            .lock()
            .map_err(|_| "Cookie jar недоступен".to_string())?;
        cookies.retain(|cookie| cookie.expires_at.is_none_or(|expiry| expiry > now));
        let mut result = cookies
            .iter()
            .map(|cookie| CookieSummary {
                id: cookie.id.clone(),
                name: cookie.name.clone(),
                domain: cookie.domain.clone(),
                path: cookie.path.clone(),
                secure: cookie.secure,
                http_only: cookie.http_only,
                expires_at: cookie.expires_at,
            })
            .collect::<Vec<_>>();
        result.sort_by(|left, right| {
            (&left.domain, &left.path, &left.name).cmp(&(&right.domain, &right.path, &right.name))
        });
        Ok(result)
    }

    fn delete(&self, cookie_id: &str) -> Result<(), String> {
        let mut cookies = self
            .inner
            .lock()
            .map_err(|_| "Cookie jar недоступен".to_string())?;
        cookies.retain(|cookie| cookie.id != cookie_id);
        Ok(())
    }

    fn clear(&self) -> Result<(), String> {
        self.inner
            .lock()
            .map_err(|_| "Cookie jar недоступен".to_string())?
            .clear();
        Ok(())
    }
}

fn domain_matches(host: &str, domain: &str, host_only: bool) -> bool {
    host == domain || (!host_only && host.ends_with(&format!(".{domain}")))
}

fn path_matches(request_path: &str, cookie_path: &str) -> bool {
    request_path == cookie_path
        || (request_path.starts_with(cookie_path)
            && (cookie_path.ends_with('/') || request_path[cookie_path.len()..].starts_with('/')))
}

fn default_path(request_path: &str) -> String {
    let path = if request_path.starts_with('/') {
        request_path
    } else {
        "/"
    };
    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(index) => path[..index].to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_matches_replaces_and_deletes_cookies() {
        let jar = CookieJar::default();
        let url = Url::parse("https://api.example.test/v1/users").unwrap();
        jar.store(&url, "session=one; Path=/v1; Secure; HttpOnly");
        assert_eq!(jar.request_header(&url).as_deref(), Some("session=one"));
        assert!(
            jar.request_header(&Url::parse("http://api.example.test/v1/users").unwrap())
                .is_none()
        );
        jar.store(&url, "session=two; Path=/v1; Secure; HttpOnly");
        assert_eq!(jar.request_header(&url).as_deref(), Some("session=two"));
        let cookie = jar.list().unwrap().remove(0);
        assert!(cookie.secure && cookie.http_only);
        jar.delete(&cookie.id).unwrap();
        assert!(jar.list().unwrap().is_empty());
    }

    #[test]
    fn isolates_workspace_jars() {
        let state = SessionState::default();
        let url = Url::parse("https://api.example.test/").unwrap();
        state.jar("first").unwrap().store(&url, "session=first");
        assert!(state.jar("second").unwrap().request_header(&url).is_none());
        state.drop_workspace("first");
        assert!(state.jar("first").unwrap().request_header(&url).is_none());
    }
}
