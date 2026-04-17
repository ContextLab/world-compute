//! T082: Proof-of-personhood verification connects to real provider.

use worldcompute::identity::personhood::{
    peer_id_to_context_id, verify_personhood, PersonhoodResult,
};

#[test]
fn personhood_verification_handles_unreachable_provider() {
    // With the real HTTP client wired, verifying a fake context ID will either:
    // - Return ProviderUnavailable if BrightID node is unreachable (network error)
    // - Return Pending if the context ID is not found (404)
    // - Return Failed in other error conditions
    // All are acceptable — the key is it doesn't panic or hang.
    match verify_personhood("test-context-nonexistent") {
        PersonhoodResult::ProviderUnavailable(msg) => {
            assert!(
                msg.contains("BrightID") || msg.contains("request failed") || msg.contains("error"),
                "Error should be descriptive, got: {msg}"
            );
        }
        PersonhoodResult::Pending { .. } => {
            // 404 response treated as "not yet verified"
        }
        PersonhoodResult::Failed(_) => {
            // Other error condition
        }
        PersonhoodResult::Verified => {
            panic!("Fake context ID should not verify as real");
        }
    }
}

#[test]
fn context_id_derivation_works() {
    let id = peer_id_to_context_id("12D3KooWTest");
    assert_eq!(id.len(), 32);
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
}
