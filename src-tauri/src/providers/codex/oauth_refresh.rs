use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const OAUTH_TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OAUTH_SCOPE: &str = "openid profile email offline_access";

pub struct RefreshedTokens {
    pub access_token: String,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in_secs: Option<u64>,
}

#[derive(Deserialize)]
struct RefreshTokenResponse {
    access_token: String,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

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

#[allow(dead_code)]
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

pub async fn refresh_access_token(refresh_token: &str) -> Result<RefreshedTokens, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("failed to build refresh client: {e}"))?;

    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", OAUTH_CLIENT_ID),
        ("scope", OAUTH_SCOPE),
    ];

    let res = client
        .post(OAUTH_TOKEN_ENDPOINT)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("token refresh request failed: {e}"))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("token refresh failed with {status}: {body}"));
    }

    let response: RefreshTokenResponse = res
        .json()
        .await
        .map_err(|e| format!("token refresh parse failed: {e}"))?;

    Ok(RefreshedTokens {
        access_token: response.access_token,
        id_token: response.id_token,
        refresh_token: response.refresh_token,
        expires_in_secs: response.expires_in,
    })
}

pub fn persist_refreshed_tokens(refreshed: &RefreshedTokens) -> Result<(), String> {
    let auth_path = codex_auth_path().ok_or_else(|| "cannot resolve home directory".to_string())?;
    let tmp_path = tmp_auth_path(&auth_path);

    let original_contents = fs::read_to_string(&auth_path)
        .map_err(|e| format!("failed to read {}: {e}", auth_path.display()))?;
    let mut auth_json: Value = serde_json::from_str(&original_contents)
        .map_err(|e| format!("failed to parse {}: {e}", auth_path.display()))?;

    if !auth_json.is_object() {
        return Err(format!("{} is not a JSON object", auth_path.display()));
    }

    let tokens = auth_json
        .as_object_mut()
        .ok_or_else(|| "auth payload is not an object".to_string())?
        .entry("tokens".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    if !tokens.is_object() {
        *tokens = Value::Object(serde_json::Map::new());
    }

    let tokens_obj = tokens
        .as_object_mut()
        .ok_or_else(|| "tokens payload is not an object".to_string())?;
    tokens_obj.insert(
        "access_token".to_string(),
        Value::String(refreshed.access_token.clone()),
    );

    if let Some(id_token) = &refreshed.id_token {
        tokens_obj.insert("id_token".to_string(), Value::String(id_token.clone()));
    }

    if let Some(refresh_token) = &refreshed.refresh_token {
        tokens_obj.insert(
            "refresh_token".to_string(),
            Value::String(refresh_token.clone()),
        );
    }

    auth_json["last_refresh"] = Value::String(Utc::now().to_rfc3339());

    let serialized = serde_json::to_string_pretty(&auth_json)
        .map_err(|e| format!("failed to serialize auth payload: {e}"))?;

    fs::write(&tmp_path, serialized)
        .map_err(|e| format!("failed to write {}: {e}", tmp_path.display()))?;

    #[cfg(unix)]
    {
        let metadata = fs::metadata(&auth_path)
            .map_err(|e| format!("failed to stat {}: {e}", auth_path.display()))?;
        let mode = metadata.permissions().mode();

        let mut perms = fs::metadata(&tmp_path)
            .map_err(|e| format!("failed to stat temp file {}: {e}", tmp_path.display()))?
            .permissions();
        perms.set_mode(mode);
        fs::set_permissions(&tmp_path, perms)
            .map_err(|e| format!("failed to set mode on {}: {e}", tmp_path.display()))?;
    }

    fs::rename(&tmp_path, &auth_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!(
            "failed to atomically replace {} with {}: {e}",
            auth_path.display(),
            tmp_path.display()
        )
    })
}

fn codex_auth_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".codex").join("auth.json"))
}

fn tmp_auth_path(auth_path: &std::path::Path) -> PathBuf {
    PathBuf::from(format!("{}.tmp", auth_path.display()))
}
