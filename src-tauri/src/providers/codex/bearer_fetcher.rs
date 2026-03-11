use chrono::{DateTime, TimeZone, Utc};
use reqwest::header::{ACCEPT, AUTHORIZATION, ORIGIN, USER_AGENT};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct CodexBearerWindow {
    pub session_used: u64,
    pub session_limit: u64,
    pub weekly_used: u64,
    pub weekly_limit: u64,
    pub resets_at: Option<DateTime<Utc>>,
    pub weekly_resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    rate_limit: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
struct RateLimit {
    primary_window: Option<Window>,
    secondary_window: Option<Window>,
}

#[derive(Debug, Deserialize)]
struct Window {
    used_percent: Option<u64>,
    reset_at: Option<i64>,
}

pub async fn fetch_usage(access_token: &str) -> Result<CodexBearerWindow, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    // Retry up to 2 times for transient failures
    let mut last_err = String::new();
    for attempt in 0..2 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        let res = match client
            .get("https://chatgpt.com/backend-api/codex/usage")
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header(ACCEPT, "application/json")
            .header(ORIGIN, "https://chatgpt.com")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("request failed: {e}");
                eprintln!("[codex] bearer attempt {}: {last_err}", attempt + 1);
                continue;
            }
        };

        let status = res.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            last_err = format!("429 rate limited (attempt {})", attempt + 1);
            eprintln!("[codex] bearer: {last_err}");
            continue;
        }

        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(format!("Codex bearer API returned {status}: {body}"));
        }

        return parse_response(res).await;
    }

    Err(format!("Codex bearer failed after retries: {last_err}"))
}

async fn parse_response(res: reqwest::Response) -> Result<CodexBearerWindow, String> {

    let resp: UsageResponse = res.json().await.map_err(|e| format!("response parse error: {e}"))?;
    let rl = resp.rate_limit.ok_or("No rate_limit in response")?;

    let primary = rl.primary_window.unwrap_or(Window { used_percent: Some(0), reset_at: None });
    let secondary = rl.secondary_window.unwrap_or(Window { used_percent: Some(0), reset_at: None });

    let session_pct = primary.used_percent.unwrap_or(0);
    let weekly_pct = secondary.used_percent.unwrap_or(0);

    // API returns percentages; convert to used/limit for consistent display
    let session_limit = 100;
    let weekly_limit = 100;
    let session_used = session_pct;
    let weekly_used = weekly_pct;

    let resets_at = primary.reset_at
        .and_then(|ts| Utc.timestamp_opt(ts, 0).single());
    let weekly_resets_at = secondary.reset_at
        .and_then(|ts| Utc.timestamp_opt(ts, 0).single());

    Ok(CodexBearerWindow {
        session_used,
        session_limit,
        weekly_used,
        weekly_limit,
        resets_at,
        weekly_resets_at,
    })
}
