use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::EnvironmentFile;

#[derive(Debug, Clone, Deserialize)]
pub struct StreamConnectConfig {
    pub protocol: String,
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub workspace_id: String,
    pub workspace_path: String,
    #[serde(default)]
    pub environment: Option<EnvironmentFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamEvent {
    pub session_id: String,
    pub kind: String,
    pub timestamp_ms: u128,
    pub data: String,
}
