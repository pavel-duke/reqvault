use serde::{Deserialize, Serialize};

use super::{EnvironmentFile, FORMAT_VERSION, RequestFile};

fn default_format_version() -> u32 {
    FORMAT_VERSION
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionGuard {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "enabled_by_default")]
    pub require_https: bool,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub blocked_methods: Vec<String>,
    #[serde(default = "enabled_by_default")]
    pub block_secrets_in_url: bool,
    #[serde(default = "enabled_by_default")]
    pub block_private_networks: bool,
    #[serde(default = "enabled_by_default")]
    pub block_cross_origin_redirects: bool,
}

impl Default for ProductionGuard {
    fn default() -> Self {
        Self {
            enabled: false,
            require_https: true,
            allowed_hosts: Vec::new(),
            blocked_methods: vec!["DELETE".to_string()],
            block_secrets_in_url: true,
            block_private_networks: true,
            block_cross_origin_redirects: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceConfig {
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub production_guard: ProductionGuard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestSummary {
    pub relative_path: String,
    pub request: RequestFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvironmentSummary {
    pub relative_path: String,
    pub environment: EnvironmentFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    pub root_path: String,
    pub config: WorkspaceConfig,
    pub requests: Vec<RequestSummary>,
    pub environments: Vec<EnvironmentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestPathChange {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestBatchResult {
    pub workspace: WorkspaceSnapshot,
    pub changes: Vec<RequestPathChange>,
}
