use std::collections::BTreeMap;

use chrono::{Duration, Utc};

use crate::usage::models::{CostEntry, ProviderId};
use crate::usage_scanners::{
    scan_claude_cost_snapshot, scan_claude_daily_usage, scan_claude_model_daily_usage,
};

pub fn cost_history(days: u32) -> Vec<CostEntry> {
    let mut points = scan_claude_model_daily_usage();
    points.sort_by(|a, b| a.day.cmp(&b.day).then(a.model.cmp(&b.model)));
    retain_recent_days(&mut points, days);

    points
        .into_iter()
        .map(|p| CostEntry {
            date: p.day,
            provider: ProviderId::Claude,
            model: p.model,
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

    // Anchor both windows on *today* (UTC), not on "the most recent day the user
    // was active". Prior code used `by_day.iter().last()` for the 5-hour window
    // and `iter().rev().take(7)` for the week, which silently surfaced stale
    // day-old or week-old counts whenever the user had been inactive. When the
    // user hasn't used Claude today yet, today_tokens should be 0.
    let now = Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let week_cutoff = (now - Duration::days(6)).format("%Y-%m-%d").to_string();

    let today_tokens = by_day.get(&today).copied().unwrap_or(0);
    let week_tokens = by_day
        .range(week_cutoff..=today)
        .map(|(_, v)| *v)
        .sum::<u64>();

    let cost_today = scan_claude_cost_snapshot().total_cost_usd;
    (today_tokens, week_tokens, cost_today)
}

fn retain_recent_days<T>(points: &mut Vec<T>, days: u32)
where
    T: HasDay,
{
    if days == 0 {
        return;
    }

    let mut keep_days = std::collections::BTreeSet::<String>::new();
    for point in points.iter().rev() {
        if keep_days.len() >= days as usize && !keep_days.contains(point.day()) {
            break;
        }
        keep_days.insert(point.day().to_string());
    }
    points.retain(|point| keep_days.contains(point.day()));
}

trait HasDay {
    fn day(&self) -> &str;
}

impl HasDay for crate::usage_scanners::ClaudeModelDailyUsagePoint {
    fn day(&self) -> &str {
        &self.day
    }
}
