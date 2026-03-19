use chrono::{DateTime, Utc};
use reqwest::header::AUTHORIZATION;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct GeminiQuota {
    pub daily_used: u64,
    pub daily_limit: u64,
    pub rpm_used: u64,
    pub rpm_limit: u64,
    pub resets_at: Option<DateTime<Utc>>,
}

pub async fn fetch_quota(access_token: &str) -> Result<GeminiQuota, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(7))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get("https://generativelanguage.googleapis.com/v1beta/quota")
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Gemini quota endpoint returned {}", res.status()));
    }

    let payload = res.json::<Value>().await.map_err(|e| e.to_string())?;
    let daily_used = find_num(&payload, &["requests_per_day.used", "day.used"]).unwrap_or(0);
    let daily_limit = find_num(&payload, &["requests_per_day.limit", "day.limit"]).unwrap_or(1_000);
    let rpm_used = find_num(&payload, &["requests_per_minute.used", "minute.used"]).unwrap_or(0);
    let rpm_limit =
        find_num(&payload, &["requests_per_minute.limit", "minute.limit"]).unwrap_or(60);

    Ok(GeminiQuota {
        daily_used,
        daily_limit,
        rpm_used,
        rpm_limit,
        resets_at: Some(Utc::now() + chrono::Duration::days(1)),
    })
}

fn find_num(payload: &Value, candidates: &[&str]) -> Option<u64> {
    for c in candidates {
        let mut cur = payload;
        let mut ok = true;
        for seg in c.split('.') {
            if let Some(next) = cur.get(seg) {
                cur = next;
            } else {
                ok = false;
                break;
            }
        }
        if ok {
            if let Some(n) = cur.as_u64() {
                return Some(n);
            }
        }
    }
    None
}
