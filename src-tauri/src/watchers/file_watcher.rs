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
        if let Some(home) = dirs::home_dir() {
            out.push((ProviderId::Claude, home.join(".claude")));
            out.push((ProviderId::Codex, home.join(".codex")));
            out.push((ProviderId::Gemini, home.join(".gemini")));
            out.push((ProviderId::Gemini, home.join(".config").join("gemini")));
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
                if last_emit.elapsed() < std::time::Duration::from_millis(250) {
                    continue;
                }
                last_emit = std::time::Instant::now();
                cb_clone(provider);
            }
        }));
    }

    handles
}
