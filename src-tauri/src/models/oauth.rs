use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OAuthResult {
    pub access_token_secret: String,
    pub refresh_token_secret: Option<String>,
    pub expires_in: Option<u64>,
    pub scope: Option<String>,
}
