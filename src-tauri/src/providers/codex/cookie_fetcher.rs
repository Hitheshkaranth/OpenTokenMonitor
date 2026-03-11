use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct CodexCookieWindow {
    pub session_used: u64,
    pub session_limit: u64,
    pub weekly_used: u64,
    pub weekly_limit: u64,
    pub resets_at: Option<DateTime<Utc>>,
}

pub async fn fetch_usage(cookie: &str) -> Result<CodexCookieWindow, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(7))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get("https://chatgpt.com/backend-api/codex/usage")
        .header("cookie", cookie)
        .header("user-agent", "OpenTokenMonitor/2.0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Cookie usage endpoint returned {}", res.status()));
    }

    let payload = res.json::<Value>().await.map_err(|e| e.to_string())?;
    let session_used = find_num(&payload, &["session", "used", "current", "consumed"]).unwrap_or(0);
    let session_limit = find_num(&payload, &["session", "limit", "max", "quota"]).unwrap_or(100);
    let weekly_used = find_num(&payload, &["weekly", "used", "current", "consumed"]).unwrap_or(0);
    let weekly_limit = find_num(&payload, &["weekly", "limit", "max", "quota"]).unwrap_or(1000);
    let resets_at = find_text(&payload, &["resets_at", "reset_at", "next_reset"])
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));

    Ok(CodexCookieWindow {
        session_used,
        session_limit,
        weekly_used,
        weekly_limit,
        resets_at,
    })
}

fn find_num(value: &Value, hints: &[&str]) -> Option<u64> {
    flatten(value)
        .into_iter()
        .find(|(k, _)| hints.iter().any(|h| k.contains(h)))
        .and_then(|(_, v)| v)
}

fn find_text(value: &Value, hints: &[&str]) -> Option<String> {
    flatten_text(value)
        .into_iter()
        .find(|(k, _)| hints.iter().any(|h| k.contains(h)))
        .map(|(_, v)| v)
}

fn flatten(value: &Value) -> Vec<(String, Option<u64>)> {
    let mut out = Vec::new();
    walk_num(value, "", &mut out);
    out
}

fn flatten_text(value: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk_text(value, "", &mut out);
    out
}

fn walk_num(value: &Value, path: &str, out: &mut Vec<(String, Option<u64>)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let next = if path.is_empty() { k.to_ascii_lowercase() } else { format!("{path}.{}", k.to_ascii_lowercase()) };
                walk_num(v, &next, out);
            }
        }
        Value::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                walk_num(item, &format!("{path}.{idx}"), out);
            }
        }
        Value::Number(n) => out.push((path.to_string(), n.as_u64())),
        _ => {}
    }
}

fn walk_text(value: &Value, path: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let next = if path.is_empty() { k.to_ascii_lowercase() } else { format!("{path}.{}", k.to_ascii_lowercase()) };
                walk_text(v, &next, out);
            }
        }
        Value::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                walk_text(item, &format!("{path}.{idx}"), out);
            }
        }
        Value::String(s) => out.push((path.to_string(), s.to_string())),
        _ => {}
    }
}
