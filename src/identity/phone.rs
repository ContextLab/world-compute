//! Phone verification for Humanity Points.
//!
//! Per FR-S073: phone verification is worth 3 HP in the Humanity Points
//! system. Verified at enrollment, re-verified at trust score recalculation.

/// Result of a phone verification attempt.
#[derive(Debug, Clone)]
pub enum PhoneResult {
    Verified { phone_hash: String },
    CodeExpired,
    InvalidCode,
    ProviderUnavailable(String),
}

/// Send a verification code to the given phone number.
///
/// TODO(T088): Implement real SMS/voice verification.
pub fn send_verification_code(_phone_number: &str) -> Result<String, String> {
    Err("Phone verification not yet implemented (see T088)".into())
}

/// Verify a code entered by the user.
///
/// TODO(T088): Implement real code verification against sent code.
pub fn verify_code(_session_id: &str, _code: &str) -> PhoneResult {
    PhoneResult::ProviderUnavailable(
        "Phone verification not yet implemented (see T088)".into(),
    )
}
