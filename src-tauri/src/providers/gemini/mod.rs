mod descriptor;
mod oauth_fetcher;
mod stats_parser;

use async_trait::async_trait;
use chrono::{Duration, NaiveTime, TimeZone, Utc};

// Gemini CLI free-tier quotas (Google personal account, OAuth):
//   • 60 model requests per minute
//   • 1,000 model requests per day  (Gemini 2.5 Pro tier)
// Quotas reset at midnight Pacific Time. There is *no* 5-hour or 4-hour rolling
// window — earlier code modeled a fictitious "session" window. We keep the
// per-minute and per-day windows below to match the real provider behavior.
const GEMINI_FREE_DAILY_REQUEST_LIMIT: u64 = 1_000;
const GEMINI_FREE_RPM_LIMIT: u64 = 60;

/// Returns the next quota reset instant. Gemini resets at 00:00 Pacific Time;
/// approximating with UTC+0 (close enough for countdown UX, drifts ≤ 8h).
fn next_daily_reset() -> chrono::DateTime<Utc> {
    let now = Utc::now();
    let tomorrow = (now + Duration::days(1)).date_naive();
    Utc.from_utc_datetime(&tomorrow.and_time(NaiveTime::MIN))
}

use crate::providers::auth::{AuthKind, AuthState};
use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataProvenance, DataSource, ProviderHealth, ProviderId, ProviderStatus,
    UsageSnapshot, UsageUnit, UsageWindow, WindowType,
};
use crate::usage_scanners::{scan_gemini_daily_usage, scan_gemini_model_daily_usage};

pub struct GeminiProvider {
    descriptor: ProviderDescriptor,
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor::descriptor(),
        }
    }
}

#[async_trait]
impl UsageProvider for GeminiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Gemini
    }

    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<UsageSnapshot, String> {
        if let Some(api_key) = ctx.api_key_for(ProviderId::Gemini) {
            if let Ok(quota) = oauth_fetcher::fetch_quota(api_key).await {
                return Ok(UsageSnapshot {
                    provider: ProviderId::Gemini,
                    windows: vec![
                        UsageWindow::exact(
                            WindowType::Daily,
                            quota.daily_used,
                            quota.daily_limit,
                            quota.resets_at.or_else(|| Some(next_daily_reset())),
                            UsageUnit::Requests,
                        ),
                        UsageWindow::exact(
                            WindowType::Session,
                            quota.rpm_used,
                            quota.rpm_limit,
                            Some(Utc::now() + Duration::minutes(1)),
                            UsageUnit::Requests,
                        ),
                    ],
                    credits: None,
                    plan: None,
                    fetched_at: Utc::now(),
                    source: DataSource::Oauth,
                    provenance: DataProvenance::Official,
                    stale: false,
                });
            }
        }

        if ctx.allow_cli_strategy {
            if let Ok(stats) = stats_parser::fetch_stats() {
                return Ok(UsageSnapshot {
                    provider: ProviderId::Gemini,
                    windows: vec![
                        UsageWindow::exact(
                            WindowType::Daily,
                            stats.daily_used,
                            stats.daily_limit,
                            Some(next_daily_reset()),
                            UsageUnit::Requests,
                        ),
                        UsageWindow::exact(
                            WindowType::Session,
                            stats.session_used,
                            stats.session_limit,
                            Some(Utc::now() + Duration::minutes(1)),
                            UsageUnit::Tokens,
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

        let daily = scan_gemini_daily_usage();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let today_point = daily.iter().find(|p| p.day == today);

        // Local session files only record token totals — we can't recover the
        // exact request count from them. Bound by the free-tier daily request
        // quota so utilization display stays meaningful.
        let daily_request_estimate = today_point
            .map(|p| p.total_tokens.min(GEMINI_FREE_DAILY_REQUEST_LIMIT))
            .unwrap_or(0);

        Ok(UsageSnapshot {
            provider: ProviderId::Gemini,
            windows: vec![
                UsageWindow::approximate(
                    WindowType::Daily,
                    daily_request_estimate,
                    GEMINI_FREE_DAILY_REQUEST_LIMIT,
                    Some(next_daily_reset()),
                    UsageUnit::Requests,
                    "Daily quota for Gemini CLI free tier (1,000 requests/day, resets midnight Pacific). Estimated from local session files.",
                ),
                UsageWindow::approximate(
                    WindowType::Session,
                    0,
                    GEMINI_FREE_RPM_LIMIT,
                    Some(Utc::now() + Duration::minutes(1)),
                    UsageUnit::Requests,
                    "Per-minute request quota for Gemini CLI free tier (60 requests/minute). Local logs do not expose live RPM.",
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

    async fn fetch_cost_history(&self, _days: u32) -> Result<Vec<CostEntry>, String> {
        let points = scan_gemini_model_daily_usage();
        Ok(points
            .into_iter()
            .map(|p| CostEntry {
                date: p.day,
                provider: ProviderId::Gemini,
                model: p.model,
                input_tokens: p.input_tokens,
                output_tokens: p.output_tokens,
                cache_read_tokens: p.cache_read_tokens,
                cache_write_tokens: 0,
                estimated_cost_usd: p.cost_usd,
            })
            .collect())
    }

    async fn check_status(&self) -> ProviderStatus {
        let has_cli = stats_parser::supports_stats_command();
        let has_data = dirs::home_dir()
            .map(|h| {
                h.join(".gemini").exists()
                    || h.join(".config").join("gemini").exists()
                    || h.join(".gemini").join("tmp").exists()
            })
            .unwrap_or(false);
        let active = has_cli || has_data;

        ProviderStatus {
            provider: ProviderId::Gemini,
            health: if active {
                ProviderHealth::Active
            } else {
                ProviderHealth::Waiting
            },
            message: if has_cli {
                "Gemini CLI detected".to_string()
            } else if has_data {
                "Gemini local data detected".to_string()
            } else {
                "Waiting for Gemini credentials/session files".to_string()
            },
            checked_at: Utc::now(),
        }
    }

    fn compute_auth_state(&self, ctx: &FetchContext) -> AuthState {
        if ctx.api_key_for(ProviderId::Gemini).is_some() {
            return AuthState {
                provider: ProviderId::Gemini,
                kind: AuthKind::ApiKey,
                source_path: "runtime api_keys map".to_string(),
                expires_at_unix_secs: None,
                last_refresh_iso: None,
                has_refresh_token: false,
                last_error: None,
            };
        }
        AuthState::none(ProviderId::Gemini)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id() {
        let p = GeminiProvider::new();
        assert_eq!(p.id(), ProviderId::Gemini);
    }
}
