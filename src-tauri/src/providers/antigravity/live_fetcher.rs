use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::path::PathBuf;

// The Antigravity CLI/IDE ("agy") runs a local Language Server that exposes a
// Connect/JSON API on a random loopback port. It is already authenticated with
// the user's Google account, so we read live quota straight from it — no tokens,
// no refresh, no credential risk. When agy is not running we simply get no port
// and the provider falls back to local logs.
const LS_SERVICE: &str = "exa.language_server_pb.LanguageServerService";

#[derive(Debug, Clone)]
pub struct ModelQuota {
    pub name: String,
    /// 0.0..=1.0 of the rolling quota still available.
    pub remaining_fraction: f64,
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct AntigravityLive {
    pub plan: Option<String>,
    pub available_prompt_credits: Option<u64>,
    pub monthly_prompt_credits: Option<u64>,
    pub available_flow_credits: Option<u64>,
    pub models: Vec<ModelQuota>,
}

fn cli_log_path() -> Option<PathBuf> {
    // Symlink that always points at the current session's log.
    dirs::home_dir().map(|h| h.join(".gemini").join("antigravity-cli").join("cli.log"))
}

/// Parse the agy session log for the HTTP (Connect) loopback port. Lines look
/// like: `... listening on random port at 65007 for HTTP`. The sibling HTTPS
/// (gRPC) port is deliberately skipped — we speak plain Connect/JSON.
pub fn discover_port() -> Option<u16> {
    let raw = std::fs::read_to_string(cli_log_path()?).ok()?;
    let mut found = None;
    for line in raw.lines() {
        let Some(idx) = line.find("listening on random port at ") else {
            continue;
        };
        if !line.contains("for HTTP") || line.contains("HTTPS") {
            continue;
        }
        let rest = &line[idx + "listening on random port at ".len()..];
        let port: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(p) = port.parse::<u16>() {
            found = Some(p); // keep the last (most recent) match
        }
    }
    found
}

async fn connect_call(client: &reqwest::Client, port: u16, method: &str) -> Result<Value, String> {
    let url = format!("http://127.0.0.1:{port}/{LS_SERVICE}/{method}");
    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .json(&json!({}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("{method} returned {}", res.status()));
    }
    res.json::<Value>().await.map_err(|e| e.to_string())
}

pub async fn fetch_quota() -> Result<AntigravityLive, String> {
    let port = discover_port().ok_or("Antigravity CLI not running (no loopback port)")?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?;

    let status = connect_call(&client, port, "GetUserStatus").await?;
    let models = connect_call(&client, port, "GetAvailableModels").await?;

    let plan_status = status.pointer("/userStatus/planStatus");
    let plan_info = plan_status.and_then(|p| p.get("planInfo"));

    let live = AntigravityLive {
        plan: plan_info
            .and_then(|p| p.get("planName"))
            .and_then(Value::as_str)
            .map(str::to_string),
        available_prompt_credits: plan_status
            .and_then(|p| p.get("availablePromptCredits"))
            .and_then(as_u64_loose),
        monthly_prompt_credits: plan_info
            .and_then(|p| p.get("monthlyPromptCredits"))
            .and_then(as_u64_loose),
        available_flow_credits: plan_status
            .and_then(|p| p.get("availableFlowCredits"))
            .and_then(as_u64_loose),
        models: parse_models(&models),
    };

    if live.models.is_empty() && live.monthly_prompt_credits.is_none() {
        return Err("Antigravity live server returned no quota data".to_string());
    }
    Ok(live)
}

fn parse_models(resp: &Value) -> Vec<ModelQuota> {
    let Some(map) = resp.pointer("/response/models").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for model in map.values() {
        // Only user-facing models carry a display name; skip internal placeholders.
        let Some(name) = model.get("displayName").and_then(Value::as_str) else {
            continue;
        };
        let Some(qi) = model.get("quotaInfo") else {
            continue;
        };
        let Some(frac) = qi.get("remainingFraction").and_then(Value::as_f64) else {
            continue;
        };
        if !seen.insert(name.to_string()) {
            continue;
        }
        out.push(ModelQuota {
            name: name.to_string(),
            remaining_fraction: frac.clamp(0.0, 1.0),
            resets_at: qi
                .get("resetTime")
                .and_then(Value::as_str)
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc)),
        });
    }
    out
}

/// Credit fields come back as JSON numbers, but the API serializes some int64s
/// as strings — accept both.
fn as_u64_loose(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
}
