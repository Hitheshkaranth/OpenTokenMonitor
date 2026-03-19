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

/// Minimum seconds between OAuth API calls to avoid 429s.
/// The Claude usage API has known persistent 429 issues — keep cooldown generous.
const OAUTH_COOLDOWN_SECS: u64 = 120;

pub struct ClaudeProvider {
    descriptor: ProviderDescriptor,
    /// (last_attempt, cached_result) — prevents calling OAuth API more than once per cooldown.
    oauth_cache: Mutex<(Option<Instant>, Option<UsageSnapshot>)>,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor::descriptor(),
            oauth_cache: Mutex::new((None, None)),
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
        // Check cooldown: return cached result or skip OAuth if called too recently
        if let Ok(guard) = self.oauth_cache.lock() {
            if let Some(last_attempt) = guard.0 {
                if last_attempt.elapsed().as_secs() < OAUTH_COOLDOWN_SECS {
                    if let Some(ref snap) = guard.1 {
                        return Ok(snap.clone());
                    }
                    // Last attempt was recent but failed — skip OAuth, fall through to local
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
            // Mark attempt timestamp before calling API
            if let Ok(mut guard) = self.oauth_cache.lock() {
                guard.0 = Some(Instant::now());
            }

            match oauth_fetcher::fetch_usage(&token).await {
                Err(e) => {
                    eprintln!("[claude] OAuth failed: {e}");
                    // If we have a stale cached result, return it marked stale
                    if let Ok(guard) = self.oauth_cache.lock() {
                        if let Some(ref cached) = guard.1 {
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
                        *guard = (Some(Instant::now()), Some(snapshot.clone()));
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
        let has_dir = dirs::home_dir()
            .map(|h| h.join(".claude").exists())
            .unwrap_or(false);
        ProviderStatus {
            provider: ProviderId::Claude,
            health: if has_dir {
                ProviderHealth::Active
            } else {
                ProviderHealth::Waiting
            },
            message: if has_dir {
                "Claude logs detected".to_string()
            } else {
                "Waiting for ~/.claude data".to_string()
            },
            checked_at: Utc::now(),
        }
    }
}
