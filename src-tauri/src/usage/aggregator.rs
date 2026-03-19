use crate::providers::registry::ProviderRegistry;
use crate::providers::FetchContext;
use crate::usage::models::{ProviderId, UsageSnapshot};
use crate::usage::store::UsageStore;

pub async fn refresh_provider(
    registry: &ProviderRegistry,
    store: &UsageStore,
    provider: ProviderId,
    ctx: &FetchContext,
) -> Result<UsageSnapshot, String> {
    let provider_impl = registry
        .get(provider)
        .ok_or_else(|| format!("Provider {provider:?} not registered"))?;

    let snapshot = provider_impl.fetch_usage(ctx).await?;
    store.save_snapshot(&snapshot)?;

    let history = provider_impl.fetch_cost_history(30).await?;
    if !history.is_empty() {
        store.save_cost_entries(&history)?;
    }

    Ok(snapshot)
}

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
