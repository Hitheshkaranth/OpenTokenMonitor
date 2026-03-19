use serde_json::Value;
use std::process::Command;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct GeminiCliStats {
    pub daily_used: u64,
    pub daily_limit: u64,
    pub session_used: u64,
    pub session_limit: u64,
}

pub fn fetch_stats() -> Result<GeminiCliStats, String> {
    if !supports_stats_command() {
        return Err("gemini CLI does not support --stats".to_string());
    }

    let output = Command::new(gemini_command())
        .args(["--stats", "--json"])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("gemini --stats failed".to_string());
    }

    let payload: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let daily_used = payload
        .get("daily")
        .and_then(|v| v.get("used"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let daily_limit = payload
        .get("daily")
        .and_then(|v| v.get("limit"))
        .and_then(Value::as_u64)
        .unwrap_or(1000);
    let session_used = payload
        .get("session")
        .and_then(|v| v.get("tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let session_limit = payload
        .get("session")
        .and_then(|v| v.get("limit"))
        .and_then(Value::as_u64)
        .unwrap_or(100000);

    Ok(GeminiCliStats {
        daily_used,
        daily_limit,
        session_used,
        session_limit,
    })
}

pub fn supports_stats_command() -> bool {
    static SUPPORTS: OnceLock<bool> = OnceLock::new();
    *SUPPORTS.get_or_init(|| {
        let Ok(output) = Command::new(gemini_command()).arg("--help").output() else {
            return false;
        };
        if !output.status.success() {
            return false;
        }
        let help = String::from_utf8_lossy(&output.stdout);
        help.contains("--stats")
    })
}

#[cfg(target_os = "windows")]
fn gemini_command() -> &'static str {
    "gemini"
}

#[cfg(not(target_os = "windows"))]
fn gemini_command() -> &'static str {
    "gemini"
}
