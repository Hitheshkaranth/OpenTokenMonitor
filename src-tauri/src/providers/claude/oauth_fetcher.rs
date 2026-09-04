use chrono::{DateTime, Utc};
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde_json::Value;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct ClaudeOauthWindow {
    pub five_hour_utilization: f64,
    pub seven_day_utilization: f64,
    pub seven_day_opus_utilization: f64,
    /// Whether the API actually reported an Opus window. Distinguishes "Opus
    /// quota is at 0%" from "this plan has no Opus window", so the gauge does
    /// not vanish and reappear between polls.
    pub has_opus_window: bool,
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

/// Marker prefix on rate-limit errors so `fetch_usage` can stop early instead of
/// retrying a second endpoint behind the same limiter.
const RATE_LIMITED_PREFIX: &str = "429 rate limited";

/// Endpoints to try in order. The legacy `usage` endpoint has a known response
/// format; the newer `client_data` endpoint (used by Claude Code v2.1+) is tried
/// as a fallback when the primary is unavailable or returns an unexpected shape.
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
        debug!("[claude] trying endpoint: {endpoint}");
        match try_endpoint(&client, endpoint, token).await {
            Ok(window) => return Ok(window),
            Err(e) => {
                warn!("[claude] endpoint {endpoint} failed: {e}");
                let rate_limited = e.starts_with(RATE_LIMITED_PREFIX);
                last_err = e;
                if rate_limited {
                    // Both endpoints share one rate limiter, so trying the next
                    // one just burns another request and deepens the throttle.
                    // Bail out and let the provider-level backoff serve the
                    // last known-good snapshot instead.
                    break;
                }
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
    // One retry for a transient 429. Anything more multiplies request volume
    // against the same limiter that just rejected us; `ClaudeProvider` already
    // applies a 30s→600s exponential backoff on top of this.
    let mut last_err = String::new();
    for attempt in 0..2 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
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
                warn!("[claude] OAuth attempt {}: {last_err}", attempt + 1);
                continue;
            }
        };

        let status = res.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let body = res.text().await.unwrap_or_default();
            last_err = format!("{RATE_LIMITED_PREFIX} (attempt {}): {body}", attempt + 1);
            warn!("[claude] OAuth: {last_err}");
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

    Err(last_err)
}

fn parse_usage_response(payload: Value) -> Result<ClaudeOauthWindow, String> {
    // Try to find the usage data — it could be at the top level (legacy endpoint),
    // nested under "usage" or "rate_limits" (client_data endpoint), or elsewhere.
    //
    // If no recognizable window object is present we MUST fail rather than fall
    // through: every field below is read with `unwrap_or(0.0)`, so an unexpected
    // 200 response would otherwise parse as a perfectly valid "0% used"
    // snapshot. That snapshot then gets cached as the last known-good value and
    // persisted over the real one, which is what makes the gauges collapse to
    // zero for no visible reason.
    let Some(root) = find_usage_root(&payload) else {
        return Err(
            "response contained no five_hour/seven_day usage window (unexpected payload shape)"
                .to_string(),
        );
    };

    let f = root.get("five_hour").cloned().unwrap_or(Value::Null);
    let s = root.get("seven_day").cloned().unwrap_or(Value::Null);
    let o = root.get("seven_day_opus").cloned().unwrap_or(Value::Null);

    let extra_usage_val = root
        .get("extra_usage")
        .or_else(|| payload.get("extra_usage"));
    let extra_usage = extra_usage_val.and_then(|eu| {
        let enabled = eu
            .get("is_enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !enabled {
            return None;
        }
        Some(ExtraUsageInfo {
            monthly_limit_usd: eu
                .get("monthly_limit")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            used_credits_usd: eu
                .get("used_credits")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            utilization: eu.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        })
    });

    let window = ClaudeOauthWindow {
        five_hour_utilization: f.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        seven_day_utilization: s.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        seven_day_opus_utilization: o.get("utilization").and_then(Value::as_f64).unwrap_or(0.0),
        has_opus_window: o.is_object(),
        five_hour_resets_at: parse_dt(f.get("resets_at").and_then(Value::as_str)),
        seven_day_resets_at: parse_dt(s.get("resets_at").and_then(Value::as_str)),
        extra_usage,
    };
    info!(
        "[claude] OAuth OK: 5h={:.1}% 7d={:.1}% opus={:.1}%{}",
        window.five_hour_utilization,
        window.seven_day_utilization,
        window.seven_day_opus_utilization,
        window
            .extra_usage
            .as_ref()
            .map(|eu| format!(" extra={:.1}%", eu.utilization))
            .unwrap_or_default(),
    );
    Ok(window)
}

/// Walk the JSON payload to find the object containing `five_hour` / `seven_day` keys.
/// Supports: top-level, nested under "usage", "rate_limits", or one level deep in any key.
///
/// Returns `None` when no such object exists. Callers must treat that as a hard
/// error — see the note in `parse_usage_response`.
fn find_usage_root(payload: &Value) -> Option<&Value> {
    fn has_window(value: &Value) -> bool {
        value.get("five_hour").is_some() || value.get("seven_day").is_some()
    }

    // Direct top-level match
    if has_window(payload) {
        return Some(payload);
    }

    // Known nesting keys
    for key in &["usage", "rate_limits", "rateLimits", "data"] {
        if let Some(inner) = payload.get(key) {
            if has_window(inner) {
                return Some(inner);
            }
        }
    }

    // Scan one level deep for any object containing usage fields
    if let Some(obj) = payload.as_object() {
        for val in obj.values() {
            if val.is_object() && has_window(val) {
                return Some(val);
            }
        }
    }

    None
}

fn parse_dt(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
}
