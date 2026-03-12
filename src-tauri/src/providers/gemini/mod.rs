mod descriptor;
mod oauth_fetcher;
mod stats_parser;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;

use crate::providers::{FetchContext, ProviderDescriptor, UsageProvider};
use crate::usage::models::{
    CostEntry, DataProvenance, DataSource, ProviderHealth, ProviderId, ProviderStatus, UsageSnapshot, UsageUnit,
    UsageWindow, WindowType,
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

        let daily = local_daily_count();
        // Local session counts are raw; use generous limits for sensible utilization display
        let daily_limit = daily.max(1000);
        let session_limit = daily.max(2000);
        Ok(UsageSnapshot {
            provider: ProviderId::Gemini,
            windows: vec![
                UsageWindow::approximate(
                    WindowType::Daily,
                    daily,
                    daily_limit,
                    Some(Utc::now() + Duration::days(1)),
                    UsageUnit::Unknown,
                    "Estimated from local Gemini session files; official daily quota data requires live auth or CLI stats.",
                ),
                UsageWindow::approximate(
                    WindowType::Session,
                    daily,
                    session_limit,
                    Some(Utc::now() + Duration::hours(4)),
                    UsageUnit::Unknown,
                    "Estimated from local Gemini session files; not an official provider remaining counter.",
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
        let has_cli = stats_parser::supports_stats_command();
        let has_data = dirs::home_dir()
            .map(|h| h.join(".gemini").exists() || h.join(".config").join("gemini").exists())
            .unwrap_or(false);
        let active = has_cli || has_data;

        ProviderStatus {
            provider: ProviderId::Gemini,
            health: if active { ProviderHealth::Active } else { ProviderHealth::Waiting },
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

fn local_daily_count() -> u64 {
    local_session_points().iter().rev().next().map(|(_, c)| *c).unwrap_or(0)
}

fn local_session_points() -> Vec<(String, u64)> {
    let mut out = std::collections::BTreeMap::<String, u64>::new();
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Look in both specialized sessions folders and the root config folders
        roots.push(home.join(".config").join("gemini").join("sessions"));
        roots.push(home.join(".gemini").join("sessions"));
        roots.push(home.join(".config").join("gemini"));
        roots.push(home.join(".gemini"));
    }

    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for entry in rd.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "json" && ext != "jsonl" {
                continue;
            }
            let Ok(file) = std::fs::File::open(&path) else { continue };

            // For Gemini, we handle both single JSON objects and JSONL (one object per line)
            if ext == "jsonl" {
                let reader = std::io::BufReader::new(file);
                for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
                    if let Ok(json) = serde_json::from_str::<Value>(&line) {
                        process_session_json(json, &mut out);
                    }
                }
            } else {
                if let Ok(json) = serde_json::from_reader::<_, Value>(file) {
                    process_session_json(json, &mut out);
                }
            }
        }
    }

    out.into_iter().collect()
}

fn process_session_json(json: Value, out: &mut std::collections::BTreeMap<String, u64>) {
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
    // Convert tokens to a rough request count or similar unit for the simple meter
    *slot = slot.saturating_add(tokens / 1000 + 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_local_session_points_parsing() {
        let dir = tempdir().unwrap();
        let sessions_dir = dir.path().join("sessions");
        create_dir_all(&sessions_dir).unwrap();

        // Create a JSON file in sessions/
        let json_path = sessions_dir.join("s1.json");
        let mut f = File::create(json_path).unwrap();
        writeln!(f, "{}", r#"{"timestamp":"2026-03-12T10:00:00Z", "usage":{"total_tokens":2500}}"#).unwrap();

        // Create a JSONL file in root
        let jsonl_path = dir.path().join("history.jsonl");
        let mut f = File::create(jsonl_path).unwrap();
        writeln!(f, "{}", r#"{"timestamp":"2026-03-12T11:00:00Z", "usage":{"total_tokens":1000}}"#).unwrap();
        writeln!(f, "{}", r#"{"timestamp":"2026-03-11T09:00:00Z", "usage":{"total_tokens":500}}"#).unwrap();

        let mut out = std::collections::BTreeMap::new();
        let roots = vec![sessions_dir, dir.path().to_path_buf()];

        for root in roots {
            let rd = std::fs::read_dir(root).unwrap();
            for entry in rd.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext != "json" && ext != "jsonl" { continue; }
                let file = File::open(&path).unwrap();
                if ext == "jsonl" {
                    let reader = std::io::BufReader::new(file);
                    for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
                        if let Ok(json) = serde_json::from_str::<Value>(&line) {
                            process_session_json(json, &mut out);
                        }
                    }
                } else {
                    if let Ok(json) = serde_json::from_reader::<_, Value>(file) {
                        process_session_json(json, &mut out);
                    }
                }
            }
        }

        // 2026-03-12: 2500 tokens -> 3, 1000 tokens -> 2. Total = 5
        // 2026-03-11: 500 tokens -> 1. Total = 1
        assert_eq!(out.get("2026-03-12"), Some(&5));
        assert_eq!(out.get("2026-03-11"), Some(&1));
    }
}
