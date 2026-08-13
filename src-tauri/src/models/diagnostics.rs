use serde::{Deserialize, Serialize};

use super::WorkspaceSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticIssue {
    pub severity: String,
    pub code: String,
    pub path: String,
    pub message: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationPlan {
    pub required: bool,
    pub current_version: u32,
    pub target_version: u32,
    pub files: Vec<String>,
    pub changes: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationResult {
    pub backup_id: Option<String>,
    pub workspace: WorkspaceSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceDiagnostics {
    pub checked_at_ms: u64,
    pub fingerprint: String,
    pub files: usize,
    pub requests: usize,
    pub environments: usize,
    pub errors: usize,
    pub warnings: usize,
    pub issues: Vec<DiagnosticIssue>,
    pub migration: MigrationPlan,
}
