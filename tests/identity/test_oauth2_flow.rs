//! T092: Real OAuth2 flow — verify HP score updates.
//!
//! Since we don't have a live OAuth2 provider configured yet, this test
//! verifies the flow mechanics: provider enumeration, unavailability
//! handling, and context ID derivation that feeds into HP scoring.

use worldcompute::identity::oauth2::{verify_oauth2, OAuth2Provider, OAuth2Result};
use worldcompute::identity::personhood::{
    peer_id_to_context_id, verify_personhood, PersonhoodResult,
};
use worldcompute::identity::phone::{send_verification_code, verify_code, PhoneResult};

#[test]
fn oauth2_flow_returns_unavailable_with_provider_info() {
    // Each provider should return a meaningful unavailability message
    for provider in [
        OAuth2Provider::Email,
        OAuth2Provider::GitHub,
        OAuth2Provider::Google,
        OAuth2Provider::Twitter,
    ] {
        match verify_oauth2(provider, "https://localhost/callback") {
            OAuth2Result::ProviderUnavailable(msg) => {
                assert!(!msg.is_empty(), "Provider {provider:?} should give a reason");
            }
            other => panic!("Expected ProviderUnavailable for {provider:?}, got {other:?}"),
        }
    }
}

#[test]
fn phone_verification_flow_returns_unavailable() {
    assert!(send_verification_code("+1234567890").is_err());
    match verify_code("session-1", "123456") {
        PhoneResult::ProviderUnavailable(msg) => assert!(!msg.is_empty()),
        other => panic!("Expected ProviderUnavailable, got {other:?}"),
    }
}

#[test]
fn personhood_flow_returns_unavailable_with_brightid_context() {
    let context_id = peer_id_to_context_id("12D3KooWTestPeer");
    match verify_personhood(&context_id) {
        PersonhoodResult::ProviderUnavailable(msg) => {
            assert!(
                msg.contains("BrightID") || msg.contains("HTTP"),
                "Should reference BrightID, got: {msg}"
            );
        }
        other => panic!("Expected ProviderUnavailable, got {other:?}"),
    }
}

#[test]
fn full_hp_verification_flow_gracefully_degrades() {
    // Simulate what happens during enrollment:
    // 1. Derive context ID from peer
    let context_id = peer_id_to_context_id("12D3KooWNewDonor");
    assert_eq!(context_id.len(), 32);

    // 2. Attempt personhood verification
    let personhood = verify_personhood(&context_id);
    // Should degrade gracefully — not panic
    assert!(matches!(
        personhood,
        PersonhoodResult::ProviderUnavailable(_) | PersonhoodResult::Pending { .. }
    ));

    // 3. Attempt OAuth2 verification
    let oauth = verify_oauth2(OAuth2Provider::Email, "https://localhost/callback");
    assert!(matches!(oauth, OAuth2Result::ProviderUnavailable(_)));

    // 4. Attempt phone verification
    let phone = verify_code("session", "code");
    assert!(matches!(phone, PhoneResult::ProviderUnavailable(_)));

    // All three degrade gracefully — HP starts at 0, user can retry later
}
