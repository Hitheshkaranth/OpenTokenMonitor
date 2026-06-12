//! System-tray icon, menu, and tooltip.
//!
//! The tray is the app's only persistent UI surface when the main window is
//! hidden (autostart launch, minimize-to-tray). It exposes:
//! - a left-click toggle for show/hide of the main window
//! - a right-click menu (Show/Hide, Refresh All, Quit)
//! - a tooltip showing each provider's primary-window utilization
//!
//! The tray icon handle lives inside [`TrayState`] (kept in Tauri-managed
//! state) because the tooltip is updated from multiple paths after startup.

use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItemBuilder};
use tauri::{AppHandle, Emitter, Manager};

use crate::alerts::snapshot_percent;
use crate::usage::aggregator;
use crate::usage::models::{ProviderId, UsageSnapshot};
use crate::usage_scanners;
use crate::AppState;

/// Tauri-managed wrapper around the tray icon handle so we can mutate it from
/// any command or background task.
pub struct TrayState {
    pub icon: Mutex<Option<tauri::tray::TrayIcon>>,
}

/// Build the tooltip line shown on tray hover. Always lists all three
/// providers in a fixed order so the layout is stable.
fn format_tray_tooltip(snapshots: &[UsageSnapshot]) -> String {
    let mut claude = 0.0;
    let mut codex = 0.0;
    let mut antigravity = 0.0;

    for snapshot in snapshots {
        let p = snapshot_percent(snapshot);
        match snapshot.provider {
            ProviderId::Claude => claude = p,
            ProviderId::Codex => codex = p,
            ProviderId::Antigravity => antigravity = p,
        }
    }

    format!(
        "OpenTokenMonitor\nClaude: {:.0}%  Codex: {:.0}%  Antigravity: {:.0}%",
        claude, codex, antigravity
    )
}

/// Replace the tray tooltip with the latest provider utilizations.
pub fn update_tray_tooltip(app: &AppHandle, snapshots: &[UsageSnapshot]) {
    let tooltip = format_tray_tooltip(snapshots);
    if let Some(tray_state) = app.try_state::<TrayState>() {
        if let Ok(mut icon_slot) = tray_state.icon.lock() {
            if let Some(icon) = icon_slot.as_mut() {
                let _ = icon.set_tooltip(Some(tooltip));
            }
        }
    }
}

/// Toggle the main window's visibility. Used by both the menu item and the
/// tray icon left-click handler.
fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Trigger a refresh of every provider in the background and emit the result
/// to the frontend. Runs on its own task so the tray click stays responsive.
fn spawn_refresh_all(app: &AppHandle) {
    let app_inner = app.clone();
    tauri::async_runtime::spawn(async move {
        usage_scanners::invalidate_activity_cache();
        let state = app_inner.state::<AppState>();
        if let Ok(snapshots) =
            aggregator::refresh_all(&state.registry, &state.store, &state.fetch_context()).await
        {
            let _ = app_inner.emit("usage-updated", snapshots);
        }
    });
}

/// Build the tray icon, menu, and click handlers, then store the icon in
/// Tauri-managed state so tooltip updates can find it later.
pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let icon_bytes_png: &[u8] = include_bytes!("../../open_token_monitor_icon.png");
    let img = image::load_from_memory_with_format(icon_bytes_png, image::ImageFormat::Png)
        .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?
        .to_rgba8();
    let (width, height) = image::GenericImageView::dimensions(&img);
    let tray_icon_image = Image::new_owned(img.into_raw(), width, height);

    let show_hide = MenuItemBuilder::new("Show / Hide")
        .id("show-hide")
        .build(app)?;
    let refresh = MenuItemBuilder::new("Refresh All")
        .id("refresh-all")
        .build(app)?;
    let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;
    let tray_menu = Menu::with_items(app, &[&show_hide, &refresh, &quit])?;

    let tray_icon = tauri::tray::TrayIconBuilder::new()
        .icon(tray_icon_image)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "refresh-all" => spawn_refresh_all(app),
            "show-hide" => toggle_main_window(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    app.manage(TrayState {
        icon: Mutex::new(Some(tray_icon)),
    });
    update_tray_tooltip(app.handle(), &[]);

    Ok(())
}
