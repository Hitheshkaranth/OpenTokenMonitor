mod bearer_fetcher;
mod cookie_fetcher;
mod descriptor;
mod log_parser;
mod oauth_refresh;
mod rpc_fetcher;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use tracing::{debug, info, warn};

use crate::providers::auth::{AuthKind, AuthState};
use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataProvenance, DataSource, ProviderHealth, ProviderId, ProviderStatus,
    UsageSnapshot, UsageUnit, UsageWindow, WindowType,
};
use crate::usage_scanners::read_codex_auth_bridge;

pub struct CodexProvider {
    descriptor: ProviderDescriptor,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor::descriptor(),
        }
    }

    fn auth_message(state: &AuthState) -> String {
        match state.kind {
            AuthKind::ApiKey => format!("Codex API key auth loaded from {}", state.source_path),
            AuthKind::Oauth => {
                if state.is_expired_with_skew(60) {
                    if state.has_refresh_token {
                        format!(
                            "Codex OAuth loaded from {} (token expired, refresh available)",
                            state.source_path
                        )
                    } else {
                        format!(
                            "Codex OAuth loaded from {} (token expired, no refresh token)",
                            state.source_path
                        )
                    }
                } else {
                    format!("Codex OAuth loaded from {}", state.source_path)
                }
            }
            AuthKind::Cookie => format!("Codex cookie auth loaded from {}", state.source_path),
            AuthKind::Cli => format!("Codex CLI auth loaded from {}", state.source_path),
            AuthKind::None => "Waiting for ~/.codex data".to_string(),
        }
    }
}

#[async_trait]
impl UsageProvider for CodexProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Codex
    }

    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<UsageSnapshot, String> {
        if ctx.allow_cookie_strategy {
            if let Some(cookie) = ctx
                .api_key_for(ProviderId::Codex)
                .filter(|v| v.contains("session"))
            {
                if let Ok(cookie_snapshot) = cookie_fetcher::fetch_usage(cookie).await {
                    return Ok(UsageSnapshot {
                        provider: ProviderId::Codex,
                        windows: vec![
                            UsageWindow::exact(
                                WindowType::Session,
                                cookie_snapshot.session_used,
                                cookie_snapshot.session_limit,
                                cookie_snapshot.resets_at,
                                UsageUnit::Unknown,
                            ),
                            UsageWindow::exact(
                                WindowType::Weekly,
                                cookie_snapshot.weekly_used,
                                cookie_snapshot.weekly_limit,
                                cookie_snapshot.resets_at,
                                UsageUnit::Unknown,
                            ),
                        ],
                        credits: None,
                        plan: None,
                        fetched_at: Utc::now(),
                        source: DataSource::Cookie,
                        provenance: DataProvenance::Internal,
                        stale: false,
                    });
                }
            }
        }

        // Try Bearer token from ~/.codex/auth.json
        let auth = read_codex_auth_bridge();
        let auth_state = self.compute_auth_state(ctx);
        debug!(
            "[codex] auth token len={}, source={}",
            auth.access_token.len(),
            auth.source_path
        );
        let mut local_log_reason: Option<String> = None;
        let mut bearer_token = auth.access_token.clone();

        if !bearer_token.is_empty()
            && auth_state.kind == AuthKind::Oauth
            && auth_state.is_expired_with_skew(60)
        {
            if let Some(refresh_token) = auth.refresh_token.as_deref() {
                match oauth_refresh::refresh_access_token(refresh_token).await {
                    Ok(refreshed) => {
                        if let Some(expires_in) = refreshed.expires_in_secs {
                            info!("[codex] token refresh OK, expires_in={expires_in}s");
                        }
                        if let Err(e) = oauth_refresh::persist_refreshed_tokens(&refreshed) {
                            warn!("[codex] token refresh persist FAILED: {e}");
                        }
                        bearer_token = refreshed.access_token;
                    }
                    Err(e) => {
                        let reason = format!("Codex token expired and refresh failed: {e}");
                        warn!("[codex] {reason}");
                        local_log_reason = Some(reason);
                    }
                }
            } else {
                local_log_reason =
                    Some("Codex token expired and no refresh token was available".to_string());
            }
        }

        if !bearer_token.is_empty() {
            match bearer_fetcher::fetch_usage(&bearer_token).await {
                Ok(bearer) => {
                    info!(
                        "[codex] bearer OK: session={}/{} weekly={}/{}",
                        bearer.session_used,
                        bearer.session_limit,
                        bearer.weekly_used,
                        bearer.weekly_limit
                    );
                    return Ok(UsageSnapshot {
                        provider: ProviderId::Codex,
                        windows: vec![
                            UsageWindow::percent(
                                WindowType::Session,
                                bearer.session_used as f64,
                                bearer.resets_at,
                                "ChatGPT Codex bearer usage exposes percent of the current session window.",
                            ),
                            UsageWindow::percent(
                                WindowType::Weekly,
                                bearer.weekly_used as f64,
                                bearer.weekly_resets_at,
                                "ChatGPT Codex bearer usage exposes percent of the current weekly window.",
                            ),
                        ],
                        credits: None,
                        plan: None,
                        fetched_at: Utc::now(),
                        source: DataSource::Oauth,
                        provenance: DataProvenance::Internal,
                        stale: false,
                    });
                }
                Err(e) => {
                    warn!("[codex] bearer FAILED: {e}");
                }
            }
        }

        if ctx.allow_cli_strategy {
            if let Ok(cli) = rpc_fetcher::fetch_usage() {
                return Ok(UsageSnapshot {
                    provider: ProviderId::Codex,
                    windows: vec![
                        UsageWindow::exact(
                            WindowType::Session,
                            cli.session_used,
                            cli.session_limit,
                            cli.resets_at,
                            UsageUnit::Unknown,
                        ),
                        UsageWindow::exact(
                            WindowType::Weekly,
                            cli.weekly_used,
                            cli.weekly_limit,
                            cli.resets_at,
                            UsageUnit::Unknown,
                        ),
                    ],
                    credits: None,
                    plan: None,
                    fetched_at: Utc::now(),
                    source: DataSource::Cli,
                    provenance: DataProvenance::Official,
                    stale: false,
                });
            }
        }

        let (session, weekly) = log_parser::usage_windows();
        // Local log tokens are raw counts; approximate Pro plan limits.
        // Real limits come from bearer API when available.
        let session_limit = 500_000;
        let weekly_limit = 5_000_000;
        let local_log_note = if let Some(reason) = local_log_reason {
            format!(
                "Estimated from local Codex session logs until a live usage window is available. {reason}"
            )
        } else {
            "Estimated from local Codex session logs until a live usage window is available."
                .to_string()
        };

        Ok(UsageSnapshot {
            provider: ProviderId::Codex,
            windows: vec![
                UsageWindow::approximate(
                    WindowType::Session,
                    session,
                    session_limit,
                    Some(Utc::now() + Duration::hours(5)),
                    UsageUnit::Tokens,
                    local_log_note,
                ),
                UsageWindow::approximate(
                    WindowType::Weekly,
                    weekly,
                    weekly_limit,
                    Some(Utc::now() + Duration::days(7)),
                    UsageUnit::Tokens,
                    "Estimated from rolling seven-day Codex log totals, not an official quota endpoint.",
                ),
            ],
            credits: None,
            plan: None,
            fetched_at: Utc::now(),
            source: DataSource::LocalLog,
            provenance: DataProvenance::DerivedLocal,
            stale: false,
        })
    }

    async fn fetch_cost_history(&self, days: u32) -> Result<Vec<CostEntry>, String> {
        Ok(log_parser::cost_history(days))
    }

    async fn check_status(&self) -> ProviderStatus {
        let auth = self.compute_auth_state(&FetchContext::default());
        let has_home = dirs::home_dir()
            .map(|h| h.join(".codex").exists())
            .unwrap_or(false);
        let message = if auth.kind != AuthKind::None {
            Self::auth_message(&auth)
        } else if has_home {
            "Codex local data detected".to_string()
        } else {
            "Waiting for ~/.codex data".to_string()
        };
        ProviderStatus {
            provider: ProviderId::Codex,
            health: if has_home {
                ProviderHealth::Active
            } else {
                ProviderHealth::Waiting
            },
            message,
            checked_at: Utc::now(),
        }
    }

    fn compute_auth_state(&self, _ctx: &FetchContext) -> AuthState {
        let auth = read_codex_auth_bridge();
        let auth_mode = auth.auth_mode.clone().unwrap_or_default();
        if auth_mode == "cookie" {
            return AuthState {
                provider: ProviderId::Codex,
                kind: AuthKind::Cookie,
                source_path: auth.source_path,
                expires_at_unix_secs: None,
                last_refresh_iso: auth.last_refresh,
                has_refresh_token: false,
                last_error: None,
            };
        }
        if auth_mode == "cli" {
            return AuthState {
                provider: ProviderId::Codex,
                kind: AuthKind::Cli,
                source_path: auth.source_path,
                expires_at_unix_secs: None,
                last_refresh_iso: auth.last_refresh,
                has_refresh_token: false,
                last_error: None,
            };
        }
        let has_api_key = auth
            .openai_api_key
            .as_deref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        if auth_mode == "apikey" || has_api_key {
            return AuthState {
                provider: ProviderId::Codex,
                kind: AuthKind::ApiKey,
                source_path: auth.source_path,
                expires_at_unix_secs: None,
                last_refresh_iso: auth.last_refresh,
                has_refresh_token: false,
                last_error: None,
            };
        }

        if !auth.access_token.trim().is_empty() {
            let has_refresh_token = auth
                .refresh_token
                .as_deref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            return AuthState {
                provider: ProviderId::Codex,
                kind: AuthKind::Oauth,
                source_path: auth.source_path,
                expires_at_unix_secs: oauth_refresh::jwt_expires_at_unix_secs(&auth.access_token),
                last_refresh_iso: auth.last_refresh,
                has_refresh_token,
                last_error: None,
            };
        }

        AuthState::none(ProviderId::Codex)
    }
}
