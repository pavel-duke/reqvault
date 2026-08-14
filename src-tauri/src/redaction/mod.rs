use std::sync::OnceLock;

use regex::{Captures, Regex};

const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "api-key",
    "x-auth-token",
    "x-access-token",
    "x-amz-security-token",
];

fn bearer_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?i)\b(Bearer|Basic)\s+[^\s,;]+").unwrap())
}

pub fn is_sensitive_header(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    SENSITIVE_HEADERS
        .iter()
        .any(|sensitive| normalized == *sensitive)
        || normalized.ends_with("-token")
        || normalized.contains("api-key")
        || normalized.contains("apikey")
        || normalized.contains("credential")
        || normalized.contains("secret")
}

pub fn redact_header(name: &str, value: &str, secrets: &[String]) -> String {
    if is_sensitive_header(name) {
        if let Some((scheme, _)) = value.split_once(' ')
            && (scheme.eq_ignore_ascii_case("bearer") || scheme.eq_ignore_ascii_case("basic"))
        {
            return format!("{scheme} ********");
        }
        return "********".to_string();
    }
    redact_text(value, secrets)
}

pub fn redact_text(input: &str, secrets: &[String]) -> String {
    let mut redacted = input.to_string();
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        redacted = redacted.replace(secret, "***REDACTED***");
    }
    bearer_pattern()
        .replace_all(&redacted, |captures: &Captures<'_>| {
            format!("{} ***REDACTED***", &captures[1])
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "REQVAULT_TEST_SECRET_DO_NOT_LEAK_123456";

    #[test]
    fn redacts_sensitive_headers() {
        let value = redact_header(
            "Authorization",
            &format!("Bearer {TEST_SECRET}"),
            &[TEST_SECRET.to_string()],
        );
        assert_eq!(value, "Bearer ********");
        assert!(!value.contains(TEST_SECRET));
        assert_eq!(redact_header("Cookie", "session=abc", &[]), "********");
        assert!(is_sensitive_header("X-Customer-Access-Token"));
        assert!(is_sensitive_header("X-Service-Credential"));
    }

    #[test]
    fn redacts_known_values_and_bearer_tokens() {
        let result = redact_text(
            &format!("value={TEST_SECRET}; Authorization: Bearer another-token"),
            &[TEST_SECRET.to_string()],
        );
        assert!(!result.contains(TEST_SECRET));
        assert!(!result.contains("another-token"));
    }
}
