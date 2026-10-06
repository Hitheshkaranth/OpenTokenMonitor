//! Time-to-limit forecasting from recorded utilization history.
//!
//! A snapshot says where a window is *now*; this module fits a line through
//! the last [`LOOKBACK_SECS`] of `snapshot_history` samples for that window
//! and projects when it reaches 100 %, so the UI and the alert engine can warn
//! "at this pace you'll hit the limit before it resets".

use chrono::Utc;
use serde::Serialize;
use tauri::State;

use crate::usage::models::{HistorySample, ProviderId, UsageSnapshot, WindowType};
use crate::usage::store::UsageStore;
use crate::AppState;

/// How far back the burn-rate fit looks.
pub const LOOKBACK_SECS: i64 = 30 * 60;
/// A utilization drop larger than this between consecutive samples means the
/// window reset.
const RESET_DROP_PCT: f64 = 5.0;
/// `resets_at` moving by more than this means a new window cycle.
const RESET_SHIFT_SECS: i64 = 120;
const MIN_SAMPLES: usize = 3;
const MIN_SPAN_SECS: i64 = 5 * 60;
/// Slower than this (percentage points per hour) counts as idle.
const MIN_RATE_PER_HOUR: f64 = 0.1;

#[derive(Debug, Clone, Serialize)]
pub struct WindowForecast {
    pub provider: ProviderId,
    pub window_type: WindowType,
    /// Latest utilization, percent.
    pub utilization: f64,
    /// Fitted burn rate in percentage points per hour.
    pub rate_per_hour: Option<f64>,
    pub eta_to_full_secs: Option<i64>,
    pub resets_in_secs: Option<i64>,
    pub will_hit_before_reset: bool,
    pub sample_count: usize,
}

/// Forecast one window from its history samples (any order).
pub fn forecast_window(
    provider: ProviderId,
    window_type: WindowType,
    samples: &[HistorySample],
    now_unix: i64,
) -> WindowForecast {
    let mut recent: Vec<&HistorySample> = samples
        .iter()
        .filter(|s| s.ts >= now_unix - LOOKBACK_SECS)
        .collect();
    recent.sort_by_key(|s| s.ts);

    // Keep only the current reset cycle: cut at the newest reset boundary.
    if let Some(newest) = recent.last().copied() {
        let mut start = 0;
        for i in (0..recent.len().saturating_sub(1)).rev() {
            let dropped = recent[i].utilization > recent[i + 1].utilization + RESET_DROP_PCT;
            let shifted = match (recent[i].resets_at, newest.resets_at) {
                (Some(a), Some(b)) => (a - b).abs() > RESET_SHIFT_SECS,
                _ => false,
            };
            if dropped || shifted {
                start = i + 1;
                break;
            }
        }
        recent.drain(..start);
    }

    let newest = recent.last().copied();
    let utilization = newest.map(|s| s.utilization).unwrap_or(0.0);
    let resets_in_secs = newest
        .and_then(|s| s.resets_at)
        .map(|r| (r - now_unix).max(0));

    let span = match (recent.first(), newest) {
        (Some(first), Some(last)) => last.ts - first.ts,
        _ => 0,
    };
    let slope_per_sec = if recent.len() >= MIN_SAMPLES && span >= MIN_SPAN_SECS {
        least_squares_slope(&recent)
    } else {
        None
    };
    let rate_per_hour = slope_per_sec.map(|s| s * 3600.0);

    let eta_to_full_secs = match (slope_per_sec, rate_per_hour) {
        (Some(slope), Some(rate)) if rate > MIN_RATE_PER_HOUR => {
            if utilization >= 100.0 {
                Some(0)
            } else {
                Some((((100.0 - utilization) / slope).round() as i64).max(0))
            }
        }
        _ => None,
    };
    let will_hit_before_reset = match eta_to_full_secs {
        Some(eta) => resets_in_secs.map_or(true, |r| eta < r),
        None => false,
    };

    WindowForecast {
        provider,
        window_type,
        utilization,
        rate_per_hour,
        eta_to_full_secs,
        resets_in_secs,
        will_hit_before_reset,
        sample_count: recent.len(),
    }
}

/// Ordinary least-squares slope of utilization over time (percent per second).
fn least_squares_slope(samples: &[&HistorySample]) -> Option<f64> {
    let n = samples.len() as f64;
    // Center timestamps so large unix values don't lose precision.
    let t0 = samples.first()?.ts as f64;
    let mean_t = samples.iter().map(|s| s.ts as f64 - t0).sum::<f64>() / n;
    let mean_u = samples.iter().map(|s| s.utilization).sum::<f64>() / n;
    let (mut num, mut den) = (0.0, 0.0);
    for s in samples {
        let dt = s.ts as f64 - t0 - mean_t;
        num += dt * (s.utilization - mean_u);
        den += dt * dt;
    }
    (den > 0.0).then(|| num / den)
}

/// Forecast every window of every snapshot that has recorded history.
pub fn forecasts_for_snapshots(
    store: &UsageStore,
    snapshots: &[UsageSnapshot],
    now_unix: i64,
) -> Vec<WindowForecast> {
    let mut out = Vec::new();
    for snapshot in snapshots {
        for window in &snapshot.windows {
            let Ok(samples) = store.get_snapshot_history(
                snapshot.provider,
                window.window_type,
                now_unix - LOOKBACK_SECS,
            ) else {
                continue;
            };
            if samples.is_empty() {
                continue;
            }
            out.push(forecast_window(
                snapshot.provider,
                window.window_type,
                &samples,
                now_unix,
            ));
        }
    }
    out
}

#[tauri::command]
pub fn get_limit_forecasts(state: State<'_, AppState>) -> Result<Vec<WindowForecast>, String> {
    let snapshots = state.store.get_all_snapshots()?;
    Ok(forecasts_for_snapshots(
        &state.store,
        &snapshots,
        Utc::now().timestamp(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn sample(secs_ago: i64, utilization: f64, resets_at: Option<i64>) -> HistorySample {
        HistorySample {
            ts: NOW - secs_ago,
            utilization,
            used: None,
            resets_at,
        }
    }

    fn forecast(samples: &[HistorySample]) -> WindowForecast {
        forecast_window(ProviderId::Claude, WindowType::FiveHour, samples, NOW)
    }

    /// One sample per minute for `minutes`, rising `per_min` points from `start`.
    fn rising(start: f64, per_min: f64, minutes: i64, resets_at: Option<i64>) -> Vec<HistorySample> {
        (0..=minutes)
            .map(|m| sample((minutes - m) * 60, start + per_min * m as f64, resets_at))
            .collect()
    }

    #[test]
    fn flat_usage_has_no_eta() {
        let f = forecast(&rising(40.0, 0.0, 10, None));
        assert!(f.rate_per_hour.unwrap().abs() < 1e-9);
        assert_eq!(f.eta_to_full_secs, None);
        assert!(!f.will_hit_before_reset);
    }

    #[test]
    fn steady_climb_projects_eta() {
        let f = forecast(&rising(40.0, 1.0, 10, None));
        assert!((f.rate_per_hour.unwrap() - 60.0).abs() < 0.01);
        assert_eq!(f.utilization, 50.0);
        assert_eq!(f.eta_to_full_secs, Some(50 * 60));
        assert!(f.will_hit_before_reset, "unknown reset counts as before-reset");
    }

    #[test]
    fn reset_drop_discards_previous_cycle() {
        let mut samples = rising(80.0, 1.0, 10, None);
        samples.truncate(6); // 80..85, 10..5 min ago
        samples.extend([sample(240, 2.0, None), sample(120, 2.0, None), sample(0, 2.0, None)]);
        let f = forecast(&samples);
        assert_eq!(f.sample_count, 3);
        assert_eq!(f.utilization, 2.0);
        assert_eq!(f.eta_to_full_secs, None);
    }

    #[test]
    fn resets_at_shift_discards_previous_cycle() {
        // Old cycle 20..10 min ago, then a new cycle (later resets_at) for the
        // last 2 minutes with no utilization drop large enough to look like a reset.
        let mut samples: Vec<_> = (10..=20)
            .rev()
            .map(|m| sample(m * 60, 10.0, Some(NOW + 100)))
            .collect();
        samples.extend(rising(10.0, 0.5, 2, Some(NOW + 18_000)));
        let f = forecast(&samples);
        assert_eq!(f.sample_count, 3);
        assert_eq!(f.rate_per_hour, None, "3 samples over ~2 min is too short a span");
    }

    #[test]
    fn too_few_samples_gives_no_rate() {
        let f = forecast(&[sample(600, 10.0, None), sample(0, 30.0, None)]);
        assert_eq!(f.rate_per_hour, None);
        assert_eq!(f.eta_to_full_secs, None);
        assert_eq!(f.utilization, 30.0);
    }

    #[test]
    fn ignores_samples_outside_lookback() {
        let mut samples = vec![sample(LOOKBACK_SECS + 60, 0.0, None)];
        samples.extend(rising(40.0, 0.0, 10, None));
        assert_eq!(forecast(&samples).sample_count, 11);
    }

    #[test]
    fn will_hit_before_reset_compares_eta_with_reset() {
        // ETA is 50 min.
        let far = forecast(&rising(40.0, 1.0, 10, Some(NOW + 3 * 3600)));
        assert!(far.will_hit_before_reset);
        assert_eq!(far.resets_in_secs, Some(3 * 3600));

        let soon = forecast(&rising(40.0, 1.0, 10, Some(NOW + 20 * 60)));
        assert!(!soon.will_hit_before_reset);
    }

    #[test]
    fn already_full_has_zero_eta() {
        let f = forecast(&rising(95.0, 1.0, 10, None));
        assert_eq!(f.eta_to_full_secs, Some(0));
    }
}
