//! The single hook every refresh path calls once it has fresh snapshots:
//! bootstrap, poll schedulers, file watchers, tray "Refresh All" and the
//! manual refresh commands. Keeps the tray tooltip, the menu-bar title and the
//! backend alert engine in step no matter which path produced the data.

use chrono::Utc;
use tauri::{AppHandle, Manager};

use crate::usage::models::{ProviderId, UsageSnapshot};
use crate::{alert_engine, tray, AppState};

pub fn on_snapshots_updated(app: &AppHandle, snapshots: &[UsageSnapshot]) {
    tray::update_tray_tooltip(app, snapshots);
    render_tray_title(app, snapshots);
    alert_engine::evaluate(app, snapshots);
}

/// Re-render the menu-bar title from the stored snapshots, e.g. after the
/// title mode changes.
pub fn rerender_tray_title(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if let Ok(snapshots) = state.store.get_all_snapshots() {
        render_tray_title(app, &snapshots);
    }
}

fn render_tray_title(app: &AppHandle, snapshots: &[UsageSnapshot]) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let mode = state.tray_title_mode.lock().map(|g| *g).unwrap_or_default();
    let today_cost = if mode == tray::TrayTitleMode::Cost {
        today_cost_usd(&state)
    } else {
        0.0
    };
    tray::set_tray_title(app, tray::format_tray_title(mode, snapshots, today_cost));
}

/// Today's estimated spend across providers. Cost rows are keyed by UTC day,
/// matching how the log scanners bucket usage.
fn today_cost_usd(state: &AppState) -> f64 {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    ProviderId::all()
        .into_iter()
        .filter_map(|p| state.store.get_cost_history(p, 1).ok())
        .flatten()
        .filter(|e| e.date == today)
        .map(|e| e.estimated_cost_usd)
        .sum()
}
