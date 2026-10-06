//! Exact per-session and per-project usage, built from the scanners'
//! per-log-file contributions (one file = one CLI session).
//!
//! This replaces the frontend's old estimate, which split each day's cost
//! across projects by prompt count. Sessions are grouped into projects by
//! working directory, so a repo used from both Claude and Codex is one project.

use std::collections::{BTreeSet, HashMap};

use chrono::{Duration, Utc};
use serde::Serialize;

use crate::usage::models::ProviderId;
use crate::usage_scanners::{self, sorted_model_costs, ModelCost, SessionUsage};

#[derive(Debug, Clone, Serialize)]
pub struct ProjectUsage {
    /// `cwd:<normalized path>`, or `unknown` for sessions without a cwd.
    pub project_id: String,
    pub label: String,
    pub path: Option<String>,
    pub providers: Vec<ProviderId>,
    pub session_count: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
    pub today_cost_usd: f64,
    pub last_ts: Option<i64>,
    /// Distinct models with their cost in the window, highest cost first.
    pub models: Vec<ModelCost>,
    /// `cache_read / (input + cache_read + cache_write)`, 0 when no input.
    pub cache_hit_ratio: f64,
}

fn normalize_path(path: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.is_empty() {
        normalized
    } else {
        trimmed.to_string()
    }
}

fn basename(path: &str) -> String {
    path.rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .to_string()
}

pub fn cache_hit_ratio(input: u64, cache_read: u64, cache_write: u64) -> f64 {
    let denominator = input + cache_read + cache_write;
    if denominator == 0 {
        0.0
    } else {
        cache_read as f64 / denominator as f64
    }
}

/// Roll sessions up into projects keyed by normalized cwd, highest cost first.
pub fn group_by_project(sessions: &[SessionUsage]) -> Vec<ProjectUsage> {
    struct Acc {
        project: ProjectUsage,
        providers: BTreeSet<&'static str>,
        model_costs: HashMap<String, f64>,
    }

    let mut by_id = HashMap::<String, Acc>::new();
    for session in sessions {
        let path = session
            .cwd
            .as_deref()
            .map(normalize_path)
            .filter(|p| !p.is_empty());
        let project_id = path
            .as_ref()
            .map(|p| format!("cwd:{}", p.to_lowercase()))
            .unwrap_or_else(|| "unknown".to_string());
        let acc = by_id.entry(project_id.clone()).or_insert_with(|| Acc {
            project: ProjectUsage {
                project_id,
                label: path
                    .as_deref()
                    .map(basename)
                    .unwrap_or_else(|| "Unattributed".to_string()),
                path: path.clone(),
                providers: Vec::new(),
                session_count: 0,
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                total_tokens: 0,
                cost_usd: 0.0,
                today_cost_usd: 0.0,
                last_ts: None,
                models: Vec::new(),
                cache_hit_ratio: 0.0,
            },
            providers: BTreeSet::new(),
            model_costs: HashMap::new(),
        });

        let p = &mut acc.project;
        p.session_count += 1;
        p.input_tokens += session.input_tokens;
        p.output_tokens += session.output_tokens;
        p.cache_read_tokens += session.cache_read_tokens;
        p.cache_write_tokens += session.cache_write_tokens;
        p.total_tokens += session.total_tokens;
        p.cost_usd += session.cost_usd;
        p.today_cost_usd += session.today_cost_usd;
        p.last_ts = p.last_ts.max(session.last_ts);
        acc.providers.insert(session.provider.as_str());
        for m in &session.models {
            *acc.model_costs.entry(m.model.clone()).or_default() += m.cost_usd;
        }
    }

    let mut out: Vec<ProjectUsage> = by_id
        .into_values()
        .map(|acc| {
            let mut p = acc.project;
            p.providers = ProviderId::all()
                .into_iter()
                .filter(|id| acc.providers.contains(id.as_str()))
                .collect();
            p.models = sorted_model_costs(acc.model_costs);
            p.cache_hit_ratio =
                cache_hit_ratio(p.input_tokens, p.cache_read_tokens, p.cache_write_tokens);
            p
        })
        .collect();
    out.sort_by(|a, b| b.cost_usd.total_cmp(&a.cost_usd));
    out
}

/// `(since_day, today)` for a window of `days` days ending today, in UTC to
/// match how the scanners bucket log lines.
fn day_window(days: u32) -> (String, String) {
    let today = Utc::now().date_naive();
    let since = today - Duration::days(i64::from(days.max(1)) - 1);
    (
        since.format("%Y-%m-%d").to_string(),
        today.format("%Y-%m-%d").to_string(),
    )
}

async fn scan_sessions(
    days: u32,
    provider: Option<ProviderId>,
) -> Result<Vec<SessionUsage>, String> {
    let (since, today) = day_window(days);
    let mut sessions = tauri::async_runtime::spawn_blocking(move || {
        usage_scanners::scan_session_usage(&since, &today)
    })
    .await
    .map_err(|e| e.to_string())?;
    if let Some(provider) = provider {
        sessions.retain(|s| s.provider == provider);
    }
    Ok(sessions)
}

/// Most recent sessions first; `provider` narrows to one provider.
#[tauri::command]
pub async fn get_session_usage(
    days: u32,
    limit: Option<u32>,
    provider: Option<ProviderId>,
) -> Result<Vec<SessionUsage>, String> {
    let mut sessions = scan_sessions(days, provider).await?;
    sessions.truncate(limit.unwrap_or(50).max(1) as usize);
    Ok(sessions)
}

/// Projects by cost; `provider` counts only that provider's sessions.
#[tauri::command]
pub async fn get_project_usage(
    days: u32,
    provider: Option<ProviderId>,
) -> Result<Vec<ProjectUsage>, String> {
    Ok(group_by_project(&scan_sessions(days, provider).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(
        provider: ProviderId,
        cwd: Option<&str>,
        cost: f64,
        models: &[&str],
    ) -> SessionUsage {
        SessionUsage {
            provider,
            session_id: format!("{}-{cost}", provider.as_str()),
            cwd: cwd.map(str::to_string),
            first_ts: Some(100),
            last_ts: Some((cost * 10.0) as i64),
            input_tokens: 100,
            output_tokens: 10,
            cache_read_tokens: 300,
            cache_write_tokens: 0,
            total_tokens: 110,
            cost_usd: cost,
            models: models
                .iter()
                .map(|m| ModelCost {
                    model: m.to_string(),
                    cost_usd: cost,
                })
                .collect(),
            today_cost_usd: cost / 2.0,
        }
    }

    #[test]
    fn merges_providers_sharing_a_cwd() {
        let projects = group_by_project(&[
            session(ProviderId::Codex, Some("/work/app/"), 1.0, &["gpt-5"]),
            session(
                ProviderId::Claude,
                Some("/work/app"),
                3.0,
                &["claude-opus-4-5"],
            ),
            session(
                ProviderId::Claude,
                Some("\\work\\app"),
                2.0,
                &["claude-sonnet-4-5"],
            ),
        ]);
        assert_eq!(projects.len(), 1);
        let p = &projects[0];
        assert_eq!(p.label, "app");
        assert_eq!(p.path.as_deref(), Some("/work/app"));
        assert_eq!(p.providers, vec![ProviderId::Claude, ProviderId::Codex]);
        assert_eq!(p.session_count, 3);
        assert!((p.cost_usd - 6.0).abs() < 1e-9);
        assert!((p.today_cost_usd - 3.0).abs() < 1e-9);
        assert_eq!(p.last_ts, Some(30));
        assert_eq!(p.models[0].model, "claude-opus-4-5");
        assert_eq!(p.models.len(), 3);
    }

    #[test]
    fn sessions_without_cwd_are_unattributed() {
        let projects = group_by_project(&[session(ProviderId::Codex, None, 1.0, &[])]);
        assert_eq!(projects[0].project_id, "unknown");
        assert_eq!(projects[0].label, "Unattributed");
        assert_eq!(projects[0].path, None);
    }

    #[test]
    fn projects_sort_by_cost_and_compute_cache_ratio() {
        let projects = group_by_project(&[
            session(ProviderId::Claude, Some("/a"), 1.0, &[]),
            session(ProviderId::Claude, Some("/b"), 5.0, &[]),
        ]);
        assert_eq!(projects[0].label, "b");
        assert!((projects[0].cache_hit_ratio - 0.75).abs() < 1e-9);
        assert_eq!(cache_hit_ratio(0, 0, 0), 0.0);
    }

    #[test]
    fn day_window_spans_requested_days() {
        let (since, today) = day_window(1);
        assert_eq!(since, today);
        let (since, today) = day_window(7);
        assert!(since < today);
    }
}
