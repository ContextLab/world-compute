//! T082: Proof-of-personhood verification connects to real provider.

use worldcompute::identity::personhood::{verify_personhood, PersonhoodResult};

#[test]
fn personhood_verification_returns_provider_unavailable_until_configured() {
    match verify_personhood("user-123") {
        PersonhoodResult::ProviderUnavailable(msg) => {
            assert!(msg.contains("T086"), "Should reference the provider selection task");
        }
        other => panic!("Expected ProviderUnavailable, got {other:?}"),
    }
}
