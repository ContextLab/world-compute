//! Proof-of-personhood verification integration.
//!
//! Per FR-S070: connects the `proof_of_personhood: bool` field in
//! HumanityPoints to a real verification provider. Provider selection
//! is deferred to T086 (Phase 8).

/// Result of a proof-of-personhood verification attempt.
#[derive(Debug, Clone)]
pub enum PersonhoodResult {
    /// Verification succeeded.
    Verified,
    /// Verification failed with reason.
    Failed(String),
    /// Provider is unavailable.
    ProviderUnavailable(String),
}

/// Verify proof-of-personhood for a user.
///
/// TODO(T086): Select and integrate concrete provider (BrightID,
/// government ID, or equivalent).
pub fn verify_personhood(_user_id: &str) -> PersonhoodResult {
    // Placeholder until provider is selected in T086
    PersonhoodResult::ProviderUnavailable(
        "Proof-of-personhood provider not yet selected (see T086)".into(),
    )
}
