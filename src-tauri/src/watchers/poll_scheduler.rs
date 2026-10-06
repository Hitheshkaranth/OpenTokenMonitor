use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::async_runtime::JoinHandle;

use crate::usage::models::{ProviderId, RefreshCadence};

#[derive(Clone, Default)]
pub struct PollScheduler {
    inner: Arc<Mutex<Option<JoinHandle<()>>>>,
    per_provider: Arc<Mutex<HashMap<ProviderId, JoinHandle<()>>>>,
}

impl PollScheduler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
            per_provider: Arc::new(Mutex::new(HashMap::new())),
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

    /// Spawn one poll timer per provider.
    ///
    /// Unlike [`Self::restart`], which drives a single global timer, this keeps
    /// an independent timer for each provider so different providers can run at
    /// different cadences. The `per_provider` closure is invoked on every tick
    /// of the provider whose id it is, so it is responsible for resolving that
    /// provider's effective cadence (per-provider override, falling back to the
    /// global default) and deciding whether a refresh is due. A 30s base is
    /// used as the shared wake-up resolution; the closure's own throttle keeps
    /// slower providers from over-firing.
    pub fn restart_per_provider<F>(&self, per_provider: F)
    where
        F: Fn(ProviderId) + Send + Sync + 'static,
    {
        if let Ok(mut guard) = self.per_provider.lock() {
            for handle in guard.drain() {
                handle.1.abort();
            }

            let callback = Arc::new(per_provider);
            for provider in ProviderId::all() {
                let cb = Arc::clone(&callback);
                let handle = tauri::async_runtime::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                    loop {
                        interval.tick().await;
                        cb(provider);
                    }
                });
                guard.insert(provider, handle);
            }
        }
    }
}
