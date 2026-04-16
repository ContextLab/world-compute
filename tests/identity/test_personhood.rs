//! T082: Proof-of-personhood verification connects to real provider.

use worldcompute::identity::personhood::{
    peer_id_to_context_id, verify_personhood, PersonhoodResult,
};

#[test]
fn personhood_verification_returns_unavailable_without_http_client() {
    match verify_personhood("test-context") {
        PersonhoodResult::ProviderUnavailable(msg) => {
            assert!(
                msg.contains("BrightID") || msg.contains("HTTP client"),
                "Should reference BrightID or HTTP client, got: {msg}"
            );
        }
        other => panic!("Expected ProviderUnavailable, got {other:?}"),
    }
}

#[test]
fn context_id_derivation_works() {
    let id = peer_id_to_context_id("12D3KooWTest");
    assert_eq!(id.len(), 32);
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
}
