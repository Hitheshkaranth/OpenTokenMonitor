use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;

use crate::usage::models::ProviderId;

pub fn start<F>(callback: F) -> Vec<std::thread::JoinHandle<()>>
where
    F: Fn(ProviderId) + Send + Sync + 'static,
{
    let cb = Arc::new(callback);
    let mut handles = Vec::new();

    let watches: Vec<(ProviderId, PathBuf)> = {
        let mut out = Vec::new();
        if let Ok(custom) = std::env::var("CODEX_HOME") {
            let custom = custom.trim();
            if !custom.is_empty() {
                let root = PathBuf::from(custom);
                out.push((ProviderId::Codex, root.join("sessions")));
                out.push((ProviderId::Codex, root.join("archived_sessions")));
            }
        }
        if let Some(home) = dirs::home_dir() {
            // Watch only the session/log roots used by the local scanners.
            // Watching whole provider home dirs lets CLI probes trigger their
            // own refreshes by touching auth and temp files.
            out.push((
                ProviderId::Claude,
                home.join(".config").join("claude").join("projects"),
            ));
            out.push((ProviderId::Claude, home.join(".claude").join("projects")));
            out.push((ProviderId::Codex, home.join(".codex").join("sessions")));
            out.push((
                ProviderId::Codex,
                home.join(".codex").join("archived_sessions"),
            ));
            out.push((ProviderId::Gemini, home.join(".gemini").join("tmp")));
        }
        out
    };

    for (provider, dir) in watches {
        if !dir.exists() {
            continue;
        }
        let cb_clone = Arc::clone(&cb);
        handles.push(std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
                Ok(w) => w,
                Err(_) => return,
            };
            if watcher.watch(&dir, RecursiveMode::Recursive).is_err() {
                return;
            }

            let _watcher = watcher;
            let mut last_emit = std::time::Instant::now() - std::time::Duration::from_secs(2);
            for _evt in rx {
                if last_emit.elapsed() < std::time::Duration::from_secs(2) {
                    continue;
                }
                last_emit = std::time::Instant::now();
                cb_clone(provider);
            }
        }));
    }

    handles
}
