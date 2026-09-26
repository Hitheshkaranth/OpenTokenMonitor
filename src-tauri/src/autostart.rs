//! OS-level "launch at startup" support.
//!
//! Wraps `tauri-plugin-autostart` so the rest of the codebase doesn't have to
//! repeat the cfg-gating. On unsupported platforms these helpers are no-ops
//! that report `false`.

use tauri::AppHandle;

/// Detects whether the current process was launched by the OS autostart entry.
///
/// We pass `--autostart` as a launch argument when registering the autostart
/// shortcut (see `lib.rs::run`). Some launch contexts (certain LaunchAgent or
/// shortcut configurations) strip process arguments, so `OTM_AUTOSTART=1` in
/// the environment is accepted as an equivalent autostart signal.
pub fn is_autostart_launch() -> bool {
    std::env::args().any(|arg| arg == "--autostart")
        || std::env::var("OTM_AUTOSTART")
            .map(|v| v == "1")
            .unwrap_or(false)
}

/// Returns whether the OS autostart entry is currently enabled.
pub fn launch_at_startup_enabled(app: &AppHandle) -> Result<bool, String> {
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

/// Enables or disables the OS autostart entry. Returns the resulting state
/// (which can differ from the requested state if the OS rejected the change).
pub fn set_launch_at_startup_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    // Development (`tauri dev` / debug) builds must NEVER write the OS
    // launch-at-startup entry. The debug binary lives in `target/debug`, loads
    // its UI from the Vite dev server (`devUrl`), and is rebuilt/cleaned
    // constantly. If a dev session registers itself for autostart, the machine
    // tries to launch that dev binary at every boot with no dev server running,
    // so the app comes up with a dead webview and appears to crash. Only
    // release / installed builds are allowed to manage autostart.
    #[cfg(debug_assertions)]
    {
        let _ = enabled;
        tracing::warn!(
            "[autostart] ignoring launch-at-startup change in a debug build; \
             only release/installed builds manage autostart"
        );
        launch_at_startup_enabled(app)
    }

    #[cfg(not(debug_assertions))]
    {
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
}

/// Re-register the autostart entry so it points at the *current* executable,
/// overwriting any stale path (e.g. a `target/debug` path left behind by a
/// previous `tauri dev` session). No-op unless autostart is already enabled and
/// this is a release build. Best-effort: failures are logged, not fatal.
pub fn heal_autostart_path(app: &AppHandle) {
    #[cfg(all(
        not(debug_assertions),
        any(target_os = "macos", target_os = "windows", target_os = "linux")
    ))]
    {
        use tauri_plugin_autostart::ManagerExt;

        let manager = app.autolaunch();
        if manager.is_enabled().unwrap_or(false) {
            // disable() + enable() guarantees the value is rewritten to point at
            // this binary even if the previous entry used a different path.
            let _ = manager.disable();
            if let Err(e) = manager.enable() {
                tracing::warn!("[autostart] failed to refresh autostart path: {e}");
            } else {
                tracing::info!("[autostart] refreshed autostart entry to current executable");
            }
        }
    }

    #[cfg(not(all(
        not(debug_assertions),
        any(target_os = "macos", target_os = "windows", target_os = "linux")
    )))]
    {
        let _ = app;
    }
}
