mod descriptor;
mod live_fetcher;

use async_trait::async_trait;
use chrono::Utc;
use std::cmp::Ordering;

// Local-log fallback caps (placeholders; the live server supplies real limits).
const ANTIGRAVITY_FIVE_HOUR_REQUEST_LIMIT: u64 = 50;
const ANTIGRAVITY_CLI_DAILY_REQUEST_CAP: u64 = 200;

use crate::providers::auth::{AuthKind, AuthState};
use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataProvenance, DataSource, PlanInfo, ProviderHealth, ProviderId, ProviderStatus,
    UsageSnapshot, UsageUnit, UsageWindow, WindowType,
};
use crate::usage_scanners::{scan_antigravity_daily_usage, scan_antigravity_model_daily_usage};

pub struct AntigravityProvider {
    descriptor: ProviderDescriptor,
}

impl AntigravityProvider {
    pub fn new() -> Self {
        Self {
            descriptor: descriptor::descriptor(),
        }
    }
}

fn antigravity_installed() -> bool {
    dirs::home_dir()
        .map(|h| {
            h.join(".antigravity").exists()
                || h.join(".gemini").join("antigravity-cli").exists()
                || h.join("Library")
                    .join("Application Support")
                    .join("Antigravity")
                    .exists()
        })
        .unwrap_or(false)
}

#[async_trait]
impl UsageProvider for AntigravityProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Antigravity
    }

    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    async fn fetch_usage(&self, _ctx: &FetchContext) -> Result<UsageSnapshot, String> {
        // Primary: live quota from the running Antigravity CLI/IDE local server.
        // Already authenticated — no tokens touched (see live_fetcher).
        if let Ok(live) = live_fetcher::fetch_quota().await {
            let mut windows = Vec::new();

            // Headline the most-constrained model's rolling quota — the limit the
            // user hits first. The API exposes a remaining fraction + reset time.
            if let Some(model) = live.models.iter().min_by(|a, b| {
                a.remaining_fraction
                    .partial_cmp(&b.remaining_fraction)
                    .unwrap_or(Ordering::Equal)
            }) {
                let pct_used = (1.0 - model.remaining_fraction) * 100.0;
                windows.push(UsageWindow::percent(
                    WindowType::FiveHour,
                    pct_used,
                    model.resets_at,
                    format!("{} — rolling model quota", model.name),
                ));
            }

            // Monthly prompt credits as an exact meter.
            if let Some(monthly) = live.monthly_prompt_credits {
                let available = live.available_prompt_credits.unwrap_or(monthly);
                windows.push(UsageWindow::exact(
                    WindowType::Monthly,
                    monthly.saturating_sub(available),
                    monthly,
                    None,
                    UsageUnit::Requests,
                ));
            }

            if !windows.is_empty() {
                let note = match (live.available_prompt_credits, live.available_flow_credits) {
                    (Some(p), Some(f)) => Some(format!("{p} prompt / {f} flow credits left")),
                    _ => None,
                };
                return Ok(UsageSnapshot {
                    provider: ProviderId::Antigravity,
                    windows,
                    credits: None,
                    plan: Some(PlanInfo {
                        tier: live.plan,
                        note,
                    }),
                    fetched_at: Utc::now(),
                    source: DataSource::Cli,
                    provenance: DataProvenance::Official,
                    stale: false,
                });
            }
        }

        // Fallback: derive an approximate daily figure from local session logs.
        let daily = scan_antigravity_daily_usage();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let daily_request_estimate = daily
            .iter()
            .find(|p| p.day == today)
            .map(|p| p.total_tokens.min(ANTIGRAVITY_CLI_DAILY_REQUEST_CAP))
            .unwrap_or(0);

        Ok(UsageSnapshot {
            provider: ProviderId::Antigravity,
            windows: vec![
                UsageWindow::approximate(
                    WindowType::FiveHour,
                    0,
                    ANTIGRAVITY_FIVE_HOUR_REQUEST_LIMIT,
                    None,
                    UsageUnit::Requests,
                    "Rolling model quota. Start the Antigravity CLI/IDE so the monitor can read live figures.",
                ),
                UsageWindow::approximate(
                    WindowType::Daily,
                    daily_request_estimate,
                    ANTIGRAVITY_CLI_DAILY_REQUEST_CAP,
                    None,
                    UsageUnit::Requests,
                    "Daily request cap for Antigravity CLI. Estimated from local session logs.",
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
        let points = scan_antigravity_model_daily_usage();
        Ok(points
            .into_iter()
            .map(|p| CostEntry {
                date: p.day,
                provider: ProviderId::Antigravity,
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
        let live = live_fetcher::discover_port().is_some();
        let installed = antigravity_installed();

        let (health, message) = if live {
            (ProviderHealth::Active, "Antigravity running — live quota".to_string())
        } else if installed {
            (
                ProviderHealth::Active,
                "Antigravity installed — start it for live quota".to_string(),
            )
        } else {
            (
                ProviderHealth::Waiting,
                "Waiting for Antigravity install".to_string(),
            )
        };

        ProviderStatus {
            provider: ProviderId::Antigravity,
            health,
            message,
            checked_at: Utc::now(),
        }
    }

    fn compute_auth_state(&self, _ctx: &FetchContext) -> AuthState {
        // Auth is handled by the local Antigravity server, not a user-entered key.
        if antigravity_installed() {
            return AuthState {
                provider: ProviderId::Antigravity,
                kind: AuthKind::Cli,
                source_path: "Antigravity local server".to_string(),
                expires_at_unix_secs: None,
                last_refresh_iso: None,
                has_refresh_token: false,
                last_error: if live_fetcher::discover_port().is_some() {
                    None
                } else {
                    Some("Start the Antigravity CLI/IDE for live quota.".to_string())
                },
            };
        }
        AuthState::none(ProviderId::Antigravity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id() {
        let p = AntigravityProvider::new();
        assert_eq!(p.id(), ProviderId::Antigravity);
    }
}
