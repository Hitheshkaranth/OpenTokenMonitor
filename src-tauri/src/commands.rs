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

use crate::alerts::{build_alerts_with_thresholds, ThresholdConfig};
use crate::autostart::{launch_at_startup_enabled, set_launch_at_startup_enabled};
use crate::post_refresh::on_snapshots_updated;
use crate::providers::auth::AuthState;
use crate::tray::TrayTitleMode;
use crate::usage::aggregator;
use crate::usage::models::{
    CostEntry, ModelBreakdownEntry, ProviderId, ProviderStatus, RecentActivityEntry,
    RefreshCadence, TrendData, UsageReport, UsageSnapshot,
};
use crate::usage_scanners;
use crate::{
    clear_persisted_api_key, persist_api_key, resolve_log_dir, restart_per_provider_scheduler,
    restart_scheduler, AppState,
};

// ───────────────────────── Updater ─────────────────────────

/// Placeholder shipped in `tauri.conf.json` before a real signing keypair is
/// generated. See BUILDING.md → "Release signing".
const UPDATER_PUBKEY_PLACEHOLDER: &str = "PUBKEY_PLACEHOLDER";

/// Whether this build has a real updater signing key configured.
///
/// The frontend checks this before calling `check()`. Without it, a build with
/// an unconfigured key fires a doomed request at the release endpoint on every
/// launch and surfaces a console error the user can do nothing about.
#[tauri::command]
pub fn is_updater_configured(app: AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|cfg| cfg.get("pubkey"))
        .and_then(|key| key.as_str())
        .map(|key| {
            let key = key.trim();
            !key.is_empty() && key != UPDATER_PUBKEY_PLACEHOLDER
        })
        .unwrap_or(false)
}

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
    days: u32,
    state: State<'_, AppState>,
) -> Result<TrendData, String> {
    state.store.get_usage_trends(provider, days.max(1))
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

    // Build alerts per-provider so a user's custom threshold ladder applies to
    // reports (falling back to the fixed 75/90/95 ladder when a provider has no
    // configured thresholds).
    let thresholds = state
        .per_provider_thresholds
        .lock()
        .map_err(|_| "thresholds lock poisoned")?;
    let mut alerts = Vec::new();
    for provider in ProviderId::all() {
        let provider_snapshots: Vec<_> = snapshots
            .iter()
            .filter(|snapshot| snapshot.provider == provider)
            .cloned()
            .collect();
        let config = thresholds
            .get(&provider)
            .cloned()
            .unwrap_or_else(ThresholdConfig::default);
        alerts.extend(build_alerts_with_thresholds(&provider_snapshots, &config));
    }

    Ok(UsageReport {
        generated_at: Utc::now(),
        alerts,
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
        on_snapshots_updated(&app, &all);
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
    on_snapshots_updated(&app, &snapshots);
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
        warn!("failed to persist api key for {}: {err}", provider.as_str());
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
pub async fn set_per_provider_cadence(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: ProviderId,
    cadence: RefreshCadence,
) -> Result<(), String> {
    {
        let mut cadence_slot = state
            .per_provider_cadence
            .lock()
            .map_err(|_| "cadence lock poisoned".to_string())?;
        cadence_slot.insert(provider, cadence);
    }
    let app_handle = app.clone();
    restart_per_provider_scheduler(&app_handle, &state);
    Ok(())
}

#[tauri::command]
pub async fn get_per_provider_cadence(
    provider: ProviderId,
    state: State<'_, AppState>,
) -> Result<RefreshCadence, String> {
    let cadence = state
        .per_provider_cadence
        .lock()
        .map_err(|_| "cadence lock poisoned".to_string())?
        .get(&provider)
        .copied()
        .unwrap_or(
            *state
                .cadence
                .lock()
                .map_err(|_| "cadence lock poisoned".to_string())?,
        );
    Ok(cadence)
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

// ───────────────────────── Menu bar & notifications ─────────────────────────

/// Async so it runs off the main thread: re-rendering the tray waits on the
/// main thread, which must stay free.
#[tauri::command]
pub async fn set_tray_title_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: TrayTitleMode,
) -> Result<(), String> {
    *state
        .tray_title_mode
        .lock()
        .map_err(|_| "tray title lock poisoned")? = mode;
    crate::post_refresh::rerender_tray_title(&app);
    Ok(())
}

#[tauri::command]
pub fn get_tray_title_mode(state: State<'_, AppState>) -> Result<TrayTitleMode, String> {
    Ok(*state
        .tray_title_mode
        .lock()
        .map_err(|_| "tray title lock poisoned")?)
}

/// Backend notifications (threshold escalations, pace and budget warnings)
/// honour this switch; see `alert_engine`.
#[tauri::command]
pub fn set_notifications_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    *state
        .notifications_enabled
        .lock()
        .map_err(|_| "notifications lock poisoned")? = enabled;
    Ok(())
}
