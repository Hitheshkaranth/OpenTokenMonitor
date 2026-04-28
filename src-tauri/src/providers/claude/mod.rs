mod descriptor;
mod keychain;
mod log_parser;
mod oauth_fetcher;

use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, CreditsInfo, DataProvenance, DataSource, ProviderHealth, ProviderId, ProviderStatus,
    UsageSnapshot, UsageUnit, UsageWindow, WindowType,
};
use crate::usage_scanners::read_claude_oauth_credentials;

/// Minimum seconds between *successful* OAuth API calls — avoids hammering the
/// rate-limited usage endpoint when we already have fresh data.
const OAUTH_COOLDOWN_SECS: u64 = 120;
/// Minimum seconds between OAuth retries after a *failure*. Short enough that a
/// transient 429/network blip doesn't lock the UI into local-log mode for 2
/// minutes, long enough that we don't spam the endpoint.
const OAUTH_FAILURE_BACKOFF_SECS: u64 = 25;

pub struct ClaudeProvider {
    descriptor: ProviderDescriptor,
    /// Cached state for the OAuth path. `last_success` paces the success
    /// cooldown; `last_failure` paces the (much shorter) failure backoff;
    /// `cached_snapshot` is the most recent good response we can reuse while
    /// throttled or to mark stale on consecutive failures.
    oauth_cache: Mutex<OauthCache>,
}

#[derive(Default)]
struct OauthCache {
    last_success: Option<Instant>,
    last_failure: Option<Instant>,
    cached_snapshot: Option<UsageSnapshot>,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor::descriptor(),
            oauth_cache: Mutex::new(OauthCache::default()),
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
        // Cooldown logic:
        //   - If we have a fresh cached snapshot from the last *successful* call
        //     within OAUTH_COOLDOWN_SECS, return it (avoids hammering the API).
        //   - If the last call FAILED, only skip OAuth for OAUTH_FAILURE_BACKOFF_SECS
        //     so transient blips don't pin the UI to local-log mode for 2 minutes.
        //   - Otherwise fall through and retry OAuth.
        if let Ok(guard) = self.oauth_cache.lock() {
            if let Some(last_success) = guard.last_success {
                if last_success.elapsed().as_secs() < OAUTH_COOLDOWN_SECS {
                    if let Some(ref snap) = guard.cached_snapshot {
                        return Ok(snap.clone());
                    }
                }
            }
            if let Some(last_failure) = guard.last_failure {
                if last_failure.elapsed().as_secs() < OAUTH_FAILURE_BACKOFF_SECS {
                    // Recent failure: prefer stale cache over yet another API hit.
                    if let Some(ref cached) = guard.cached_snapshot {
                        let mut stale = cached.clone();
                        stale.stale = true;
                        return Ok(stale);
                    }
                    drop(guard);
                    return self.local_log_snapshot();
                }
            }
        }

        let token = ctx
            .api_key_for(ProviderId::Claude)
            .map(ToOwned::to_owned)
            .or_else(keychain::read_access_token);

        if let Some(token) = token {
            match oauth_fetcher::fetch_usage(&token).await {
                Err(e) => {
                    eprintln!("[claude] OAuth failed: {e}");
                    if let Ok(mut guard) = self.oauth_cache.lock() {
                        guard.last_failure = Some(Instant::now());
                        if let Some(ref cached) = guard.cached_snapshot {
                            eprintln!("[claude] returning stale cached OAuth data");
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
                    if let Ok(mut guard) = self.oauth_cache.lock() {
                        guard.last_success = Some(Instant::now());
                        guard.last_failure = None;
                        guard.cached_snapshot = Some(snapshot.clone());
                    }
                    return Ok(snapshot);
                }
            }
        } else {
            eprintln!("[claude] no OAuth token found, using local logs");
        }

        self.local_log_snapshot()
    }

    async fn fetch_cost_history(&self, days: u32) -> Result<Vec<CostEntry>, String> {
        Ok(log_parser::cost_history(days))
    }

    async fn check_status(&self) -> ProviderStatus {
        let creds = read_claude_oauth_credentials();
        let has_token = !creds.access_token.trim().is_empty();
        let has_dir = dirs::home_dir()
            .map(|h| h.join(".claude").exists() || h.join(".config").join("claude").exists())
            .unwrap_or(false);
        ProviderStatus {
            provider: ProviderId::Claude,
            health: if has_token || has_dir {
                ProviderHealth::Active
            } else {
                ProviderHealth::Waiting
            },
            message: if has_token {
                format!("Claude auth loaded from {}", creds.source_path)
            } else if has_dir {
                "Claude logs detected".to_string()
            } else {
                "Waiting for Claude credentials/session files".to_string()
            },
            checked_at: Utc::now(),
        }
    }
}
