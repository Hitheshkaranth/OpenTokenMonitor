use crate::usage::models::{CostEntry, ProviderId};
use crate::usage_scanners::{scan_codex_cost_snapshot, scan_codex_daily_usage};

pub fn cost_history(days: u32) -> Vec<CostEntry> {
    let mut points = scan_codex_daily_usage();
    points.sort_by(|a, b| a.day.cmp(&b.day));
    if days > 0 && points.len() > days as usize {
        points = points.split_off(points.len() - days as usize);
    }

    points
        .into_iter()
        .map(|p| CostEntry {
            date: p.day,
            provider: ProviderId::Codex,
            model: "gpt-mixed".to_string(),
            input_tokens: p.input_tokens,
            output_tokens: p.output_tokens,
            cache_read_tokens: p.cached_input_tokens,
            cache_write_tokens: 0,
            estimated_cost_usd: p.cost_usd,
        })
        .collect()
}

pub fn usage_windows() -> (u64, u64) {
    let mut points = scan_codex_daily_usage();
    points.sort_by(|a, b| a.day.cmp(&b.day));
    let session = points.last().map(|p| p.total_tokens).unwrap_or(0);
    let weekly = points.iter().rev().take(7).map(|p| p.total_tokens).sum::<u64>();
    let fallback = scan_codex_cost_snapshot();
    let safe_session = if session == 0 { fallback.total_tokens } else { session };
    (safe_session, weekly)
}
