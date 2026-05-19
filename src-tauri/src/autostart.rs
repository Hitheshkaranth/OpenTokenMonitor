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
