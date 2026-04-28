//! Alert generation from usage snapshots.
//!
//! Converts per-window utilization percentages into [`UsageAlert`]s that the
//! frontend renders as warnings. The thresholds (75 / 90 / 95 %) are
//! intentionally fixed here rather than configured per-provider so the alert
//! ladder stays predictable across the whole UI.

use crate::usage::models::{AlertSeverity, UsageAlert, UsageSnapshot, WindowType};

/// First-window utilization for tray summaries. Each provider's "primary"
/// window is its first one (by convention 5-hour for Claude, daily for
/// Codex/Gemini), so we surface that for the at-a-glance tray tooltip.
pub fn snapshot_percent(snapshot: &UsageSnapshot) -> f64 {
    snapshot
        .windows
        .first()
        .map(|w| w.utilization)
        .unwrap_or(0.0)
}

/// Build alerts for every window that crosses a threshold band.
///
/// Bands:
/// - `>= 95%` → Critical
/// - `>= 90%` → High
/// - `>= 75%` → Warning
/// - below   → no alert
pub fn build_alerts(snapshots: &[UsageSnapshot]) -> Vec<UsageAlert> {
    let mut alerts = Vec::new();
    for snapshot in snapshots {
        for window in &snapshot.windows {
            let utilization = window.utilization.clamp(0.0, 100.0);
            let (threshold_percent, severity) = match utilization {
                u if u >= 95.0 => (95, Some(AlertSeverity::Critical)),
                u if u >= 90.0 => (90, Some(AlertSeverity::High)),
                u if u >= 75.0 => (75, Some(AlertSeverity::Warning)),
                _ => (0, None),
            };

            let Some(severity) = severity else { continue };
            alerts.push(UsageAlert {
                provider: snapshot.provider,
                window_type: window.window_type,
                utilization,
                threshold_percent,
                severity,
                message: format!(
                    "{} {} reached {:.0}% (threshold {}%)",
                    snapshot.provider.as_str(),
                    format_window_label(window.window_type),
                    utilization,
                    threshold_percent
                ),
            });
        }
    }
    alerts
}

/// Human-friendly label for a window type, used in alert messages.
pub fn format_window_label(window_type: WindowType) -> &'static str {
    match window_type {
        WindowType::FiveHour => "5h window",
        WindowType::SevenDay => "7d window",
        WindowType::Daily => "daily window",
        WindowType::Monthly => "monthly window",
        WindowType::Session => "session window",
        WindowType::Weekly => "weekly window",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::models::{
        DataProvenance, DataSource, ProviderId, UsageUnit, UsageWindow, WindowAccuracy, WindowType,
    };
    use chrono::Utc;

    fn snapshot_with_utilization(provider: ProviderId, utilization: f64) -> UsageSnapshot {
        UsageSnapshot {
            provider,
            windows: vec![UsageWindow {
                window_type: WindowType::Weekly,
                utilization,
                used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                reset_countdown_secs: None,
                unit: UsageUnit::Percent,
                accuracy: WindowAccuracy::PercentOnly,
                note: None,
            }],
            credits: None,
            plan: None,
            fetched_at: Utc::now(),
            source: DataSource::LocalLog,
            provenance: DataProvenance::DerivedLocal,
            stale: false,
        }
    }

    #[test]
    fn build_alerts_respects_threshold_bands() {
        let alerts = build_alerts(&[
            snapshot_with_utilization(ProviderId::Claude, 76.0),
            snapshot_with_utilization(ProviderId::Codex, 91.0),
            snapshot_with_utilization(ProviderId::Gemini, 96.0),
        ]);

        assert_eq!(alerts.len(), 3);
        assert_eq!(alerts[0].threshold_percent, 75);
        assert_eq!(alerts[1].threshold_percent, 90);
        assert_eq!(alerts[2].threshold_percent, 95);
    }
}
