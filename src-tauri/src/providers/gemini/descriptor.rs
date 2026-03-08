use crate::providers::ProviderDescriptor;
use crate::usage::models::ProviderId;

pub fn descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        id: ProviderId::Gemini,
        display_name: "Gemini",
        brand_color: "#4285f4",
    }
}
