use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

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

#[derive(Clone, Debug, Default)]
struct ClaudeFileCache {
    mtime_ms: u64,
    size: u64,
    parsed_bytes: u64,
    contribution: ClaudeContribution,
    seen_stream_ids: HashSet<String>,
}

#[derive(Default)]
struct ClaudeScannerCache {
    files: HashMap<String, ClaudeFileCache>,
}

static CODEX_CACHE: OnceLock<Mutex<CodexScannerCache>> = OnceLock::new();
static CLAUDE_CACHE: OnceLock<Mutex<ClaudeScannerCache>> = OnceLock::new();

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

    let scopes = pick_first_array_strings(&json, &[
        &["claudeAiOauth", "scopes"],
        &["scopes"],
        &["scope"],
        &["oauth", "scopes"],
    ]);
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
        expires_at: pick_first_u64(&json, &[
            &["claudeAiOauth", "expiresAt"],
            &["claudeAiOauth", "expires_at"],
            &["expires_at"],
            &["expiresAt"],
            &["exp"],
        ]),
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
        let deduped = dedupe_codex_contributions(self.files.values().map(|f| f.contribution.clone()).collect());
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
        for item in dedupe_codex_contributions(self.files.values().map(|f| f.contribution.clone()).collect()) {
            for (day, point) in item.daily {
                let slot = merged.entry(day.clone()).or_insert_with(|| CodexDailyUsagePoint {
                    day,
                    ..CodexDailyUsagePoint::default()
                });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cached_input_tokens = slot.cached_input_tokens.saturating_add(point.cached_input_tokens);
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
        for item in dedupe_codex_contributions(self.files.values().map(|f| f.contribution.clone()).collect()) {
            for (key, point) in item.daily_by_model {
                let slot = merged.entry(key).or_insert_with(|| CodexModelDailyUsagePoint {
                    day: point.day.clone(),
                    model: point.model.clone(),
                    ..CodexModelDailyUsagePoint::default()
                });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cached_input_tokens = slot.cached_input_tokens.saturating_add(point.cached_input_tokens);
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
            snap.cache_read_input_tokens = snap.cache_read_input_tokens.saturating_add(c.cache_read_input);
            snap.cache_creation_input_tokens = snap.cache_creation_input_tokens.saturating_add(c.cache_creation_input);
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
                let slot = merged.entry(day.clone()).or_insert_with(|| ClaudeDailyUsagePoint {
                    day: day.clone(),
                    ..ClaudeDailyUsagePoint::default()
                });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cache_read_input_tokens = slot.cache_read_input_tokens.saturating_add(point.cache_read_input_tokens);
                slot.cache_creation_input_tokens = slot.cache_creation_input_tokens.saturating_add(point.cache_creation_input_tokens);
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
                let slot = merged.entry(key.clone()).or_insert_with(|| ClaudeModelDailyUsagePoint {
                    day: point.day.clone(),
                    model: point.model.clone(),
                    ..ClaudeModelDailyUsagePoint::default()
                });
                slot.input_tokens = slot.input_tokens.saturating_add(point.input_tokens);
                slot.cache_read_input_tokens = slot.cache_read_input_tokens.saturating_add(point.cache_read_input_tokens);
                slot.cache_creation_input_tokens = slot.cache_creation_input_tokens.saturating_add(point.cache_creation_input_tokens);
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
    let Ok(mut file) = File::open(path) else { return; };
    if file.seek(SeekFrom::Start(cache.parsed_bytes)).is_err() {
        return;
    }
    let mut buf = Vec::<u8>::new();
    if file.read_to_end(&mut buf).is_err() {
        return;
    }
    let Ok(new_offset) = file.stream_position() else { return; };
    cache.parsed_bytes = new_offset;

    let reader = BufReader::new(buf.as_slice());
    for line in reader.lines().map_while(Result::ok) {
        let Ok(json) = serde_json::from_str::<Value>(&line) else { continue; };
        let entry_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match entry_type {
            "session_meta" => {
                if let Some(sid) = pick_first_str(&json, &[&["session_id"], &["session", "id"]]) {
                    cache.session_id = Some(sid.clone());
                    cache.contribution.session_id = Some(sid);
                }
            }
            "turn_context" => {
                if let Some(model) = pick_first_str(&json, &[&["model"], &["payload", "model"], &["turn", "model"]]) {
                    cache.last_model = Some(normalize_codex_model(&model));
                    cache.contribution.model_hint = cache.last_model.clone();
                }
            }
            "event_msg" => {
                let payload_type = pick_first_str(&json, &[&["payload", "type"], &["payload_type"]]).unwrap_or_default();
                if payload_type != "token_count" { continue; }
                let info = json.get("payload").and_then(|p| p.get("info")).or_else(|| json.get("info"));
                let Some(info) = info else { continue; };

                let model = pick_first_str(info, &[&["model"], &["model_name"], &["metadata", "model"]])
                    .or_else(|| cache.last_model.clone())
                    .unwrap_or_default();
                let model_norm = normalize_codex_model(&model);

                let mut delta_input = 0u64;
                let mut delta_cached = 0u64;
                let mut delta_output = 0u64;

                if let Some(total) = info.get("total_token_usage") {
                    let total_input = pick_first_u64(total, &[&["input_tokens"], &["prompt_tokens"], &["input"]]).unwrap_or(0);
                    let total_cached = pick_first_u64(total, &[&["cached_input_tokens"], &["cache_read_input_tokens"]]).unwrap_or(0).min(total_input);
                    let total_output = pick_first_u64(total, &[&["output_tokens"], &["completion_tokens"], &["output"]]).unwrap_or(0);

                    delta_input = total_input.saturating_sub(cache.last_totals.input);
                    delta_cached = total_cached.saturating_sub(cache.last_totals.cached_input);
                    delta_output = total_output.saturating_sub(cache.last_totals.output);

                    cache.last_totals.input = total_input;
                    cache.last_totals.cached_input = total_cached;
                    cache.last_totals.output = total_output;
                } else if let Some(last) = info.get("last_token_usage") {
                    delta_input = pick_first_u64(last, &[&["input_tokens"], &["prompt_tokens"]]).unwrap_or(0);
                    delta_cached = pick_first_u64(last, &[&["cached_input_tokens"], &["cache_read_input_tokens"]]).unwrap_or(0).min(delta_input);
                    delta_output = pick_first_u64(last, &[&["output_tokens"], &["completion_tokens"]]).unwrap_or(0);
                }

                if delta_input == 0 && delta_output == 0 { continue; }

                let delta_total = delta_input.saturating_add(delta_output);
                let delta_cost = codex_cost_usd(&model_norm, delta_input, delta_cached, delta_output);
                cache.contribution.input = cache.contribution.input.saturating_add(delta_input);
                cache.contribution.cached_input = cache.contribution.cached_input.saturating_add(delta_cached);
                cache.contribution.output = cache.contribution.output.saturating_add(delta_output);
                cache.contribution.total = cache.contribution.total.saturating_add(delta_total);
                cache.contribution.cost += delta_cost;
                cache.contribution.model_hint = Some(model_norm.clone());
                if cache.contribution.session_id.is_none() {
                    cache.contribution.session_id = cache.session_id.clone();
                }

                let day = day_from_json_or_now(&json);
                let daily = cache.contribution.daily.entry(day.clone()).or_insert_with(|| CodexDailyUsagePoint {
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
                model_daily.cached_input_tokens = model_daily.cached_input_tokens.saturating_add(delta_cached);
                model_daily.output_tokens = model_daily.output_tokens.saturating_add(delta_output);
                model_daily.total_tokens = model_daily.total_tokens.saturating_add(delta_total);
                model_daily.cost_usd += delta_cost;
            }
            _ => {}
        }
    }
}

fn parse_claude_file_incremental(path: &Path, cache: &mut ClaudeFileCache) {
    let Ok(mut file) = File::open(path) else { return; };
    if file.seek(SeekFrom::Start(cache.parsed_bytes)).is_err() {
        return;
    }
    let mut buf = Vec::<u8>::new();
    if file.read_to_end(&mut buf).is_err() {
        return;
    }
    let Ok(new_offset) = file.stream_position() else { return; };
    cache.parsed_bytes = new_offset;

    let reader = BufReader::new(buf.as_slice());
    for line in reader.lines().map_while(Result::ok) {
        let Ok(json) = serde_json::from_str::<Value>(&line) else { continue; };
        if json.get("type").and_then(|v| v.as_str()) != Some("assistant") { continue; }
        let Some(usage) = json.get("message").and_then(|m| m.get("usage")) else { continue; };

        let message_id = pick_first_str(&json, &[&["message", "id"], &["message_id"]]).unwrap_or_default();
        let request_id = pick_first_str(&json, &[&["requestId"], &["request_id"], &["request", "id"]]).unwrap_or_default();
        if !message_id.is_empty() && !request_id.is_empty() {
            let dedupe_key = format!("{message_id}:{request_id}");
            if !cache.seen_stream_ids.insert(dedupe_key) {
                cache.contribution.deduped_chunks = cache.contribution.deduped_chunks.saturating_add(1);
                continue;
            }
        }

        let input = pick_first_u64(usage, &[&["input_tokens"]]).unwrap_or(0);
        let cache_read = pick_first_u64(usage, &[&["cache_read_input_tokens"]]).unwrap_or(0);
        let cache_create = pick_first_u64(usage, &[&["cache_creation_input_tokens"]]).unwrap_or(0);
        let output = pick_first_u64(usage, &[&["output_tokens"]]).unwrap_or(0);
        if input == 0 && cache_read == 0 && cache_create == 0 && output == 0 { continue; }

        let model = pick_first_str(&json, &[&["message", "model"], &["model"], &["message", "model_name"]]).unwrap_or_default();
        let model_norm = normalize_claude_model(&model);
        let cost = claude_cost_usd(&model_norm, input, cache_read, cache_create, output);
        let total = input.saturating_add(cache_create).saturating_add(output);

        cache.contribution.input = cache.contribution.input.saturating_add(input);
        cache.contribution.cache_read_input = cache.contribution.cache_read_input.saturating_add(cache_read);
        cache.contribution.cache_creation_input = cache.contribution.cache_creation_input.saturating_add(cache_create);
        cache.contribution.output = cache.contribution.output.saturating_add(output);
        cache.contribution.total = cache.contribution.total.saturating_add(total);
        cache.contribution.cost += cost;

        let day = day_from_json_or_now(&json);
        let daily = cache.contribution.daily.entry(day.clone()).or_insert_with(|| ClaudeDailyUsagePoint {
            day: day.clone(),
            ..ClaudeDailyUsagePoint::default()
        });
        daily.input_tokens = daily.input_tokens.saturating_add(input);
        daily.cache_read_input_tokens = daily.cache_read_input_tokens.saturating_add(cache_read);
        daily.cache_creation_input_tokens = daily.cache_creation_input_tokens.saturating_add(cache_create);
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
        model_daily.cache_read_input_tokens = model_daily.cache_read_input_tokens.saturating_add(cache_read);
        model_daily.cache_creation_input_tokens = model_daily.cache_creation_input_tokens.saturating_add(cache_create);
        model_daily.output_tokens = model_daily.output_tokens.saturating_add(output);
        model_daily.total_tokens = model_daily.total_tokens.saturating_add(total);
        model_daily.cost_usd += cost;
    }
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
    let Ok(entries) = std::fs::read_dir(root) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue; };
        if ft.is_dir() {
            collect_jsonl_recursive(&path, out);
            continue;
        }
        let is_jsonl = path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("jsonl")).unwrap_or(false);
        if is_jsonl {
            out.push(path);
        }
    }
}

fn codex_cost_usd(model: &str, input: u64, cached_input: u64, output: u64) -> f64 {
    let Some((in_per_m, cached_per_m, out_per_m)) = codex_price_table(model) else { return 0.0; };
    let non_cached = input.saturating_sub(cached_input);
    (non_cached as f64 / 1_000_000.0) * in_per_m
        + (cached_input as f64 / 1_000_000.0) * cached_per_m
        + (output as f64 / 1_000_000.0) * out_per_m
}

fn claude_cost_usd(model: &str, input: u64, cache_read: u64, cache_create: u64, output: u64) -> f64 {
    let Some((in_per_m, read_per_m, create_per_m, out_per_m)) = claude_price_table(model) else { return 0.0; };
    (input as f64 / 1_000_000.0) * in_per_m
        + (cache_read as f64 / 1_000_000.0) * read_per_m
        + (cache_create as f64 / 1_000_000.0) * create_per_m
        + (output as f64 / 1_000_000.0) * out_per_m
}

fn codex_price_table(model: &str) -> Option<(f64, f64, f64)> {
    let m = model.to_ascii_lowercase();
    if m.contains("gpt-5") && m.contains("nano") { return Some((0.05, 0.005, 0.40)); }
    if m.contains("gpt-5") && m.contains("mini") { return Some((0.25, 0.025, 2.00)); }
    if m.contains("gpt-5") { return Some((1.25, 0.125, 10.00)); }
    if m.contains("gpt-4.1-mini") { return Some((0.40, 0.04, 1.60)); }
    if m.contains("gpt-4.1") { return Some((2.00, 0.20, 8.00)); }
    None
}

fn claude_price_table(model: &str) -> Option<(f64, f64, f64, f64)> {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") { return Some((15.0, 1.5, 18.75, 75.0)); }
    if m.contains("sonnet") { return Some((3.0, 0.3, 3.75, 15.0)); }
    if m.contains("haiku") { return Some((0.25, 0.03, 0.30, 1.25)); }
    None
}

fn normalize_codex_model(model: &str) -> String {
    let raw = model.trim().to_ascii_lowercase();
    if raw.contains("gpt-5-codex") || raw.contains("openai/gpt-5") || raw == "gpt-5" { return "gpt-5".to_string(); }
    if raw.contains("gpt-5-mini") { return "gpt-5-mini".to_string(); }
    if raw.contains("gpt-5-nano") { return "gpt-5-nano".to_string(); }
    if raw.contains("gpt-4.1-mini") { return "gpt-4.1-mini".to_string(); }
    if raw.contains("gpt-4.1") { return "gpt-4.1".to_string(); }
    raw
}

fn normalize_claude_model(model: &str) -> String {
    let raw = model.trim().to_ascii_lowercase();
    if raw.contains("opus") { return "claude-opus".to_string(); }
    if raw.contains("sonnet") { return "claude-sonnet".to_string(); }
    if raw.contains("haiku") { return "claude-haiku".to_string(); }
    raw
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
    if let Some(ts) = pick_first_u64(json, &[&["timestamp"], &["ts"], &["created_at"], &["message", "created_at"]]) {
        return if ts > 10_000_000_000 {
            day_from_ms(ts)
        } else {
            day_from_ms(ts.saturating_mul(1000))
        };
    }

    // Then try ISO 8601 / RFC 3339 string timestamps (e.g. "2026-01-16T18:10:57.334Z")
    for path in &[&["timestamp"][..], &["ts"], &["created_at"], &["message", "created_at"]] {
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
            if i == 4 || i == 7 { b == b'-' } else { b.is_ascii_digit() }
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
        if days < dy { break; }
        days -= dy;
        year += 1;
    }
    let months = [31u64, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for dm in months {
        if days < dm { break; }
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
    meta.modified().ok().and_then(|m| m.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn epoch_ms_now() -> u64 {
    std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
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
        let mut f = OpenOptions::new().create(true).append(true).open(&file).unwrap();
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
    fn claude_chunk_dedupe_by_message_and_request() {
        let dir = temp_dir("claude-dedupe");
        let file = dir.join("session.jsonl");
        let mut f = OpenOptions::new().create(true).append(true).open(&file).unwrap();
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
    fn unknown_model_keeps_tokens_without_cost() {
        assert_eq!(codex_cost_usd("unknown-model", 1000, 0, 500), 0.0);
        assert_eq!(claude_cost_usd("unknown-model", 1000, 0, 0, 500), 0.0);
    }
}
