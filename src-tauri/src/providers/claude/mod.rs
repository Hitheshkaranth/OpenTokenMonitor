mod descriptor;
mod keychain;
mod log_parser;
mod oauth_fetcher;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataSource, ProviderHealth, ProviderId, ProviderStatus, UsageSnapshot, UsageWindow, WindowType,
};

pub struct ClaudeProvider {
    descriptor: ProviderDescriptor,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self { descriptor: descriptor::descriptor() }
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
        let token = ctx
            .api_key_for(ProviderId::Claude)
            .map(ToOwned::to_owned)
            .or_else(keychain::read_access_token);

        if let Some(token) = token {
            if let Ok(oauth) = oauth_fetcher::fetch_usage(&token).await {
                let five_limit = 100u64;
                let week_limit = 1000u64;
                let five_used = ((oauth.five_hour_utilization / 100.0) * five_limit as f64) as u64;
                let week_used = ((oauth.seven_day_utilization / 100.0) * week_limit as f64) as u64;
                let opus_used = ((oauth.seven_day_opus_utilization / 100.0) * week_limit as f64) as u64;
                return Ok(UsageSnapshot {
                    provider: ProviderId::Claude,
                    windows: vec![
                        UsageWindow::new(WindowType::FiveHour, five_used, five_limit, oauth.five_hour_resets_at),
                        UsageWindow::new(WindowType::SevenDay, week_used, week_limit, oauth.seven_day_resets_at),
                        UsageWindow::new(WindowType::Weekly, opus_used, week_limit, oauth.seven_day_resets_at),
                    ],
                    credits: None,
                    plan: None,
                    fetched_at: Utc::now(),
                    source: DataSource::Oauth,
                    stale: false,
                });
            }
        }

        let (day_tokens, week_tokens, _) = log_parser::usage_windows();
        let now = Utc::now();
        Ok(UsageSnapshot {
            provider: ProviderId::Claude,
            windows: vec![
                UsageWindow::new(WindowType::FiveHour, day_tokens, 50_000, Some(now + Duration::hours(5))),
                UsageWindow::new(WindowType::SevenDay, week_tokens, 500_000, Some(now + Duration::days(7))),
            ],
            credits: None,
            plan: None,
            fetched_at: now,
            source: DataSource::LocalLog,
            stale: false,
        })
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
            health: if has_dir { ProviderHealth::Active } else { ProviderHealth::Waiting },
            message: if has_dir {
                "Claude logs detected".to_string()
            } else {
                "Waiting for ~/.claude data".to_string()
            },
            checked_at: Utc::now(),
        }
    }
}
