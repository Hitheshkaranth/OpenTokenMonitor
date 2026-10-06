//! Backend-owned desktop notifications.
//!
//! Runs after every refresh (see [`crate::post_refresh`]), so alerts fire even
//! while the window is hidden. Each provider window remembers the highest
//! severity it has already notified for its current reset cycle, so a
//! notification fires once per real escalation (warning → high → critical)
//! instead of on every poll. Also raises a one-off "on pace to hit the limit
//! before reset" warning and a projected-budget-overrun warning.

use std::collections::HashMap;

use chrono::Utc;
use tauri::{AppHandle, Manager};

use crate::alerts::{format_window_label, ThresholdConfig};
use crate::budgets;
use crate::limit_forecast::{self, LOOKBACK_SECS};
use crate::notifications::show_notification;
use crate::usage::models::{AlertSeverity, ProviderId, UsageSnapshot, WindowType};
use crate::AppState;

/// Pace warnings only fire when the projected time-to-full is this close.
const ETA_WARN_SECS: i64 = 30 * 60;
/// `resets_at` moving by more than this means the window started a new cycle.
const RESET_SHIFT_SECS: i64 = 120;

/// What has already been notified for one provider window in its current cycle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NotifiedState {
    /// 0 = nothing, 1 = warning, 2 = high, 3 = critical.
    pub level: u8,
    pub resets_at: Option<i64>,
    pub eta_notified: bool,
}

pub type AlertMemory = HashMap<(ProviderId, WindowType), NotifiedState>;

pub fn level_for(utilization: f64, t: &ThresholdConfig) -> u8 {
    match utilization {
        u if u >= t.critical as f64 => 3,
        u if u >= t.high as f64 => 2,
        u if u >= t.warning as f64 => 1,
        _ => 0,
    }
}

fn severity_for(level: u8) -> Option<AlertSeverity> {
    match level {
        1 => Some(AlertSeverity::Warning),
        2 => Some(AlertSeverity::High),
        3 => Some(AlertSeverity::Critical),
        _ => None,
    }
}

/// Advance a window's notified state and return the severity to notify, if
/// any. Re-arms when the window resets (`resets_at` moves) or utilization
/// falls back under the warning line; never repeats a level already sent.
pub fn decide(
    prev: Option<NotifiedState>,
    utilization: f64,
    resets_at: Option<i64>,
    t: &ThresholdConfig,
) -> (NotifiedState, Option<AlertSeverity>) {
    let mut state = prev.unwrap_or_default();
    let new_cycle = matches!(
        (state.resets_at, resets_at),
        (Some(a), Some(b)) if (a - b).abs() > RESET_SHIFT_SECS
    );
    if new_cycle || utilization < t.warning as f64 {
        state = NotifiedState::default();
    }
    state.resets_at = resets_at;

    let level = level_for(utilization, t);
    if level > state.level {
        state.level = level;
        (state, severity_for(level))
    } else {
        (state, None)
    }
}

pub fn should_notify_eta(
    state: &NotifiedState,
    utilization: f64,
    eta_to_full_secs: Option<i64>,
    will_hit_before_reset: bool,
) -> bool {
    !state.eta_notified
        && will_hit_before_reset
        && utilization < 100.0
        && eta_to_full_secs.is_some_and(|eta| eta <= ETA_WARN_SECS)
}

pub fn format_eta_minutes(secs: i64) -> String {
    if secs < 60 {
        "<1 min".to_string()
    } else {
        format!("~{} min", (secs as f64 / 60.0).round() as i64)
    }
}

fn provider_name(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Claude => "Claude",
        ProviderId::Codex => "Codex",
        ProviderId::Antigravity => "Antigravity",
    }
}

fn severity_word(severity: AlertSeverity) -> &'static str {
    match severity {
        AlertSeverity::Warning => "warning",
        AlertSeverity::High => "high usage",
        AlertSeverity::Critical => "critical",
    }
}

struct PendingNotification {
    title: String,
    body: String,
}

/// Evaluate thresholds, pace and budgets for the latest snapshots and fire any
/// new notifications. Memory is advanced even while notifications are off, so
/// turning them back on doesn't replay alerts that were already crossed.
pub fn evaluate(app: &AppHandle, snapshots: &[UsageSnapshot]) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let enabled = state.notifications_enabled.lock().map(|g| *g).unwrap_or(false);
    let thresholds = state
        .per_provider_thresholds
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let now = Utc::now().timestamp();
    let mut pending = Vec::<PendingNotification>::new();

    // Read history before taking the memory lock: the store has its own mutex.
    let forecasts: HashMap<(ProviderId, WindowType), limit_forecast::WindowForecast> = snapshots
        .iter()
        .flat_map(|s| s.windows.iter().map(move |w| (s.provider, w.window_type)))
        .filter_map(|(provider, window_type)| {
            let samples = state
                .store
                .get_snapshot_history(provider, window_type, now - LOOKBACK_SECS)
                .ok()?;
            Some((
                (provider, window_type),
                limit_forecast::forecast_window(provider, window_type, &samples, now),
            ))
        })
        .collect();

    if let Ok(mut memory) = state.alert_memory.lock() {
        for snapshot in snapshots {
            let t = thresholds
                .get(&snapshot.provider)
                .copied()
                .unwrap_or_default();
            for window in &snapshot.windows {
                let key = (snapshot.provider, window.window_type);
                let resets_at = window.resets_at.map(|d| d.timestamp());
                let (mut next, fired) =
                    decide(memory.get(&key).copied(), window.utilization, resets_at, &t);
                let label = format_window_label(window.window_type);
                let name = provider_name(snapshot.provider);

                if let (Some(severity), false) = (fired, snapshot.stale) {
                    pending.push(PendingNotification {
                        title: format!("OpenTokenMonitor — {name} {}", severity_word(severity)),
                        body: format!("{label} at {:.0}%", window.utilization),
                    });
                }

                if let Some(fc) = forecasts.get(&key) {
                    if should_notify_eta(
                        &next,
                        window.utilization,
                        fc.eta_to_full_secs,
                        fc.will_hit_before_reset,
                    ) {
                        next.eta_notified = true;
                        if !snapshot.stale {
                            let eta = format_eta_minutes(fc.eta_to_full_secs.unwrap_or(0));
                            pending.push(PendingNotification {
                                title: format!("OpenTokenMonitor — {name} pace warning"),
                                body: format!(
                                    "{label} will hit its limit in {eta} at the current pace, before it resets"
                                ),
                            });
                        }
                    }
                }
                memory.insert(key, next);
            }
        }
    }

    for budget in budgets::load_budgets(app) {
        if budget.amount_usd <= 0.0 {
            continue;
        }
        let forecast = budgets::compute_forecast(
            &state.store,
            Some(&budget),
            budget.provider,
            budget.period_days,
        );
        let Ok(mut notified) = state.budget_notified.lock() else {
            break;
        };
        let was = notified.get(&budget.provider).copied().unwrap_or(false);
        notified.insert(budget.provider, forecast.projected_exceeds_budget);
        if forecast.projected_exceeds_budget && !was {
            pending.push(PendingNotification {
                title: format!("OpenTokenMonitor — {} budget", provider_name(budget.provider)),
                body: format!(
                    "Projected ${:.2} exceeds the ${:.2} budget over {}d",
                    forecast.projected_spend_usd, budget.amount_usd, budget.period_days
                ),
            });
        }
    }

    if enabled {
        for n in pending {
            show_notification(app, &n.title, &n.body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> ThresholdConfig {
        ThresholdConfig::default() // 75 / 90 / 95
    }

    fn step(prev: Option<NotifiedState>, u: f64, resets_at: Option<i64>) -> (NotifiedState, Option<AlertSeverity>) {
        decide(prev, u, resets_at, &t())
    }

    #[test]
    fn warning_fires_once() {
        let (s, fired) = step(None, 76.0, Some(1000));
        assert_eq!(fired, Some(AlertSeverity::Warning));
        let (_, again) = step(Some(s), 76.0, Some(1000));
        assert_eq!(again, None);
    }

    #[test]
    fn escalation_fires_each_new_level_only() {
        let (s, _) = step(None, 76.0, None);
        let (s, fired) = step(Some(s), 91.0, None);
        assert_eq!(fired, Some(AlertSeverity::High));
        let (s, fired) = step(Some(s), 80.0, None);
        assert_eq!(fired, None, "dropping back within warning band stays quiet");
        let (s, fired) = step(Some(s), 97.0, None);
        assert_eq!(fired, Some(AlertSeverity::Critical));
        assert_eq!(s.level, 3);
    }

    #[test]
    fn jumping_straight_to_critical_fires_critical() {
        assert_eq!(step(None, 99.0, None).1, Some(AlertSeverity::Critical));
    }

    #[test]
    fn reset_rearms() {
        let (s, _) = step(None, 91.0, Some(1000));
        let (s, fired) = step(Some(s), 76.0, Some(1000 + 18_000));
        assert_eq!(fired, Some(AlertSeverity::Warning));
        assert_eq!(s.resets_at, Some(19_000));
    }

    #[test]
    fn small_resets_at_jitter_does_not_rearm() {
        let (s, _) = step(None, 76.0, Some(1000));
        assert_eq!(step(Some(s), 76.0, Some(1060)).1, None);
    }

    #[test]
    fn falling_below_warning_rearms() {
        let (s, _) = step(None, 76.0, None);
        let (s, fired) = step(Some(s), 10.0, None);
        assert_eq!(fired, None);
        assert_eq!(s.level, 0);
        assert_eq!(step(Some(s), 76.0, None).1, Some(AlertSeverity::Warning));
    }

    #[test]
    fn reset_clears_eta_flag() {
        let s = NotifiedState { level: 1, resets_at: Some(1000), eta_notified: true };
        let (s, _) = step(Some(s), 80.0, Some(50_000));
        assert!(!s.eta_notified);
    }

    #[test]
    fn eta_notification_rules() {
        let fresh = NotifiedState::default();
        assert!(should_notify_eta(&fresh, 60.0, Some(600), true));
        assert!(!should_notify_eta(&fresh, 60.0, Some(600), false), "resets first");
        assert!(!should_notify_eta(&fresh, 60.0, Some(3 * 3600), true), "too far out");
        assert!(!should_notify_eta(&fresh, 60.0, None, true));
        assert!(!should_notify_eta(&fresh, 100.0, Some(0), true), "already full");
        let sent = NotifiedState { eta_notified: true, ..fresh };
        assert!(!should_notify_eta(&sent, 60.0, Some(600), true));
    }

    #[test]
    fn eta_minutes_format() {
        assert_eq!(format_eta_minutes(30), "<1 min");
        assert_eq!(format_eta_minutes(90), "~2 min");
        assert_eq!(format_eta_minutes(3600), "~60 min");
    }
}
