use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::usage::models::{CostEntry, ProviderId, ProviderStatus, UsageSnapshot};

pub mod claude;
pub mod codex;
pub mod gemini;
pub mod registry;

#[derive(Debug, Clone)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub brand_color: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct FetchContext {
    pub api_keys: HashMap<ProviderId, String>,
    pub allow_cookie_strategy: bool,
    pub allow_cli_strategy: bool,
}

impl FetchContext {
    // Providers pull auth and feature switches from a shared context so command
    // handlers do not need to know anything about provider-specific auth shapes.
    pub fn api_key_for(&self, provider: ProviderId) -> Option<&str> {
        self.api_keys
            .get(&provider)
            .map(String::as_str)
            .filter(|v| !v.trim().is_empty())
    }
}

#[async_trait]
// Every provider implementation exposes the same small contract: fetch current
// usage, fetch cost history, and report health. The registry and aggregator rely
// on this trait to keep provider-specific branching out of the command layer.
pub trait UsageProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn descriptor(&self) -> &ProviderDescriptor;
    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<UsageSnapshot, String>;
    async fn fetch_cost_history(&self, days: u32) -> Result<Vec<CostEntry>, String>;
    async fn check_status(&self) -> ProviderStatus;
}

pub type DynProvider = Arc<dyn UsageProvider>;
