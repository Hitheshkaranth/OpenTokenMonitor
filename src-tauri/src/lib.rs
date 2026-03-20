use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItemBuilder};
use tauri::{AppHandle, Emitter, Manager, State};

mod providers;
mod usage;
mod usage_scanners;
mod watchers;

use providers::registry::ProviderRegistry;
use providers::FetchContext;
use usage::aggregator;
use usage::models::{
    AlertSeverity, CostEntry, ModelBreakdownEntry, ProviderId, ProviderStatus, RecentActivityEntry,
    RefreshCadence, TrendData, UsageAlert, UsageReport, UsageSnapshot,
};
use usage::store::UsageStore;
use watchers::poll_scheduler::PollScheduler;

// The tray handle lives in state because the tooltip is updated from multiple
// command and background-refresh paths after startup.
struct TrayState {
    _icon: Mutex<Option<tauri::tray::TrayIcon>>,
}

// AppState is the backend composition root: registered providers, persisted
// store, transient auth, and the active poll scheduler all live here.
pub struct AppState {
    registry: ProviderRegistry,
    store: UsageStore,
    api_keys: Mutex<HashMap<ProviderId, String>>,
    cadence: Mutex<RefreshCadence>,
    scheduler: PollScheduler,
}

impl AppState {
    fn new(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let db_path = data_dir.join("usage.db");
        let store = UsageStore::open(&db_path)?;
        let registry = ProviderRegistry::new();
        let provider_banner = registry
            .descriptors()
            .into_iter()
            .map(|d| format!("{}:{}({})", d.id.as_str(), d.display_name, d.brand_color))
            .collect::<Vec<_>>()
            .join(", ");
        println!("Loaded providers: {provider_banner}");

        Ok(Self {
            registry,
            store,
            api_keys: Mutex::new(HashMap::new()),
            cadence: Mutex::new(RefreshCadence::Every1m),
            scheduler: PollScheduler::new(),
        })
    }

    // Provider fetchers all need the same small set of auth/runtime inputs.
    fn fetch_context(&self) -> FetchContext {
        let api_keys = self.api_keys.lock().map(|g| g.clone()).unwrap_or_default();
        FetchContext {
            api_keys,
            allow_cookie_strategy: true,
        }
    }
}

#[tauri::command]
async fn get_usage_snapshot(
    provider: ProviderId,
    state: State<'_, AppState>,
) -> Result<UsageSnapshot, String> {
    // Cached snapshots keep the UI responsive; the explicit refresh commands are
    // the paths that force a live backend fetch.
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
async fn get_all_snapshots(state: State<'_, AppState>) -> Result<Vec<UsageSnapshot>, String> {
    let snapshots = state.store.get_all_snapshots()?;
    // First boot has no cache yet, so bootstrap by refreshing providers once.
    if snapshots.is_empty() {
        return aggregator::refresh_all(&state.registry, &state.store, &state.fetch_context())
            .await;
    }
    Ok(snapshots)
}

#[tauri::command]
async fn get_cost_history(
    provider: ProviderId,
    days: u32,
    state: State<'_, AppState>,
) -> Result<Vec<CostEntry>, String> {
    state.store.get_cost_history(provider, days)
}

#[tauri::command]
async fn get_usage_trends(
    provider: ProviderId,
    state: State<'_, AppState>,
) -> Result<TrendData, String> {
    state.store.get_usage_trends(provider, 30)
}

#[tauri::command]
async fn get_model_breakdown(
    provider: ProviderId,
    days: u32,
    state: State<'_, AppState>,
) -> Result<Vec<ModelBreakdownEntry>, String> {
    state.store.get_model_breakdown(provider, days)
}

#[tauri::command]
async fn get_recent_activity(
    provider: ProviderId,
    limit: u32,
) -> Result<Vec<RecentActivityEntry>, String> {
    Ok(usage_scanners::scan_recent_activity(
        provider,
        limit.max(1) as usize,
    ))
}

#[tauri::command]
async fn export_usage_report(days: u32, state: State<'_, AppState>) -> Result<UsageReport, String> {
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

#[tauri::command]
async fn refresh_provider(
    provider: ProviderId,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UsageSnapshot, String> {
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
async fn refresh_all(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<UsageSnapshot>, String> {
    let snapshots =
        aggregator::refresh_all(&state.registry, &state.store, &state.fetch_context()).await?;
    update_tray_tooltip(&app, &snapshots);
    let _ = app.emit("usage-updated", snapshots.clone());
    Ok(snapshots)
}

#[tauri::command]
async fn set_api_key(
    provider: ProviderId,
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut keys = state
        .api_keys
        .lock()
        .map_err(|_| "api key lock poisoned".to_string())?;
    keys.insert(provider, key);
    Ok(())
}

#[tauri::command]
async fn get_provider_status(
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
async fn set_refresh_cadence(
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
async fn get_launch_at_startup(app: AppHandle) -> Result<bool, String> {
    launch_at_startup_enabled(&app)
}

#[tauri::command]
async fn set_launch_at_startup(enabled: bool, app: AppHandle) -> Result<bool, String> {
    set_launch_at_startup_enabled(&app, enabled)
}

#[tauri::command]
async fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

fn launch_at_startup_enabled(app: &AppHandle) -> Result<bool, String> {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        use tauri_plugin_autostart::ManagerExt;

        app.autolaunch().is_enabled().map_err(|e| e.to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = app;
        Ok(false)
    }
}

fn set_launch_at_startup_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        use tauri_plugin_autostart::ManagerExt;

        let manager = app.autolaunch();
        if enabled {
            manager.enable().map_err(|e| e.to_string())?;
        } else {
            manager.disable().map_err(|e| e.to_string())?;
        }
        manager.is_enabled().map_err(|e| e.to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (app, enabled);
        Ok(false)
    }
}

fn is_autostart_launch() -> bool {
    std::env::args().any(|arg| arg == "--autostart")
}

fn restart_scheduler(app: &AppHandle, state: &AppState, cadence: RefreshCadence) {
    let app_handle = app.clone();
    state.scheduler.restart(cadence, move || {
        let app_inner = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let state_inner = app_inner.state::<AppState>();
            if let Ok(snapshots) = aggregator::refresh_all(
                &state_inner.registry,
                &state_inner.store,
                &state_inner.fetch_context(),
            )
            .await
            {
                update_tray_tooltip(&app_inner, &snapshots);
                let _ = app_inner.emit("usage-updated", snapshots);
            }
        });
    });
}

fn start_file_watchers(app: &AppHandle) {
    let app_handle = app.clone();
    // File-system changes are treated the same as manual refreshes: refresh the
    // affected provider, update the tray, then emit the new snapshot to the UI.
    let _handles = watchers::file_watcher::start(move |provider| {
        let app_inner = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_inner.state::<AppState>();
            if let Ok(snapshot) = aggregator::refresh_provider(
                &state.registry,
                &state.store,
                provider,
                &state.fetch_context(),
            )
            .await
            {
                if let Ok(all) = state.store.get_all_snapshots() {
                    update_tray_tooltip(&app_inner, &all);
                }
                let _ = app_inner.emit("usage-updated", snapshot);
            }
        });
    });
}

fn snapshot_percent(snapshot: &UsageSnapshot) -> f64 {
    snapshot
        .windows
        .first()
        .map(|w| w.utilization)
        .unwrap_or(0.0)
}

fn build_alerts(snapshots: &[UsageSnapshot]) -> Vec<UsageAlert> {
    let mut alerts = Vec::new();
    for snapshot in snapshots {
        for window in &snapshot.windows {
            let utilization = window.utilization.clamp(0.0, 100.0);
            let (threshold_percent, severity) = if utilization >= 95.0 {
                (95, Some(AlertSeverity::Critical))
            } else if utilization >= 90.0 {
                (90, Some(AlertSeverity::High))
            } else if utilization >= 75.0 {
                (75, Some(AlertSeverity::Warning))
            } else {
                (0, None)
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

fn format_window_label(window_type: usage::models::WindowType) -> &'static str {
    match window_type {
        usage::models::WindowType::FiveHour => "5h window",
        usage::models::WindowType::SevenDay => "7d window",
        usage::models::WindowType::Daily => "daily window",
        usage::models::WindowType::Monthly => "monthly window",
        usage::models::WindowType::Session => "session window",
        usage::models::WindowType::Weekly => "weekly window",
    }
}

fn format_tray_tooltip(snapshots: &[UsageSnapshot]) -> String {
    let mut claude = 0.0;
    let mut codex = 0.0;
    let mut gemini = 0.0;

    for snapshot in snapshots {
        let p = snapshot_percent(snapshot);
        match snapshot.provider {
            ProviderId::Claude => claude = p,
            ProviderId::Codex => codex = p,
            ProviderId::Gemini => gemini = p,
        }
    }

    format!(
        "OpenTokenMonitor\nClaude: {:.0}%  Codex: {:.0}%  Gemini: {:.0}%",
        claude, codex, gemini
    )
}

fn update_tray_tooltip(app: &AppHandle, snapshots: &[UsageSnapshot]) {
    let tooltip = format_tray_tooltip(snapshots);
    if let Some(tray_state) = app.try_state::<TrayState>() {
        if let Ok(mut icon_slot) = tray_state._icon.lock() {
            if let Some(icon) = icon_slot.as_mut() {
                let _ = icon.set_tooltip(Some(tooltip));
            }
        }
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let icon_bytes_png: &[u8] = include_bytes!("../../open_token_monitor_icon.png");
    let img = image::load_from_memory_with_format(icon_bytes_png, image::ImageFormat::Png)
        .map_err(|e| tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
        .to_rgba8();
    let (width, height) = image::GenericImageView::dimensions(&img);
    let tray_icon = Image::new_owned(img.into_raw(), width, height);

    let show_hide = MenuItemBuilder::new("Show / Hide")
        .id("show-hide")
        .build(app)?;
    let refresh = MenuItemBuilder::new("Refresh All")
        .id("refresh-all")
        .build(app)?;
    let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;
    let tray_menu = Menu::with_items(app, &[&show_hide, &refresh, &quit])?;

    let tray_icon = tauri::tray::TrayIconBuilder::new()
        .icon(tray_icon)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "refresh-all" => {
                let app_inner = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_inner.state::<AppState>();
                    if let Ok(snapshots) = aggregator::refresh_all(
                        &state.registry,
                        &state.store,
                        &state.fetch_context(),
                    )
                    .await
                    {
                        let _ = app_inner.emit("usage-updated", snapshots);
                    }
                });
            }
            "show-hide" => {
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    app.manage(TrayState {
        _icon: Mutex::new(Some(tray_icon)),
    });
    update_tray_tooltip(app.handle(), &[]);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use usage::models::{
        DataProvenance, DataSource, UsageUnit, UsageWindow, WindowAccuracy, WindowType,
    };

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            {
                use tauri_plugin_autostart::MacosLauncher;

                app.handle().plugin(tauri_plugin_autostart::init(
                    MacosLauncher::LaunchAgent,
                    Some(vec!["--autostart"]),
                ))?;
            }

            let state = AppState::new(app.handle())
                .map_err(|e| tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            app.manage(state);

            let launched_from_autostart = is_autostart_launch();
            if launched_from_autostart {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            setup_tray(app)?;
            start_file_watchers(app.handle());

            {
                let state_ref = app.state::<AppState>();
                restart_scheduler(app.handle(), &state_ref, RefreshCadence::Every1m);
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                if let Ok(snapshots) =
                    aggregator::refresh_all(&state.registry, &state.store, &state.fetch_context())
                        .await
                {
                    update_tray_tooltip(&app_handle, &snapshots);
                    let _ = app_handle.emit("usage-updated", snapshots);
                }
            });

            if !launched_from_autostart {
                // Ensure the main window is visible/focused even if previous tray mode hid it.
                let show_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
                    if let Some(window) = show_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_usage_snapshot,
            get_all_snapshots,
            get_cost_history,
            get_usage_trends,
            get_model_breakdown,
            get_recent_activity,
            export_usage_report,
            refresh_provider,
            refresh_all,
            set_api_key,
            get_provider_status,
            set_refresh_cadence,
            get_launch_at_startup,
            set_launch_at_startup,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
