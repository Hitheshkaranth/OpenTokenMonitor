//! Tauri command handlers — the public surface area called from the React
//! frontend via `invoke(...)`.
//!
//! Each handler is a thin coordinator: it pulls shared state, delegates to
//! the relevant module (provider registry, usage store, aggregator), and
//! emits `usage-updated` events when snapshots change so background polls and
//! manual refreshes both feed the same UI listener.
//!
//! Handlers in this file are registered in `lib.rs::run` via
//! `tauri::generate_handler![...]`.

use chrono::Utc;
use tauri::{AppHandle, Emitter, State};
use tracing::warn;

use crate::alerts::build_alerts;
use crate::autostart::{launch_at_startup_enabled, set_launch_at_startup_enabled};
use crate::providers::auth::AuthState;
use crate::tray::update_tray_tooltip;
use crate::usage::aggregator;
use crate::usage::models::{
    CostEntry, ModelBreakdownEntry, ProviderId, ProviderStatus, RecentActivityEntry,
    RefreshCadence, TrendData, UsageReport, UsageSnapshot,
};
use crate::usage_scanners;
use crate::{
    clear_persisted_api_key, persist_api_key, resolve_log_dir, restart_scheduler, AppState,
};

// ───────────────────────── Snapshot reads ─────────────────────────

#[tauri::command]
pub async fn get_usage_snapshot(
    provider: ProviderId,
    state: State<'_, AppState>,
) -> Result<UsageSnapshot, String> {
    // Cached snapshots keep the UI responsive; the explicit refresh commands
    // are the paths that force a live backend fetch.
    if let Some(snapshot) = state.store.get_snapshot(provider)? {
        return Ok(snapshot);
    }
    aggregator::refresh_provider(
        &state.registry,
        &state.store,
        provider,
        &state.fetch_context(),
    )
    .await
}

#[tauri::command]
pub async fn get_all_snapshots(state: State<'_, AppState>) -> Result<Vec<UsageSnapshot>, String> {
    let snapshots = state.store.get_all_snapshots()?;
    // First boot has no cache yet, so bootstrap by refreshing providers once.
    if snapshots.is_empty() {
        return aggregator::refresh_all(&state.registry, &state.store, &state.fetch_context())
            .await;
    }
    Ok(snapshots)
}

// ───────────────────────── History / trends ─────────────────────────

#[tauri::command]
pub async fn get_cost_history(
    provider: ProviderId,
    days: u32,
    state: State<'_, AppState>,
) -> Result<Vec<CostEntry>, String> {
    state.store.get_cost_history(provider, days)
}

#[tauri::command]
pub async fn get_usage_trends(
    provider: ProviderId,
    state: State<'_, AppState>,
) -> Result<TrendData, String> {
    state.store.get_usage_trends(provider, 30)
}

#[tauri::command]
pub async fn get_model_breakdown(
    provider: ProviderId,
    days: u32,
    state: State<'_, AppState>,
) -> Result<Vec<ModelBreakdownEntry>, String> {
    state.store.get_model_breakdown(provider, days)
}

#[tauri::command]
pub async fn get_recent_activity(
    provider: ProviderId,
    limit: u32,
) -> Result<Vec<RecentActivityEntry>, String> {
    Ok(usage_scanners::scan_recent_activity(
        provider,
        limit.max(1) as usize,
    ))
}

#[tauri::command]
pub async fn export_usage_report(
    days: u32,
    state: State<'_, AppState>,
) -> Result<UsageReport, String> {
    let snapshots = state.store.get_all_snapshots()?;
    let mut model_breakdowns = Vec::new();
    for provider in ProviderId::all() {
        model_breakdowns.extend(state.store.get_model_breakdown(provider, days.max(1))?);
    }

    Ok(UsageReport {
        generated_at: Utc::now(),
        alerts: build_alerts(&snapshots),
        snapshots,
        model_breakdowns,
    })
}

// ───────────────────────── Refresh ─────────────────────────

#[tauri::command]
pub async fn refresh_provider(
    provider: ProviderId,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UsageSnapshot, String> {
    usage_scanners::invalidate_activity_cache();
    let snapshot = aggregator::refresh_provider(
        &state.registry,
        &state.store,
        provider,
        &state.fetch_context(),
    )
    .await?;
    let _ = app.emit("usage-updated", snapshot.clone());
    if let Ok(all) = state.store.get_all_snapshots() {
        update_tray_tooltip(&app, &all);
    }
    Ok(snapshot)
}

#[tauri::command]
pub async fn refresh_all(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<UsageSnapshot>, String> {
    usage_scanners::invalidate_activity_cache();
    let snapshots =
        aggregator::refresh_all(&state.registry, &state.store, &state.fetch_context()).await?;
    update_tray_tooltip(&app, &snapshots);
    let _ = app.emit("usage-updated", snapshots.clone());
    Ok(snapshots)
}

// ───────────────────────── Settings / lifecycle ─────────────────────────

#[tauri::command]
pub async fn set_api_key(
    provider: ProviderId,
    key: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut keys = state
        .api_keys
        .lock()
        .map_err(|_| "api key lock poisoned".to_string())?;
    if key.trim().is_empty() {
        keys.remove(&provider);
        if let Err(err) = clear_persisted_api_key(&app, provider) {
            warn!(
                "failed to clear persisted api key for {}: {err}",
                provider.as_str()
            );
        }
        return Ok(());
    }

    keys.insert(provider, key.clone());
    if let Err(err) = persist_api_key(&app, provider, &key) {
        warn!(
            "failed to persist api key for {}: {err}",
            provider.as_str()
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_api_key(
    provider: ProviderId,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut keys = state
        .api_keys
        .lock()
        .map_err(|_| "api key lock poisoned".to_string())?;
    keys.remove(&provider);
    if let Err(err) = clear_persisted_api_key(&app, provider) {
        warn!(
            "failed to clear persisted api key for {}: {err}",
            provider.as_str()
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn get_provider_status(
    provider: ProviderId,
    state: State<'_, AppState>,
) -> Result<ProviderStatus, String> {
    let p = state
        .registry
        .get(provider)
        .ok_or_else(|| format!("Provider {provider:?} not found"))?;
    Ok(p.check_status().await)
}

#[tauri::command]
pub async fn get_auth_state(
    provider: ProviderId,
    state: State<'_, AppState>,
) -> Result<AuthState, String> {
    let p = state
        .registry
        .get(provider)
        .ok_or_else(|| format!("Provider {provider:?} not found"))?;
    Ok(p.compute_auth_state(&state.fetch_context()))
}

#[tauri::command]
pub fn get_log_directory(app: AppHandle) -> Result<String, String> {
    Ok(resolve_log_dir(Some(&app)).to_string_lossy().to_string())
}

#[tauri::command]
pub async fn set_refresh_cadence(
    cadence: RefreshCadence,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut cadence_slot = state
            .cadence
            .lock()
            .map_err(|_| "cadence lock poisoned".to_string())?;
        *cadence_slot = cadence;
    }
    restart_scheduler(&app, &state, cadence);
    Ok(())
}

#[tauri::command]
pub async fn get_launch_at_startup(app: AppHandle) -> Result<bool, String> {
    launch_at_startup_enabled(&app)
}

#[tauri::command]
pub async fn set_launch_at_startup(enabled: bool, app: AppHandle) -> Result<bool, String> {
    set_launch_at_startup_enabled(&app, enabled)
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
