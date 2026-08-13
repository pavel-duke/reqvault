use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResponseHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HttpResponse {
    pub request_id: String,
    pub status: u16,
    pub status_text: String,
    pub duration_ms: u128,
    pub size_bytes: usize,
    pub headers: Vec<ResponseHeader>,
    pub body: String,
    pub is_json: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HttpError {
    pub message: String,
    pub details: Option<String>,
    pub error_type: String,
}
