use std::collections::HashMap;
use std::sync::Arc;

use crate::providers::antigravity::AntigravityProvider;
use crate::providers::claude::ClaudeProvider;
use crate::providers::codex::CodexProvider;
use crate::providers::DynProvider;
use crate::providers::ProviderDescriptor;
use crate::usage::models::ProviderId;

#[derive(Clone, Default)]
pub struct ProviderRegistry {
    providers: HashMap<ProviderId, DynProvider>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut providers: HashMap<ProviderId, DynProvider> = HashMap::new();

        // This is the single registration point for supported providers. Adding a
        // provider means implementing the trait and inserting it here.
        let claude: DynProvider = Arc::new(ClaudeProvider::new());
        let codex: DynProvider = Arc::new(CodexProvider::new());
        let antigravity: DynProvider = Arc::new(AntigravityProvider::new());

        providers.insert(claude.id(), claude);
        providers.insert(codex.id(), codex);
        providers.insert(antigravity.id(), antigravity);

        Self { providers }
    }

    pub fn get(&self, id: ProviderId) -> Option<DynProvider> {
        self.providers.get(&id).cloned()
    }

    pub fn descriptors(&self) -> Vec<ProviderDescriptor> {
        self.providers
            .values()
            .map(|p| p.descriptor().clone())
            .collect()
    }
}
