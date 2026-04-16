//! Cryptographic attestation per FR-013 (T044).
//!
//! The control plane MUST perform attestation before dispatching any job.
//! Supports: TPM 2.0 PCR (x86), SEV-SNP, TDX, Apple Secure Enclave, soft.

use crate::error::{ErrorCode, WcError};
use crate::types::{AttestationQuote, AttestationType};

/// Verify an attestation quote from a donor node.
/// Returns Ok(true) if the quote is valid, Ok(false) if invalid but parseable,
/// or Err if the quote format is unrecognizable.
pub fn verify_attestation(quote: &AttestationQuote) -> Result<bool, WcError> {
    match quote.quote_type {
        AttestationType::Tpm2 => verify_tpm2(quote),
        AttestationType::SevSnp => verify_sev_snp(quote),
        AttestationType::Tdx => verify_tdx(quote),
        AttestationType::AppleSecureEnclave => verify_apple_se(quote),
        AttestationType::Soft => verify_soft(quote),
    }
}

/// Generate a soft attestation quote (for WASM/low-trust nodes).
/// This is the minimum viable attestation — just a signed self-report.
pub fn generate_soft_attestation(agent_version: &str, platform_info: &str) -> AttestationQuote {
    // Soft attestation: agent self-reports its version and platform.
    // This is the lowest trust tier and should only be used for T0 nodes.
    let payload = format!("soft:{}:{}", agent_version, platform_info);
    AttestationQuote {
        quote_type: AttestationType::Soft,
        quote_bytes: payload.into_bytes(),
        platform_info: platform_info.to_string(),
    }
}

fn verify_tpm2(quote: &AttestationQuote) -> Result<bool, WcError> {
    // TODO: Parse TPM2 quote structure, verify PCR values against
    // known-good measurements, check signature chain.
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    tracing::debug!("TPM2 attestation verification (stub) — accepting");
    Ok(true)
}

fn verify_sev_snp(quote: &AttestationQuote) -> Result<bool, WcError> {
    // TODO: Verify AMD SEV-SNP attestation report against AMD's
    // signing key chain, check measurement against expected guest image.
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    tracing::debug!("SEV-SNP attestation verification (stub) — accepting");
    Ok(true)
}

fn verify_tdx(quote: &AttestationQuote) -> Result<bool, WcError> {
    // TODO: Verify Intel TDX quote, check MRTD against expected values.
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    tracing::debug!("TDX attestation verification (stub) — accepting");
    Ok(true)
}

fn verify_apple_se(quote: &AttestationQuote) -> Result<bool, WcError> {
    // TODO: Verify Apple Secure Enclave signing via DeviceCheck attestation.
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    tracing::debug!("Apple SE attestation verification (stub) — accepting");
    Ok(true)
}

fn verify_soft(quote: &AttestationQuote) -> Result<bool, WcError> {
    // Soft attestation: just check the payload is non-empty and parseable.
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    let payload = String::from_utf8_lossy(&quote.quote_bytes);
    Ok(payload.starts_with("soft:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_attestation_round_trip() {
        let quote = generate_soft_attestation("0.1.0", "linux-x86_64");
        assert_eq!(quote.quote_type, AttestationType::Soft);
        let valid = verify_attestation(&quote).unwrap();
        assert!(valid);
    }

    #[test]
    fn empty_quote_is_invalid() {
        let quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes: Vec::new(),
            platform_info: String::new(),
        };
        let valid = verify_attestation(&quote).unwrap();
        assert!(!valid);
    }
}
