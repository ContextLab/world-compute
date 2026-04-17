//! Phone verification for Humanity Points.
//!
//! Per FR-S073: phone verification is worth 3 HP in the Humanity Points
//! system. Verified at enrollment, re-verified at trust score recalculation.
//!
//! Uses the Twilio Verify API for sending and checking verification codes.
//! Configuration is loaded from environment variables:
//!   TWILIO_ACCOUNT_SID
//!   TWILIO_AUTH_TOKEN
//!   TWILIO_VERIFY_SID

/// Result of a phone verification attempt.
#[derive(Debug, Clone)]
pub enum PhoneResult {
    Verified { phone_hash: String },
    CodeExpired,
    InvalidCode,
    ProviderUnavailable(String),
}

/// Twilio Verify API configuration.
#[derive(Debug, Clone)]
pub struct SmsProviderConfig {
    /// Twilio Account SID.
    pub account_sid: String,
    /// Twilio Auth Token.
    pub auth_token: String,
    /// Twilio Verify Service SID.
    pub verify_service_sid: String,
}

impl SmsProviderConfig {
    /// Load configuration from environment variables.
    ///
    /// Reads `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, and `TWILIO_VERIFY_SID`.
    /// Returns `None` if any required variable is missing (T050: clear error,
    /// no panic).
    pub fn from_env() -> Option<Self> {
        let account_sid = std::env::var("TWILIO_ACCOUNT_SID").ok()?;
        let auth_token = std::env::var("TWILIO_AUTH_TOKEN").ok()?;
        let verify_service_sid = std::env::var("TWILIO_VERIFY_SID").ok()?;
        Some(Self {
            account_sid,
            auth_token,
            verify_service_sid,
        })
    }

    /// Twilio Verify API base URL for this service.
    fn verifications_url(&self) -> String {
        format!(
            "https://verify.twilio.com/v2/Services/{}/Verifications",
            self.verify_service_sid
        )
    }

    /// Twilio Verify check URL for this service.
    fn verification_check_url(&self) -> String {
        format!(
            "https://verify.twilio.com/v2/Services/{}/VerificationCheck",
            self.verify_service_sid
        )
    }
}

/// Send a verification code to the given phone number via Twilio Verify.
///
/// Returns the verification SID as a session identifier on success.
/// When Twilio credentials are not configured, returns an `Err` with a
/// descriptive message (T050).
pub fn send_verification_code(phone_number: &str) -> Result<String, String> {
    let config = SmsProviderConfig::from_env().ok_or_else(|| {
        "SMS provider credentials not configured. Set TWILIO_ACCOUNT_SID, \
         TWILIO_AUTH_TOKEN, and TWILIO_VERIFY_SID environment variables (see T088)."
            .to_string()
    })?;

    let client = reqwest::blocking::Client::new();

    let response = client
        .post(config.verifications_url())
        .basic_auth(&config.account_sid, Some(config.auth_token.clone()))
        .form(&[("To", phone_number), ("Channel", "sms")])
        .send()
        .map_err(|e| format!("Failed to send verification request: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Twilio API returned HTTP {status}: {body}"
        ));
    }

    let body: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse Twilio response: {e}"))?;

    body.get("sid")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| "Twilio response missing 'sid' field".to_string())
}

/// Verify a code entered by the user against the Twilio Verify API.
///
/// The `session_id` is the phone number (Twilio identifies verifications
/// by the phone number, not by SID for the check endpoint).
/// When credentials are missing, returns `ProviderUnavailable` (T050).
pub fn verify_code(phone_number: &str, code: &str) -> PhoneResult {
    let config = match SmsProviderConfig::from_env() {
        Some(c) => c,
        None => {
            return PhoneResult::ProviderUnavailable(
                "SMS provider credentials not configured. Set TWILIO_ACCOUNT_SID, \
                 TWILIO_AUTH_TOKEN, and TWILIO_VERIFY_SID environment variables (see T088)."
                    .to_string(),
            );
        }
    };

    let client = reqwest::blocking::Client::new();

    let response = match client
        .post(config.verification_check_url())
        .basic_auth(&config.account_sid, Some(config.auth_token.clone()))
        .form(&[("To", phone_number), ("Code", code)])
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            return PhoneResult::ProviderUnavailable(format!(
                "Failed to reach Twilio API: {e}"
            ));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        // Twilio returns 404 for expired verifications
        if status.as_u16() == 404 {
            return PhoneResult::CodeExpired;
        }
        return PhoneResult::ProviderUnavailable(format!(
            "Twilio API returned HTTP {status}: {body}"
        ));
    }

    let body: serde_json::Value = match response.json() {
        Ok(b) => b,
        Err(e) => {
            return PhoneResult::ProviderUnavailable(format!(
                "Failed to parse Twilio response: {e}"
            ));
        }
    };

    let status = body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    match status {
        "approved" => {
            // Hash the phone number for privacy
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(phone_number.as_bytes());
            let hash = hex::encode(hasher.finalize());
            PhoneResult::Verified { phone_hash: hash }
        }
        "pending" => PhoneResult::InvalidCode,
        "expired" => PhoneResult::CodeExpired,
        _ => PhoneResult::InvalidCode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_env_returns_none_when_missing() {
        if std::env::var("TWILIO_ACCOUNT_SID").is_err() {
            assert!(SmsProviderConfig::from_env().is_none());
        }
    }

    #[test]
    fn send_verification_code_fails_without_credentials() {
        if std::env::var("TWILIO_ACCOUNT_SID").is_err() {
            let result = send_verification_code("+1234567890");
            assert!(result.is_err());
            let msg = result.unwrap_err();
            assert!(msg.contains("TWILIO_ACCOUNT_SID"));
            assert!(msg.contains("T088"));
        }
    }

    #[test]
    fn verify_code_returns_unavailable_without_credentials() {
        if std::env::var("TWILIO_ACCOUNT_SID").is_err() {
            match verify_code("+1234567890", "123456") {
                PhoneResult::ProviderUnavailable(msg) => {
                    assert!(msg.contains("TWILIO_ACCOUNT_SID"));
                    assert!(msg.contains("T088"));
                }
                other => panic!("Expected ProviderUnavailable, got {other:?}"),
            }
        }
    }

    #[test]
    fn verification_urls_are_well_formed() {
        let config = SmsProviderConfig {
            account_sid: "AC_TEST".into(),
            auth_token: "token".into(),
            verify_service_sid: "VA_TEST".into(),
        };
        assert!(config.verifications_url().contains("VA_TEST"));
        assert!(config.verification_check_url().contains("VA_TEST"));
        assert!(config.verifications_url().starts_with("https://"));
        assert!(config.verification_check_url().starts_with("https://"));
    }
}
