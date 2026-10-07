//! System-tray icon, menu, and tooltip.
//!
//! The tray is the app's only persistent UI surface when the main window is
//! hidden (autostart launch, minimize-to-tray). It exposes:
//! - a left-click toggle for show/hide of the main window
//! - a right-click menu (Show/Hide, Refresh All, Quit)
//! - a tooltip showing each provider's primary-window utilization
//! - optional menu-bar badges: the tray icon becomes a strip of provider
//!   logos wrapped in usage rings (see [`crate::tray_badges`]); clicking a
//!   badge opens that provider
//!
//! The tray icon handle lives inside [`TrayState`] (kept in Tauri-managed
//! state) because the tooltip is updated from multiple paths after startup.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
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
    /// The app icon, restored when badges are turned off.
    pub default_icon: Image<'static>,
    /// Providers currently drawn in the badge strip, in display order, with
    /// the rounded percentages it was last rendered at.
    pub badges: Mutex<Vec<(ProviderId, (i64, i64))>>,
    /// Windows only: one tray icon per provider. Notification-area icons are
    /// fixed squares there, so a wide strip would be squashed into one slot;
    /// separate icons sit side by side instead. Created once at startup in a
    /// fixed order so Explorer keeps recognising (and promoting) them across
    /// launches; shown/hidden rather than recreated. Empty elsewhere.
    pub provider_icons: Vec<(ProviderId, tauri::tray::TrayIcon)>,
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

/// What the tray shows. `Percent` and `Cost` both draw the ring badges (the
/// rings *are* the usage %); `Cost` adds today's spend as a title. `Off`
/// keeps the plain app icon. Only macOS renders tray titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayTitleMode {
    Off,
    #[default]
    Percent,
    Cost,
}

/// One provider's menu-bar badge.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderBadge {
    pub provider: ProviderId,
    /// Primary / secondary window utilization, percent.
    pub primary: f64,
    pub secondary: Option<f64>,
    pub tooltip: String,
}

/// Title beside the badge strip: today's total spend in `Cost` mode.
pub fn strip_title(mode: TrayTitleMode, today_cost_usd: f64) -> Option<String> {
    (mode == TrayTitleMode::Cost).then(|| format!("${today_cost_usd:.2} today"))
}

fn provider_name(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Claude => "Claude",
        ProviderId::Codex => "Codex",
        ProviderId::Antigravity => "Antigravity",
    }
}

/// Build the badges for `mode`, in fixed order Claude, Codex, Antigravity,
/// for providers that have a snapshot.
pub fn build_badges(mode: TrayTitleMode, snapshots: &[UsageSnapshot]) -> Vec<ProviderBadge> {
    if mode == TrayTitleMode::Off {
        return Vec::new();
    }
    ProviderId::all()
        .into_iter()
        .filter_map(|provider| {
            let snapshot = snapshots.iter().find(|s| s.provider == provider)?;
            let primary = snapshot_percent(snapshot);
            let secondary = snapshot.windows.get(1).map(|w| w.utilization);
            let windows: Vec<String> = snapshot
                .windows
                .iter()
                .take(2)
                .map(|w| {
                    format!(
                        "{} {:.0}%",
                        crate::alerts::format_window_label(w.window_type),
                        w.utilization
                    )
                })
                .collect();
            Some(ProviderBadge {
                provider,
                primary,
                secondary,
                tooltip: format!("{} — {}", provider_name(provider), windows.join(" · ")),
            })
        })
        .collect()
}

/// A clone of the tray icon handle. Callers must not hold any `TrayState`
/// lock while calling tray APIs: on macOS they block until the main thread
/// runs them, and the main thread may itself be waiting on that lock (a
/// synchronous command), which deadlocks the app.
fn tray_icon(app: &AppHandle) -> Option<tauri::tray::TrayIcon> {
    app.try_state::<TrayState>()?.icon.lock().ok()?.clone()
}

/// Show `badges` as the tray icon (or restore the app icon when empty).
/// The strip is only re-rendered when a provider appears/disappears or a
/// rounded percentage moves.
pub fn sync_provider_badges(app: &AppHandle, badges: &[ProviderBadge], title: Option<String>) {
    let Some(tray_state) = app.try_state::<TrayState>() else {
        return;
    };
    let Some(icon) = tray_icon(app) else {
        return;
    };
    let next: Vec<(ProviderId, (i64, i64))> = badges
        .iter()
        .map(|b| {
            (
                b.provider,
                (
                    b.primary.round() as i64,
                    b.secondary.map_or(-1, |s| s.round() as i64),
                ),
            )
        })
        .collect();
    // Decide under the lock, call the tray API after releasing it.
    let changed = match tray_state.badges.lock() {
        Ok(mut shown) if *shown != next => {
            *shown = next;
            true
        }
        Ok(_) => false,
        Err(_) => return,
    };

    if changed && !tray_state.provider_icons.is_empty() {
        sync_split_badges(&icon, &tray_state.provider_icons, badges, tray_slot_px(app));
        return;
    }
    if changed {
        let image = if badges.is_empty() {
            tray_state.default_icon.clone()
        } else {
            let strip = crate::tray_badges::render_strip(
                &badges
                    .iter()
                    .map(|b| (b.provider, b.primary, b.secondary))
                    .collect::<Vec<_>>(),
            );
            let (w, h) = strip.dimensions();
            Image::new_owned(strip.into_raw(), w, h)
        };
        let _ = icon.set_icon(Some(image));
        let _ = icon.set_icon_as_template(false);
    }
    let _ = icon.set_title(title);
    if !badges.is_empty() {
        let lines: Vec<&str> = badges.iter().map(|b| b.tooltip.as_str()).collect();
        let _ = icon.set_tooltip(Some(format!("OpenTokenMonitor\n{}", lines.join("\n"))));
    }
}

/// Per-provider icons (Windows): show a badge for each provider that has
/// one and hide the rest. The app icon is hidden while any badge is up —
/// every badge carries the same menu — and comes back when badges go away.
fn sync_split_badges(
    main: &tauri::tray::TrayIcon,
    provider_icons: &[(ProviderId, tauri::tray::TrayIcon)],
    badges: &[ProviderBadge],
    slot_px: u32,
) {
    for (provider, icon) in provider_icons {
        match badges.iter().find(|b| b.provider == *provider) {
            Some(badge) => {
                let img = crate::tray_badges::render_tray_badge(
                    *provider,
                    badge.primary,
                    badge.secondary,
                    slot_px,
                );
                let (w, h) = img.dimensions();
                // Show first: on Windows icon/tooltip updates modify the
                // existing tray entry and fail while it's hidden.
                let _ = icon.set_visible(true);
                let _ = icon.set_icon(Some(Image::new_owned(img.into_raw(), w, h)));
                let _ = icon.set_tooltip(Some(badge.tooltip.clone()));
            }
            None => {
                let _ = icon.set_visible(false);
            }
        }
    }
    let _ = main.set_visible(badges.is_empty());
}

/// Edge length of a notification-area icon: 16px scaled by the primary
/// display's DPI (the taskbar's home), e.g. 20px at 125%.
fn tray_slot_px(app: &AppHandle) -> u32 {
    let scale = app
        .primary_monitor()
        .ok()
        .flatten()
        .map_or(1.0, |m| m.scale_factor());
    (16.0 * scale).round() as u32
}

/// Left-click: on a badge strip, open the provider under the cursor;
/// otherwise toggle the main window.
fn handle_left_click(app: &AppHandle, position_x: f64, rect: &tauri::Rect) {
    let providers: Vec<ProviderId> = app
        .try_state::<TrayState>()
        .and_then(|s| {
            s.badges
                .lock()
                .ok()
                .map(|b| b.iter().map(|(p, _)| *p).collect())
        })
        .unwrap_or_default();
    if providers.is_empty() {
        toggle_main_window(app);
        return;
    }
    let origin = rect.position.to_physical::<f64>(1.0);
    let size = rect.size.to_physical::<f64>(1.0);
    // macOS draws the icon ~18pt tall in a ~24pt item, keeping its aspect
    // ratio; any title sits to the right of the strip.
    use crate::tray_badges::{BADGE_SIZE, STRIP_GAP};
    let n = providers.len() as f64;
    let aspect = (n * BADGE_SIZE as f64 + (n - 1.0) * STRIP_GAP as f64) / BADGE_SIZE as f64;
    let strip_w = (size.height * 0.75 * aspect).min(size.width);
    let fraction = (position_x - origin.x) / strip_w.max(1.0);
    match crate::tray_badges::badge_index_at(fraction, providers.len()) {
        Some(i) if fraction <= 1.0 => open_provider(app, providers[i]),
        _ => toggle_main_window(app),
    }
}

/// Bring the main window forward on a provider's detail page.
fn open_provider(app: &AppHandle, provider: ProviderId) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    let _ = app.emit("tray-navigate", provider);
}

/// Replace the tray tooltip with the latest provider utilizations.
pub fn update_tray_tooltip(app: &AppHandle, snapshots: &[UsageSnapshot]) {
    if let Some(icon) = tray_icon(app) {
        let _ = icon.set_tooltip(Some(format_tray_tooltip(snapshots)));
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
            crate::post_refresh::on_snapshots_updated(&app_inner, &snapshots);
            let _ = app_inner.emit("usage-updated", snapshots);
        }
    });
}

/// Build the tray icon, menu, and click handlers, then store the icon in
/// Tauri-managed state so tooltip updates can find it later.
pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    // Sourced from `public/` so the tray icon and the webview favicon are the
    // same single file. There used to be an identical copy at the repo root
    // purely for this `include_bytes!`.
    let icon_bytes_png: &[u8] = include_bytes!("../../public/open_token_monitor_icon.png");
    let img = image::load_from_memory_with_format(icon_bytes_png, image::ImageFormat::Png)
        .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?
        .to_rgba8();
    let (width, height) = image::GenericImageView::dimensions(&img);
    let tray_icon_image = Image::new_owned(img.into_raw(), width, height);
    let default_icon = tray_icon_image.clone();

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
                position,
                rect,
                ..
            } = event
            {
                handle_left_click(tray.app_handle(), position.x, &rect);
            }
        })
        .build(app)?;

    #[cfg(target_os = "windows")]
    let provider_icons = build_provider_icons(app, &tray_menu)?;
    #[cfg(not(target_os = "windows"))]
    let provider_icons = Vec::new();

    app.manage(TrayState {
        icon: Mutex::new(Some(tray_icon)),
        default_icon,
        badges: Mutex::new(Vec::new()),
        provider_icons,
    });
    update_tray_tooltip(app.handle(), &[]);

    #[cfg(target_os = "windows")]
    crate::tray_promote::promote_in_background();

    Ok(())
}

/// One hidden tray icon per provider, in the fixed order Claude, Codex,
/// Antigravity. They share the app menu, whose handler is registered once on
/// the main icon (Tauri menu handlers are global). Left-click opens that
/// provider. Each starts as its provider's logo with empty rings, never the
/// app icon, until the first refresh fills the rings in.
#[cfg(target_os = "windows")]
fn build_provider_icons(
    app: &tauri::App,
    menu: &Menu<tauri::Wry>,
) -> tauri::Result<Vec<(ProviderId, tauri::tray::TrayIcon)>> {
    let slot_px = tray_slot_px(app.handle());
    ProviderId::all()
        .into_iter()
        .map(|provider| {
            let empty = crate::tray_badges::render_tray_badge(provider, 0.0, None, slot_px);
            let (w, h) = empty.dimensions();
            let icon =
                tauri::tray::TrayIconBuilder::with_id(format!("badge-{}", provider.as_str()))
                    .icon(Image::new_owned(empty.into_raw(), w, h))
                    .tooltip(provider_name(provider))
                    .menu(menu)
                    .show_menu_on_left_click(false)
                    .on_tray_icon_event(move |tray, event| {
                        if let tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        } = event
                        {
                            open_provider(tray.app_handle(), provider);
                        }
                    })
                    .build(app)?;
            icon.set_visible(false)?;
            Ok((provider, icon))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::models::{
        DataProvenance, DataSource, UsageUnit, UsageWindow, WindowAccuracy, WindowType,
    };
    use chrono::Utc;

    fn snap(provider: ProviderId, utilization: f64) -> UsageSnapshot {
        UsageSnapshot {
            provider,
            windows: vec![UsageWindow {
                window_type: WindowType::FiveHour,
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
            source: DataSource::Oauth,
            provenance: DataProvenance::Official,
            stale: false,
        }
    }

    fn snap2(provider: ProviderId, primary: f64, secondary: f64) -> UsageSnapshot {
        let mut s = snap(provider, primary);
        let mut second = s.windows[0].clone();
        second.window_type = WindowType::SevenDay;
        second.utilization = secondary;
        s.windows.push(second);
        s
    }

    #[test]
    fn badges_follow_fixed_order_and_skip_missing() {
        let snaps = [
            snap(ProviderId::Antigravity, 12.4),
            snap2(ProviderId::Claude, 71.6, 92.0),
        ];
        let badges = build_badges(TrayTitleMode::Percent, &snaps);
        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].provider, ProviderId::Claude);
        assert_eq!(badges[0].secondary, Some(92.0));
        assert_eq!(badges[0].tooltip, "Claude — 5h window 72% · 7d window 92%");
        assert_eq!(badges[1].provider, ProviderId::Antigravity);
        assert_eq!(badges[1].secondary, None);
    }

    #[test]
    fn cost_mode_adds_title_and_off_mode_hides_badges() {
        let snaps = [snap(ProviderId::Codex, 5.0)];
        assert_eq!(build_badges(TrayTitleMode::Cost, &snaps).len(), 1);
        assert_eq!(
            strip_title(TrayTitleMode::Cost, 4.2).as_deref(),
            Some("$4.20 today")
        );
        assert_eq!(strip_title(TrayTitleMode::Percent, 4.2), None);
        assert!(build_badges(TrayTitleMode::Off, &snaps).is_empty());
    }
}
