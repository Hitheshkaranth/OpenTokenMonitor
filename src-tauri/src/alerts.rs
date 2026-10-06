//! Alert generation from usage snapshots.
//!
//! Converts per-window utilization percentages into [`UsageAlert`]s that the
//! frontend renders as warnings. The default ladder is fixed at 75 / 90 / 95 %
//! via [`ThresholdConfig`]; [`build_alerts_with_thresholds`] exposes the same
//! builder with per-provider overrides while [`build_alerts`] keeps the
//! original signature the frontend and tests depend on.

use crate::usage::models::{AlertSeverity, UsageAlert, UsageSnapshot, WindowType};

/// Threshold bands (utilization percentages) that map a window's utilization
/// to an [`AlertSeverity`].
///
/// A [`ThresholdConfig`] lets a caller override the default 75 / 90 / 95 %
/// ladder per-provider without changing the public [`build_alerts`] signature.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdConfig {
    /// Utilization at/above which a `Warning` is emitted.
    pub warning: u8,
    /// Utilization at/above which a `High` alert is emitted.
    pub high: u8,
    /// Utilization at/above which a `Critical` alert is emitted.
    pub critical: u8,
}

impl ThresholdConfig {
    /// The fixed ladder used by [`build_alerts`]: 75 / 90 / 95 %.
    pub fn fixed() -> Self {
        Self {
            warning: 75,
            high: 90,
            critical: 95,
        }
    }
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self::fixed()
    }
}

/// First-window utilization for tray summaries. Each provider's "primary"
/// window is its first one (by convention 5-hour for Claude/Antigravity, daily for
/// Codex), so we surface that for the at-a-glance tray tooltip.
pub fn snapshot_percent(snapshot: &UsageSnapshot) -> f64 {
    snapshot
        .windows
        .first()
        .map(|w| w.utilization)
        .unwrap_or(0.0)
}

/// Build alerts for every window that crosses a threshold band using the
/// supplied [`ThresholdConfig`] bands.
///
/// Bands (evaluated highest-to-lowest):
/// - `>= critical` → Critical, `threshold_percent = critical`
/// - `>= high`     → High, `threshold_percent = high`
/// - `>= warning`  → Warning, `threshold_percent = warning`
/// - below         → no alert
pub fn build_alerts_with_thresholds(snapshots: &[UsageSnapshot], thresholds: &ThresholdConfig) -> Vec<UsageAlert> {
    let mut alerts = Vec::new();
    for snapshot in snapshots {
        for window in &snapshot.windows {
            let utilization = window.utilization.clamp(0.0, 100.0);
            let (threshold_percent, severity): (u8, Option<AlertSeverity>) = match utilization {
                u if u >= thresholds.critical as f64 => (thresholds.critical, Some(AlertSeverity::Critical)),
                u if u >= thresholds.high as f64 => (thresholds.high, Some(AlertSeverity::High)),
                u if u >= thresholds.warning as f64 => (thresholds.warning, Some(AlertSeverity::Warning)),
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

/// Build alerts using the fixed 75 / 90 / 95 % ladder.
///
/// Thin wrapper over [`build_alerts_with_thresholds`] so existing callers keep
/// working without passing a [`ThresholdConfig`].
pub fn build_alerts(snapshots: &[UsageSnapshot]) -> Vec<UsageAlert> {
    build_alerts_with_thresholds(snapshots, &ThresholdConfig::default())
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
            snapshot_with_utilization(ProviderId::Antigravity, 96.0),
        ]);

        assert_eq!(alerts.len(), 3);
        assert_eq!(alerts[0].threshold_percent, 75);
        assert_eq!(alerts[1].threshold_percent, 90);
        assert_eq!(alerts[2].threshold_percent, 95);
    }

    #[test]
    fn build_alerts_uses_fixed_defaults() {
        let cfg = ThresholdConfig::default();
        assert_eq!(cfg.warning, 75);
        assert_eq!(cfg.high, 90);
        assert_eq!(cfg.critical, 95);
    }

    #[test]
    fn custom_thresholds_are_applied() {
        let custom = ThresholdConfig { warning: 60, high: 80, critical: 92 };
        let alerts = build_alerts_with_thresholds(
            &[
                snapshot_with_utilization(ProviderId::Claude, 65.0),
                snapshot_with_utilization(ProviderId::Codex, 85.0),
                snapshot_with_utilization(ProviderId::Antigravity, 93.0),
            ],
            &custom,
        );

        assert_eq!(alerts.len(), 3);
        assert_eq!(alerts[0].threshold_percent, 60);
        assert_eq!(alerts[1].threshold_percent, 80);
        assert_eq!(alerts[2].threshold_percent, 92);
    }
}
