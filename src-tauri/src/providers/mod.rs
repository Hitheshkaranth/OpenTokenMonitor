use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::usage::models::{CostEntry, ProviderId, ProviderStatus, UsageSnapshot};

pub mod antigravity;
pub mod auth;
pub mod claude;
pub mod codex;
pub mod registry;

#[derive(Debug, Clone)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub brand_color: &'static str,
}

#[derive(Clone, Default)]
pub struct FetchContext {
    pub api_keys: HashMap<ProviderId, String>,
    pub allow_cookie_strategy: bool,
    pub allow_cli_strategy: bool,
}

// Hand-written so an accidental `{:?}` of the context never logs API keys.
impl std::fmt::Debug for FetchContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchContext")
            .field("api_keys", &self.api_keys.keys().collect::<Vec<_>>())
            .field("allow_cookie_strategy", &self.allow_cookie_strategy)
            .field("allow_cli_strategy", &self.allow_cli_strategy)
            .finish()
    }
}

impl FetchContext {
    // Providers pull auth and feature switches from a shared context so command
    // handlers do not need to know anything about provider-specific auth shapes.
    pub fn api_key_for(&self, provider: ProviderId) -> Option<&str> {
        self.api_keys
            .get(&provider)
            .map(String::as_str)
            .filter(|v| !v.trim().is_empty())
    }
}

// `async_trait` expands each method with a `#[must_use]` on a type that is
// already must-use; newer clippy flags that inside the macro output.
#[allow(clippy::double_must_use)]
#[async_trait]
// Every provider implementation exposes the same small contract: fetch current
// usage, fetch cost history, and report health. The registry and aggregator rely
// on this trait to keep provider-specific branching out of the command layer.
pub trait UsageProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn descriptor(&self) -> &ProviderDescriptor;
    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<UsageSnapshot, String>;
    async fn fetch_cost_history(&self, days: u32) -> Result<Vec<CostEntry>, String>;
    async fn check_status(&self) -> ProviderStatus;
    fn compute_auth_state(&self, ctx: &FetchContext) -> auth::AuthState;
}

pub type DynProvider = Arc<dyn UsageProvider>;

/// Upper bound on how much of an upstream error body we buffer.
const ERROR_BODY_READ_CAP: usize = 64 * 1024;
/// How much of that body we keep for logs / error strings.
const ERROR_BODY_SNIPPET_CHARS: usize = 200;

/// Read a non-2xx response body through a size cap and return a short,
/// sanitised snippet safe to log or surface. Large HTML challenge pages are
/// never buffered whole, and any `*_token` values are redacted.
pub(crate) async fn error_body_snippet(mut res: reqwest::Response) -> String {
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < ERROR_BODY_READ_CAP {
        match res.chunk().await {
            Ok(Some(chunk)) => buf.extend_from_slice(&chunk),
            _ => break,
        }
    }
    buf.truncate(ERROR_BODY_READ_CAP);
    sanitize_error_body(&String::from_utf8_lossy(&buf))
}

fn sanitize_error_body(body: &str) -> String {
    let collapsed: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out: String = collapsed.chars().take(ERROR_BODY_SNIPPET_CHARS).collect();
    if collapsed.chars().count() > ERROR_BODY_SNIPPET_CHARS {
        out.push('…');
    }
    redact_token_values(&out)
}

/// Replace the value of any JSON-ish `"<x>_token": "<value>"` pair with `[redacted]`.
fn redact_token_values(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find("_token\"") {
        let key_end = pos + "_token\"".len();
        out.push_str(&rest[..key_end]);
        let after = &rest[key_end..];
        let trimmed = after.trim_start_matches([' ', ':']);
        if trimmed.len() < after.len() && trimmed.starts_with('"') {
            let value = &trimmed[1..];
            out.push_str(&after[..after.len() - trimmed.len()]);
            out.push_str("\"[redacted]\"");
            // Skip to the closing quote of the value (or the end if truncated).
            rest = match value.find('"') {
                Some(end) => &value[end + 1..],
                None => "",
            };
        } else {
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_truncates_and_redacts() {
        let body = r#"{"error":"bad","refresh_token": "abc.def", "id_token":"xyz"}"#;
        let out = sanitize_error_body(body);
        assert!(!out.contains("abc.def"));
        assert!(!out.contains("xyz"));
        assert!(out.contains("[redacted]"));

        let long = "x".repeat(5000);
        let out = sanitize_error_body(&long);
        assert_eq!(out.chars().count(), ERROR_BODY_SNIPPET_CHARS + 1);
    }
}
