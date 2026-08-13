use serde::{Deserialize, Serialize};

fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseAssertion {
    Status {
        expected: u16,
        #[serde(default = "enabled_by_default")]
        enabled: bool,
    },
    Header {
        name: String,
        operator: String,
        #[serde(default)]
        expected: String,
        #[serde(default = "enabled_by_default")]
        enabled: bool,
    },
    JsonPath {
        path: String,
        operator: String,
        #[serde(default)]
        expected: String,
        #[serde(default = "enabled_by_default")]
        enabled: bool,
    },
    BodyContains {
        expected: String,
        #[serde(default = "enabled_by_default")]
        enabled: bool,
    },
    ResponseTime {
        max_ms: u128,
        #[serde(default = "enabled_by_default")]
        enabled: bool,
    },
}

impl ResponseAssertion {
    pub fn enabled(&self) -> bool {
        match self {
            Self::Status { enabled, .. }
            | Self::Header { enabled, .. }
            | Self::JsonPath { enabled, .. }
            | Self::BodyContains { enabled, .. }
            | Self::ResponseTime { enabled, .. } => *enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssertionResult {
    pub passed: bool,
    pub label: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestRunResult {
    pub relative_path: String,
    pub request_name: String,
    pub method: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u128>,
    pub passed: bool,
    pub assertions: Vec<AssertionResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionRunReport {
    pub started_at_ms: u128,
    pub duration_ms: u128,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<RequestRunResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionRunOptions {
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default)]
    pub stop_on_failure: bool,
}
