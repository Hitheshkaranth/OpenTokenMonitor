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
    /// Extra/Max usage credits info (when enabled on the plan).
    pub extra_usage: Option<ExtraUsageInfo>,
}

#[derive(Debug, Clone)]
pub struct ExtraUsageInfo {
    pub monthly_limit_usd: f64,
    pub used_credits_usd: f64,
    pub utilization: f64,
}

/// Endpoints to try in order. The legacy `usage` endpoint has a known response
/// format; the newer `client_data` endpoint (used by Claude Code v2.1+) is tried
/// as a fallback when the primary is rate-limited or unavailable.
const ENDPOINTS: &[&str] = &[
    "https://api.anthropic.com/api/oauth/usage",
    "https://api.anthropic.com/api/oauth/claude_cli/client_data",
];

pub async fn fetch_usage(token: &str) -> Result<ClaudeOauthWindow, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut last_err = String::new();

    for endpoint in ENDPOINTS {
        eprintln!("[claude] trying endpoint: {endpoint}");
        match try_endpoint(&client, endpoint, token).await {
            Ok(window) => return Ok(window),
            Err(e) => {
                eprintln!("[claude] endpoint {endpoint} failed: {e}");
                last_err = e;
            }
        }
    }

    Err(format!("Claude OAuth failed on all endpoints: {last_err}"))
}

async fn try_endpoint(
    client: &reqwest::Client,
    endpoint: &str,
    token: &str,
) -> Result<ClaudeOauthWindow, String> {
    // Retry up to 3 times with exponential backoff for 429 errors
    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            let delay = std::time::Duration::from_millis(500 * (1 << attempt)); // 1s, 2s
            tokio::time::sleep(delay).await;
        }

        let res = match client
            .get(endpoint)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("anthropic-beta", "oauth-2025-04-20")
            .header(USER_AGENT, "claude-code/latest")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("request failed: {e}");
                eprintln!("[claude] OAuth attempt {}: {last_err}", attempt + 1);
                continue;
            }
        };

        let status = res.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let body = res.text().await.unwrap_or_default();
            last_err = format!("429 rate limited (attempt {}): {body}", attempt + 1);
            eprintln!("[claude] OAuth: {last_err}");
            continue;
        }

        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            // Non-retryable error — bail out of this endpoint immediately
            return Err(format!("{status}: {body}"));
        }

        // Success — parse response
        let payload = res.json::<Value>().await.map_err(|e| e.to_string())?;
        return parse_usage_response(payload);
    }

    Err(format!("rate limited after 3 attempts: {last_err}"))
}

fn parse_usage_response(payload: Value) -> Result<ClaudeOauthWindow, String> {
    // Try to find the usage data — it could be at the top level (legacy endpoint),
    // nested under "usage" or "rate_limits" (client_data endpoint), or elsewhere.
    let root = find_usage_root(&payload);

    let f = root.get("five_hour").cloned().unwrap_or(Value::Null);
    let s = root.get("seven_day").cloned().unwrap_or(Value::Null);
    let o = root.get("seven_day_opus").cloned().unwrap_or(Value::Null);

    let extra_usage_val = root.get("extra_usage").or_else(|| payload.get("extra_usage"));
    let extra_usage = extra_usage_val.and_then(|eu| {
        let enabled = eu.get("is_enabled").and_then(Value::as_bool).unwrap_or(false);
        if !enabled { return None; }
        Some(ExtraUsageInfo {
            monthly_limit_usd: eu.get("monthly_limit").and_then(Value::as_f64).unwrap_or(0.0),
            used_credits_usd: eu.get("used_credits").and_then(Value::as_f64).unwrap_or(0.0),
            utilization: eu.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        })
    });

    let window = ClaudeOauthWindow {
        five_hour_utilization: f.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        seven_day_utilization: s.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        seven_day_opus_utilization: o.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        five_hour_resets_at: parse_dt(f.get("resets_at").and_then(Value::as_str)),
        seven_day_resets_at: parse_dt(s.get("resets_at").and_then(Value::as_str)),
        extra_usage,
    };
    eprintln!(
        "[claude] OAuth OK: 5h={:.1}% 7d={:.1}% opus={:.1}%{}",
        window.five_hour_utilization,
        window.seven_day_utilization,
        window.seven_day_opus_utilization,
        window.extra_usage.as_ref().map(|eu| format!(" extra={:.1}%", eu.utilization)).unwrap_or_default(),
    );
    Ok(window)
}

/// Walk the JSON payload to find the object containing `five_hour` / `seven_day` keys.
/// Supports: top-level, nested under "usage", "rate_limits", or one level deep in any key.
fn find_usage_root(payload: &Value) -> &Value {
    // Direct top-level match
    if payload.get("five_hour").is_some() || payload.get("seven_day").is_some() {
        return payload;
    }

    // Known nesting keys
    for key in &["usage", "rate_limits", "rateLimits", "data"] {
        if let Some(inner) = payload.get(key) {
            if inner.get("five_hour").is_some() || inner.get("seven_day").is_some() {
                return inner;
            }
        }
    }

    // Scan one level deep for any object containing usage fields
    if let Some(obj) = payload.as_object() {
        for (_key, val) in obj {
            if val.is_object()
                && (val.get("five_hour").is_some() || val.get("seven_day").is_some())
            {
                return val;
            }
        }
    }

    // Fallback — return payload as-is and let the caller handle missing fields
    payload
}

fn parse_dt(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
}
