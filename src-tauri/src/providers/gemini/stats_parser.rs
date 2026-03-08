use serde_json::Value;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GeminiCliStats {
    pub daily_used: u64,
    pub daily_limit: u64,
    pub session_used: u64,
    pub session_limit: u64,
}

pub fn fetch_stats() -> Result<GeminiCliStats, String> {
    let output = Command::new("gemini")
        .args(["--stats", "--json"])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("gemini --stats failed".to_string());
    }

    let payload: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let daily_used = payload.get("daily").and_then(|v| v.get("used")).and_then(Value::as_u64).unwrap_or(0);
    let daily_limit = payload.get("daily").and_then(|v| v.get("limit")).and_then(Value::as_u64).unwrap_or(1000);
    let session_used = payload.get("session").and_then(|v| v.get("tokens")).and_then(Value::as_u64).unwrap_or(0);
    let session_limit = payload.get("session").and_then(|v| v.get("limit")).and_then(Value::as_u64).unwrap_or(100000);

    Ok(GeminiCliStats {
        daily_used,
        daily_limit,
        session_used,
        session_limit,
    })
}
