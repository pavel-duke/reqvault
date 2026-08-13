use serde::{Deserialize, Serialize};

use super::ResponseHeader;

fn default_history_limit() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistorySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_history_limit")]
    pub max_entries: u32,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            max_entries: default_history_limit(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistorySummary {
    pub id: String,
    pub created_at_ms: u64,
    pub request_name: String,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub duration_ms: u128,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    #[serde(flatten)]
    pub summary: HistorySummary,
    pub status_text: String,
    pub headers: Vec<ResponseHeader>,
    pub body: String,
    pub is_json: bool,
}
