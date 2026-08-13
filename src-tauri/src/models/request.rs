use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::FORMAT_VERSION;

fn default_format_version() -> u32 {
    FORMAT_VERSION
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_timeout_ms() -> u64 {
    30_000
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyValue {
    pub name: String,
    pub value: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKeyHeader {
        name: String,
        value: String,
    },
    ApiKeyQuery {
        name: String,
        value: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BodyConfig {
    #[default]
    None,
    Json {
        value: String,
    },
    Raw {
        value: String,
        #[serde(default)]
        content_type: String,
    },
    FormUrlencoded {
        #[serde(default)]
        fields: Vec<KeyValue>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestFile {
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    pub name: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub query: Vec<KeyValue>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub body: BodyConfig,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "enabled_by_default")]
    pub follow_redirects: bool,
}

impl Default for RequestFile {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            name: "Новый запрос".to_string(),
            method: default_method(),
            url: String::new(),
            headers: BTreeMap::new(),
            query: Vec::new(),
            auth: AuthConfig::None,
            body: BodyConfig::None,
            timeout_ms: default_timeout_ms(),
            follow_redirects: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvironmentFile {
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    pub name: String,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
}

impl Default for EnvironmentFile {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            name: "local".to_string(),
            variables: BTreeMap::new(),
        }
    }
}
