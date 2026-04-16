//! T083: OAuth2 email verification flow.

use worldcompute::identity::oauth2::{verify_oauth2, OAuth2Provider, OAuth2Result};

#[test]
fn oauth2_returns_unavailable_until_implemented() {
    match verify_oauth2(OAuth2Provider::Email, "https://example.com/callback") {
        OAuth2Result::ProviderUnavailable(msg) => {
            assert!(msg.contains("T088"), "Should reference the implementation task");
        }
        other => panic!("Expected ProviderUnavailable, got {other:?}"),
    }
}

#[test]
fn all_providers_return_unavailable() {
    for provider in [
        OAuth2Provider::Email,
        OAuth2Provider::GitHub,
        OAuth2Provider::Google,
        OAuth2Provider::Twitter,
    ] {
        assert!(matches!(
            verify_oauth2(provider, "https://example.com/callback"),
            OAuth2Result::ProviderUnavailable(_)
        ));
    }
}
