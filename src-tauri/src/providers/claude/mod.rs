mod descriptor;
mod keychain;
mod log_parser;
mod oauth_fetcher;

use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::providers::auth::{AuthKind, AuthState};
use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, CreditsInfo, DataProvenance, DataSource, ProviderHealth, ProviderId, ProviderStatus,
    UsageSnapshot, UsageUnit, UsageWindow, WindowType,
};
use tracing::{info, warn};

const BACKOFF_STEPS_SECS: [u64; 4] = [30, 60, 120, 300];
const BACKOFF_MAX_SECS: u64 = 600;

#[derive(Debug, Clone, Default)]
struct BackoffState {
    last_attempt_at: Option<Instant>,
    consecutive_failures: u32,
    cached_success_snapshot: Option<UsageSnapshot>,
}

impl BackoffState {
    fn backoff_secs(&self) -> u64 {
        if self.consecutive_failures == 0 {
            return 0;
        }
        let idx = (self.consecutive_failures as usize).saturating_sub(1);
        let base = BACKOFF_STEPS_SECS
            .get(idx)
            .copied()
            .unwrap_or(BACKOFF_MAX_SECS);
        base.min(BACKOFF_MAX_SECS)
    }
}

pub struct ClaudeProvider {
    descriptor: ProviderDescriptor,
    backoff: Mutex<BackoffState>,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor::descriptor(),
            backoff: Mutex::new(BackoffState::default()),
        }
    }
}

impl ClaudeProvider {
    fn local_log_snapshot(&self) -> Result<UsageSnapshot, String> {
        let (day_tokens, week_tokens, _) = log_parser::usage_windows();
        let now = Utc::now();
        // Local log tokens are raw counts; approximate Pro plan limits.
        // Real limits come from OAuth API when available.
        let five_hour_limit = 500_000;
        let seven_day_limit = 5_000_000;
        Ok(UsageSnapshot {
            provider: ProviderId::Claude,
            windows: vec![
                UsageWindow::approximate(
                    WindowType::FiveHour,
                    day_tokens,
                    five_hour_limit,
                    Some(now + Duration::hours(5)),
                    UsageUnit::Tokens,
                    "Estimated from Claude local logs until OAuth rolling-window data is available.",
                ),
                UsageWindow::approximate(
                    WindowType::SevenDay,
                    week_tokens,
                    seven_day_limit,
                    Some(now + Duration::days(7)),
                    UsageUnit::Tokens,
                    "Estimated from Claude seven-day log totals, not the provider's live subscription counter.",
                ),
            ],
            credits: None,
            plan: None,
            fetched_at: now,
            source: DataSource::LocalLog,
            provenance: DataProvenance::DerivedLocal,
            stale: false,
        })
    }

    fn auth_message(state: &AuthState) -> String {
        if state.kind == AuthKind::None {
            return "Waiting for Claude credentials/session files".to_string();
        }
        if state.is_expired_with_skew(60) {
            if state.has_refresh_token {
                return format!(
                    "Claude OAuth loaded from {} (token expired, refresh available)",
                    state.source_path
                );
            }
            return format!(
                "Claude OAuth loaded from {} (token expired, no refresh token)",
                state.source_path
            );
        }
        if let Some(exp) = state.expires_at_unix_secs {
            let now = Utc::now().timestamp().max(0) as u64;
            let remaining = exp.saturating_sub(now);
            return format!(
                "Claude OAuth loaded from {} (expires in {}m)",
                state.source_path,
                remaining / 60
            );
        }
        format!("Claude OAuth loaded from {}", state.source_path)
    }
}

#[async_trait]
impl UsageProvider for ClaudeProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Claude
    }

    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<UsageSnapshot, String> {
        if let Ok(guard) = self.backoff.lock() {
            let backoff_secs = guard.backoff_secs();
            if backoff_secs > 0 {
                if let Some(last_attempt_at) = guard.last_attempt_at {
                    if last_attempt_at.elapsed().as_secs() < backoff_secs {
                        if let Some(ref cached) = guard.cached_success_snapshot {
                            let mut stale = cached.clone();
                            stale.stale = true;
                            info!(
                                "[claude] within backoff window, returning stale cached OAuth data"
                            );
                            return Ok(stale);
                        }
                        info!("[claude] within backoff window, falling back to local logs");
                        return self.local_log_snapshot();
                    }
                }
            }
        }

        let supplied_token = ctx.api_key_for(ProviderId::Claude).map(ToOwned::to_owned);
        let using_supplied_token = supplied_token.is_some();
        let auth_state = self.compute_auth_state(ctx);
        let token = supplied_token.or_else(keychain::read_access_token);

        if let Some(token) = token {
            if !using_supplied_token {
                if auth_state.is_expired_with_skew(60) && !auth_state.has_refresh_token {
                    warn!(
                        "[claude] OAuth token from {} is expired and has no refresh token; using local logs",
                        auth_state.source_path
                    );
                    return self.local_log_snapshot();
                }
            }

            match oauth_fetcher::fetch_usage(&token).await {
                Err(e) => {
                    warn!("[claude] OAuth failed: {e}");
                    if let Ok(mut guard) = self.backoff.lock() {
                        guard.last_attempt_at = Some(Instant::now());
                        guard.consecutive_failures = guard.consecutive_failures.saturating_add(1);
                        if let Some(ref cached) = guard.cached_success_snapshot {
                            info!("[claude] returning stale cached OAuth data");
                            let mut stale = cached.clone();
                            stale.stale = true;
                            return Ok(stale);
                        }
                    }
                }
                Ok(oauth) => {
                    let mut windows = vec![
                        UsageWindow::percent(
                            WindowType::FiveHour,
                            oauth.five_hour_utilization,
                            oauth.five_hour_resets_at,
                            "Claude OAuth reports utilization for the 5-hour subscriber window.",
                        ),
                        UsageWindow::percent(
                            WindowType::SevenDay,
                            oauth.seven_day_utilization,
                            oauth.seven_day_resets_at,
                            "Claude OAuth reports utilization for the 7-day subscriber window.",
                        ),
                    ];
                    // Only add the Opus window if the API actually reports it (non-zero)
                    if oauth.seven_day_opus_utilization > 0.0 {
                        windows.push(UsageWindow::percent(
                            WindowType::Weekly,
                            oauth.seven_day_opus_utilization,
                            oauth.seven_day_resets_at,
                            "Opus weekly usage is exposed as utilization percent, not absolute tokens.",
                        ));
                    }

                    let credits = oauth.extra_usage.as_ref().map(|eu| CreditsInfo {
                        balance_usd: Some(eu.monthly_limit_usd - eu.used_credits_usd),
                        spent_usd: Some(eu.used_credits_usd),
                    });

                    let snapshot = UsageSnapshot {
                        provider: ProviderId::Claude,
                        windows,
                        credits,
                        plan: None,
                        fetched_at: Utc::now(),
                        source: DataSource::Oauth,
                        provenance: DataProvenance::Internal,
                        stale: false,
                    };
                    if let Ok(mut guard) = self.backoff.lock() {
                        guard.last_attempt_at = Some(Instant::now());
                        guard.consecutive_failures = 0;
                        guard.cached_success_snapshot = Some(snapshot.clone());
                    }
                    return Ok(snapshot);
                }
            }
        } else {
            info!("[claude] no OAuth token found, using local logs");
        }

        self.local_log_snapshot()
    }

    async fn fetch_cost_history(&self, days: u32) -> Result<Vec<CostEntry>, String> {
        Ok(log_parser::cost_history(days))
    }

    async fn check_status(&self) -> ProviderStatus {
        let auth = self.compute_auth_state(&FetchContext::default());
        let has_dir = dirs::home_dir()
            .map(|h| h.join(".claude").exists() || h.join(".config").join("claude").exists())
            .unwrap_or(false);
        let has_token = auth.kind != AuthKind::None;
        ProviderStatus {
            provider: ProviderId::Claude,
            health: if has_token || has_dir {
                ProviderHealth::Active
            } else {
                ProviderHealth::Waiting
            },
            message: if has_token {
                Self::auth_message(&auth)
            } else if has_dir {
                "Claude logs detected".to_string()
            } else {
                "Waiting for Claude credentials/session files".to_string()
            },
            checked_at: Utc::now(),
        }
    }

    fn compute_auth_state(&self, _ctx: &FetchContext) -> AuthState {
        let Some(creds) = keychain::read_credentials() else {
            return AuthState::none(ProviderId::Claude);
        };
        let expires_at_unix_secs = creds.expires_at.map(|ms| ms / 1000);
        AuthState {
            provider: ProviderId::Claude,
            kind: AuthKind::Oauth,
            source_path: creds.source_path,
            expires_at_unix_secs,
            last_refresh_iso: None,
            has_refresh_token: creds
                .refresh_token
                .as_ref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false),
            last_error: None,
        }
    }
}
