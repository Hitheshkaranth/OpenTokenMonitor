use std::collections::BTreeMap;

use crate::usage::models::{CostEntry, ProviderId};
use crate::usage_scanners::{scan_claude_cost_snapshot, scan_claude_daily_usage};

pub fn cost_history(days: u32) -> Vec<CostEntry> {
    let mut points = scan_claude_daily_usage();
    points.sort_by(|a, b| a.day.cmp(&b.day));
    if days > 0 && points.len() > days as usize {
        points = points.split_off(points.len() - days as usize);
    }

    points
        .into_iter()
        .map(|p| CostEntry {
            date: p.day,
            provider: ProviderId::Claude,
            model: "claude-mixed".to_string(),
            input_tokens: p.input_tokens,
            output_tokens: p.output_tokens,
            cache_read_tokens: p.cache_read_input_tokens,
            cache_write_tokens: p.cache_creation_input_tokens,
            estimated_cost_usd: p.cost_usd,
        })
        .collect()
}

pub fn usage_windows() -> (u64, u64, f64) {
    let daily = scan_claude_daily_usage();
    let mut by_day = BTreeMap::<String, u64>::new();
    for point in daily {
        by_day.insert(point.day, point.total_tokens);
    }
    let latest_tokens = by_day.iter().last().map(|(_, v)| *v).unwrap_or(0);
    let week_tokens = by_day.iter().rev().take(7).map(|(_, v)| *v).sum::<u64>();
    let cost_today = scan_claude_cost_snapshot().total_cost_usd;
    (latest_tokens, week_tokens, cost_today)
}
