//! OAuth2 verification flows for Humanity Points.
//!
//! Per FR-S073: implements real OAuth2 verification for email and
//! social account linking. Verified at enrollment, re-verified at
//! trust score recalculation intervals.

/// OAuth2 provider types supported for HP verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuth2Provider {
    Email,
    GitHub,
    Google,
    Twitter,
}

/// Result of an OAuth2 verification flow.
#[derive(Debug, Clone)]
pub enum OAuth2Result {
    Verified { provider: OAuth2Provider, account_id: String },
    Failed(String),
    ProviderUnavailable(String),
}

/// Initiate OAuth2 verification for the given provider.
///
/// TODO(T088): Implement real OAuth2 flows with provider-specific adapters.
pub fn verify_oauth2(_provider: OAuth2Provider, _redirect_uri: &str) -> OAuth2Result {
    OAuth2Result::ProviderUnavailable(
        "OAuth2 verification flows not yet implemented (see T088)".into(),
    )
}
