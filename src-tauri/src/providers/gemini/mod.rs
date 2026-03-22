mod descriptor;
mod oauth_fetcher;
mod stats_parser;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataProvenance, DataSource, ProviderHealth, ProviderId, ProviderStatus,
    UsageSnapshot, UsageUnit, UsageWindow, WindowType,
};
use crate::usage_scanners::{
    scan_gemini_daily_usage, scan_gemini_model_daily_usage,
};

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
                            quota.resets_at,
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

        if let Ok(stats) = stats_parser::fetch_stats() {
            return Ok(UsageSnapshot {
                provider: ProviderId::Gemini,
                windows: vec![
                    UsageWindow::exact(
                        WindowType::Daily,
                        stats.daily_used,
                        stats.daily_limit,
                        Some(Utc::now() + Duration::days(1)),
                        UsageUnit::Requests,
                    ),
                    UsageWindow::exact(
                        WindowType::Session,
                        stats.session_used,
                        stats.session_limit,
                        Some(Utc::now() + Duration::hours(1)),
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

        let daily = scan_gemini_daily_usage();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let today_point = daily.iter().find(|p| p.day == today);
        
        let daily_used = today_point.map(|p| p.total_tokens).unwrap_or(0);

        // Local session counts are raw tokens; use generous limits for sensible utilization display.
        // Free tier is ~50M tokens/day or similar depending on the model.
        let daily_limit = 50_000_000; 
        let session_limit = daily_used.max(1_000_000) * 2;

        Ok(UsageSnapshot {
            provider: ProviderId::Gemini,
            windows: vec![
                UsageWindow::approximate(
                    WindowType::Daily,
                    daily_used,
                    daily_limit,
                    Some(Utc::now() + Duration::days(1)),
                    UsageUnit::Tokens,
                    "Estimated from local session files. Limit is set to a generous 50M tokens for visualization.",
                ),
                UsageWindow::approximate(
                    WindowType::Session,
                    daily_used,
                    session_limit,
                    Some(Utc::now() + Duration::hours(4)),
                    UsageUnit::Tokens,
                    "Rolling session usage estimated from local files.",
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
