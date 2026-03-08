use std::collections::HashMap;
use std::sync::Mutex;

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
use usage::models::{CostEntry, ProviderId, ProviderStatus, RefreshCadence, TrendData, UsageSnapshot};
use usage::store::UsageStore;
use watchers::poll_scheduler::PollScheduler;

struct TrayState {
    _icon: Mutex<Option<tauri::tray::TrayIcon>>,
}

pub struct AppState {
    registry: ProviderRegistry,
    store: UsageStore,
    api_keys: Mutex<HashMap<ProviderId, String>>,
    cadence: Mutex<RefreshCadence>,
    scheduler: PollScheduler,
}

impl AppState {
    fn new(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?;
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

    fn fetch_context(&self) -> FetchContext {
        let api_keys = self
            .api_keys
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        FetchContext {
            api_keys,
            allow_cookie_strategy: true,
        }
    }
}

#[tauri::command]
async fn get_usage_snapshot(provider: ProviderId, state: State<'_, AppState>) -> Result<UsageSnapshot, String> {
    if let Some(snapshot) = state.store.get_snapshot(provider)? {
        return Ok(snapshot);
    }
    aggregator::refresh_provider(&state.registry, &state.store, provider, &state.fetch_context()).await
}

#[tauri::command]
async fn get_all_snapshots(state: State<'_, AppState>) -> Result<Vec<UsageSnapshot>, String> {
    let snapshots = state.store.get_all_snapshots()?;
    if snapshots.is_empty() {
        return aggregator::refresh_all(&state.registry, &state.store, &state.fetch_context()).await;
    }
    Ok(snapshots)
}

#[tauri::command]
async fn get_cost_history(provider: ProviderId, days: u32, state: State<'_, AppState>) -> Result<Vec<CostEntry>, String> {
    state.store.get_cost_history(provider, days)
}

#[tauri::command]
async fn get_usage_trends(provider: ProviderId, state: State<'_, AppState>) -> Result<TrendData, String> {
    state.store.get_usage_trends(provider, 30)
}

#[tauri::command]
async fn refresh_provider(
    provider: ProviderId,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UsageSnapshot, String> {
    let snapshot = aggregator::refresh_provider(&state.registry, &state.store, provider, &state.fetch_context()).await?;
    let _ = app.emit("usage-updated", snapshot.clone());
    Ok(snapshot)
}

#[tauri::command]
async fn refresh_all(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<UsageSnapshot>, String> {
    let snapshots = aggregator::refresh_all(&state.registry, &state.store, &state.fetch_context()).await?;
    let _ = app.emit("usage-updated", snapshots.clone());
    Ok(snapshots)
}

#[tauri::command]
async fn set_api_key(provider: ProviderId, key: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut keys = state.api_keys.lock().map_err(|_| "api key lock poisoned".to_string())?;
    keys.insert(provider, key);
    Ok(())
}

#[tauri::command]
async fn get_provider_status(provider: ProviderId, state: State<'_, AppState>) -> Result<ProviderStatus, String> {
    let p = state
        .registry
        .get(provider)
        .ok_or_else(|| format!("Provider {provider:?} not found"))?;
    Ok(p.check_status().await)
}

#[tauri::command]
async fn set_refresh_cadence(cadence: RefreshCadence, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cadence_slot = state.cadence.lock().map_err(|_| "cadence lock poisoned".to_string())?;
        *cadence_slot = cadence;
    }
    restart_scheduler(&app, &state, cadence);
    Ok(())
}

#[tauri::command]
async fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
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
                let _ = app_inner.emit("usage-updated", snapshots);
            }
        });
    });
}

fn start_file_watchers(app: &AppHandle) {
    let app_handle = app.clone();
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
                let _ = app_inner.emit("usage-updated", snapshot);
            }
        });
    });
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let icon_bytes_png: &[u8] = include_bytes!("../../open_token_monitor_icon.png");
    let img = image::load_from_memory_with_format(icon_bytes_png, image::ImageFormat::Png)
        .map_err(|e| tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
        .to_rgba8();
    let (width, height) = image::GenericImageView::dimensions(&img);
    let tray_icon = Image::new_owned(img.into_raw(), width, height);

    let show_hide = MenuItemBuilder::new("Show / Hide").id("show-hide").build(app)?;
    let refresh = MenuItemBuilder::new("Refresh All").id("refresh-all").build(app)?;
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

    Ok(())
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
            let state = AppState::new(app.handle())
                .map_err(|e| tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            app.manage(state);

            setup_tray(app)?;
            start_file_watchers(app.handle());

            {
                let state_ref = app.state::<AppState>();
                restart_scheduler(app.handle(), &state_ref, RefreshCadence::Every1m);
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                if let Ok(snapshots) = aggregator::refresh_all(
                    &state.registry,
                    &state.store,
                    &state.fetch_context(),
                )
                .await
                {
                    let _ = app_handle.emit("usage-updated", snapshots);
                }
            });

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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_usage_snapshot,
            get_all_snapshots,
            get_cost_history,
            get_usage_trends,
            refresh_provider,
            refresh_all,
            set_api_key,
            get_provider_status,
            set_refresh_cadence,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
