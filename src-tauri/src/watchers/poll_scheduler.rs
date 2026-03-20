use std::sync::{Arc, Mutex};

use tauri::async_runtime::JoinHandle;

use crate::usage::models::RefreshCadence;

#[derive(Clone, Default)]
pub struct PollScheduler {
    inner: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl PollScheduler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    pub fn restart<F>(&self, cadence: RefreshCadence, tick: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if let Ok(mut guard) = self.inner.lock() {
            // Replacing the existing task keeps cadence changes simple: one timer,
            // one callback, and no chance of overlapping poll loops.
            if let Some(handle) = guard.take() {
                handle.abort();
            }

            let Some(seconds) = cadence.seconds() else {
                return;
            };

            let callback = Arc::new(tick);
            let cb = Arc::clone(&callback);
            *guard = Some(tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(seconds));
                loop {
                    interval.tick().await;
                    cb();
                }
            }));
        }
    }
}
