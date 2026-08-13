use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpResponse {
    pub request_id: String,
    pub status: u16,
    pub status_text: String,
    pub duration_ms: u128,
    pub size_bytes: usize,
    pub headers: Vec<ResponseHeader>,
    pub body: String,
    pub is_json: bool,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default = "default_body_kind")]
    pub body_kind: String,
    #[serde(default)]
    pub truncated: bool,
}

fn default_content_type() -> String {
    "text/plain".to_string()
}

fn default_body_kind() -> String {
    "text".to_string()
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HttpError {
    pub message: String,
    pub details: Option<String>,
    pub error_type: String,
}
