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

pub async fn fetch_usage(token: &str) -> Result<ClaudeOauthWindow, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    // Retry up to 3 times with exponential backoff for 429 errors
    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            let delay = std::time::Duration::from_millis(500 * (1 << attempt)); // 1s, 2s
            tokio::time::sleep(delay).await;
        }

        let res = match client
            .get("https://api.anthropic.com/api/oauth/usage")
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
            return Err(format!("Claude OAuth returned {status}: {body}"));
        }

        // Success — parse response
        let payload = res.json::<Value>().await.map_err(|e| e.to_string())?;
        return parse_usage_response(payload);
    }

    Err(format!("Claude OAuth failed after 3 attempts: {last_err}"))
}

fn parse_usage_response(payload: Value) -> Result<ClaudeOauthWindow, String> {
    let f = payload.get("five_hour").cloned().unwrap_or(Value::Null);
    let s = payload.get("seven_day").cloned().unwrap_or(Value::Null);
    let o = payload.get("seven_day_opus").cloned().unwrap_or(Value::Null);

    let extra_usage = payload.get("extra_usage").and_then(|eu| {
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

fn parse_dt(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
}
