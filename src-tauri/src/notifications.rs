//! Native OS notifications (macOS menu-bar popups) for usage-threshold alerts
//! and manually-triggered messages such as "budget reached".
//!
//! This module is intentionally panic-free: any failure to surface a
//! notification (permission prompts, platform quirks, or a missing runtime in
//! tests) is logged-and-ignored so the monitor's polling loop is never
//! disturbed by a UI concern.

use tauri::{AppHandle, State};
use tauri_plugin_notification::NotificationExt;
use tracing::warn;

use crate::alerts::ThresholdConfig;
use crate::usage::models::UsageAlert;
use crate::usage::models::{AlertSeverity, ProviderId};
use crate::AppState;

/// App display name used to anchor notifications to OpenTokenMonitor on macOS
/// and as a title prefix. Kept as a literal so this module stays
/// self-contained rather than depending on the Tauri config object.
const APP_DISPLAY_NAME: &str = "OpenTokenMonitor";

/// Returns every alert that belongs to `provider`.
fn provider_alerts(alerts: &[UsageAlert], provider: ProviderId) -> Vec<&UsageAlert> {
    alerts.iter().filter(|a| a.provider == provider).collect()
}

/// Fires one combined desktop notification for every alert belonging to
/// `provider`.
///
/// All alerts for a single provider collapse into a single pop-up (rather than
/// one per alert) so a burst of refreshes does not stack a long notification
/// list in the menu bar.
#[tauri::command]
pub fn notify_usage_alerts(
    app: AppHandle,
    provider: ProviderId,
    alerts: Vec<UsageAlert>,
) -> Result<(), String> {
    let provider_alerts = provider_alerts(&alerts, provider);
    if provider_alerts.is_empty() {
        return Ok(());
    }

    let mut body = String::new();
    for alert in &provider_alerts {
        body.push_str(&alert.message);
        body.push('\n');
    }

    let highest = provider_alerts
        .iter()
        .map(|alert| alert.severity)
        .fold(AlertSeverity::Warning, max_severity);
    let title = format!(
        "{APP_DISPLAY_NAME} — {} {}",
        provider.as_str(),
        severity_label(highest)
    );

    show_notification(&app, &title, &body);
    Ok(())
}

/// Fires a custom message for `provider` — used by the frontend to surface
/// threshold-driven messages such as a budget cap being reached.
///
/// The `message` argument is the entire notification body the user sees (for
/// example `"Monthly budget reached"`); the title is derived from the provider
/// name.
#[tauri::command]
pub fn notify_threshold_alert(
    app: AppHandle,
    provider: ProviderId,
    message: String,
) -> Result<(), String> {
    let title = format!("{APP_DISPLAY_NAME}: {} alert", provider.as_str());
    show_notification(&app, &title, &message);
    Ok(())
}

/// Surfaces a single desktop notification. Swallows and logs any failure so
/// callers never have to handle a notification error.
pub(crate) fn show_notification(app: &AppHandle, title: &str, body: &str) {
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        warn!("failed to show notification: {err}");
    }
}

/// Ordering for severity so the most urgent alert in a group wins.
fn max_severity(a: AlertSeverity, b: AlertSeverity) -> AlertSeverity {
    match (a, b) {
        (AlertSeverity::Critical, _) | (_, AlertSeverity::Critical) => AlertSeverity::Critical,
        (AlertSeverity::High, _) | (_, AlertSeverity::High) => AlertSeverity::High,
        _ => AlertSeverity::Warning,
    }
}

/// Snake_case label mirroring [`AlertSeverity`] for display in the title.
fn severity_label(severity: AlertSeverity) -> &'static str {
    match severity {
        AlertSeverity::Warning => "warning",
        AlertSeverity::High => "high",
        AlertSeverity::Critical => "critical",
    }
}

/// Persist a per-provider utilization alert ladder.
///
/// The frontend keeps its own copy in settings (persisted) and pushes it here
/// so reports and background alerts use the user's custom warning/high/critical
/// bands for that provider.
#[tauri::command]
pub fn set_thresholds(
    state: State<'_, AppState>,
    provider: ProviderId,
    warning: u8,
    high: u8,
    critical: u8,
) -> Result<(), String> {
    let config = ThresholdConfig {
        warning,
        high,
        critical,
    };
    let mut map = state
        .per_provider_thresholds
        .lock()
        .map_err(|_| "thresholds lock poisoned")?;
    map.insert(provider, config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::models::WindowType;

    fn alert_for(provider: ProviderId, utilization: f64, severity: AlertSeverity) -> UsageAlert {
        UsageAlert {
            provider,
            window_type: WindowType::Weekly,
            utilization,
            threshold_percent: 90,
            severity,
            message: format!("{} window reached {:.0}%", provider.as_str(), utilization),
        }
    }

    #[test]
    fn severity_label_matches_severity() {
        assert_eq!(severity_label(AlertSeverity::Warning), "warning");
        assert_eq!(severity_label(AlertSeverity::High), "high");
        assert_eq!(severity_label(AlertSeverity::Critical), "critical");
    }

    #[test]
    fn max_severity_prefers_critical_then_high() {
        assert_eq!(
            max_severity(AlertSeverity::Warning, AlertSeverity::High),
            AlertSeverity::High
        );
        assert_eq!(
            max_severity(AlertSeverity::High, AlertSeverity::Critical),
            AlertSeverity::Critical
        );
        assert_eq!(
            max_severity(AlertSeverity::Warning, AlertSeverity::Warning),
            AlertSeverity::Warning
        );
    }

    #[test]
    fn only_matching_provider_alerts_are_selected() {
        let alerts = vec![
            alert_for(ProviderId::Claude, 96.0, AlertSeverity::Critical),
            alert_for(ProviderId::Codex, 96.0, AlertSeverity::Critical),
        ];
        let claude = provider_alerts(&alerts, ProviderId::Claude);
        assert_eq!(claude.len(), 1);
        assert_eq!(claude[0].provider, ProviderId::Claude);

        let antigravity = provider_alerts(&alerts, ProviderId::Antigravity);
        assert!(antigravity.is_empty());
    }
}
