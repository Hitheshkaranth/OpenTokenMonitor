use crate::providers::ProviderDescriptor;
use crate::usage::models::ProviderId;

pub fn descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        id: ProviderId::Claude,
        display_name: "Claude",
        brand_color: "#d97757",
    }
}
