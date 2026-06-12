use crate::providers::ProviderDescriptor;
use crate::usage::models::ProviderId;

pub fn descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        id: ProviderId::Antigravity,
        display_name: "Antigravity",
        // TODO: confirm official brand color. Placeholder: Indigo/Blue
        brand_color: "#4f6bed",
    }
}
