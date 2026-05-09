use serde::Serialize;

use crate::usage::models::ProviderId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    Oauth,
    ApiKey,
    Cookie,
    Cli,
    None,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthState {
    pub provider: ProviderId,
    pub kind: AuthKind,
    pub source_path: String,
    pub expires_at_unix_secs: Option<u64>,
    pub last_refresh_iso: Option<String>,
    pub has_refresh_token: bool,
    pub last_error: Option<String>,
}

impl AuthState {
    pub fn none(provider: ProviderId) -> Self {
        Self {
            provider,
            kind: AuthKind::None,
            source_path: "local logs".to_string(),
            expires_at_unix_secs: None,
            last_refresh_iso: None,
            has_refresh_token: false,
            last_error: None,
        }
    }

    pub fn is_expired_with_skew(&self, skew_secs: u64) -> bool {
        let Some(exp) = self.expires_at_unix_secs else {
            return false;
        };
        let now = chrono::Utc::now().timestamp();
        if now < 0 {
            return true;
        }
        exp <= (now as u64).saturating_add(skew_secs)
    }
}
