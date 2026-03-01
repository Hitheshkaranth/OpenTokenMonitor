use tauri::{AppHandle, Manager, Emitter};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItemBuilder};
use notify::{Watcher, RecursiveMode, Config};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufRead, BufReader};

// ── Shared data types ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CliActivity {
    provider:  String,
    command:   String,
    timestamp: u64,
    project:   Option<String>,
}

/// Full summary stats computed from an entire CLI history file.
/// Returned by get_claude_stats / get_codex_stats.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CliSummaryStats {
    pub total_commands:  usize,
    pub commands_today:  usize,
    pub unique_sessions: usize,
    pub unique_projects: usize,
    pub projects:        Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeminiStats {
    pub project_count: usize,
    pub session_count: usize,
    pub projects:      Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ProviderUsageWindow {
    pub last_4h: usize,
    pub last_7d: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UsageWindows {
    pub anthropic: ProviderUsageWindow,
    pub openai:    ProviderUsageWindow,
    pub google:    ProviderUsageWindow,
}

/// Rich usage data parsed from ~/.claude/stats-cache.json.
/// Claude Code writes this file itself — same source as the /usage command.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ClaudeUsageCache {
    pub total_messages:   u64,
    pub total_sessions:   u64,
    pub messages_today:   u64,
    pub sessions_today:   u64,
    pub tokens_today:     u64,   // sum across all models for today
    pub tokens_total:     u64,   // all-time input + output across all models
}

// ── Path helpers ──────────────────────────────────────────────────────────────

/// Returns ~/.claude/history.jsonl (or None if home dir can't be determined).
fn claude_history_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("history.jsonl"))
}

/// Returns ~/.claude/stats-cache.json
fn claude_stats_cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("stats-cache.json"))
}

/// Returns ~/.codex/history.jsonl
fn codex_history_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".codex").join("history.jsonl"))
}

/// Returns ~/.gemini/projects.json
fn gemini_projects_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".gemini").join("projects.json"))
}

/// Returns ~/.gemini/ (the whole directory, watched recursively for any change)
fn gemini_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".gemini"))
}

// ── Claude Code ───────────────────────────────────────────────────────────────

#[tauri::command]
fn get_claude_history() -> Vec<CliActivity> {
    let Some(path) = claude_history_path() else { return vec![] };
    read_claude_jsonl(&path, "anthropic", 20)
}

/// Reads ~/.claude/stats-cache.json — the same file the Claude Code /usage
/// command uses. Returns real message counts, session counts, and token totals.
#[tauri::command]
fn get_claude_usage_cache() -> ClaudeUsageCache {
    let Some(path) = claude_stats_cache_path() else { return ClaudeUsageCache::default() };
    let Ok(file)   = File::open(&path)           else { return ClaudeUsageCache::default() };
    let Ok(json)   = serde_json::from_reader::<_, serde_json::Value>(file)
                                                  else { return ClaudeUsageCache::default() };

    let total_messages = json["totalMessages"].as_u64().unwrap_or(0);
    let total_sessions = json["totalSessions"].as_u64().unwrap_or(0);

    // Today's date as "YYYY-MM-DD" in UTC (good enough for daily bucketing)
    let today = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_secs();
        let days = secs / 86400;
        // Simple date math: days since epoch → YYYY-MM-DD
        let mut y = 1970u64; let mut rem = days;
        loop {
            let dy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
            if rem < dy { break; } rem -= dy; y += 1;
        }
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let months = [31u64,if leap{29}else{28},31,30,31,30,31,31,30,31,30,31];
        let mut m = 1u64;
        for dm in months { if rem < dm { break; } rem -= dm; m += 1; }
        format!("{:04}-{:02}-{:02}", y, m, rem + 1)
    };

    // Messages and sessions for today from dailyActivity
    let (messages_today, sessions_today) = json["dailyActivity"]
        .as_array()
        .and_then(|arr| arr.iter().find(|e| e["date"].as_str() == Some(&today)))
        .map(|e| (e["messageCount"].as_u64().unwrap_or(0), e["sessionCount"].as_u64().unwrap_or(0)))
        .unwrap_or((0, 0));

    // Tokens for today from dailyModelTokens (sum all models)
    let tokens_today = json["dailyModelTokens"]
        .as_array()
        .and_then(|arr| arr.iter().find(|e| e["date"].as_str() == Some(&today)))
        .and_then(|e| e["tokensByModel"].as_object())
        .map(|m| m.values().filter_map(|v| v.as_u64()).sum::<u64>())
        .unwrap_or(0);

    // All-time tokens: sum inputTokens + outputTokens across all models
    let tokens_total = json["modelUsage"]
        .as_object()
        .map(|models| {
            models.values().map(|m| {
                m["inputTokens"].as_u64().unwrap_or(0)
                + m["outputTokens"].as_u64().unwrap_or(0)
            }).sum::<u64>()
        })
        .unwrap_or(0);

    ClaudeUsageCache { total_messages, total_sessions, messages_today, sessions_today, tokens_today, tokens_total }
}

/// Returns aggregate stats for the full Claude history (not capped to last N).
#[tauri::command]
fn get_claude_stats() -> CliSummaryStats {
    let Some(path) = claude_history_path() else { return CliSummaryStats::default_empty() };
    let Ok(file) = File::open(&path) else { return CliSummaryStats::default_empty() };

    let today_ms = today_start_ms();
    let mut total = 0usize;
    let mut today = 0usize;
    let mut sessions  = std::collections::HashSet::<String>::new();
    let mut projects  = std::collections::HashSet::<String>::new();

    for line in BufReader::new(file).lines().flatten() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            total += 1;
            let ts = json["timestamp"].as_u64().unwrap_or(0);
            if ts >= today_ms { today += 1; }
            if let Some(s) = json["sessionId"].as_str() { sessions.insert(s.to_string()); }
            if let Some(p) = json["project"].as_str()   { projects.insert(p.to_string()); }
        }
    }

    CliSummaryStats {
        total_commands:  total,
        commands_today:  today,
        unique_sessions: sessions.len(),
        unique_projects: projects.len(),
        projects:        projects.into_iter().collect(),
    }
}

/// Returns aggregate stats for the full Codex history.
#[tauri::command]
fn get_codex_stats() -> CliSummaryStats {
    let Some(path) = codex_history_path() else { return CliSummaryStats::default_empty() };
    let Ok(file) = File::open(&path) else { return CliSummaryStats::default_empty() };

    let today_ms = today_start_ms();
    let mut total = 0usize;
    let mut today = 0usize;
    let mut sessions = std::collections::HashSet::<String>::new();

    for line in BufReader::new(file).lines().flatten() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            total += 1;
            let ts = json["ts"].as_u64().unwrap_or(0) * 1000; // seconds → ms
            if ts >= today_ms { today += 1; }
            if let Some(s) = json["session_id"].as_str() { sessions.insert(s.to_string()); }
        }
    }

    CliSummaryStats {
        total_commands:  total,
        commands_today:  today,
        unique_sessions: sessions.len(),
        unique_projects: 0,
        projects:        vec![],
    }
}

impl CliSummaryStats {
    fn default_empty() -> Self {
        Self { total_commands: 0, commands_today: 0, unique_sessions: 0, unique_projects: 0, projects: vec![] }
    }
}

/// Returns milliseconds since epoch for today at 00:00:00 local time.
fn today_start_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    // Rough UTC offset doesn't matter much for "today" — use midnight UTC as proxy
    let day_secs = now_secs - (now_secs % 86400);
    day_secs * 1000
}

fn read_claude_jsonl(path: &Path, provider: &str, limit: usize) -> Vec<CliActivity> {
    let Ok(file) = File::open(path) else { return vec![] };
    let mut activities: Vec<CliActivity> = BufReader::new(file)
        .lines()
        .flatten()
        .filter_map(|line| {
            let json = serde_json::from_str::<serde_json::Value>(&line).ok()?;
            Some(CliActivity {
                provider:  provider.to_string(),
                command:   json["display"].as_str().unwrap_or("").to_string(),
                timestamp: json["timestamp"].as_u64().unwrap_or(0),
                project:   json["project"].as_str().map(|s| s.to_string()),
            })
        })
        .collect();
    activities.reverse();
    activities.truncate(limit);
    activities
}

fn parse_claude_line(line: &str, provider: &str) -> Option<CliActivity> {
    let json = serde_json::from_str::<serde_json::Value>(line).ok()?;
    Some(CliActivity {
        provider:  provider.to_string(),
        command:   json["display"].as_str().unwrap_or("").to_string(),
        timestamp: json["timestamp"].as_u64().unwrap_or(0),
        project:   json["project"].as_str().map(|s| s.to_string()),
    })
}

// ── OpenAI Codex ──────────────────────────────────────────────────────────────

#[tauri::command]
fn get_codex_history() -> Vec<CliActivity> {
    let Some(path) = codex_history_path() else { return vec![] };
    let Ok(file) = File::open(&path) else { return vec![] };
    let mut activities: Vec<CliActivity> = BufReader::new(file)
        .lines()
        .flatten()
        .filter_map(|line| parse_codex_line(&line, "openai"))
        .collect();
    activities.reverse();
    activities.truncate(20);
    activities
}

fn parse_codex_line(line: &str, provider: &str) -> Option<CliActivity> {
    let json = serde_json::from_str::<serde_json::Value>(line).ok()?;
    Some(CliActivity {
        provider:  provider.to_string(),
        command:   json["text"].as_str().unwrap_or("").to_string(),
        // Codex timestamps are in seconds; convert to milliseconds
        timestamp: json["ts"].as_u64().unwrap_or(0) * 1000,
        project:   None,
    })
}

// ── Gemini CLI ────────────────────────────────────────────────────────────────

#[tauri::command]
fn get_gemini_stats() -> GeminiStats {
    let mut projects: Vec<String> = Vec::new();

    // Read project list from projects.json
    if let Some(p) = gemini_projects_path() {
        if let Ok(file) = File::open(p) {
            if let Ok(json) = serde_json::from_reader::<_, serde_json::Value>(file) {
                // Try both {"projects": {...}} and {"projects": [...]} shapes
                if let Some(map) = json["projects"].as_object() {
                    for (_, v) in map {
                        if let Some(n) = v.as_str() { projects.push(n.to_string()); }
                    }
                } else if let Some(arr) = json["projects"].as_array() {
                    for v in arr {
                        if let Some(n) = v.as_str() { projects.push(n.to_string()); }
                    }
                }
            }
        }
    }

    // Count sessions: any sub-directory of ~/.gemini/ that looks like a session
    let session_count = gemini_dir()
        .and_then(|d| std::fs::read_dir(d).ok())
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .count()
        })
        .unwrap_or(0);

    GeminiStats { project_count: projects.len(), session_count, projects }
}

#[tauri::command]
fn get_usage_windows() -> UsageWindows {
    // Aggregate recent usage windows from local provider data sources.
    let anthropic = claude_history_path()
        .map(|p| scan_claude_windows(&p))
        .unwrap_or_default();
    let openai = codex_history_path()
        .map(|p| scan_codex_windows(&p))
        .unwrap_or_default();
    let google = gemini_dir()
        .map(|p| scan_gemini_windows(&p))
        .unwrap_or_default();
    UsageWindows { anthropic, openai, google }
}

fn scan_claude_windows(path: &Path) -> ProviderUsageWindow {
    // Claude history stores timestamps in milliseconds.
    let Ok(file) = File::open(path) else { return ProviderUsageWindow::default() };
    let now = epoch_ms_now();
    let cutoff_4h = now.saturating_sub(4 * 60 * 60 * 1000);
    let cutoff_7d = now.saturating_sub(7 * 24 * 60 * 60 * 1000);
    let mut out = ProviderUsageWindow::default();

    for line in BufReader::new(file).lines().flatten() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            let ts = json["timestamp"].as_u64().unwrap_or(0);
            if ts >= cutoff_7d {
                out.last_7d += 1;
                if ts >= cutoff_4h {
                    out.last_4h += 1;
                }
            }
        }
    }
    out
}

fn scan_codex_windows(path: &Path) -> ProviderUsageWindow {
    // Codex history stores timestamps in seconds; convert to milliseconds.
    let Ok(file) = File::open(path) else { return ProviderUsageWindow::default() };
    let now = epoch_ms_now();
    let cutoff_4h = now.saturating_sub(4 * 60 * 60 * 1000);
    let cutoff_7d = now.saturating_sub(7 * 24 * 60 * 60 * 1000);
    let mut out = ProviderUsageWindow::default();

    for line in BufReader::new(file).lines().flatten() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            let ts = json["ts"].as_u64().unwrap_or(0).saturating_mul(1000);
            if ts >= cutoff_7d {
                out.last_7d += 1;
                if ts >= cutoff_4h {
                    out.last_4h += 1;
                }
            }
        }
    }
    out
}

fn scan_gemini_windows(path: &Path) -> ProviderUsageWindow {
    // Gemini does not expose a simple history feed here; use session folder
    // modification times as a practical proxy for recent activity.
    let Ok(rd) = std::fs::read_dir(path) else { return ProviderUsageWindow::default() };
    let now = epoch_ms_now();
    let cutoff_4h = now.saturating_sub(4 * 60 * 60 * 1000);
    let cutoff_7d = now.saturating_sub(7 * 24 * 60 * 60 * 1000);
    let mut out = ProviderUsageWindow::default();

    for entry in rd.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        if !file_type.is_dir() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) else { continue };
        let ts = dur.as_millis() as u64;
        if ts >= cutoff_7d {
            out.last_7d += 1;
            if ts >= cutoff_4h {
                out.last_4h += 1;
            }
        }
    }
    out
}

fn epoch_ms_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[tauri::command]
fn get_codex_access_token() -> String {
    let Some(home) = dirs::home_dir() else { return String::new() };
    let path = home.join(".codex").join("auth.json");
    let Ok(file) = File::open(path) else { return String::new() };
    let Ok(json) = serde_json::from_reader::<_, serde_json::Value>(file) else { return String::new() };
    json["tokens"]["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

// ── File watchers ─────────────────────────────────────────────────────────────

/// Watches a JSONL history file and emits a "cli-activity" event for every new
/// line appended to it. Safely handles the case where the file / parent dir
/// does not yet exist.
fn watch_cli_file(app: AppHandle, path: PathBuf, provider: &'static str, is_codex: bool) {
    let Some(parent) = path.parent().map(|p| p.to_path_buf()) else { return };
    if !parent.exists() { return; }   // directory not present — skip silently

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = match notify::RecommendedWatcher::new(tx, Config::default()) {
        Ok(w)  => w,
        Err(_) => return,
    };
    let _ = watcher.watch(&parent, RecursiveMode::NonRecursive);

    std::thread::spawn(move || {
        let _watcher = watcher;   // keep alive for the thread's lifetime
        for res in rx {
            let Ok(event) = res else { continue };
            // Only react when our specific file changed
            let target = path.file_name().unwrap_or_default();
            if !event.paths.iter().any(|p| p.file_name() == Some(target)) {
                continue;
            }
            // Read the last line and emit it as a live activity event
            if let Ok(file) = File::open(&path) {
                if let Some(Ok(last_line)) = BufReader::new(file).lines().last() {
                    let activity = if is_codex {
                        parse_codex_line(&last_line, provider)
                    } else {
                        parse_claude_line(&last_line, provider)
                    };
                    if let Some(act) = activity {
                        let _ = app.emit("cli-activity", act);
                    }
                }
            }
        }
    });
}

/// Watches the entire ~/.gemini/ directory. Any change (new session, new
/// history file, project list update) triggers a fresh stats recompute.
fn watch_gemini(app: AppHandle) {
    let Some(dir) = gemini_dir() else { return };
    if !dir.exists() { return; }

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = match notify::RecommendedWatcher::new(tx, Config::default()) {
        Ok(w)  => w,
        Err(_) => return,
    };
    // Recursive so we catch new files in sub-directories (session folders, etc.)
    let _ = watcher.watch(&dir, RecursiveMode::Recursive);

    std::thread::spawn(move || {
        let _watcher = watcher;
        for res in rx {
            let Ok(_event) = res else { continue };
            let stats = get_gemini_stats();
            let _ = app.emit("gemini-stats", stats);
        }
    });
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Start all file watchers (each runs in its own background thread)
            if let Some(claude_path) = claude_history_path() {
                watch_cli_file(app.handle().clone(), claude_path, "anthropic", false);
            }
            if let Some(codex_path) = codex_history_path() {
                watch_cli_file(app.handle().clone(), codex_path, "openai", true);
            }
            watch_gemini(app.handle().clone());

            // Build the system tray icon from the embedded PNG asset
            let icon_bytes: &[u8] = include_bytes!("../../open_token_monitor_icon.png");
            let img = image::load_from_memory_with_format(icon_bytes, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8();
            let (width, height) = image::GenericImageView::dimensions(&img);
            let tray_icon = Image::new_owned(img.into_raw(), width, height);

            // Right-click context menu: Show/Hide and Quit
            let show_hide = MenuItemBuilder::new("Show / Hide").id("show-hide").build(app)?;
            let quit      = MenuItemBuilder::new("Quit").id("quit").build(app)?;
            let tray_menu = Menu::with_items(app, &[&show_hide, &quit])?;

            let _ = tauri::tray::TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&tray_menu)
                // Right-click menu handler
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show-hide" => {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                    _ => {}
                })
                // Left-click toggles the window (existing behaviour)
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_claude_history,
            get_claude_stats,
            get_claude_usage_cache,
            get_codex_history,
            get_codex_stats,
            get_gemini_stats,
            get_usage_windows,
            get_codex_access_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
