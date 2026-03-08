use chrono::{DateTime, Utc};
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ClaudeOauthWindow {
    pub five_hour_utilization: f64,
    pub seven_day_utilization: f64,
    pub seven_day_opus_utilization: f64,
    pub five_hour_resets_at: Option<DateTime<Utc>>,
    pub seven_day_resets_at: Option<DateTime<Utc>>,
}

pub async fn fetch_usage(token: &str) -> Result<ClaudeOauthWindow, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(7))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header(USER_AGENT, "claude-code/latest")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Claude OAuth endpoint returned {}", res.status()));
    }

    let payload = res.json::<Value>().await.map_err(|e| e.to_string())?;
    let f = payload.get("five_hour").cloned().unwrap_or(Value::Null);
    let s = payload.get("seven_day").cloned().unwrap_or(Value::Null);
    let o = payload.get("seven_day_opus").cloned().unwrap_or(Value::Null);

    Ok(ClaudeOauthWindow {
        five_hour_utilization: f.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        seven_day_utilization: s.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        seven_day_opus_utilization: o.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        five_hour_resets_at: parse_dt(f.get("resets_at").and_then(Value::as_str)),
        seven_day_resets_at: parse_dt(s.get("resets_at").and_then(Value::as_str)),
    })
}

fn parse_dt(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
}
