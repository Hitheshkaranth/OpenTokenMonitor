use std::collections::HashMap;
use std::sync::OnceLock;

use crate::providers::registry::ProviderRegistry;
use crate::providers::FetchContext;
use crate::usage::models::{DataProvenance, DataSource, ProviderId, UsageSnapshot};
use crate::usage::store::UsageStore;
use chrono::{Duration, Utc};
use tracing::debug;

/// How long a freshly-fetched live snapshot is reused instead of re-fetching.
///
/// Four independent code paths kick off a refresh during startup — the Tauri
/// `setup` hook, the poll scheduler's first tick, the React bootstrap in
/// `useUsageData`, and `App.tsx` — so every provider was getting hit three or
/// four times within a second of launch. Anthropic answers that burst with a
/// 429, which then puts the Claude provider straight into backoff on a cold
/// start, exactly when it has no cached snapshot to fall back on.
const COALESCE_WINDOW_SECS: i64 = 10;

/// Per-provider single-flight locks.
///
/// The freshness check alone is not enough: genuinely simultaneous callers all
/// miss the store before any of them writes to it. Serializing per provider
/// means the first caller performs the fetch and the rest fall through to the
/// freshness check and reuse its result.
fn refresh_locks() -> &'static HashMap<ProviderId, tokio::sync::Mutex<()>> {
    static LOCKS: OnceLock<HashMap<ProviderId, tokio::sync::Mutex<()>>> = OnceLock::new();
    LOCKS.get_or_init(|| {
        ProviderId::all()
            .into_iter()
            .map(|id| (id, tokio::sync::Mutex::new(())))
            .collect()
    })
}

/// A live snapshot this recent is worth reusing rather than re-fetching.
fn is_fresh_live(snapshot: &UsageSnapshot) -> bool {
    !matches!(snapshot.source, DataSource::LocalLog)
        && (Utc::now() - snapshot.fetched_at) < Duration::seconds(COALESCE_WINDOW_SECS)
}

// Refresh one provider end-to-end: fetch current usage, persist it, then persist
// optional cost history if the provider exposes any.
pub async fn refresh_provider(
    registry: &ProviderRegistry,
    store: &UsageStore,
    provider: ProviderId,
    ctx: &FetchContext,
) -> Result<UsageSnapshot, String> {
    let provider_impl = registry
        .get(provider)
        .ok_or_else(|| format!("Provider {provider:?} not registered"))?;

    // Hold the per-provider lock across the fetch so concurrent callers
    // coalesce onto one network round trip instead of racing each other.
    let _guard = match refresh_locks().get(&provider) {
        Some(lock) => Some(lock.lock().await),
        None => None,
    };

    if let Some(previous) = store.get_snapshot(provider)? {
        if is_fresh_live(&previous) {
            debug!(
                "[{}] reusing snapshot fetched {}s ago instead of re-fetching",
                provider.as_str(),
                (Utc::now() - previous.fetched_at).num_seconds()
            );
            return Ok(previous);
        }
    }

    let snapshot = provider_impl.fetch_usage(ctx).await?;
    if matches!(snapshot.source, DataSource::LocalLog)
        && matches!(snapshot.provenance, DataProvenance::DerivedLocal)
    {
        if let Some(mut previous) = store.get_snapshot(provider)? {
            let has_recent_live_snapshot = !matches!(previous.source, DataSource::LocalLog)
                && (Utc::now() - previous.fetched_at) < Duration::hours(24);
            if has_recent_live_snapshot {
                previous.stale = true;
                return Ok(previous);
            }
        }
    }
    store.save_snapshot(&snapshot)?;

    let history = provider_impl.fetch_cost_history(30).await?;
    if !history.is_empty() {
        store.save_cost_entries(&history)?;
    }

    Ok(snapshot)
}

// Refresh every provider but keep partial success useful. The UI can still render
// when one provider fails, so only fail the whole call when every provider fails.
pub async fn refresh_all(
    registry: &ProviderRegistry,
    store: &UsageStore,
    ctx: &FetchContext,
) -> Result<Vec<UsageSnapshot>, String> {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    for provider in ProviderId::all() {
        match refresh_provider(registry, store, provider, ctx).await {
            Ok(snapshot) => out.push(snapshot),
            Err(err) => errors.push(format!("{}: {}", provider.as_str(), err)),
        }
    }

    if out.is_empty() {
        if errors.is_empty() {
            return Err("No providers produced usage data".to_string());
        }
        return Err(format!(
            "All providers failed to refresh. {}",
            errors.join(" | ")
        ));
    }

    Ok(out)
}
