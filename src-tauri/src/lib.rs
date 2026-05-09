//! Backend composition root.
//!
//! This file wires the Tauri app together: it owns [`AppState`], registers
//! Tauri commands, starts the system tray, attaches file watchers, and starts
//! the background poll scheduler. The actual implementation of each piece
//! lives in a dedicated module:
//!
//! | Concern                       | Module           |
//! |-------------------------------|------------------|
//! | Tauri command handlers        | [`commands`]     |
//! | Tray icon, menu, tooltip      | [`tray`]         |
//! | Alert generation              | [`alerts`]       |
//! | OS launch-at-startup wrapper  | [`autostart`]    |
//! | Provider implementations      | [`providers`]    |
//! | Snapshot persistence (SQLite) | [`usage::store`] |
//! | Background refresh logic      | [`usage::aggregator`] |
//! | Local CLI artifact scanning   | [`usage_scanners`]|
//! | Filesystem + poll watchers    | [`watchers`]     |

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, EnvFilter, Registry};

mod alerts;
mod autostart;
mod commands;
mod pricing;
mod providers;
mod tray;
mod usage;
mod usage_scanners;
mod watchers;

use providers::registry::ProviderRegistry;
use providers::FetchContext;
use tray::update_tray_tooltip;
use usage::aggregator;
use usage::models::{ProviderId, RefreshCadence};
use usage::store::UsageStore;
use watchers::poll_scheduler::PollScheduler;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Backend composition root: registered providers, persisted store, transient
/// auth keys, and the active poll scheduler. One instance is created at
/// startup and stored in Tauri-managed state so commands can access it via
/// `State<'_, AppState>`.
pub struct AppState {
    pub registry: ProviderRegistry,
    pub store: UsageStore,
    pub api_keys: Mutex<HashMap<ProviderId, String>>,
    pub cadence: Mutex<RefreshCadence>,
    pub scheduler: PollScheduler,
}

impl AppState {
    fn new(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let db_path = data_dir.join("usage.db");
        let store = UsageStore::open(&db_path)?;
        let initial_keys = load_persisted_api_keys(app);
        let registry = ProviderRegistry::new();

        let provider_banner = registry
            .descriptors()
            .into_iter()
            .map(|d| format!("{}:{}({})", d.id.as_str(), d.display_name, d.brand_color))
            .collect::<Vec<_>>()
            .join(", ");
        info!("Loaded providers: {provider_banner}");

        Ok(Self {
            registry,
            store,
            api_keys: Mutex::new(initial_keys),
            cadence: Mutex::new(RefreshCadence::Every1m),
            scheduler: PollScheduler::new(),
        })
    }

    /// Build a [`FetchContext`] for live commands (CLI strategies allowed).
    pub fn fetch_context(&self) -> FetchContext {
        self.fetch_context_with_cli(true)
    }

    /// Build a [`FetchContext`] for refreshes triggered by file-watcher
    /// events. CLI strategies are disabled here because watcher-triggered
    /// refreshes can fire frequently, and CLI invocations are expensive +
    /// can themselves touch watched files (causing refresh loops).
    pub fn file_watcher_context(&self) -> FetchContext {
        self.fetch_context_with_cli(false)
    }

    fn fetch_context_with_cli(&self, allow_cli_strategy: bool) -> FetchContext {
        let api_keys = self.api_keys.lock().map(|g| g.clone()).unwrap_or_default();
        FetchContext {
            api_keys,
            allow_cookie_strategy: true,
            allow_cli_strategy,
        }
    }
}

// ─────────────────────────── secrets persistence ───────────────────────────

fn api_key_store_key(provider: ProviderId) -> String {
    format!("api_key.{}", provider.as_str())
}

fn secrets_store_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_default()
        .join("secrets.json")
}

fn load_persisted_api_keys(app: &AppHandle) -> HashMap<ProviderId, String> {
    let mut out = HashMap::new();
    let store_path = secrets_store_path(app);
    let Ok(store) = app.store(store_path) else {
        return out;
    };

    for provider in ProviderId::all() {
        let key = api_key_store_key(provider);
        if let Some(value) = store.get(&key).and_then(|v| v.as_str().map(str::to_string)) {
            if !value.trim().is_empty() {
                out.insert(provider, value);
            }
        }
    }
    out
}

pub(crate) fn persist_api_key(
    app: &AppHandle,
    provider: ProviderId,
    key: &str,
) -> Result<(), String> {
    let store = app
        .store(secrets_store_path(app))
        .map_err(|e| e.to_string())?;
    store.set(
        api_key_store_key(provider),
        serde_json::Value::String(key.to_string()),
    );
    store.save().map_err(|e| e.to_string())
}

pub(crate) fn clear_persisted_api_key(app: &AppHandle, provider: ProviderId) -> Result<(), String> {
    let store = app
        .store(secrets_store_path(app))
        .map_err(|e| e.to_string())?;
    store.delete(api_key_store_key(provider));
    store.save().map_err(|e| e.to_string())
}

// ─────────────────────────── tracing & panic hook ───────────────────────────

pub(crate) fn resolve_log_dir(app: Option<&AppHandle>) -> PathBuf {
    if let Some(app) = app {
        if let Ok(path) = app.path().app_data_dir() {
            return path.join("logs");
        }
    }
    if let Some(path) = dirs::data_dir() {
        return path.join("OpenTokenMonitor").join("logs");
    }
    std::env::temp_dir().join("OpenTokenMonitor").join("logs")
}

fn init_tracing() {
    let log_dir = resolve_log_dir(None);
    if let Err(err) = fs::create_dir_all(&log_dir) {
        let _ = writeln!(
            std::io::stderr(),
            "failed to create log dir {}: {err}",
            log_dir.display()
        );
        return;
    }

    let file_appender = tracing_appender::rolling::daily(&log_dir, "otm.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,open_token_monitor=info"));
    let console_layer = fmt::layer()
        .compact()
        .with_writer(std::io::stderr)
        .with_target(true);
    let file_layer = fmt::layer()
        .compact()
        .with_ansi(false)
        .with_writer(non_blocking);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer);
    let _ = tracing::subscriber::set_global_default(subscriber);
    info!("tracing initialized; log_dir={}", log_dir.display());
}

fn panic_log_path() -> PathBuf {
    if let Some(dir) = dirs::data_local_dir() {
        return dir.join("OpenTokenMonitor").join("last_panic.log");
    }
    if let Some(home) = dirs::home_dir() {
        return home
            .join(".local")
            .join("share")
            .join("OpenTokenMonitor")
            .join("last_panic.log");
    }
    std::env::temp_dir().join("OpenTokenMonitor_last_panic.log")
}

fn append_panic_log(message: &str) {
    let path = panic_log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{message}");
    }
}

// ─────────────────────────── scheduler & watchers ───────────────────────────

/// Restart the cadence-based poll scheduler with the given cadence.
///
/// Each tick refreshes every provider, updates the tray tooltip, and emits
/// `usage-updated` so the frontend re-renders. Restarting cancels any
/// in-flight timer; concurrent ticks are not possible by construction.
pub fn restart_scheduler(app: &AppHandle, state: &AppState, cadence: RefreshCadence) {
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

/// Attach filesystem watchers for each provider's local artifact directories.
///
/// File-system events are treated the same as manual refreshes: refresh the
/// affected provider, update the tray, then emit the new snapshot. CLI
/// strategies are disabled (see [`AppState::file_watcher_context`]) to avoid
/// refresh loops.
fn start_file_watchers(app: &AppHandle) {
    let app_handle = app.clone();
    let _handles = watchers::file_watcher::start(move |provider| {
        let app_inner = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            usage_scanners::invalidate_activity_cache();
            let state = app_inner.state::<AppState>();
            if let Ok(snapshot) = aggregator::refresh_provider(
                &state.registry,
                &state.store,
                provider,
                &state.file_watcher_context(),
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|panic_info| {
        let now = chrono::Utc::now().to_rfc3339();
        let thread = std::thread::current()
            .name()
            .map(str::to_string)
            .unwrap_or_else(|| "unnamed".to_string());
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_string()
        };
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let backtrace = std::backtrace::Backtrace::force_capture();
        let log_line = format!(
            "[{now}] panic thread={thread} location={location} payload={payload}\nbacktrace:\n{backtrace}\n"
        );
        append_panic_log(&log_line);
        eprintln!("{log_line}");
    }));

    init_tracing();

    let mut builder = tauri::Builder::default();

    // Single-instance enforcement: when a second copy launches (e.g. user
    // clicks the shortcut while the autostart copy is already running in the
    // tray), focus the existing instance instead of spawning a duplicate.
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    let run_result = builder
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
                .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?;
            app.manage(state);

            let launched_from_autostart = autostart::is_autostart_launch();
            if launched_from_autostart {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            tray::setup_tray(app)?;
            start_file_watchers(app.handle());

            {
                let state_ref = app.state::<AppState>();
                restart_scheduler(app.handle(), &state_ref, RefreshCadence::Every1m);
            }

            // Initial bootstrap refresh so the UI has data on first paint.
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

            // For non-autostart launches, force-show the main window after a
            // brief delay. The delay covers edge cases where an earlier
            // hide() (e.g. previous autostart session) is still in flight.
            if !launched_from_autostart {
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
            commands::get_usage_snapshot,
            commands::get_all_snapshots,
            commands::get_cost_history,
            commands::get_usage_trends,
            commands::get_model_breakdown,
            commands::get_recent_activity,
            commands::export_usage_report,
            commands::refresh_provider,
            commands::refresh_all,
            commands::set_api_key,
            commands::clear_api_key,
            commands::get_provider_status,
            commands::get_auth_state,
            commands::get_log_directory,
            commands::set_refresh_cadence,
            commands::get_launch_at_startup,
            commands::set_launch_at_startup,
            commands::quit_app,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = run_result {
        let now = chrono::Utc::now().to_rfc3339();
        let log_line = format!("[{now}] tauri runtime error: {err}");
        append_panic_log(&log_line);
        error!("{log_line}");
    }
}
