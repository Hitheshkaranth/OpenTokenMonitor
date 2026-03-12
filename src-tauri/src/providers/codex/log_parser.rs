use crate::usage::models::{CostEntry, ProviderId};
use crate::usage_scanners::{scan_codex_cost_snapshot, scan_codex_daily_usage, scan_codex_model_daily_usage};

pub fn cost_history(days: u32) -> Vec<CostEntry> {
    let mut points = scan_codex_model_daily_usage();
    points.sort_by(|a, b| a.day.cmp(&b.day).then(a.model.cmp(&b.model)));
    retain_recent_days(&mut points, days);

    points
        .into_iter()
        .map(|p| CostEntry {
            date: p.day,
            provider: ProviderId::Codex,
            model: p.model,
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

impl HasDay for crate::usage_scanners::CodexModelDailyUsagePoint {
    fn day(&self) -> &str {
        &self.day
    }
}
