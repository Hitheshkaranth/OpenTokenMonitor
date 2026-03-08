mod descriptor;
mod oauth_fetcher;
mod stats_parser;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;

use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataSource, ProviderHealth, ProviderId, ProviderStatus, UsageSnapshot, UsageWindow, WindowType,
};

pub struct GeminiProvider {
    descriptor: ProviderDescriptor,
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self { descriptor: descriptor::descriptor() }
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
                        UsageWindow::new(WindowType::Daily, quota.daily_used, quota.daily_limit, quota.resets_at),
                        UsageWindow::new(WindowType::Session, quota.rpm_used, quota.rpm_limit, Some(Utc::now() + Duration::minutes(1))),
                    ],
                    credits: None,
                    plan: None,
                    fetched_at: Utc::now(),
                    source: DataSource::Oauth,
                    stale: false,
                });
            }
        }

        if let Ok(stats) = stats_parser::fetch_stats() {
            return Ok(UsageSnapshot {
                provider: ProviderId::Gemini,
                windows: vec![
                    UsageWindow::new(WindowType::Daily, stats.daily_used, stats.daily_limit, Some(Utc::now() + Duration::days(1))),
                    UsageWindow::new(WindowType::Session, stats.session_used, stats.session_limit, Some(Utc::now() + Duration::hours(1))),
                ],
                credits: None,
                plan: None,
                fetched_at: Utc::now(),
                source: DataSource::Cli,
                stale: false,
            });
        }

        let daily = local_daily_count();
        Ok(UsageSnapshot {
            provider: ProviderId::Gemini,
            windows: vec![
                UsageWindow::new(WindowType::Daily, daily, 1000, Some(Utc::now() + Duration::days(1))),
                UsageWindow::new(WindowType::Session, daily, 2000, Some(Utc::now() + Duration::hours(4))),
            ],
            credits: None,
            plan: None,
            fetched_at: Utc::now(),
            source: DataSource::LocalLog,
            stale: false,
        })
    }

    async fn fetch_cost_history(&self, days: u32) -> Result<Vec<CostEntry>, String> {
        let mut points = local_session_points();
        points.sort_by(|a, b| a.0.cmp(&b.0));
        if days > 0 && points.len() > days as usize {
            points = points.split_off(points.len() - days as usize);
        }
        Ok(points
            .into_iter()
            .map(|(date, count)| CostEntry {
                date,
                provider: ProviderId::Gemini,
                model: "gemini-mixed".to_string(),
                input_tokens: count.saturating_mul(1000),
                output_tokens: count.saturating_mul(300),
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                estimated_cost_usd: 0.0,
            })
            .collect())
    }

    async fn check_status(&self) -> ProviderStatus {
        let has_data = dirs::home_dir()
            .map(|h| h.join(".gemini").exists() || h.join(".config").join("gemini").exists())
            .unwrap_or(false);
        ProviderStatus {
            provider: ProviderId::Gemini,
            health: if has_data { ProviderHealth::Active } else { ProviderHealth::Waiting },
            message: if has_data {
                "Gemini local data detected".to_string()
            } else {
                "Waiting for Gemini credentials/session files".to_string()
            },
            checked_at: Utc::now(),
        }
    }
}

fn local_daily_count() -> u64 {
    local_session_points().iter().rev().next().map(|(_, c)| *c).unwrap_or(0)
}

fn local_session_points() -> Vec<(String, u64)> {
    let mut out = std::collections::BTreeMap::<String, u64>::new();
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".config").join("gemini").join("sessions"));
        roots.push(home.join(".gemini").join("sessions"));
    }

    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(file) = std::fs::File::open(&path) else { continue };
            let Ok(json) = serde_json::from_reader::<_, Value>(file) else { continue };
            let date = json
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
            let tokens = json
                .get("usage")
                .and_then(|v| v.get("total_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(1);
            let slot = out.entry(date).or_insert(0);
            *slot = slot.saturating_add(tokens / 1000 + 1);
        }
    }

    out.into_iter().collect()
}
