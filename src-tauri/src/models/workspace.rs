use serde::{Deserialize, Serialize};

use super::{EnvironmentFile, FORMAT_VERSION, RequestFile};

fn default_format_version() -> u32 {
    FORMAT_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceConfig {
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    pub id: String,
    pub name: String,
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
