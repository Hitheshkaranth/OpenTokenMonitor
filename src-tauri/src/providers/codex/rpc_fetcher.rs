use chrono::{DateTime, Utc};
use serde_json::Value;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone)]
pub struct CodexCliWindow {
    pub session_used: u64,
    pub session_limit: u64,
    pub weekly_used: u64,
    pub weekly_limit: u64,
    pub resets_at: Option<DateTime<Utc>>,
}

pub fn fetch_usage() -> Result<CodexCliWindow, String> {
    let output = cli_command()
        .args(["--usage", "--json"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("codex --usage failed".to_string());
    }

    let payload: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let session_used = pick_u64(&payload, &["session", "used"]).unwrap_or(0);
    let session_limit = pick_u64(&payload, &["session", "limit"]).unwrap_or(100);
    let weekly_used = pick_u64(&payload, &["weekly", "used"]).unwrap_or(0);
    let weekly_limit = pick_u64(&payload, &["weekly", "limit"]).unwrap_or(1000);
    let resets_at = payload
        .get("session")
        .and_then(|v| v.get("resets_at"))
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));

    Ok(CodexCliWindow {
        session_used,
        session_limit,
        weekly_used,
        weekly_limit,
        resets_at,
    })
}

#[cfg(target_os = "windows")]
fn codex_command() -> &'static str {
    "codex.cmd"
}

#[cfg(not(target_os = "windows"))]
fn codex_command() -> &'static str {
    "codex"
}

fn cli_command() -> Command {
    let mut command = Command::new(codex_command());
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

fn pick_u64(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_u64()
}
