mod cookie_fetcher;
mod descriptor;
mod log_parser;
mod rpc_fetcher;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataSource, ProviderHealth, ProviderId, ProviderStatus, UsageSnapshot, UsageWindow, WindowType,
};
use crate::usage_scanners::read_codex_auth_bridge;

pub struct CodexProvider {
    descriptor: ProviderDescriptor,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self { descriptor: descriptor::descriptor() }
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
            if let Some(cookie) = ctx.api_key_for(ProviderId::Codex).filter(|v| v.contains("session")) {
                if let Ok(cookie_snapshot) = cookie_fetcher::fetch_usage(cookie).await {
                    return Ok(UsageSnapshot {
                        provider: ProviderId::Codex,
                        windows: vec![
                            UsageWindow::new(WindowType::Session, cookie_snapshot.session_used, cookie_snapshot.session_limit, cookie_snapshot.resets_at),
                            UsageWindow::new(WindowType::Weekly, cookie_snapshot.weekly_used, cookie_snapshot.weekly_limit, cookie_snapshot.resets_at),
                        ],
                        credits: None,
                        plan: None,
                        fetched_at: Utc::now(),
                        source: DataSource::Cookie,
                        stale: false,
                    });
                }
            }
        }

        if let Ok(cli) = rpc_fetcher::fetch_usage() {
            return Ok(UsageSnapshot {
                provider: ProviderId::Codex,
                windows: vec![
                    UsageWindow::new(WindowType::Session, cli.session_used, cli.session_limit, cli.resets_at),
                    UsageWindow::new(WindowType::Weekly, cli.weekly_used, cli.weekly_limit, cli.resets_at),
                ],
                credits: None,
                plan: None,
                fetched_at: Utc::now(),
                source: DataSource::Cli,
                stale: false,
            });
        }

        let (session, weekly) = log_parser::usage_windows();
        Ok(UsageSnapshot {
            provider: ProviderId::Codex,
            windows: vec![
                UsageWindow::new(WindowType::Session, session, 60_000, Some(Utc::now() + Duration::hours(5))),
                UsageWindow::new(WindowType::Weekly, weekly, 600_000, Some(Utc::now() + Duration::days(7))),
            ],
            credits: None,
            plan: None,
            fetched_at: Utc::now(),
            source: DataSource::LocalLog,
            stale: false,
        })
    }

    async fn fetch_cost_history(&self, days: u32) -> Result<Vec<CostEntry>, String> {
        Ok(log_parser::cost_history(days))
    }

    async fn check_status(&self) -> ProviderStatus {
        let auth = read_codex_auth_bridge();
        let has_home = dirs::home_dir().map(|h| h.join(".codex").exists()).unwrap_or(false);
        let message = if !auth.access_token.is_empty() {
            format!("Codex auth loaded from {}", auth.source_path)
        } else if has_home {
            "Codex local data detected".to_string()
        } else {
            "Waiting for ~/.codex data".to_string()
        };
        ProviderStatus {
            provider: ProviderId::Codex,
            health: if has_home { ProviderHealth::Active } else { ProviderHealth::Waiting },
            message,
            checked_at: Utc::now(),
        }
    }
}
