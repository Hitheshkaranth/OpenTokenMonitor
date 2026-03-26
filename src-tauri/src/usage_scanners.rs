use crate::usage::models::{ProviderId, RecentActivityEntry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, UNIX_EPOCH};

/// Cached results for scan_recent_activity, keyed by (ProviderId, limit).
/// Each entry stores (timestamp, results). Entries older than CACHE_TTL are stale.
static ACTIVITY_CACHE: OnceLock<
    Mutex<HashMap<(ProviderId, usize), (Instant, Vec<RecentActivityEntry>)>>,
> = OnceLock::new();

const ACTIVITY_CACHE_TTL_SECS: u64 = 5;

fn activity_cache(
) -> &'static Mutex<HashMap<(ProviderId, usize), (Instant, Vec<RecentActivityEntry>)>> {
    ACTIVITY_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Clear cached activity results for all providers. Called after refreshes
/// so the next activity query returns fresh data.
pub fn invalidate_activity_cache() {
    if let Ok(mut cache) = activity_cache().lock() {
        cache.clear();
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CodexAuthBridge {
    pub access_token: String,
    pub token_type: Option<String>,
    pub account_id: Option<String>,
    pub expires_at: Option<u64>,
    pub source_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ClaudeOauthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: Option<u64>,
    pub source_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CodexCostSnapshot {
    pub source: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub files_scanned: u64,
    pub sessions_counted: u64,
    pub scan_errors: Vec<String>,
    pub last_scan_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CodexDailyUsagePoint {
    pub day: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CodexModelDailyUsagePoint {
    pub day: String,
    pub model: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ClaudeCostSnapshot {
    pub source: String,
    pub input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub files_scanned: u64,
    pub deduped_chunks: u64,
    pub scan_errors: Vec<String>,
    pub last_scan_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ClaudeDailyUsagePoint {
    pub day: String,
    pub input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ClaudeModelDailyUsagePoint {
    pub day: String,
    pub model: String,
    pub input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Clone, Debug)]
struct GeminiLogFile {
    terminal_label: String,
    path: PathBuf,
}

#[derive(Clone, Debug, Default)]
struct CodexRunningTotals {
    input: u64,
    cached_input: u64,
    output: u64,
}

#[derive(Clone, Debug, Default)]
struct CodexContribution {
    session_id: Option<String>,
    model_hint: Option<String>,
    input: u64,
    cached_input: u64,
    output: u64,
    total: u64,
    cost: f64,
    daily: HashMap<String, CodexDailyUsagePoint>,
    daily_by_model: HashMap<String, CodexModelDailyUsagePoint>,
    is_archived: bool,
    mtime_ms: u64,
}

#[derive(Clone, Debug, Default)]
struct CodexFileCache {
    mtime_ms: u64,
    size: u64,
    parsed_bytes: u64,
    last_model: Option<String>,
    session_id: Option<String>,
    last_totals: CodexRunningTotals,
    contribution: CodexContribution,
}

#[derive(Default)]
struct CodexScannerCache {
    files: HashMap<String, CodexFileCache>,
}

#[derive(Clone, Debug, Default)]
struct ClaudeContribution {
    input: u64,
    cache_read_input: u64,
    cache_creation_input: u64,
    output: u64,
    total: u64,
    cost: f64,
    daily: HashMap<String, ClaudeDailyUsagePoint>,
    daily_by_model: HashMap<String, ClaudeModelDailyUsagePoint>,
    deduped_chunks: u64,
}

/// Token values previously added for a given streaming message,
/// so that when a later chunk arrives we can subtract the old and add the new.
#[derive(Clone, Debug, Default)]
struct ClaudeStreamEntry {
    input: u64,
    cache_read: u64,
    cache_create: u64,
    output: u64,
    cost: f64,
    day: String,
    model: String,
}

#[derive(Clone, Debug, Default)]
struct ClaudeFileCache {
    mtime_ms: u64,
    size: u64,
    parsed_bytes: u64,
    contribution: ClaudeContribution,
    seen_stream_ids: HashMap<String, ClaudeStreamEntry>,
}

#[derive(Default)]
struct ClaudeScannerCache {
    files: HashMap<String, ClaudeFileCache>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeminiDailyUsagePoint {
    pub day: String,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeminiModelDailyUsagePoint {
    pub day: String,
    pub model: String,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Default)]
struct GeminiContribution {
    input: u64,
    cache_read: u64,
    output: u64,
    total: u64,
    cost: f64,
    daily: HashMap<String, GeminiDailyUsagePoint>,
    daily_by_model: HashMap<String, GeminiModelDailyUsagePoint>,
}

#[derive(Clone, Debug, Default)]
struct GeminiFileCache {
    mtime_ms: u64,
    size: u64,
    parsed_bytes: u64,
    contribution: GeminiContribution,
}

#[derive(Default)]
struct GeminiScannerCache {
    files: HashMap<String, GeminiFileCache>,
}

static CODEX_CACHE: OnceLock<Mutex<CodexScannerCache>> = OnceLock::new();
static CLAUDE_CACHE: OnceLock<Mutex<ClaudeScannerCache>> = OnceLock::new();
static GEMINI_CACHE: OnceLock<Mutex<GeminiScannerCache>> = OnceLock::new();

pub fn read_codex_auth_bridge() -> CodexAuthBridge {
    let Some(home) = dirs::home_dir() else {
        return CodexAuthBridge::default();
    };
    let path = home.join(".codex").join("auth.json");
    let Ok(file) = File::open(&path) else {
        return CodexAuthBridge::default();
    };
    let Ok(json) = serde_json::from_reader::<_, Value>(file) else {
        return CodexAuthBridge::default();
    };

    CodexAuthBridge {
        access_token: pick_first_str(
            &json,
            &[
                &["tokens", "access_token"],
                &["access_token"],
                &["token"],
                &["auth", "access_token"],
            ],
        )
        .unwrap_or_default(),
        token_type: pick_first_str(&json, &[&["tokens", "token_type"], &["token_type"]]),
        account_id: pick_first_str(
            &json,
            &[&["account_id"], &["user", "id"], &["tokens", "account_id"]],
        ),
        expires_at: pick_first_u64(
            &json,
            &[&["tokens", "expires_at"], &["expires_at"], &["exp"]],
        ),
        source_path: path.display().to_string(),
    }
}

pub fn read_claude_oauth_credentials() -> ClaudeOauthCredentials {
    let Some(home) = dirs::home_dir() else {
        return ClaudeOauthCredentials::default();
    };
    let path = home.join(".claude").join(".credentials.json");
    let Ok(file) = File::open(&path) else {
        return ClaudeOauthCredentials::default();
    };
    let Ok(json) = serde_json::from_reader::<_, Value>(file) else {
        return ClaudeOauthCredentials::default();
    };

    let scopes = pick_first_array_strings(
        &json,
        &[
            &["claudeAiOauth", "scopes"],
            &["scopes"],
            &["scope"],
            &["oauth", "scopes"],
        ],
    );
    ClaudeOauthCredentials {
        access_token: pick_first_str(
            &json,
            &[
                &["claudeAiOauth", "accessToken"],
                &["claudeAiOauth", "access_token"],
                &["access_token"],
                &["accessToken"],
                &["oauth", "access_token"],
            ],
        )
        .unwrap_or_default(),
        refresh_token: pick_first_str(
            &json,
            &[
                &["claudeAiOauth", "refreshToken"],
                &["claudeAiOauth", "refresh_token"],
                &["refresh_token"],
                &["refreshToken"],
                &["oauth", "refresh_token"],
            ],
        ),
        scopes,
        expires_at: pick_first_u64(
            &json,
            &[
                &["claudeAiOauth", "expiresAt"],
                &["claudeAiOauth", "expires_at"],
                &["expires_at"],
                &["expiresAt"],
                &["exp"],
            ],
        ),
        source_path: path.display().to_string(),
    }
}

pub fn scan_codex_cost_snapshot() -> CodexCostSnapshot {
    let cache = CODEX_CACHE.get_or_init(|| Mutex::new(CodexScannerCache::default()));
    let mut guard = cache.lock().expect("codex scanner cache lock poisoned");
    guard.refresh_codex();
    guard.codex_snapshot()
}

pub fn scan_codex_daily_usage() -> Vec<CodexDailyUsagePoint> {
    let cache = CODEX_CACHE.get_or_init(|| Mutex::new(CodexScannerCache::default()));
    let mut guard = cache.lock().expect("codex scanner cache lock poisoned");
    guard.refresh_codex();
    guard.codex_daily()
}

pub fn scan_codex_model_daily_usage() -> Vec<CodexModelDailyUsagePoint> {
    let cache = CODEX_CACHE.get_or_init(|| Mutex::new(CodexScannerCache::default()));
    let mut guard = cache.lock().expect("codex scanner cache lock poisoned");
    guard.refresh_codex();
    guard.codex_model_daily()
}

pub fn scan_claude_cost_snapshot() -> ClaudeCostSnapshot {
    let cache = CLAUDE_CACHE.get_or_init(|| Mutex::new(ClaudeScannerCache::default()));
    let mut guard = cache.lock().expect("claude scanner cache lock poisoned");
    guard.refresh_claude();
    guard.claude_snapshot()
}

pub fn scan_claude_daily_usage() -> Vec<ClaudeDailyUsagePoint> {
    let cache = CLAUDE_CACHE.get_or_init(|| Mutex::new(ClaudeScannerCache::default()));
    let mut guard = cache.lock().expect("claude scanner cache lock poisoned");
    guard.refresh_claude();
    guard.claude_daily()
}

pub fn scan_claude_model_daily_usage() -> Vec<ClaudeModelDailyUsagePoint> {
    let cache = CLAUDE_CACHE.get_or_init(|| Mutex::new(ClaudeScannerCache::default()));
    let mut guard = cache.lock().expect("claude scanner cache lock poisoned");
    guard.refresh_claude();
    guard.claude_model_daily()
}

pub fn scan_gemini_daily_usage() -> Vec<GeminiDailyUsagePoint> {
    let cache = GEMINI_CACHE.get_or_init(|| Mutex::new(GeminiScannerCache::default()));
    let mut guard = cache.lock().expect("gemini scanner cache lock poisoned");
    guard.refresh_gemini();
    guard.gemini_daily()
}

pub fn scan_gemini_model_daily_usage() -> Vec<GeminiModelDailyUsagePoint> {
    let cache = GEMINI_CACHE.get_or_init(|| Mutex::new(GeminiScannerCache::default()));
    let mut guard = cache.lock().expect("gemini scanner cache lock poisoned");
    guard.refresh_gemini();
    guard.gemini_model_daily()
}

pub fn scan_recent_activity(provider: ProviderId, limit: usize) -> Vec<RecentActivityEntry> {
    // Return cached result if fresh enough.
    if let Ok(cache) = activity_cache().lock() {
        if let Some((ts, entries)) = cache.get(&(provider, limit)) {
            if ts.elapsed().as_secs() < ACTIVITY_CACHE_TTL_SECS {
                return entries.clone();
            }
        }
    }

    let result = match provider {
        ProviderId::Codex => scan_codex_recent_activity(limit),
        ProviderId::Claude => scan_claude_recent_activity(limit),
        ProviderId::Gemini => scan_gemini_recent_activity(limit),
    };

    // Store in cache.
    if let Ok(mut cache) = activity_cache().lock() {
        cache.insert((provider, limit), (Instant::now(), result.clone()));
    }

    result
}

fn scan_codex_recent_activity(limit: usize) -> Vec<RecentActivityEntry> {
    let mut out = Vec::<RecentActivityEntry>::new();

    for path in discover_codex_jsonl_files() {
        let Ok(file) = File::open(path) else {
            continue;
        };
        let reader = BufReader::new(file);
        let mut session_id: Option<String> = None;
        let mut current_cwd: Option<String> = None;
        let mut current_model: Option<String> = None;
        let mut pending: Option<RecentActivityEntry> = None;

        for line in reader.lines().map_while(Result::ok) {
            let Ok(json) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let entry_type = json
                .get("type")
                .and_then(|value| value.as_str())
                .unwrap_or("");

            if entry_type == "session_meta" {
                session_id = pick_first_str(
                    &json,
                    &[&["session_id"], &["payload", "id"], &["session", "id"]],
                )
                .or(session_id);
                current_cwd =
                    pick_first_str(&json, &[&["cwd"], &["payload", "cwd"]]).or(current_cwd);
                continue;
            }

            if entry_type == "turn_context" {
                current_cwd = pick_first_str(&json, &[&["payload", "cwd"]]).or(current_cwd);
                current_model = extract_codex_recent_model(&json).or(current_model.clone());
                continue;
            }

            if entry_type == "event_msg" {
                match json
                    .get("payload")
                    .and_then(|payload| payload.get("type"))
                    .and_then(|value| value.as_str())
                {
                    Some("user_message") => {
                        push_recent_entry(&mut out, pending.take());
                        let Some(prompt) = pick_first_str(&json, &[&["payload", "message"]])
                            .map(|value| normalize_recent_prompt(&value))
                        else {
                            continue;
                        };
                        if prompt.is_empty() {
                            continue;
                        }

                        let timestamp = pick_first_str(&json, &[&["timestamp"]])
                            .and_then(|value| timestamp_from_iso(&value))
                            .unwrap_or_else(chrono::Utc::now);
                        let terminal_label =
                            current_cwd.as_deref().and_then(terminal_label_from_cwd);

                        pending = Some(RecentActivityEntry {
                            provider: ProviderId::Codex,
                            prompt,
                            response: None,
                            timestamp,
                            session_id: session_id.clone(),
                            terminal_label,
                            cwd: current_cwd.clone(),
                            model: current_model.clone(),
                        });
                    }
                    Some("agent_message") => {
                        if let Some(entry) = pending.as_mut() {
                            let timestamp = pick_first_str(&json, &[&["timestamp"]])
                                .and_then(|value| timestamp_from_iso(&value));
                            let response = pick_first_str(&json, &[&["payload", "message"]])
                                .map(|value| normalize_recent_prompt(&value));
                            update_recent_response(entry, response, timestamp);
                        }
                    }
                    _ => {}
                }
                continue;
            }

            if entry_type == "response_item"
                && json
                    .get("payload")
                    .and_then(|payload| payload.get("type"))
                    .and_then(|value| value.as_str())
                    == Some("message")
                && json
                    .get("payload")
                    .and_then(|payload| payload.get("role"))
                    .and_then(|value| value.as_str())
                    == Some("assistant")
            {
                let Some(entry) = pending.as_mut() else {
                    continue;
                };
                let response = extract_codex_assistant_text(&json);
                let timestamp = pick_first_str(&json, &[&["timestamp"]])
                    .and_then(|value| timestamp_from_iso(&value));
                update_recent_response(entry, response, timestamp);
                if entry.model.is_none() {
                    entry.model = current_model.clone();
                }
            }
        }

        push_recent_entry(&mut out, pending);
    }

    sort_and_limit_recent(out, limit)
}

fn scan_claude_recent_activity(limit: usize) -> Vec<RecentActivityEntry> {
    let mut files = Vec::<PathBuf>::new();
    for root in resolve_claude_project_roots() {
        collect_jsonl_recursive(&root, &mut files);
    }

    let mut out = Vec::<RecentActivityEntry>::new();
    for path in files {
        let Ok(file) = File::open(path) else {
            continue;
        };
        let reader = BufReader::new(file);
        let mut messages = Vec::<Value>::new();
        for line in reader.lines().map_while(Result::ok) {
            let Ok(json) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            messages.push(json);
        }

        out.extend(collect_claude_recent_entries(&messages));
    }

    sort_and_limit_recent(out, limit)
}

fn scan_gemini_recent_activity(limit: usize) -> Vec<RecentActivityEntry> {
    let mut out = Vec::<RecentActivityEntry>::new();

    for file in discover_gemini_log_files() {
        let Some(root) = file.path.parent() else {
            continue;
        };
        let mut found_chat_history = false;

        for path in discover_gemini_chat_files(root) {
            found_chat_history = true;
            let Ok(handle) = File::open(path) else {
                continue;
            };
            let Ok(json) = serde_json::from_reader::<_, Value>(BufReader::new(handle)) else {
                continue;
            };
            out.extend(collect_gemini_recent_entries(
                &json,
                file.terminal_label.clone(),
            ));
        }

        if !found_chat_history {
            let chat_models = read_gemini_chat_model_lookup(root);
            let Ok(handle) = File::open(&file.path) else {
                continue;
            };
            let Ok(json) = serde_json::from_reader::<_, Value>(BufReader::new(handle)) else {
                continue;
            };
            let Some(items) = json.as_array() else {
                continue;
            };

            for item in items {
                if item.get("type").and_then(|v| v.as_str()) != Some("user") {
                    continue;
                }

                let Some(prompt) = pick_first_str(item, &[&["message"], &["prompt"]]) else {
                    continue;
                };
                let prompt = normalize_recent_prompt(&prompt);
                if prompt.is_empty() {
                    continue;
                }

                let timestamp = pick_first_str(item, &[&["timestamp"]])
                    .and_then(|value| timestamp_from_iso(&value))
                    .unwrap_or_else(chrono::Utc::now);
                let session_id = pick_first_str(item, &[&["sessionId"], &["session_id"]]);
                let message_id = pick_first_u64(item, &[&["messageId"]]);
                let model = session_id
                    .as_ref()
                    .zip(message_id)
                    .and_then(|(sid, idx)| chat_models.get(&(sid.clone(), idx)).cloned());

                out.push(RecentActivityEntry {
                    provider: ProviderId::Gemini,
                    prompt,
                    response: None,
                    timestamp,
                    session_id,
                    terminal_label: Some(file.terminal_label.clone()),
                    cwd: None,
                    model,
                });
            }
        }
    }

    sort_and_limit_recent(out, limit)
}

fn discover_gemini_log_files() -> Vec<GeminiLogFile> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let root = home.join(".gemini").join("tmp");
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };

    let mut out = Vec::<GeminiLogFile>::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        let log_path = path.join("logs.json");
        if !log_path.exists() {
            continue;
        }

        let Some(terminal_label) = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        out.push(GeminiLogFile {
            terminal_label,
            path: log_path,
        });
    }
    out
}

fn discover_gemini_chat_files(root: &Path) -> Vec<PathBuf> {
    let chats_dir = root.join("chats");
    let Ok(entries) = std::fs::read_dir(chats_dir) else {
        return Vec::new();
    };

    let mut out = Vec::<PathBuf>::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_file() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        out.push(path);
    }
    out
}

fn prompt_text_from_claude_message(json: &Value) -> Option<String> {
    let content = json
        .get("message")
        .and_then(|message| message.get("content"))?;
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }

    let items = content.as_array()?;
    let mut segments = Vec::<String>::new();
    for item in items {
        if item.get("type").and_then(|value| value.as_str()) != Some("text") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
            let normalized = normalize_recent_prompt(text);
            if !normalized.is_empty() {
                segments.push(normalized);
            }
        }
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments.join(" "))
    }
}

fn collect_claude_recent_entries(messages: &[Value]) -> Vec<RecentActivityEntry> {
    let mut out = Vec::<RecentActivityEntry>::new();
    let mut pending: Option<RecentActivityEntry> = None;

    // Claude sessions often emit several assistant records per turn
    // (thinking, tool use, then a visible text reply). Keep the latest
    // visible assistant text attached to the most recent user prompt.
    for json in messages {
        match json.get("type").and_then(|value| value.as_str()) {
            Some("user") => {
                let Some(prompt) = prompt_text_from_claude_message(json) else {
                    continue;
                };
                let prompt = normalize_recent_prompt(&prompt);
                if prompt.is_empty() {
                    continue;
                }

                push_recent_entry(&mut out, pending.take());
                let timestamp = pick_first_str(json, &[&["timestamp"], &["message", "created_at"]])
                    .and_then(|value| timestamp_from_iso(&value))
                    .unwrap_or_else(chrono::Utc::now);
                let cwd = pick_first_str(json, &[&["cwd"]]);

                pending = Some(RecentActivityEntry {
                    provider: ProviderId::Claude,
                    prompt,
                    response: None,
                    timestamp,
                    session_id: pick_first_str(json, &[&["sessionId"], &["session_id"]]),
                    terminal_label: cwd.as_deref().and_then(terminal_label_from_cwd),
                    cwd,
                    model: None,
                });
            }
            Some("assistant") => {
                let Some(entry) = pending.as_mut() else {
                    continue;
                };
                if let Some(model) = extract_claude_message_model(json) {
                    entry.model = Some(model);
                }
                let timestamp = pick_first_str(json, &[&["timestamp"], &["message", "created_at"]])
                    .and_then(|value| timestamp_from_iso(&value));
                let response = assistant_text_from_claude_message(json);
                update_recent_response(entry, response, timestamp);
            }
            _ => {}
        }
    }

    push_recent_entry(&mut out, pending);
    out
}

fn collect_gemini_recent_entries(
    session: &Value,
    terminal_label: String,
) -> Vec<RecentActivityEntry> {
    let Some(messages) = session.get("messages").and_then(|value| value.as_array()) else {
        return Vec::new();
    };

    let session_id = pick_first_str(session, &[&["sessionId"]]);
    let mut out = Vec::<RecentActivityEntry>::new();
    let mut pending: Option<RecentActivityEntry> = None;

    for message in messages {
        match message.get("type").and_then(|value| value.as_str()) {
            Some("user") => {
                let Some(prompt) = extract_gemini_user_prompt(message) else {
                    continue;
                };
                let prompt = normalize_recent_prompt(&prompt);
                if prompt.is_empty() {
                    continue;
                }

                push_recent_entry(&mut out, pending.take());
                let timestamp = pick_first_str(message, &[&["timestamp"]])
                    .and_then(|value| timestamp_from_iso(&value))
                    .unwrap_or_else(chrono::Utc::now);
                pending = Some(RecentActivityEntry {
                    provider: ProviderId::Gemini,
                    prompt,
                    response: None,
                    timestamp,
                    session_id: session_id.clone(),
                    terminal_label: Some(terminal_label.clone()),
                    cwd: None,
                    model: None,
                });
            }
            Some("gemini") => {
                let Some(entry) = pending.as_mut() else {
                    continue;
                };
                if let Some(model) = pick_first_str(message, &[&["model"]]) {
                    entry.model = Some(normalize_gemini_model(&model));
                }
                let timestamp = pick_first_str(message, &[&["timestamp"]])
                    .and_then(|value| timestamp_from_iso(&value));
                let response = extract_gemini_assistant_text(message);
                update_recent_response(entry, response, timestamp);
            }
            _ => {}
        }
    }

    push_recent_entry(&mut out, pending);
    out
}

fn read_gemini_chat_model_lookup(root: &Path) -> HashMap<(String, u64), String> {
    let mut out = HashMap::<(String, u64), String>::new();

    for path in discover_gemini_chat_files(root) {
        let Ok(file) = File::open(path) else {
            continue;
        };
        let Ok(json) = serde_json::from_reader::<_, Value>(BufReader::new(file)) else {
            continue;
        };
        insert_gemini_chat_models_from_session(&json, &mut out);
    }

    out
}

fn insert_gemini_chat_models_from_session(
    session: &Value,
    out: &mut HashMap<(String, u64), String>,
) {
    let Some(session_id) = pick_first_str(session, &[&["sessionId"]]) else {
        return;
    };
    let Some(messages) = session.get("messages").and_then(|value| value.as_array()) else {
        return;
    };

    let mut user_index = messages
        .iter()
        .filter(|message| message.get("type").and_then(|value| value.as_str()) == Some("user"))
        .count() as u64;
    let mut next_model: Option<String> = None;

    for message in messages.iter().rev() {
        match message.get("type").and_then(|value| value.as_str()) {
            Some("gemini") => {
                if let Some(model) = pick_first_str(message, &[&["model"]]) {
                    next_model = Some(normalize_gemini_model(&model));
                }
            }
            Some("user") => {
                user_index = user_index.saturating_sub(1);
                if let Some(model) = next_model.clone() {
                    out.insert((session_id.clone(), user_index), model);
                }
            }
            _ => {}
        }
    }
}

fn extract_codex_assistant_text(json: &Value) -> Option<String> {
    let items = json
        .get("payload")
        .and_then(|payload| payload.get("content"))
        .and_then(|value| value.as_array())?;

    let mut segments = Vec::<String>::new();
    for item in items {
        if item.get("type").and_then(|value| value.as_str()) != Some("output_text") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
            let normalized = normalize_recent_prompt(text);
            if !normalized.is_empty() {
                segments.push(normalized);
            }
        }
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments.join(" "))
    }
}

fn extract_codex_recent_model(json: &Value) -> Option<String> {
    pick_first_str(
        json,
        &[
            &["model"],
            &["payload", "model"],
            &["turn", "model"],
            &["payload", "info", "model"],
            &["payload", "info", "model_name"],
            &["info", "model"],
            &["info", "model_name"],
            &["message", "model"],
        ],
    )
    .map(|model| normalize_codex_model(&model))
}

fn extract_claude_message_model(json: &Value) -> Option<String> {
    pick_first_str(
        json,
        &[
            &["message", "model"],
            &["model"],
            &["message", "model_name"],
            &["payload", "model"],
        ],
    )
    .map(|model| normalize_claude_model(&model))
}

fn assistant_text_from_claude_message(json: &Value) -> Option<String> {
    let content = json
        .get("message")
        .and_then(|message| message.get("content"))?;
    if let Some(text) = content.as_str() {
        let normalized = normalize_recent_prompt(text);
        return (!normalized.is_empty()).then_some(normalized);
    }

    let items = content.as_array()?;
    let mut segments = Vec::<String>::new();
    for item in items {
        if item.get("type").and_then(|value| value.as_str()) != Some("text") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
            let normalized = normalize_recent_prompt(text);
            if !normalized.is_empty() {
                segments.push(normalized);
            }
        }
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments.join(" "))
    }
}

fn extract_gemini_user_prompt(message: &Value) -> Option<String> {
    let content = message.get("content")?.as_array()?;
    let mut segments = Vec::<String>::new();
    for item in content {
        if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
            let normalized = normalize_recent_prompt(text);
            if !normalized.is_empty() {
                segments.push(normalized);
            }
        }
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments.join(" "))
    }
}

fn extract_gemini_assistant_text(message: &Value) -> Option<String> {
    pick_first_str(message, &[&["content"]]).and_then(|value| {
        let normalized = normalize_recent_prompt(&value);
        (!normalized.is_empty()).then_some(normalized)
    })
}

fn update_recent_response(
    entry: &mut RecentActivityEntry,
    response: Option<String>,
    timestamp: Option<chrono::DateTime<chrono::Utc>>,
) {
    if let Some(response) = response {
        entry.response = Some(response);
    }
    if let Some(timestamp) = timestamp {
        entry.timestamp = timestamp;
    }
}

fn push_recent_entry(out: &mut Vec<RecentActivityEntry>, entry: Option<RecentActivityEntry>) {
    if let Some(entry) = entry {
        out.push(entry);
    }
}

fn timestamp_from_iso(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
}

fn terminal_label_from_cwd(value: &str) -> Option<String> {
    Path::new(value)
        .file_name()
        .and_then(|item| item.to_str())
        .map(ToOwned::to_owned)
        .filter(|item| !item.trim().is_empty())
}

fn normalize_recent_prompt(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut trimmed = collapsed.trim().to_string();
    if trimmed.chars().count() <= 220 {
        return trimmed;
    }
    trimmed = trimmed.chars().take(217).collect::<String>();
    trimmed.push_str("...");
    trimmed
}

fn sort_and_limit_recent(
    mut entries: Vec<RecentActivityEntry>,
    limit: usize,
) -> Vec<RecentActivityEntry> {
    let mut deduped = HashMap::<String, RecentActivityEntry>::new();
    for entry in entries.drain(..) {
        let key = format!(
            "{}|{}|{}|{}",
            entry.provider.as_str(),
            entry.session_id.as_deref().unwrap_or(""),
            entry.timestamp.timestamp_millis(),
            entry.prompt
        );
        let replace = deduped
            .get(&key)
            .map(|existing| score_recent_entry(&entry) >= score_recent_entry(existing))
            .unwrap_or(true);
        if replace {
            deduped.insert(key, entry);
        }
    }

    let mut entries: Vec<RecentActivityEntry> = deduped.into_values().collect();
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries.truncate(limit.max(1));
    entries
}

fn score_recent_entry(entry: &RecentActivityEntry) -> usize {
    let mut score = 0;
    if entry.response.is_some() {
        score += 4;
    }
    if entry.model.is_some() {
        score += 2;
    }
    if entry.cwd.is_some() || entry.terminal_label.is_some() {
        score += 1;
    }
    score
}

impl CodexScannerCache {
    fn refresh_codex(&mut self) {
        let discovered = discover_codex_jsonl_files();
        let mut keep = HashSet::<String>::new();
        for path in discovered {
            let key = file_key(&path);
            keep.insert(key.clone());
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let mtime_ms = file_mtime_ms(&meta);
            let size = meta.len();
            let is_archived = path.to_string_lossy().contains("archived_sessions");

            let entry = self.files.entry(key).or_default();
            let unchanged = entry.mtime_ms == mtime_ms && entry.size == size;
            if unchanged {
                continue;
            }

            if size < entry.parsed_bytes {
                *entry = CodexFileCache::default();
            }

            entry.mtime_ms = mtime_ms;
            entry.size = size;
            entry.contribution.is_archived = is_archived;
            entry.contribution.mtime_ms = mtime_ms;

            parse_codex_file_incremental(&path, entry);
        }

        self.files.retain(|k, _| keep.contains(k));
    }

    fn codex_snapshot(&self) -> CodexCostSnapshot {
        let mut snap = CodexCostSnapshot {
            source: "cli-derived".to_string(),
            last_scan_at: epoch_ms_now(),
            ..CodexCostSnapshot::default()
        };
        let deduped = dedupe_codex_contributions(
            self.files
                .values()
                .map(|f| f.contribution.clone())
                .collect(),
        );
        for item in deduped {
            snap.input_tokens = snap.input_tokens.saturating_add(item.input);
            snap.cached_input_tokens = snap.cached_input_tokens.saturating_add(item.cached_input);
            snap.output_tokens = snap.output_tokens.saturating_add(item.output);
            snap.total_tokens = snap.total_tokens.saturating_add(item.total);
            snap.total_cost_usd += item.cost;
            snap.sessions_counted = snap.sessions_counted.saturating_add(1);
        }
        snap.files_scanned = self.files.len() as u64;
        snap
    }

    fn codex_daily(&self) -> Vec<CodexDailyUsagePoint> {
        let mut merged = HashMap::<String, CodexDailyUsagePoint>::new();
        for item in dedupe_codex_contributions(
            self.files
                .values()
                .map(|f| f.contribution.clone())
                .collect(),
        ) {
            for (day, point) in item.daily {
                let slot = merged
                    .entry(day.clone())
                    .or_insert_with(|| CodexDailyUsagePoint {
                        day,
                        ..CodexDailyUsagePoint::default()
                    });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cached_input_tokens = slot
                    .cached_input_tokens
                    .saturating_add(point.cached_input_tokens);
                slot.output_tokens = slot.output_tokens.saturating_add(point.output_tokens);
                slot.total_tokens = slot.total_tokens.saturating_add(point.total_tokens);
                slot.cost_usd += point.cost_usd;
            }
        }
        let mut out: Vec<_> = merged.into_values().collect();
        out.sort_by(|a, b| a.day.cmp(&b.day));
        out
    }

    fn codex_model_daily(&self) -> Vec<CodexModelDailyUsagePoint> {
        let mut merged = HashMap::<String, CodexModelDailyUsagePoint>::new();
        for item in dedupe_codex_contributions(
            self.files
                .values()
                .map(|f| f.contribution.clone())
                .collect(),
        ) {
            for (key, point) in item.daily_by_model {
                let slot = merged
                    .entry(key)
                    .or_insert_with(|| CodexModelDailyUsagePoint {
                        day: point.day.clone(),
                        model: point.model.clone(),
                        ..CodexModelDailyUsagePoint::default()
                    });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cached_input_tokens = slot
                    .cached_input_tokens
                    .saturating_add(point.cached_input_tokens);
                slot.output_tokens = slot.output_tokens.saturating_add(point.output_tokens);
                slot.total_tokens = slot.total_tokens.saturating_add(point.total_tokens);
                slot.cost_usd += point.cost_usd;
            }
        }
        let mut out: Vec<_> = merged.into_values().collect();
        out.sort_by(|a, b| a.day.cmp(&b.day).then(a.model.cmp(&b.model)));
        out
    }
}

impl ClaudeScannerCache {
    fn refresh_claude(&mut self) {
        let roots = resolve_claude_project_roots();
        let mut files = Vec::<PathBuf>::new();
        for root in roots {
            collect_jsonl_recursive(&root, &mut files);
        }

        let mut keep = HashSet::<String>::new();
        for path in files {
            let key = file_key(&path);
            keep.insert(key.clone());
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let mtime_ms = file_mtime_ms(&meta);
            let size = meta.len();

            let entry = self.files.entry(key).or_default();
            let unchanged = entry.mtime_ms == mtime_ms && entry.size == size;
            if unchanged {
                continue;
            }

            if size < entry.parsed_bytes {
                *entry = ClaudeFileCache::default();
            }

            entry.mtime_ms = mtime_ms;
            entry.size = size;
            parse_claude_file_incremental(&path, entry);
        }

        self.files.retain(|k, _| keep.contains(k));
    }

    fn claude_snapshot(&self) -> ClaudeCostSnapshot {
        let mut snap = ClaudeCostSnapshot {
            source: "cli-derived".to_string(),
            last_scan_at: epoch_ms_now(),
            ..ClaudeCostSnapshot::default()
        };
        for file in self.files.values() {
            let c = &file.contribution;
            snap.input_tokens = snap.input_tokens.saturating_add(c.input);
            snap.cache_read_input_tokens = snap
                .cache_read_input_tokens
                .saturating_add(c.cache_read_input);
            snap.cache_creation_input_tokens = snap
                .cache_creation_input_tokens
                .saturating_add(c.cache_creation_input);
            snap.output_tokens = snap.output_tokens.saturating_add(c.output);
            snap.total_tokens = snap.total_tokens.saturating_add(c.total);
            snap.total_cost_usd += c.cost;
            snap.deduped_chunks = snap.deduped_chunks.saturating_add(c.deduped_chunks);
        }
        snap.files_scanned = self.files.len() as u64;
        snap
    }

    fn claude_daily(&self) -> Vec<ClaudeDailyUsagePoint> {
        let mut merged = HashMap::<String, ClaudeDailyUsagePoint>::new();
        for file in self.files.values() {
            for (day, point) in &file.contribution.daily {
                let slot = merged
                    .entry(day.clone())
                    .or_insert_with(|| ClaudeDailyUsagePoint {
                        day: day.clone(),
                        ..ClaudeDailyUsagePoint::default()
                    });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cache_read_input_tokens = slot
                    .cache_read_input_tokens
                    .saturating_add(point.cache_read_input_tokens);
                slot.cache_creation_input_tokens = slot
                    .cache_creation_input_tokens
                    .saturating_add(point.cache_creation_input_tokens);
                slot.output_tokens = slot.output_tokens.saturating_add(point.output_tokens);
                slot.total_tokens = slot.total_tokens.saturating_add(point.total_tokens);
                slot.cost_usd += point.cost_usd;
            }
        }
        let mut out: Vec<_> = merged.into_values().collect();
        out.sort_by(|a, b| a.day.cmp(&b.day));
        out
    }

    fn claude_model_daily(&self) -> Vec<ClaudeModelDailyUsagePoint> {
        let mut merged = HashMap::<String, ClaudeModelDailyUsagePoint>::new();
        for file in self.files.values() {
            for (key, point) in &file.contribution.daily_by_model {
                let slot =
                    merged
                        .entry(key.clone())
                        .or_insert_with(|| ClaudeModelDailyUsagePoint {
                            day: point.day.clone(),
                            model: point.model.clone(),
                            ..ClaudeModelDailyUsagePoint::default()
                        });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cache_read_input_tokens = slot
                    .cache_read_input_tokens
                    .saturating_add(point.cache_read_input_tokens);
                slot.cache_creation_input_tokens = slot
                    .cache_creation_input_tokens
                    .saturating_add(point.cache_creation_input_tokens);
                slot.output_tokens = slot.output_tokens.saturating_add(point.output_tokens);
                slot.total_tokens = slot.total_tokens.saturating_add(point.total_tokens);
                slot.cost_usd += point.cost_usd;
            }
        }
        let mut out: Vec<_> = merged.into_values().collect();
        out.sort_by(|a, b| a.day.cmp(&b.day).then(a.model.cmp(&b.model)));
        out
    }
}
impl GeminiScannerCache {
    fn refresh_gemini(&mut self) {
        let mut files = Vec::<PathBuf>::new();
        for log_file in discover_gemini_log_files() {
            if let Some(root) = log_file.path.parent() {
                files.extend(discover_gemini_chat_files(root));
            }
        }

        let mut keep = HashSet::<String>::new();
        for path in files {
            let key = file_key(&path);
            keep.insert(key.clone());
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let mtime_ms = file_mtime_ms(&meta);
            let size = meta.len();

            let entry = self.files.entry(key).or_default();
            let unchanged = entry.mtime_ms == mtime_ms && entry.size == size;
            if unchanged {
                continue;
            }

            if size < entry.parsed_bytes {
                *entry = GeminiFileCache::default();
            }

            entry.mtime_ms = mtime_ms;
            entry.size = size;
            parse_gemini_file_incremental(&path, entry);
        }

        self.files.retain(|k, _| keep.contains(k));
    }

    fn gemini_daily(&self) -> Vec<GeminiDailyUsagePoint> {
        let mut merged = HashMap::<String, GeminiDailyUsagePoint>::new();
        for file in self.files.values() {
            for (day, point) in &file.contribution.daily {
                let slot = merged
                    .entry(day.clone())
                    .or_insert_with(|| GeminiDailyUsagePoint {
                        day: day.clone(),
                        ..GeminiDailyUsagePoint::default()
                    });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cache_read_tokens = slot
                    .cache_read_tokens
                    .saturating_add(point.cache_read_tokens);
                slot.output_tokens = slot.output_tokens.saturating_add(point.output_tokens);
                slot.total_tokens = slot.total_tokens.saturating_add(point.total_tokens);
                slot.cost_usd += point.cost_usd;
            }
        }
        let mut out: Vec<_> = merged.into_values().collect();
        out.sort_by(|a, b| a.day.cmp(&b.day));
        out
    }

    fn gemini_model_daily(&self) -> Vec<GeminiModelDailyUsagePoint> {
        let mut merged = HashMap::<String, GeminiModelDailyUsagePoint>::new();
        for file in self.files.values() {
            for (key, point) in &file.contribution.daily_by_model {
                let slot =
                    merged
                        .entry(key.clone())
                        .or_insert_with(|| GeminiModelDailyUsagePoint {
                            day: point.day.clone(),
                            model: point.model.clone(),
                            ..GeminiModelDailyUsagePoint::default()
                        });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cache_read_tokens = slot
                    .cache_read_tokens
                    .saturating_add(point.cache_read_tokens);
                slot.output_tokens = slot.output_tokens.saturating_add(point.output_tokens);
                slot.total_tokens = slot.total_tokens.saturating_add(point.total_tokens);
                slot.cost_usd += point.cost_usd;
            }
        }
        let mut out: Vec<_> = merged.into_values().collect();
        out.sort_by(|a, b| a.day.cmp(&b.day).then(a.model.cmp(&b.model)));
        out
    }
}

fn parse_codex_file_incremental(path: &Path, cache: &mut CodexFileCache) {
    let Ok(mut file) = File::open(path) else {
        return;
    };
    if file.seek(SeekFrom::Start(cache.parsed_bytes)).is_err() {
        return;
    }
    let mut buf = Vec::<u8>::new();
    if file.read_to_end(&mut buf).is_err() {
        return;
    }
    let Ok(new_offset) = file.stream_position() else {
        return;
    };
    cache.parsed_bytes = new_offset;

    let reader = BufReader::new(buf.as_slice());
    for line in reader.lines().map_while(Result::ok) {
        let Ok(json) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let entry_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match entry_type {
            "session_meta" => {
                if let Some(sid) = pick_first_str(&json, &[&["session_id"], &["session", "id"]]) {
                    cache.session_id = Some(sid.clone());
                    cache.contribution.session_id = Some(sid);
                }
            }
            "turn_context" => {
                if let Some(model) = pick_first_str(
                    &json,
                    &[&["model"], &["payload", "model"], &["turn", "model"]],
                ) {
                    cache.last_model = Some(normalize_codex_model(&model));
                    cache.contribution.model_hint = cache.last_model.clone();
                }
            }
            "event_msg" => {
                let payload_type =
                    pick_first_str(&json, &[&["payload", "type"], &["payload_type"]])
                        .unwrap_or_default();
                if payload_type != "token_count" {
                    continue;
                }
                let info = json
                    .get("payload")
                    .and_then(|p| p.get("info"))
                    .or_else(|| json.get("info"));
                let Some(info) = info else {
                    continue;
                };

                let model =
                    pick_first_str(info, &[&["model"], &["model_name"], &["metadata", "model"]])
                        .or_else(|| cache.last_model.clone())
                        .unwrap_or_default();
                let model_norm = normalize_codex_model(&model);

                let mut delta_input = 0u64;
                let mut delta_cached = 0u64;
                let mut delta_output = 0u64;

                if let Some(total) = info.get("total_token_usage") {
                    let total_input =
                        pick_first_u64(total, &[&["input_tokens"], &["prompt_tokens"], &["input"]])
                            .unwrap_or(0);
                    let total_cached = pick_first_u64(
                        total,
                        &[&["cached_input_tokens"], &["cache_read_input_tokens"]],
                    )
                    .unwrap_or(0)
                    .min(total_input);
                    let total_output = pick_first_u64(
                        total,
                        &[&["output_tokens"], &["completion_tokens"], &["output"]],
                    )
                    .unwrap_or(0);

                    delta_input = total_input.saturating_sub(cache.last_totals.input);
                    delta_cached = total_cached.saturating_sub(cache.last_totals.cached_input);
                    delta_output = total_output.saturating_sub(cache.last_totals.output);

                    cache.last_totals.input = total_input;
                    cache.last_totals.cached_input = total_cached;
                    cache.last_totals.output = total_output;
                } else if let Some(last) = info.get("last_token_usage") {
                    delta_input =
                        pick_first_u64(last, &[&["input_tokens"], &["prompt_tokens"]]).unwrap_or(0);
                    delta_cached = pick_first_u64(
                        last,
                        &[&["cached_input_tokens"], &["cache_read_input_tokens"]],
                    )
                    .unwrap_or(0)
                    .min(delta_input);
                    delta_output =
                        pick_first_u64(last, &[&["output_tokens"], &["completion_tokens"]])
                            .unwrap_or(0);
                }

                if delta_input == 0 && delta_output == 0 {
                    continue;
                }

                let delta_total = delta_input.saturating_add(delta_output);
                let delta_cost =
                    codex_cost_usd(&model_norm, delta_input, delta_cached, delta_output);
                cache.contribution.input = cache.contribution.input.saturating_add(delta_input);
                cache.contribution.cached_input =
                    cache.contribution.cached_input.saturating_add(delta_cached);
                cache.contribution.output = cache.contribution.output.saturating_add(delta_output);
                cache.contribution.total = cache.contribution.total.saturating_add(delta_total);
                cache.contribution.cost += delta_cost;
                cache.contribution.model_hint = Some(model_norm.clone());
                if cache.contribution.session_id.is_none() {
                    cache.contribution.session_id = cache.session_id.clone();
                }

                let day = day_from_json_or_now(&json);
                let daily = cache
                    .contribution
                    .daily
                    .entry(day.clone())
                    .or_insert_with(|| CodexDailyUsagePoint {
                        day: day.clone(),
                        ..CodexDailyUsagePoint::default()
                    });
                daily.input_tokens = daily.input_tokens.saturating_add(delta_input);
                daily.cached_input_tokens = daily.cached_input_tokens.saturating_add(delta_cached);
                daily.output_tokens = daily.output_tokens.saturating_add(delta_output);
                daily.total_tokens = daily.total_tokens.saturating_add(delta_total);
                daily.cost_usd += delta_cost;

                let model_key = format!("{day}|{model_norm}");
                let model_daily = cache
                    .contribution
                    .daily_by_model
                    .entry(model_key)
                    .or_insert_with(|| CodexModelDailyUsagePoint {
                        day: day.clone(),
                        model: model_norm.clone(),
                        ..CodexModelDailyUsagePoint::default()
                    });
                model_daily.input_tokens = model_daily.input_tokens.saturating_add(delta_input);
                model_daily.cached_input_tokens =
                    model_daily.cached_input_tokens.saturating_add(delta_cached);
                model_daily.output_tokens = model_daily.output_tokens.saturating_add(delta_output);
                model_daily.total_tokens = model_daily.total_tokens.saturating_add(delta_total);
                model_daily.cost_usd += delta_cost;
            }
            _ => {}
        }
    }
}

fn parse_claude_file_incremental(path: &Path, cache: &mut ClaudeFileCache) {
    let Ok(mut file) = File::open(path) else {
        return;
    };
    if file.seek(SeekFrom::Start(cache.parsed_bytes)).is_err() {
        return;
    }
    let mut buf = Vec::<u8>::new();
    if file.read_to_end(&mut buf).is_err() {
        return;
    }
    let Ok(new_offset) = file.stream_position() else {
        return;
    };
    cache.parsed_bytes = new_offset;

    let reader = BufReader::new(buf.as_slice());
    for line in reader.lines().map_while(Result::ok) {
        let Ok(json) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if json.get("type").and_then(|v| v.as_str()) != Some("assistant") {
            continue;
        }
        let Some(usage) = json.get("message").and_then(|m| m.get("usage")) else {
            continue;
        };

        let input = pick_first_u64(usage, &[&["input_tokens"]]).unwrap_or(0);
        let cache_read = pick_first_u64(usage, &[&["cache_read_input_tokens"]]).unwrap_or(0);
        let cache_create = pick_first_u64(usage, &[&["cache_creation_input_tokens"]]).unwrap_or(0);
        let output = pick_first_u64(usage, &[&["output_tokens"]]).unwrap_or(0);
        if input == 0 && cache_read == 0 && cache_create == 0 && output == 0 {
            continue;
        }

        let model = pick_first_str(
            &json,
            &[
                &["message", "model"],
                &["model"],
                &["message", "model_name"],
            ],
        )
        .unwrap_or_default();
        let model_norm = normalize_claude_model(&model);
        let cost = claude_cost_usd(&model_norm, input, cache_read, cache_create, output);
        let total = input.saturating_add(cache_create).saturating_add(output);
        let day = day_from_json_or_now(&json);

        // Claude Code logs multiple streaming chunks per message, each with the same
        // message_id:request_id. Later chunks carry the final (larger) output_tokens.
        // When we see a duplicate key, subtract the old values and add the new ones
        // so we always reflect the latest (most complete) token counts.
        let message_id =
            pick_first_str(&json, &[&["message", "id"], &["message_id"]]).unwrap_or_default();
        let request_id = pick_first_str(
            &json,
            &[&["requestId"], &["request_id"], &["request", "id"]],
        )
        .unwrap_or_default();
        if !message_id.is_empty() && !request_id.is_empty() {
            let dedupe_key = format!("{message_id}:{request_id}");
            if let Some(prev) = cache.seen_stream_ids.get(&dedupe_key) {
                // Subtract previous chunk's contribution before adding the new one
                cache.contribution.deduped_chunks =
                    cache.contribution.deduped_chunks.saturating_add(1);
                let prev_total = prev
                    .input
                    .saturating_add(prev.cache_create)
                    .saturating_add(prev.output);
                cache.contribution.input = cache.contribution.input.saturating_sub(prev.input);
                cache.contribution.cache_read_input = cache
                    .contribution
                    .cache_read_input
                    .saturating_sub(prev.cache_read);
                cache.contribution.cache_creation_input = cache
                    .contribution
                    .cache_creation_input
                    .saturating_sub(prev.cache_create);
                cache.contribution.output = cache.contribution.output.saturating_sub(prev.output);
                cache.contribution.total = cache.contribution.total.saturating_sub(prev_total);
                cache.contribution.cost -= prev.cost;

                if let Some(daily) = cache.contribution.daily.get_mut(&prev.day) {
                    daily.input_tokens = daily.input_tokens.saturating_sub(prev.input);
                    daily.cache_read_input_tokens = daily
                        .cache_read_input_tokens
                        .saturating_sub(prev.cache_read);
                    daily.cache_creation_input_tokens = daily
                        .cache_creation_input_tokens
                        .saturating_sub(prev.cache_create);
                    daily.output_tokens = daily.output_tokens.saturating_sub(prev.output);
                    daily.total_tokens = daily.total_tokens.saturating_sub(prev_total);
                    daily.cost_usd -= prev.cost;
                }

                let prev_model_key = format!("{}|{}", prev.day, prev.model);
                if let Some(md) = cache.contribution.daily_by_model.get_mut(&prev_model_key) {
                    md.input_tokens = md.input_tokens.saturating_sub(prev.input);
                    md.cache_read_input_tokens =
                        md.cache_read_input_tokens.saturating_sub(prev.cache_read);
                    md.cache_creation_input_tokens = md
                        .cache_creation_input_tokens
                        .saturating_sub(prev.cache_create);
                    md.output_tokens = md.output_tokens.saturating_sub(prev.output);
                    md.total_tokens = md.total_tokens.saturating_sub(prev_total);
                    md.cost_usd -= prev.cost;
                }
            }
            cache.seen_stream_ids.insert(
                dedupe_key,
                ClaudeStreamEntry {
                    input,
                    cache_read,
                    cache_create,
                    output,
                    cost,
                    day: day.clone(),
                    model: model_norm.clone(),
                },
            );
        }

        cache.contribution.input = cache.contribution.input.saturating_add(input);
        cache.contribution.cache_read_input = cache
            .contribution
            .cache_read_input
            .saturating_add(cache_read);
        cache.contribution.cache_creation_input = cache
            .contribution
            .cache_creation_input
            .saturating_add(cache_create);
        cache.contribution.output = cache.contribution.output.saturating_add(output);
        cache.contribution.total = cache.contribution.total.saturating_add(total);
        cache.contribution.cost += cost;

        let daily = cache
            .contribution
            .daily
            .entry(day.clone())
            .or_insert_with(|| ClaudeDailyUsagePoint {
                day: day.clone(),
                ..ClaudeDailyUsagePoint::default()
            });
        daily.input_tokens = daily.input_tokens.saturating_add(input);
        daily.cache_read_input_tokens = daily.cache_read_input_tokens.saturating_add(cache_read);
        daily.cache_creation_input_tokens = daily
            .cache_creation_input_tokens
            .saturating_add(cache_create);
        daily.output_tokens = daily.output_tokens.saturating_add(output);
        daily.total_tokens = daily.total_tokens.saturating_add(total);
        daily.cost_usd += cost;

        let model_key = format!("{day}|{model_norm}");
        let model_daily = cache
            .contribution
            .daily_by_model
            .entry(model_key)
            .or_insert_with(|| ClaudeModelDailyUsagePoint {
                day: day.clone(),
                model: model_norm.clone(),
                ..ClaudeModelDailyUsagePoint::default()
            });
        model_daily.input_tokens = model_daily.input_tokens.saturating_add(input);
        model_daily.cache_read_input_tokens = model_daily
            .cache_read_input_tokens
            .saturating_add(cache_read);
        model_daily.cache_creation_input_tokens = model_daily
            .cache_creation_input_tokens
            .saturating_add(cache_create);
        model_daily.output_tokens = model_daily.output_tokens.saturating_add(output);
        model_daily.total_tokens = model_daily.total_tokens.saturating_add(total);
        model_daily.cost_usd += cost;
    }
}
struct GeminiPricePoint {
    input_usd_per_1m: f64,
    output_usd_per_1m: f64,
}

static GEMINI_PRICES: OnceLock<HashMap<&'static str, GeminiPricePoint>> = OnceLock::new();

fn gemini_cost_usd(model: &str, input: u64, _cached: u64, output: u64) -> f64 {
    let prices = GEMINI_PRICES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(
            "gemini-2.0-flash",
            GeminiPricePoint {
                input_usd_per_1m: 0.10,
                output_usd_per_1m: 0.40,
            },
        );
        m.insert(
            "gemini-2.0-flash-lite",
            GeminiPricePoint {
                input_usd_per_1m: 0.075,
                output_usd_per_1m: 0.30,
            },
        );
        m.insert(
            "gemini-1.5-flash",
            GeminiPricePoint {
                input_usd_per_1m: 0.10,
                output_usd_per_1m: 0.40,
            },
        );
        m.insert(
            "gemini-1.5-pro",
            GeminiPricePoint {
                input_usd_per_1m: 1.25,
                output_usd_per_1m: 5.00,
            },
        );
        m
    });

    if let Some(price) = prices.get(model) {
        let input_cost = (input as f64 / 1_000_000.0) * price.input_usd_per_1m;
        let output_cost = (output as f64 / 1_000_000.0) * price.output_usd_per_1m;
        input_cost + output_cost
    } else {
        let input_cost = (input as f64 / 1_000_000.0) * 0.15;
        let output_cost = (output as f64 / 1_000_000.0) * 0.60;
        input_cost + output_cost
    }
}

fn parse_gemini_file_incremental(path: &Path, cache: &mut GeminiFileCache) {
    let Ok(file) = File::open(path) else {
        return;
    };
    // Gemini session files are full JSON objects, not JSONL.
    // We read the whole thing and update our contribution.
    // Since they are small, we don't need to be truly incremental here,
    // we just replace the contribution from this file.
    let Ok(json) = serde_json::from_reader::<_, Value>(BufReader::new(file)) else {
        return;
    };

    let mut contribution = GeminiContribution::default();
    let day = pick_first_str(&json, &[&["startTime"], &["timestamp"]])
        .and_then(|s| try_iso_to_day(&s))
        .unwrap_or_else(|| day_from_ms(epoch_ms_now()));

    if let Some(messages) = json.get("messages").and_then(|v| v.as_array()) {
        for msg in messages {
            if let Some(tokens) = msg.get("tokens") {
                let input = pick_first_u64(tokens, &[&["input"]]).unwrap_or(0);
                let output = pick_first_u64(tokens, &[&["output"]]).unwrap_or(0);
                let cached = pick_first_u64(tokens, &[&["cached"]]).unwrap_or(0);
                let total = pick_first_u64(tokens, &[&["total"]]).unwrap_or(input + output);

                let model = msg
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("gemini-mixed");
                let model_norm = normalize_gemini_model(model);

                let cost = gemini_cost_usd(&model_norm, input, cached, output);

                contribution.input = contribution.input.saturating_add(input);

                contribution.cache_read = contribution.cache_read.saturating_add(cached);
                contribution.output = contribution.output.saturating_add(output);
                contribution.total = contribution.total.saturating_add(total);
                contribution.cost += cost;

                let daily = contribution.daily.entry(day.clone()).or_insert_with(|| {
                    GeminiDailyUsagePoint {
                        day: day.clone(),
                        ..GeminiDailyUsagePoint::default()
                    }
                });
                daily.input_tokens = daily.input_tokens.saturating_add(input);
                daily.cache_read_tokens = daily.cache_read_tokens.saturating_add(cached);
                daily.output_tokens = daily.output_tokens.saturating_add(output);
                daily.total_tokens = daily.total_tokens.saturating_add(total);
                daily.cost_usd += cost;

                let model_key = format!("{day}|{model_norm}");
                let model_daily =
                    contribution
                        .daily_by_model
                        .entry(model_key)
                        .or_insert_with(|| GeminiModelDailyUsagePoint {
                            day: day.clone(),
                            model: model_norm.clone(),
                            ..GeminiModelDailyUsagePoint::default()
                        });
                model_daily.input_tokens = model_daily.input_tokens.saturating_add(input);
                model_daily.cache_read_tokens =
                    model_daily.cache_read_tokens.saturating_add(cached);
                model_daily.output_tokens = model_daily.output_tokens.saturating_add(output);
                model_daily.total_tokens = model_daily.total_tokens.saturating_add(total);
                model_daily.cost_usd += cost;
            }
        }
    }

    cache.contribution = contribution;
}

fn dedupe_codex_contributions(items: Vec<CodexContribution>) -> Vec<CodexContribution> {
    let mut by_session = HashMap::<String, CodexContribution>::new();
    let mut no_session = Vec::<CodexContribution>::new();

    for item in items {
        if let Some(sid) = &item.session_id {
            let keep = match by_session.get(sid) {
                Some(existing) => should_replace_codex(existing, &item),
                None => true,
            };
            if keep {
                by_session.insert(sid.clone(), item);
            }
        } else {
            no_session.push(item);
        }
    }

    let mut out: Vec<CodexContribution> = by_session.into_values().collect();
    out.extend(no_session);
    out
}

fn should_replace_codex(existing: &CodexContribution, next: &CodexContribution) -> bool {
    if existing.is_archived != next.is_archived {
        return existing.is_archived && !next.is_archived;
    }
    next.mtime_ms >= existing.mtime_ms
}

fn discover_codex_jsonl_files() -> Vec<PathBuf> {
    let mut roots = Vec::<PathBuf>::new();
    if let Ok(custom) = std::env::var("CODEX_HOME") {
        if !custom.trim().is_empty() {
            roots.push(PathBuf::from(custom));
        }
    }
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".codex"));
    }

    let mut out = Vec::<PathBuf>::new();
    for root in roots {
        collect_jsonl_recursive(&root.join("sessions"), &mut out);
        collect_jsonl_recursive(&root.join("archived_sessions"), &mut out);
    }
    out
}

fn resolve_claude_project_roots() -> Vec<PathBuf> {
    let mut roots = Vec::<PathBuf>::new();
    if let Ok(config_roots) = std::env::var("CLAUDE_CONFIG_DIR") {
        for part in config_roots.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            roots.push(PathBuf::from(trimmed).join("projects"));
        }
    }
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".config").join("claude").join("projects"));
        roots.push(home.join(".claude").join("projects"));
    }
    roots
}

fn collect_jsonl_recursive(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_dir() {
            collect_jsonl_recursive(&path, out);
            continue;
        }
        let is_jsonl = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("jsonl"))
            .unwrap_or(false);
        if is_jsonl {
            out.push(path);
        }
    }
}

fn codex_cost_usd(model: &str, input: u64, cached_input: u64, output: u64) -> f64 {
    let Some((in_per_m, cached_per_m, out_per_m)) = codex_price_table(model) else {
        return 0.0;
    };
    let non_cached = input.saturating_sub(cached_input);
    (non_cached as f64 / 1_000_000.0) * in_per_m
        + (cached_input as f64 / 1_000_000.0) * cached_per_m
        + (output as f64 / 1_000_000.0) * out_per_m
}

fn claude_cost_usd(
    model: &str,
    input: u64,
    cache_read: u64,
    cache_create: u64,
    output: u64,
) -> f64 {
    let Some((in_per_m, read_per_m, create_per_m, out_per_m)) = claude_price_table(model) else {
        return 0.0;
    };
    (input as f64 / 1_000_000.0) * in_per_m
        + (cache_read as f64 / 1_000_000.0) * read_per_m
        + (cache_create as f64 / 1_000_000.0) * create_per_m
        + (output as f64 / 1_000_000.0) * out_per_m
}

fn codex_price_table(model: &str) -> Option<(f64, f64, f64)> {
    let m = model.to_ascii_lowercase();
    if m.contains("gpt-5") && m.contains("nano") {
        return Some((0.05, 0.005, 0.40));
    }
    if m.contains("gpt-5") && m.contains("mini") {
        return Some((0.25, 0.025, 2.00));
    }
    if m.contains("gpt-5") {
        return Some((1.25, 0.125, 10.00));
    }
    if m.contains("gpt-4.1-mini") {
        return Some((0.40, 0.04, 1.60));
    }
    if m.contains("gpt-4.1") {
        return Some((2.00, 0.20, 8.00));
    }
    None
}

fn claude_price_table(model: &str) -> Option<(f64, f64, f64, f64)> {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        return Some((15.0, 1.5, 18.75, 75.0));
    }
    if m.contains("sonnet") {
        return Some((3.0, 0.3, 3.75, 15.0));
    }
    if m.contains("haiku") {
        return Some((0.25, 0.03, 0.30, 1.25));
    }
    None
}

fn normalize_codex_model(model: &str) -> String {
    let raw = model.trim().to_ascii_lowercase();
    if raw.contains("gpt-5-codex") || raw.contains("openai/gpt-5") || raw == "gpt-5" {
        return "gpt-5".to_string();
    }
    if raw.contains("gpt-5-mini") {
        return "gpt-5-mini".to_string();
    }
    if raw.contains("gpt-5-nano") {
        return "gpt-5-nano".to_string();
    }
    if raw.contains("gpt-4.1-mini") {
        return "gpt-4.1-mini".to_string();
    }
    if raw.contains("gpt-4.1") {
        return "gpt-4.1".to_string();
    }
    raw
}

fn normalize_claude_model(model: &str) -> String {
    let raw = model.trim().to_ascii_lowercase();
    if raw.contains("opus") {
        return "claude-opus".to_string();
    }
    if raw.contains("sonnet") {
        return "claude-sonnet".to_string();
    }
    if raw.contains("haiku") {
        return "claude-haiku".to_string();
    }
    raw
}

fn normalize_gemini_model(model: &str) -> String {
    model.trim().to_ascii_lowercase()
}

fn pick_first_str(value: &Value, candidates: &[&[&str]]) -> Option<String> {
    for path in candidates {
        if let Some(v) = get_at_path(value, path).and_then(|v| v.as_str()) {
            if !v.trim().is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn pick_first_u64(value: &Value, candidates: &[&[&str]]) -> Option<u64> {
    for path in candidates {
        if let Some(v) = get_at_path(value, path) {
            if let Some(n) = v.as_u64() {
                return Some(n);
            }
            if let Some(s) = v.as_str() {
                if let Ok(n) = s.parse::<u64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn pick_first_array_strings(value: &Value, candidates: &[&[&str]]) -> Vec<String> {
    for path in candidates {
        if let Some(arr) = get_at_path(value, path).and_then(|v| v.as_array()) {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .filter(|s| !s.trim().is_empty())
                .collect();
        }
    }
    Vec::new()
}

fn get_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn day_from_json_or_now(json: &Value) -> String {
    // First try numeric timestamps
    if let Some(ts) = pick_first_u64(
        json,
        &[
            &["timestamp"],
            &["ts"],
            &["created_at"],
            &["message", "created_at"],
        ],
    ) {
        return if ts > 10_000_000_000 {
            day_from_ms(ts)
        } else {
            day_from_ms(ts.saturating_mul(1000))
        };
    }

    // Then try ISO 8601 / RFC 3339 string timestamps (e.g. "2026-01-16T18:10:57.334Z")
    for path in &[
        &["timestamp"][..],
        &["ts"],
        &["created_at"],
        &["message", "created_at"],
    ] {
        if let Some(s) = get_at_path(json, path).and_then(|v| v.as_str()) {
            if let Some(day) = try_iso_to_day(s) {
                return day;
            }
        }
    }

    // Fallback to current time
    day_from_ms(epoch_ms_now())
}

/// Extract "YYYY-MM-DD" from an ISO 8601 / RFC 3339 string like "2026-01-16T18:10:57.334Z".
fn try_iso_to_day(s: &str) -> Option<String> {
    // Expect at least "YYYY-MM-DD" (10 chars) with dashes at positions 4 and 7
    if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        let date_part = &s[..10];
        // Basic validation: all other chars should be digits
        let valid = date_part.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
        if valid {
            return Some(date_part.to_string());
        }
    }
    None
}

fn day_from_ms(ms: u64) -> String {
    let total_days = (ms / 1000) / 86_400;
    ymd_from_days(total_days)
}

fn ymd_from_days(mut days: u64) -> String {
    let mut year = 1970u64;
    loop {
        let dy = if is_leap(year) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }
    let months = [
        31u64,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for dm in months {
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
    }
    let day = days + 1;
    format!("{year:04}-{month:02}-{day:02}")
}

fn is_leap(year: u64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn file_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn file_mtime_ms(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn epoch_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, remove_dir_all, OpenOptions};
    use std::io::Write;

    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("otm-test-{name}-{}", epoch_ms_now()));
        let _ = remove_dir_all(&base);
        create_dir_all(&base).expect("create temp dir");
        base
    }

    #[test]
    fn model_aliases_normalize() {
        assert_eq!(normalize_codex_model("openai/gpt-5"), "gpt-5");
        assert_eq!(normalize_codex_model("gpt-5-codex"), "gpt-5");
    }

    #[test]
    fn codex_incremental_delta_works() {
        let dir = temp_dir("codex-incremental");
        let file = dir.join("session.jsonl");
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)
            .unwrap();
        writeln!(f, "{{\"type\":\"session_meta\",\"session_id\":\"s1\"}}").unwrap();
        writeln!(
            f,
            "{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"total_token_usage\":{{\"input_tokens\":100,\"cached_input_tokens\":20,\"output_tokens\":30}},\"model\":\"gpt-5\"}}}}}}"
        )
        .unwrap();

        let mut cache = CodexFileCache::default();
        parse_codex_file_incremental(&file, &mut cache);
        assert_eq!(cache.contribution.input, 100);
        assert_eq!(cache.contribution.output, 30);

        writeln!(
            f,
            "{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"total_token_usage\":{{\"input_tokens\":150,\"cached_input_tokens\":25,\"output_tokens\":60}},\"model\":\"gpt-5\"}}}}}}"
        )
        .unwrap();
        parse_codex_file_incremental(&file, &mut cache);
        assert_eq!(cache.contribution.input, 150);
        assert_eq!(cache.contribution.output, 60);

        let _ = remove_dir_all(dir);
    }

    #[test]
    fn codex_dedupe_prefers_active_over_archived() {
        let active = CodexContribution {
            session_id: Some("sid-1".to_string()),
            input: 100,
            is_archived: false,
            ..CodexContribution::default()
        };
        let archived = CodexContribution {
            session_id: Some("sid-1".to_string()),
            input: 999,
            is_archived: true,
            ..CodexContribution::default()
        };
        let out = dedupe_codex_contributions(vec![archived, active.clone()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].input, active.input);
    }

    #[test]
    fn gemini_session_parsing_v2() {
        let dir = temp_dir("gemini-v2");
        let file = dir.join("session-1.json");
        let mut f = File::create(&file).unwrap();
        writeln!(
            f,
            "{}",
            r#"{
  "startTime": "2026-03-22T17:25:19.501Z",
  "messages": [
    {
      "type": "user",
      "tokens": { "input": 100, "output": 0, "total": 100 }
    },
    {
      "type": "gemini",
      "model": "gemini-3-flash",
      "tokens": { "input": 0, "output": 50, "cached": 10, "total": 60 }
    }
  ]
}"#
        )
        .unwrap();

        let mut cache = GeminiFileCache::default();
        parse_gemini_file_incremental(&file, &mut cache);

        assert_eq!(cache.contribution.input, 100);
        assert_eq!(cache.contribution.output, 50);
        assert_eq!(cache.contribution.cache_read, 10);
        assert_eq!(cache.contribution.total, 160);
        assert!((cache.contribution.cost - 0.000045).abs() < 0.000001);
        assert!(cache.contribution.daily.contains_key("2026-03-22"));

        let _ = remove_dir_all(dir);
    }

    #[test]
    fn claude_chunk_dedupe_by_message_and_request() {
        let dir = temp_dir("claude-dedupe");
        let file = dir.join("session.jsonl");
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)
            .unwrap();
        // Two identical streaming chunks for the same message — should count once
        writeln!(
            f,
            "{{\"type\":\"assistant\",\"requestId\":\"r1\",\"message\":{{\"id\":\"m1\",\"model\":\"claude-3-7-sonnet\",\"usage\":{{\"input_tokens\":10,\"cache_read_input_tokens\":2,\"cache_creation_input_tokens\":0,\"output_tokens\":3}}}}}}"
        )
        .unwrap();
        writeln!(
            f,
            "{{\"type\":\"assistant\",\"requestId\":\"r1\",\"message\":{{\"id\":\"m1\",\"model\":\"claude-3-7-sonnet\",\"usage\":{{\"input_tokens\":10,\"cache_read_input_tokens\":2,\"cache_creation_input_tokens\":0,\"output_tokens\":3}}}}}}"
        )
        .unwrap();

        let mut cache = ClaudeFileCache::default();
        parse_claude_file_incremental(&file, &mut cache);
        assert_eq!(cache.contribution.input, 10);
        assert_eq!(cache.contribution.output, 3);
        assert_eq!(cache.contribution.deduped_chunks, 1);

        let _ = remove_dir_all(dir);
    }

    #[test]
    fn claude_streaming_chunks_keep_final_output_tokens() {
        // Claude Code logs multiple streaming chunks per message.
        // The first chunk has a small output_tokens (e.g. 2), intermediate chunks
        // have partial counts, and the last chunk has the final count (e.g. 309).
        // The parser must reflect the LAST chunk's values, not the first.
        let dir = temp_dir("claude-streaming");
        let file = dir.join("session.jsonl");
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)
            .unwrap();
        // First streaming chunk — small output
        writeln!(
            f,
            "{{\"type\":\"assistant\",\"requestId\":\"r1\",\"message\":{{\"id\":\"m1\",\"model\":\"claude-3-7-sonnet\",\"usage\":{{\"input_tokens\":3,\"cache_read_input_tokens\":100,\"cache_creation_input_tokens\":200,\"output_tokens\":2}}}}}}"
        ).unwrap();
        // Intermediate chunk
        writeln!(
            f,
            "{{\"type\":\"assistant\",\"requestId\":\"r1\",\"message\":{{\"id\":\"m1\",\"model\":\"claude-3-7-sonnet\",\"usage\":{{\"input_tokens\":3,\"cache_read_input_tokens\":100,\"cache_creation_input_tokens\":200,\"output_tokens\":8}}}}}}"
        ).unwrap();
        // Final chunk — full output
        writeln!(
            f,
            "{{\"type\":\"assistant\",\"requestId\":\"r1\",\"message\":{{\"id\":\"m1\",\"model\":\"claude-3-7-sonnet\",\"usage\":{{\"input_tokens\":3,\"cache_read_input_tokens\":100,\"cache_creation_input_tokens\":200,\"output_tokens\":309}}}}}}"
        ).unwrap();
        // Second message — single chunk (no streaming)
        writeln!(
            f,
            "{{\"type\":\"assistant\",\"requestId\":\"r2\",\"message\":{{\"id\":\"m2\",\"model\":\"claude-3-7-sonnet\",\"usage\":{{\"input_tokens\":5,\"cache_read_input_tokens\":50,\"cache_creation_input_tokens\":0,\"output_tokens\":100}}}}}}"
        ).unwrap();

        let mut cache = ClaudeFileCache::default();
        parse_claude_file_incremental(&file, &mut cache);

        // Message m1: final values are input=3, cache_read=100, cache_create=200, output=309
        // Message m2: input=5, cache_read=50, cache_create=0, output=100
        // Totals: input=8, cache_read=150, cache_create=200, output=409
        assert_eq!(cache.contribution.input, 8);
        assert_eq!(cache.contribution.cache_read_input, 150);
        assert_eq!(cache.contribution.cache_creation_input, 200);
        assert_eq!(cache.contribution.output, 409);
        assert_eq!(cache.contribution.deduped_chunks, 2); // two intermediate chunks replaced

        let _ = remove_dir_all(dir);
    }

    #[test]
    fn unknown_model_keeps_tokens_without_cost() {
        assert_eq!(codex_cost_usd("unknown-model", 1000, 0, 500), 0.0);
        assert_eq!(claude_cost_usd("unknown-model", 1000, 0, 0, 500), 0.0);
    }

    #[test]
    fn claude_recent_activity_uses_next_assistant_model() {
        let messages = vec![
            serde_json::json!({
                "type": "user",
                "timestamp": "2026-03-14T02:47:35.799Z",
                "sessionId": "claude-session",
                "cwd": "C:\\Users\\hithe\\Documents\\SIDE_QUESTS\\OpenTokenMonitor",
                "message": { "role": "user", "content": "install binwalk" }
            }),
            serde_json::json!({
                "type": "assistant",
                "timestamp": "2026-03-14T02:47:38.989Z",
                "message": {
                    "model": "claude-opus-4-6",
                    "content": [{ "type": "text", "text": "I can help with that." }]
                }
            }),
        ];

        let entries = collect_claude_recent_entries(&messages);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].prompt, "install binwalk");
        assert_eq!(entries[0].model.as_deref(), Some("claude-opus"));
        assert_eq!(
            entries[0].response.as_deref(),
            Some("I can help with that.")
        );
    }

    #[test]
    fn gemini_chat_lookup_maps_user_index_to_following_model() {
        let session = serde_json::json!({
            "sessionId": "gemini-session",
            "messages": [
                {
                    "type": "user",
                    "content": [{ "text": "first prompt" }]
                },
                {
                    "type": "gemini",
                    "model": "gemini-3-flash-preview"
                },
                {
                    "type": "user",
                    "content": [{ "text": "second prompt" }]
                },
                {
                    "type": "gemini",
                    "model": "gemini-2.5-pro"
                }
            ]
        });

        let mut lookup = HashMap::<(String, u64), String>::new();
        insert_gemini_chat_models_from_session(&session, &mut lookup);

        assert_eq!(
            lookup
                .get(&(String::from("gemini-session"), 0))
                .map(String::as_str),
            Some("gemini-3-flash-preview")
        );
        assert_eq!(
            lookup
                .get(&(String::from("gemini-session"), 1))
                .map(String::as_str),
            Some("gemini-2.5-pro")
        );
    }

    #[test]
    fn gemini_recent_activity_captures_latest_reply_text() {
        let session = serde_json::json!({
            "sessionId": "gemini-session",
            "messages": [
                {
                    "type": "user",
                    "timestamp": "2026-03-14T23:56:31.608Z",
                    "content": [{ "text": "first prompt" }]
                },
                {
                    "type": "gemini",
                    "timestamp": "2026-03-14T23:56:32.608Z",
                    "content": "planning reply",
                    "model": "gemini-3-flash-preview"
                },
                {
                    "type": "gemini",
                    "timestamp": "2026-03-14T23:56:33.608Z",
                    "content": "final reply",
                    "model": "gemini-3-flash-preview"
                }
            ]
        });

        let entries = collect_gemini_recent_entries(&session, "opentokenmonitor".to_string());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].prompt, "first prompt");
        assert_eq!(entries[0].response.as_deref(), Some("final reply"));
        assert_eq!(entries[0].model.as_deref(), Some("gemini-3-flash-preview"));
    }

    #[test]
    fn codex_recent_reply_prefers_output_text() {
        let json = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [
                    { "type": "output_text", "text": "first line" },
                    { "type": "output_text", "text": "second line" }
                ]
            }
        });

        assert_eq!(
            extract_codex_assistant_text(&json).as_deref(),
            Some("first line second line")
        );
    }
}
