//! Windows 11 parks every new notification-area icon in the overflow flyout
//! (the `^` chevron), so the usage badges would never sit next to the
//! clock and battery unless the user digs through Settings. Explorer keeps a
//! per-icon record under `HKCU\Control Panel\NotifyIconSettings`; setting
//! `IsPromoted = 1` there is exactly what the Settings toggle does.
//!
//! Only records with no `IsPromoted` value are touched: once the user hides
//! an icon themselves Explorer writes `0`, and that choice is left alone.

use std::path::Path;
use std::time::Duration;

use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const SETTINGS_KEY: &str = r"Control Panel\NotifyIconSettings";

/// Explorer only creates a record once an icon has been added, so promote on
/// a background thread a couple of times after startup.
pub fn promote_in_background() {
    std::thread::spawn(|| {
        for delay in [2, 15] {
            std::thread::sleep(Duration::from_secs(delay));
            let promoted = promote_own_icons();
            if promoted > 0 {
                tracing::info!(promoted, "promoted tray icons out of the overflow flyout");
            }
        }
    });
}

/// Promote every never-configured record that belongs to this executable.
/// Returns how many records were changed.
fn promote_own_icons() -> usize {
    let Ok(exe) = std::env::current_exe() else {
        return 0;
    };
    // Absent on Windows 10, where this mechanism doesn't exist.
    let Ok(settings) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(SETTINGS_KEY) else {
        return 0;
    };
    let mut promoted = 0;
    for name in settings.enum_keys().flatten() {
        let Ok(record) = settings.open_subkey_with_flags(&name, KEY_READ | KEY_WRITE) else {
            continue;
        };
        let Ok(path) = record.get_value::<String, _>("ExecutablePath") else {
            continue;
        };
        if !is_same_executable(&path, &exe) || record.get_value::<u32, _>("IsPromoted").is_ok() {
            continue;
        }
        if record.set_value("IsPromoted", &1u32).is_ok() {
            promoted += 1;
        }
    }
    promoted
}

/// Explorer stores paths under known folders as `{FOLDERID}\rest\app.exe`
/// (e.g. Program Files), so compare the part after the GUID as a suffix.
fn is_same_executable(recorded: &str, exe: &Path) -> bool {
    let exe = exe.to_string_lossy().to_lowercase();
    let recorded = recorded.to_lowercase();
    match recorded.strip_prefix('{').and_then(|r| r.split_once('}')) {
        Some((_, rest)) => !rest.is_empty() && exe.ends_with(rest),
        None => recorded == exe,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_plain_and_known_folder_paths() {
        let exe = Path::new(r"C:\Program Files\OpenTokenMonitor\open-token-monitor.exe");
        assert!(is_same_executable(
            r"C:\Program Files\OpenTokenMonitor\open-token-monitor.exe",
            exe
        ));
        assert!(is_same_executable(
            r"{6D809377-6AF0-444B-8957-A3773F02200E}\OpenTokenMonitor\Open-Token-Monitor.exe",
            exe
        ));
        assert!(!is_same_executable(
            r"{F38BF404-1D43-42F2-9305-67DE0B28FC23}\explorer.exe",
            exe
        ));
        assert!(!is_same_executable(
            r"{6D809377-6AF0-444B-8957-A3773F02200E}",
            exe
        ));
    }
}
