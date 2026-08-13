use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SecurityReport {
    pub https: bool,
    pub host: String,
    pub secrets: usize,
    pub in_headers: usize,
    pub in_query: usize,
    pub warnings: Vec<String>,
}
