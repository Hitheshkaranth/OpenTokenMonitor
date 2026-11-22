use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use serde_json::Value;

// The monitor deliberately never refreshes Codex OAuth tokens. OpenAI rotates
// the refresh token on every use, and the Codex CLI owns ~/.codex/auth.json:
// if we spent its refresh_token (or failed to persist the rotated one) the
// CLI would later present a superseded token and could be logged out. We
// only inspect the access token's expiry and fall back to local logs.

pub fn jwt_expires_at_unix_secs(jwt: &str) -> Option<u64> {
    let mut parts = jwt.split('.');
    let (_header, payload_b64, _sig) = match (parts.next(), parts.next(), parts.next()) {
        (Some(h), Some(p), Some(s)) if !h.is_empty() && !p.is_empty() && !s.is_empty() => (h, p, s),
        _ => return None,
    };

    let payload = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let value: Value = serde_json::from_slice(&payload).ok()?;
    value.get("exp")?.as_u64()
}

pub fn is_jwt_expired_with_skew(jwt: &str, skew_secs: u64) -> bool {
    let now = Utc::now().timestamp();
    if now < 0 {
        return true;
    }
    let Some(exp) = jwt_expires_at_unix_secs(jwt) else {
        return true;
    };

    exp <= (now as u64).saturating_add(skew_secs)
}
