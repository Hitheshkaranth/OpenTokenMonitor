use crate::usage::models::{CostEntry, ProviderId};
use crate::usage_scanners::{scan_codex_daily_usage, scan_codex_model_daily_usage};

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
            // Codex reports `input_tokens` *including* the cached part. Store
            // only the uncached remainder (as Claude rows do) so summing
            // input + cache_read doesn't count cached tokens twice.
            input_tokens: p.input_tokens.saturating_sub(p.cached_input_tokens),
            output_tokens: p.output_tokens,
            cache_read_tokens: p.cached_input_tokens,
            cache_write_tokens: 0,
            estimated_cost_usd: p.cost_usd,
        })
        .collect()
}

pub fn usage_windows() -> (u64, u64) {
    // Anchor on UTC today (as the Claude parser does), not on the most recent
    // day with data: weeks-old usage must not show up as the current session.
    let by_day: std::collections::BTreeMap<String, u64> = scan_codex_daily_usage()
        .into_iter()
        .map(|p| (p.day, p.total_tokens))
        .collect();
    let now = chrono::Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let week_cutoff = (now - chrono::Duration::days(6))
        .format("%Y-%m-%d")
        .to_string();
    let session = by_day.get(&today).copied().unwrap_or(0);
    let weekly = by_day
        .range(week_cutoff..=today)
        .map(|(_, v)| *v)
        .sum::<u64>();
    (session, weekly)
}

fn retain_recent_days<T>(points: &mut Vec<T>, days: u32)
where
    T: HasDay,
{
    if days == 0 {
        return;
    }

    // Calendar window (today and the previous `days - 1` UTC days), not "the
    // last N days that happen to have data".
    let cutoff = (chrono::Utc::now().date_naive() - chrono::Duration::days(days as i64 - 1))
        .format("%Y-%m-%d")
        .to_string();
    points.retain(|point| point.day() >= cutoff.as_str());
}

trait HasDay {
    fn day(&self) -> &str;
}

impl HasDay for crate::usage_scanners::CodexModelDailyUsagePoint {
    fn day(&self) -> &str {
        &self.day
    }
}
