use crate::providers::ProviderDescriptor;
use crate::usage::models::ProviderId;

pub fn descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        id: ProviderId::Codex,
        display_name: "Codex",
        brand_color: "#10a37f",
    }
}
