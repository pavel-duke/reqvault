use serde::Serialize;

use super::WorkspaceSnapshot;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ImportResult {
    pub source: String,
    pub imported_requests: usize,
    pub imported_environments: usize,
    pub warnings: Vec<String>,
    pub workspace: WorkspaceSnapshot,
}
