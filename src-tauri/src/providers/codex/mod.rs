mod bearer_fetcher;
mod cookie_fetcher;
mod descriptor;
mod log_parser;
mod rpc_fetcher;

use async_trait::async_trait;
use chrono::{Duration, Utc};

use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataProvenance, DataSource, ProviderHealth, ProviderId, ProviderStatus, UsageSnapshot, UsageUnit,
    UsageWindow, WindowType,
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
        eprintln!("[codex] auth token len={}, source={}", auth.access_token.len(), auth.source_path);
        if !auth.access_token.is_empty() {
            match bearer_fetcher::fetch_usage(&auth.access_token).await {
                Ok(bearer) => {
                    eprintln!("[codex] bearer OK: session={}/{} weekly={}/{}", bearer.session_used, bearer.session_limit, bearer.weekly_used, bearer.weekly_limit);
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
                    eprintln!("[codex] bearer FAILED: {e}");
                }
            }
        }

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

        let (session, weekly) = log_parser::usage_windows();
        // Local log tokens are raw counts; approximate Pro plan limits.
        // Real limits come from bearer API when available.
        let session_limit = 500_000;
        let weekly_limit = 5_000_000;
        Ok(UsageSnapshot {
            provider: ProviderId::Codex,
            windows: vec![
                UsageWindow::approximate(
                    WindowType::Session,
                    session,
                    session_limit,
                    Some(Utc::now() + Duration::hours(5)),
                    UsageUnit::Tokens,
                    "Estimated from local Codex session logs until a live usage window is available.",
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
