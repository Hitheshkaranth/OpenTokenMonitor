#[cfg(target_os = "macos")]
use std::process::Command;

use serde_json::Value;

use crate::usage_scanners::read_claude_oauth_credentials;

const CLAUDE_KEYCHAIN_SERVICE: &str = "Claude Code-credentials";

#[derive(Clone, Debug)]
pub struct ClaudeKeychainCreds {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub source_path: String,
}

pub fn read_access_token() -> Option<String> {
    read_credentials().map(|c| c.access_token)
}

pub fn read_credentials() -> Option<ClaudeKeychainCreds> {
    if let Some(creds) = read_from_os_keychain() {
        return Some(creds);
    }

    let fallback = read_claude_oauth_credentials();
    if fallback.access_token.trim().is_empty() {
        None
    } else {
        Some(ClaudeKeychainCreds {
            access_token: fallback.access_token,
            refresh_token: fallback.refresh_token,
            expires_at: fallback.expires_at,
            source_path: fallback.source_path,
        })
    }
}

fn parse_secret_to_creds(secret: &str, source_path: String) -> Option<ClaudeKeychainCreds> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parsed = serde_json::from_str::<Value>(trimmed).ok();
    if let Some(json) = parsed {
        let access_token = pick_str(
            &json,
            &[
                &["claudeAiOauth", "accessToken"],
                &["claudeAiOauth", "access_token"],
                &["accessToken"],
                &["access_token"],
            ],
        );

        let refresh_token = pick_str(
            &json,
            &[
                &["claudeAiOauth", "refreshToken"],
                &["claudeAiOauth", "refresh_token"],
                &["refreshToken"],
                &["refresh_token"],
            ],
        )
        .and_then(non_empty);

        let expires_at = pick_u64(
            &json,
            &[
                &["claudeAiOauth", "expiresAt"],
                &["claudeAiOauth", "expires_at"],
                &["expiresAt"],
                &["expires_at"],
                &["exp"],
            ],
        );

        if let Some(token) = access_token.and_then(non_empty) {
            return Some(ClaudeKeychainCreds {
                access_token: token,
                refresh_token,
                expires_at,
                source_path,
            });
        }
    }

    Some(ClaudeKeychainCreds {
        access_token: trimmed.to_string(),
        refresh_token: None,
        expires_at: None,
        source_path,
    })
}

fn pick_str(json: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        traverse_path(json, path)
            .and_then(|v| v.as_str())
            .map(str::to_string)
    })
}

fn pick_u64(json: &Value, paths: &[&[&str]]) -> Option<u64> {
    paths
        .iter()
        .find_map(|path| traverse_path(json, path).and_then(|v| v.as_u64()))
}

fn traverse_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cursor = value;
    for key in path {
        cursor = cursor.get(*key)?;
    }
    Some(cursor)
}

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn read_from_os_keychain() -> Option<ClaudeKeychainCreds> {
    #[cfg(target_os = "macos")]
    {
        read_from_macos_keychain()
    }
    #[cfg(target_os = "linux")]
    {
        read_from_keyring_account(CLAUDE_KEYCHAIN_SERVICE, "Linux Secret Service")
    }
    #[cfg(target_os = "windows")]
    {
        read_from_keyring_account(CLAUDE_KEYCHAIN_SERVICE, "Windows Credential Manager")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn read_from_keyring_account(account: &str, os_label: &str) -> Option<ClaudeKeychainCreds> {
    let entry = keyring::Entry::new(CLAUDE_KEYCHAIN_SERVICE, account).ok()?;
    let secret = entry.get_password().ok()?;
    parse_secret_to_creds(&secret, format!("{os_label}: {CLAUDE_KEYCHAIN_SERVICE}"))
}

#[cfg(target_os = "macos")]
fn read_from_macos_keychain() -> Option<ClaudeKeychainCreds> {
    if let Some(account) = std::env::var("USER").ok().and_then(non_empty) {
        if let Ok(entry) = keyring::Entry::new(CLAUDE_KEYCHAIN_SERVICE, &account) {
            if let Ok(secret) = entry.get_password() {
                if let Some(creds) = parse_secret_to_creds(
                    &secret,
                    format!("macOS Keychain: {CLAUDE_KEYCHAIN_SERVICE} ({account})"),
                ) {
                    return Some(creds);
                }
            }
        }
    }

    let output = Command::new("security")
        .args(["find-generic-password", "-s", CLAUDE_KEYCHAIN_SERVICE, "-w"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }
    let secret = String::from_utf8(output.stdout).ok()?;
    parse_secret_to_creds(
        &secret,
        format!("macOS Keychain: {CLAUDE_KEYCHAIN_SERVICE}"),
    )
}
