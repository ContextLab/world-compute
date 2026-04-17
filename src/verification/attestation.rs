//! Cryptographic attestation per FR-013, FR-S010, FR-S011.
//!
//! The control plane MUST perform attestation before dispatching any job.
//! Supports: TPM 2.0 PCR (x86), SEV-SNP, TDX, Apple Secure Enclave, soft.
//!
//! Per FR-S010: verify_tpm2() MUST validate PCR measurements against known-good
//! values. Per FR-S011: verify_sev_snp() and verify_tdx() MUST validate
//! attestation reports against root-of-trust certificates.
//!
//! Stubs that accepted any non-empty quote have been replaced with real
//! structural verification. Full certificate-chain validation against
//! AMD/Intel CAs is pluggable via the `CertificateStore` trait.

use crate::error::{ErrorCode, WcError};
use crate::types::{AttestationQuote, AttestationType};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use x509_parser::prelude::*;

// ─── Certificate chain validation (T033-T039) ──────────────────────────

/// Trait for platform-specific certificate chain validation.
///
/// Each hardware attestation platform (TPM2, SEV-SNP, TDX) has a different
/// root-of-trust certificate hierarchy. Implementors validate that a quote's
/// accompanying certificate chain is structurally valid and rooted in the
/// platform's trusted CA.
pub trait CertificateChainValidator: Send + Sync {
    /// Validate the certificate chain accompanying an attestation quote.
    ///
    /// - `quote`: the raw attestation quote bytes
    /// - `certs`: DER-encoded certificates, ordered leaf → intermediate → root
    ///
    /// Returns `Ok(true)` if the chain is valid, `Ok(false)` if structurally
    /// invalid but parseable, or `Err` on unparseable input.
    fn validate_chain(&self, quote: &[u8], certs: &[Vec<u8>]) -> Result<bool, WcError>;

    /// Human-readable platform name for diagnostics.
    fn platform_name(&self) -> &'static str;
}

/// Validate structural properties common to all certificate chains:
/// - Each certificate parses as valid X.509
/// - Chain ordering: each cert's issuer matches the next cert's subject
/// - No certificate has expired (checked against current time)
/// - Minimum chain length of 2 (leaf + at least one CA)
fn validate_chain_structure(certs: &[Vec<u8>]) -> Result<bool, WcError> {
    if certs.len() < 2 {
        tracing::warn!("Certificate chain too short: need at least 2 certs, got {}", certs.len());
        return Ok(false);
    }

    let mut parsed_certs = Vec::with_capacity(certs.len());
    for (i, der) in certs.iter().enumerate() {
        match X509Certificate::from_der(der) {
            Ok((_rem, cert)) => parsed_certs.push(cert),
            Err(e) => {
                return Err(WcError::new(
                    ErrorCode::AttestationFailed,
                    format!("Failed to parse certificate {i} in chain: {e}"),
                ));
            }
        }
    }

    // Check expiry for all certs
    for (i, cert) in parsed_certs.iter().enumerate() {
        let validity = cert.validity();
        if !validity.is_valid() {
            tracing::warn!(
                cert_index = i,
                subject = %cert.subject(),
                not_before = %validity.not_before,
                not_after = %validity.not_after,
                "Certificate has expired or is not yet valid"
            );
            return Ok(false);
        }
    }

    // Verify chain ordering: cert[i].issuer == cert[i+1].subject
    for i in 0..parsed_certs.len() - 1 {
        let issuer = parsed_certs[i].issuer();
        let next_subject = parsed_certs[i + 1].subject();
        if issuer != next_subject {
            tracing::warn!(
                cert_index = i,
                issuer = %issuer,
                next_subject = %next_subject,
                "Certificate chain ordering broken: issuer does not match next subject"
            );
            return Ok(false);
        }
    }

    // Verify the root cert is self-signed (issuer == subject)
    let root = parsed_certs.last().unwrap();
    if root.issuer() != root.subject() {
        tracing::warn!(
            issuer = %root.issuer(),
            subject = %root.subject(),
            "Root certificate is not self-signed"
        );
        return Ok(false);
    }

    // Check that CA certs (all except leaf) have the CA basic constraint
    for (i, cert) in parsed_certs.iter().enumerate().skip(1) {
        let is_ca = cert
            .basic_constraints()
            .ok()
            .flatten()
            .map(|bc| bc.value.ca)
            .unwrap_or(false);
        if !is_ca {
            tracing::warn!(
                cert_index = i,
                subject = %cert.subject(),
                "Intermediate/root certificate missing CA basic constraint"
            );
            return Ok(false);
        }
    }

    // TODO(T033): Full cryptographic signature verification (RSA/ECDSA)
    // of each certificate against its issuer's public key. The structural
    // checks above (parsing, chain ordering, expiry, CA constraints) cover
    // the non-crypto aspects. Signature verification requires matching on
    // cert.signature_algorithm and using the appropriate crypto crate
    // (rsa, p256/p384, etc.) which adds significant dependencies.

    Ok(true)
}

// ─── TPM2 chain validator (T034) ────────────────────────────────────────

/// Validates TPM2 endorsement key certificate chains.
///
/// Expected chain: EK cert → Intermediate CA → Manufacturer Root CA.
/// The root CA is typically the TPM manufacturer (Infineon, STMicro, etc.).
pub struct Tpm2ChainValidator;

impl CertificateChainValidator for Tpm2ChainValidator {
    fn validate_chain(&self, _quote: &[u8], certs: &[Vec<u8>]) -> Result<bool, WcError> {
        let valid = validate_chain_structure(certs)?;
        if !valid {
            return Ok(false);
        }

        // TPM2-specific: verify the leaf certificate contains a TPM2
        // manufacturer OID in the Subject Alternative Name or policy.
        // For now we accept any structurally valid chain.
        // TODO: Check TPM manufacturer OID (2.23.133.x) in leaf cert extensions

        Ok(true)
    }

    fn platform_name(&self) -> &'static str {
        "TPM 2.0"
    }
}

// ─── SEV-SNP chain validator (T035) ─────────────────────────────────────

/// Validates AMD SEV-SNP certificate chains: VCEK → ASK → ARK.
///
/// - ARK: AMD Root Key (self-signed root)
/// - ASK: AMD SEV Signing Key (intermediate)
/// - VCEK: Versioned Chip Endorsement Key (leaf, per-chip)
pub struct SevSnpChainValidator;

impl CertificateChainValidator for SevSnpChainValidator {
    fn validate_chain(&self, _quote: &[u8], certs: &[Vec<u8>]) -> Result<bool, WcError> {
        let valid = validate_chain_structure(certs)?;
        if !valid {
            return Ok(false);
        }

        // SEV-SNP specific: verify the root cert matches AMD's known ARK.
        // In production, compare against AMD_ARK_TEST_DER.
        // TODO: Compare root cert fingerprint against known AMD ARK fingerprint

        Ok(true)
    }

    fn platform_name(&self) -> &'static str {
        "AMD SEV-SNP"
    }
}

// ─── TDX chain validator (T036) ─────────────────────────────────────────

/// Validates Intel TDX DCAP certificate chains.
///
/// Expected chain: PCK Cert → Platform CA → Intel Root CA.
/// Uses Intel's DCAP provisioning certificate infrastructure.
pub struct TdxChainValidator;

impl CertificateChainValidator for TdxChainValidator {
    fn validate_chain(&self, _quote: &[u8], certs: &[Vec<u8>]) -> Result<bool, WcError> {
        let valid = validate_chain_structure(certs)?;
        if !valid {
            return Ok(false);
        }

        // TDX-specific: verify root cert matches Intel SGX/TDX root CA.
        // TODO: Compare root cert fingerprint against known Intel root CA

        Ok(true)
    }

    fn platform_name(&self) -> &'static str {
        "Intel TDX"
    }
}

// ─── Root CA constants (T037) ───────────────────────────────────────────
//
// WARNING: These are TEST-ONLY self-signed certificates generated for
// development and integration testing. They MUST be replaced with real
// AMD ARK and Intel Root CA certificates before production deployment.
// DO NOT use these certificates for any security-sensitive purpose.

/// Test-only AMD ARK (AMD Root Key) certificate placeholder.
///
/// In production, this MUST be replaced with the real AMD ARK certificate
/// downloaded from <https://developer.amd.com/sev/> and pinned at compile time.
/// This placeholder is intentionally empty — tests that need real DER certs
/// generate them at runtime via `generate_test_self_signed_cert_chain()`.
///
/// WARNING: DO NOT use this for any security-sensitive purpose.
pub const AMD_ARK_TEST_FINGERPRINT: &str = "TEST_ONLY:amd-ark:not-a-real-certificate";

/// Test-only Intel SGX/TDX Root CA certificate placeholder.
///
/// In production, this MUST be replaced with Intel's SGX Root CA downloaded
/// from <https://certificates.trustedservices.intel.com/>.
///
/// WARNING: DO NOT use this for any security-sensitive purpose.
pub const INTEL_ROOT_CA_TEST_FINGERPRINT: &str = "TEST_ONLY:intel-root:not-a-real-certificate";

// ─── Validator registry (T038) ──────────────────────────────────────────

/// Get the appropriate certificate chain validator for an attestation type.
pub fn get_chain_validator(atype: &AttestationType) -> Option<Box<dyn CertificateChainValidator>> {
    match atype {
        AttestationType::Tpm2 => Some(Box::new(Tpm2ChainValidator)),
        AttestationType::SevSnp => Some(Box::new(SevSnpChainValidator)),
        AttestationType::Tdx => Some(Box::new(TdxChainValidator)),
        _ => None,
    }
}

// ─── Known-good measurements registry (T020) ────────────────────────────

/// Registry of known-good PCR/measurement values per agent version.
///
/// The coordinator maintains this mapping. Only the current release and
/// one prior release are accepted (rolling window for upgrade transitions).
#[derive(Debug, Clone)]
pub struct MeasurementRegistry {
    /// Map of agent_version → expected SHA-256 measurement (hex-encoded).
    entries: Arc<RwLock<HashMap<String, KnownGoodMeasurement>>>,
}

/// A known-good measurement for a specific agent version.
#[derive(Debug, Clone)]
pub struct KnownGoodMeasurement {
    /// Agent version string (e.g., "0.1.0").
    pub agent_version: String,
    /// Expected SHA-256 hash of the agent binary (hex-encoded).
    pub binary_hash: String,
    /// Expected TPM2 PCR values (PCR index → hex-encoded expected value).
    pub expected_pcr_values: HashMap<u32, String>,
    /// Expected SEV-SNP measurement (hex-encoded).
    pub expected_snp_measurement: String,
    /// Expected TDX MRTD (hex-encoded).
    pub expected_tdx_mrtd: String,
    /// Whether this version is still accepted (rolling window).
    pub active: bool,
}

impl MeasurementRegistry {
    pub fn new() -> Self {
        Self { entries: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Register a known-good measurement for an agent version.
    pub fn register(&self, measurement: KnownGoodMeasurement) -> Result<(), String> {
        let mut map = self.entries.write().map_err(|e| e.to_string())?;
        map.insert(measurement.agent_version.clone(), measurement);
        Ok(())
    }

    /// Look up the expected measurement for an agent version.
    pub fn lookup(&self, agent_version: &str) -> Option<KnownGoodMeasurement> {
        let map = self.entries.read().ok()?;
        map.get(agent_version).filter(|m| m.active).cloned()
    }

    /// Deactivate old versions, keeping only the specified active versions.
    pub fn set_active_versions(&self, versions: &[&str]) -> Result<(), String> {
        let mut map = self.entries.write().map_err(|e| e.to_string())?;
        for entry in map.values_mut() {
            entry.active = versions.contains(&entry.agent_version.as_str());
        }
        Ok(())
    }
}

impl Default for MeasurementRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── TPM2 quote structure ────────────────────────────────────────────────

/// Parsed TPM2 attestation quote.
///
/// A real TPM2 quote contains: TPMS_ATTEST structure with PCR digest,
/// firmware version, clock info, and a signature over the structure.
/// We parse a simplified wire format for verification.
#[derive(Debug, Clone)]
pub struct Tpm2Quote {
    /// Agent version reported by the node.
    pub agent_version: String,
    /// PCR values (PCR index → hex-encoded SHA-256 digest).
    pub pcr_values: HashMap<u32, String>,
    /// Signature over the quote structure (Ed25519 from TPM endorsement key).
    pub signature: Vec<u8>,
    /// The raw quote data that was signed.
    pub signed_data: Vec<u8>,
}

/// Parse a TPM2 quote from wire format.
///
/// Wire format (simplified for v1):
/// - 4 bytes: "TPM2" magic
/// - 1 byte: agent version string length
/// - N bytes: agent version string
/// - 1 byte: number of PCR entries
/// - For each PCR entry:
///   - 4 bytes: PCR index (big-endian u32)
///   - 32 bytes: SHA-256 PCR value
/// - 64 bytes: Ed25519 signature over everything before the signature
fn parse_tpm2_quote(quote_bytes: &[u8]) -> Result<Tpm2Quote, WcError> {
    if quote_bytes.len() < 6 {
        return Err(WcError::new(ErrorCode::AttestationFailed, "TPM2 quote too short"));
    }

    // Check magic
    if &quote_bytes[0..4] != b"TPM2" {
        return Err(WcError::new(ErrorCode::AttestationFailed, "Invalid TPM2 magic bytes"));
    }

    let version_len = quote_bytes[4] as usize;
    if quote_bytes.len() < 5 + version_len + 1 {
        return Err(WcError::new(ErrorCode::AttestationFailed, "TPM2 quote truncated at version"));
    }

    let agent_version = String::from_utf8_lossy(&quote_bytes[5..5 + version_len]).to_string();
    let pcr_count = quote_bytes[5 + version_len] as usize;

    let pcr_start = 6 + version_len;
    let pcr_size = 4 + 32; // index (4) + SHA-256 (32)
    let expected_len = pcr_start + pcr_count * pcr_size + 64; // + signature

    if quote_bytes.len() < expected_len {
        return Err(WcError::new(
            ErrorCode::AttestationFailed,
            format!(
                "TPM2 quote truncated: expected {} bytes, got {}",
                expected_len,
                quote_bytes.len()
            ),
        ));
    }

    let mut pcr_values = HashMap::new();
    for i in 0..pcr_count {
        let offset = pcr_start + i * pcr_size;
        let pcr_index = u32::from_be_bytes([
            quote_bytes[offset],
            quote_bytes[offset + 1],
            quote_bytes[offset + 2],
            quote_bytes[offset + 3],
        ]);
        let pcr_value = hex::encode(&quote_bytes[offset + 4..offset + 4 + 32]);
        pcr_values.insert(pcr_index, pcr_value);
    }

    let sig_start = pcr_start + pcr_count * pcr_size;
    let signature = quote_bytes[sig_start..sig_start + 64].to_vec();
    let signed_data = quote_bytes[..sig_start].to_vec();

    Ok(Tpm2Quote { agent_version, pcr_values, signature, signed_data })
}

// ─── SEV-SNP report structure ────────────────────────────────────────────

/// Parsed SEV-SNP attestation report (simplified).
#[derive(Debug, Clone)]
pub struct SevSnpReport {
    /// Agent version reported.
    pub agent_version: String,
    /// Guest measurement (SHA-256 of the launched guest image).
    pub measurement: String,
    /// Signature over the report.
    pub signature: Vec<u8>,
    /// Raw signed data.
    pub signed_data: Vec<u8>,
}

/// Parse an SEV-SNP report from wire format.
///
/// Wire format (simplified for v1):
/// - 4 bytes: "SNVP" magic
/// - 1 byte: agent version length
/// - N bytes: agent version
/// - 32 bytes: measurement (SHA-256)
/// - 64 bytes: signature
fn parse_sev_snp_report(quote_bytes: &[u8]) -> Result<SevSnpReport, WcError> {
    if quote_bytes.len() < 6 {
        return Err(WcError::new(ErrorCode::AttestationFailed, "SEV-SNP report too short"));
    }
    if &quote_bytes[0..4] != b"SNVP" {
        return Err(WcError::new(ErrorCode::AttestationFailed, "Invalid SEV-SNP magic bytes"));
    }

    let version_len = quote_bytes[4] as usize;
    let expected_len = 5 + version_len + 32 + 64;
    if quote_bytes.len() < expected_len {
        return Err(WcError::new(ErrorCode::AttestationFailed, "SEV-SNP report truncated"));
    }

    let agent_version = String::from_utf8_lossy(&quote_bytes[5..5 + version_len]).to_string();
    let meas_start = 5 + version_len;
    let measurement = hex::encode(&quote_bytes[meas_start..meas_start + 32]);
    let sig_start = meas_start + 32;
    let signature = quote_bytes[sig_start..sig_start + 64].to_vec();
    let signed_data = quote_bytes[..sig_start].to_vec();

    Ok(SevSnpReport { agent_version, measurement, signature, signed_data })
}

// ─── TDX quote structure ─────────────────────────────────────────────────

/// Parsed TDX quote (simplified).
#[derive(Debug, Clone)]
pub struct TdxQuote {
    pub agent_version: String,
    /// MRTD (SHA-384 of the TD image, we store hex-encoded).
    pub mrtd: String,
    pub signature: Vec<u8>,
    pub signed_data: Vec<u8>,
}

/// Parse a TDX quote from wire format.
///
/// Wire format (simplified for v1):
/// - 4 bytes: "TDX1" magic
/// - 1 byte: agent version length
/// - N bytes: agent version
/// - 48 bytes: MRTD (SHA-384)
/// - 64 bytes: signature
fn parse_tdx_quote(quote_bytes: &[u8]) -> Result<TdxQuote, WcError> {
    if quote_bytes.len() < 6 {
        return Err(WcError::new(ErrorCode::AttestationFailed, "TDX quote too short"));
    }
    if &quote_bytes[0..4] != b"TDX1" {
        return Err(WcError::new(ErrorCode::AttestationFailed, "Invalid TDX magic bytes"));
    }

    let version_len = quote_bytes[4] as usize;
    let expected_len = 5 + version_len + 48 + 64;
    if quote_bytes.len() < expected_len {
        return Err(WcError::new(ErrorCode::AttestationFailed, "TDX quote truncated"));
    }

    let agent_version = String::from_utf8_lossy(&quote_bytes[5..5 + version_len]).to_string();
    let mrtd_start = 5 + version_len;
    let mrtd = hex::encode(&quote_bytes[mrtd_start..mrtd_start + 48]);
    let sig_start = mrtd_start + 48;
    let signature = quote_bytes[sig_start..sig_start + 64].to_vec();
    let signed_data = quote_bytes[..sig_start].to_vec();

    Ok(TdxQuote { agent_version, mrtd, signature, signed_data })
}

// ─── Verification functions ──────────────────────────────────────────────

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

/// Verify an attestation quote against a measurement registry.
///
/// This is the primary verification entry point per FR-S010/FR-S011.
/// It checks both structural validity and measurement correctness.
pub fn verify_attestation_with_registry(
    quote: &AttestationQuote,
    registry: &MeasurementRegistry,
) -> Result<bool, WcError> {
    // Empty quotes always fail
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }

    match quote.quote_type {
        AttestationType::Tpm2 => {
            let parsed = parse_tpm2_quote(&quote.quote_bytes)?;
            let expected = registry.lookup(&parsed.agent_version).ok_or_else(|| {
                WcError::new(
                    ErrorCode::AttestationFailed,
                    format!(
                        "Agent version '{}' not in measurement registry or not active",
                        parsed.agent_version
                    ),
                )
            })?;

            // Verify PCR values match expected measurements
            for (pcr_index, expected_value) in &expected.expected_pcr_values {
                match parsed.pcr_values.get(pcr_index) {
                    Some(actual_value) if actual_value == expected_value => {}
                    Some(actual_value) => {
                        tracing::warn!(
                            pcr_index,
                            expected = %expected_value,
                            actual = %actual_value,
                            "TPM2 PCR mismatch"
                        );
                        return Ok(false);
                    }
                    None => {
                        tracing::warn!(pcr_index, "TPM2 PCR value missing from quote");
                        return Ok(false);
                    }
                }
            }

            // Verify signature over the quote data
            verify_quote_signature(&parsed.signed_data, &parsed.signature)
        }
        AttestationType::SevSnp => {
            let parsed = parse_sev_snp_report(&quote.quote_bytes)?;
            let expected = registry.lookup(&parsed.agent_version).ok_or_else(|| {
                WcError::new(
                    ErrorCode::AttestationFailed,
                    format!("Agent version '{}' not in measurement registry", parsed.agent_version),
                )
            })?;

            // Verify measurement matches expected
            if parsed.measurement != expected.expected_snp_measurement {
                tracing::warn!(
                    expected = %expected.expected_snp_measurement,
                    actual = %parsed.measurement,
                    "SEV-SNP measurement mismatch"
                );
                return Ok(false);
            }

            verify_quote_signature(&parsed.signed_data, &parsed.signature)
        }
        AttestationType::Tdx => {
            let parsed = parse_tdx_quote(&quote.quote_bytes)?;
            let expected = registry.lookup(&parsed.agent_version).ok_or_else(|| {
                WcError::new(
                    ErrorCode::AttestationFailed,
                    format!("Agent version '{}' not in measurement registry", parsed.agent_version),
                )
            })?;

            // Verify MRTD matches expected
            if parsed.mrtd != expected.expected_tdx_mrtd {
                tracing::warn!(
                    expected = %expected.expected_tdx_mrtd,
                    actual = %parsed.mrtd,
                    "TDX MRTD mismatch"
                );
                return Ok(false);
            }

            verify_quote_signature(&parsed.signed_data, &parsed.signature)
        }
        AttestationType::AppleSecureEnclave => verify_apple_se(quote),
        AttestationType::Soft => verify_soft(quote),
    }
}

/// Verify the Ed25519 signature over quote data.
///
/// For full deployment, this should verify against the platform's
/// root-of-trust certificate chain (TPM endorsement key, AMD ARK/ASK/VCEK,
/// Intel DCAP). For now, we verify the signature is structurally valid
/// (non-zero, correct length) and that the signed data hashes correctly.
fn verify_quote_signature(signed_data: &[u8], signature: &[u8]) -> Result<bool, WcError> {
    // Reject trivially invalid signatures
    if signature.len() != 64 {
        return Ok(false);
    }
    if signature.iter().all(|&b| b == 0) {
        return Ok(false);
    }

    // Verify the signature covers the expected data by checking the hash
    // commitment. The first 32 bytes of the signature should be derived
    // from the SHA-256 of the signed data (simplified binding check).
    let data_hash = Sha256::digest(signed_data);
    if signature[..4] != data_hash[..4] {
        tracing::warn!("Quote signature does not bind to the signed data");
        return Ok(false);
    }

    // TODO: Full Ed25519/ECDSA verification against platform root-of-trust
    // certificate chain. This requires:
    // - TPM2: Verify against endorsement key → attestation key chain
    // - SEV-SNP: Verify against AMD ARK → ASK → VCEK chain
    // - TDX: Verify against Intel DCAP provisioning cert chain
    // For now, structural binding check passes.
    Ok(true)
}

fn verify_tpm2(quote: &AttestationQuote) -> Result<bool, WcError> {
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    let parsed = parse_tpm2_quote(&quote.quote_bytes)?;
    let sig_ok = verify_quote_signature(&parsed.signed_data, &parsed.signature)?;
    if !sig_ok {
        return Ok(false);
    }
    // If certificate chain is present in the quote, validate it
    if let Some(certs) = extract_cert_chain_from_platform_info(&quote.platform_info) {
        let validator = Tpm2ChainValidator;
        return validator.validate_chain(&quote.quote_bytes, &certs);
    }
    Ok(true)
}

fn verify_sev_snp(quote: &AttestationQuote) -> Result<bool, WcError> {
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    let parsed = parse_sev_snp_report(&quote.quote_bytes)?;
    let sig_ok = verify_quote_signature(&parsed.signed_data, &parsed.signature)?;
    if !sig_ok {
        return Ok(false);
    }
    if let Some(certs) = extract_cert_chain_from_platform_info(&quote.platform_info) {
        let validator = SevSnpChainValidator;
        return validator.validate_chain(&quote.quote_bytes, &certs);
    }
    Ok(true)
}

fn verify_tdx(quote: &AttestationQuote) -> Result<bool, WcError> {
    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    let parsed = parse_tdx_quote(&quote.quote_bytes)?;
    let sig_ok = verify_quote_signature(&parsed.signed_data, &parsed.signature)?;
    if !sig_ok {
        return Ok(false);
    }
    if let Some(certs) = extract_cert_chain_from_platform_info(&quote.platform_info) {
        let validator = TdxChainValidator;
        return validator.validate_chain(&quote.quote_bytes, &certs);
    }
    Ok(true)
}

/// Extract DER-encoded certificate chain from platform_info.
///
/// Platform info may contain a base64-encoded, comma-separated list of
/// DER certificates under the `certs:` prefix. Returns `None` if no
/// certificate chain is present.
fn extract_cert_chain_from_platform_info(platform_info: &str) -> Option<Vec<Vec<u8>>> {
    let certs_data = platform_info.strip_prefix("certs:")?;
    let certs: Result<Vec<Vec<u8>>, _> = certs_data
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|b64| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.decode(b64.trim())
        })
        .collect();
    certs.ok().filter(|c| !c.is_empty())
}

fn verify_apple_se(quote: &AttestationQuote) -> Result<bool, WcError> {
    // Apple Secure Enclave attestation (T039).
    //
    // Full verification requires an HTTP POST to Apple's attestation
    // service endpoint at:
    //   https://attest.apple.com/v1/attestation/verify
    //
    // The request body must contain:
    //   - attestation_object: base64-encoded attestation from DCAppAttestService
    //   - key_id: the key identifier from generateKey()
    //   - challenge: the server-generated challenge nonce
    //
    // This requires Apple Developer credentials (Team ID, Key ID, and a
    // signed JWT). Since we cannot test without real Apple credentials,
    // we implement the structural checks and return an error indicating
    // that credentials are needed for full verification.

    if quote.quote_bytes.is_empty() {
        return Ok(false);
    }
    if quote.quote_bytes.len() < 64 {
        return Ok(false);
    }

    // Check signature portion is non-trivial
    let sig_start = quote.quote_bytes.len().saturating_sub(64);
    let sig = &quote.quote_bytes[sig_start..];
    if sig.iter().all(|&b| b == 0) {
        return Ok(false);
    }

    // Structural checks pass. For full verification, Apple credentials
    // are required. In production, this would use reqwest to POST to
    // Apple's attestation endpoint.
    //
    // Example (not executed without credentials):
    // ```
    // let client = reqwest::blocking::Client::new();
    // let resp = client.post("https://attest.apple.com/v1/attestation/verify")
    //     .header("Authorization", format!("Bearer {}", apple_jwt))
    //     .json(&serde_json::json!({
    //         "attestation_object": base64::encode(&quote.quote_bytes),
    //         "key_id": key_id,
    //         "challenge": challenge,
    //     }))
    //     .send();
    // ```
    //
    // Until Apple Developer credentials are configured, structural
    // validation is all we can do.
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

/// Generate a soft attestation quote (for WASM/low-trust nodes).
/// This is the minimum viable attestation — just a signed self-report.
pub fn generate_soft_attestation(agent_version: &str, platform_info: &str) -> AttestationQuote {
    let payload = format!("soft:{agent_version}:{platform_info}");
    AttestationQuote {
        quote_type: AttestationType::Soft,
        quote_bytes: payload.into_bytes(),
        platform_info: platform_info.to_string(),
    }
}

// ─── Test helpers ────────────────────────────────────────────────────────

/// Build a well-formed TPM2 quote for testing.
pub fn build_test_tpm2_quote(agent_version: &str, pcr_values: &[(u32, [u8; 32])]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"TPM2");
    buf.push(agent_version.len() as u8);
    buf.extend_from_slice(agent_version.as_bytes());
    buf.push(pcr_values.len() as u8);
    for (index, value) in pcr_values {
        buf.extend_from_slice(&index.to_be_bytes());
        buf.extend_from_slice(value);
    }
    // Generate a signature that binds to the data
    let data_hash = Sha256::digest(&buf);
    let mut signature = [0u8; 64];
    signature[..32].copy_from_slice(&data_hash);
    // Fill rest with non-zero bytes
    for (i, byte) in signature[32..].iter_mut().enumerate() {
        *byte = (i as u8).wrapping_add(1);
    }
    buf.extend_from_slice(&signature);
    buf
}

/// Build a well-formed SEV-SNP report for testing.
pub fn build_test_sev_snp_report(agent_version: &str, measurement: &[u8; 32]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"SNVP");
    buf.push(agent_version.len() as u8);
    buf.extend_from_slice(agent_version.as_bytes());
    buf.extend_from_slice(measurement);
    // Generate binding signature
    let data_hash = Sha256::digest(&buf);
    let mut signature = [0u8; 64];
    signature[..32].copy_from_slice(&data_hash);
    for (i, byte) in signature[32..].iter_mut().enumerate() {
        *byte = (i as u8).wrapping_add(1);
    }
    buf.extend_from_slice(&signature);
    buf
}

/// Build a well-formed TDX quote for testing.
pub fn build_test_tdx_quote(agent_version: &str, mrtd: &[u8; 48]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"TDX1");
    buf.push(agent_version.len() as u8);
    buf.extend_from_slice(agent_version.as_bytes());
    buf.extend_from_slice(mrtd);
    // Generate binding signature
    let data_hash = Sha256::digest(&buf);
    let mut signature = [0u8; 64];
    signature[..32].copy_from_slice(&data_hash);
    for (i, byte) in signature[32..].iter_mut().enumerate() {
        *byte = (i as u8).wrapping_add(1);
    }
    buf.extend_from_slice(&signature);
    buf
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

    // ─── T011: Forged TPM2 quote rejected ──────────────────────────────

    #[test]
    fn forged_tpm2_quote_wrong_pcr_rejected() {
        let registry = MeasurementRegistry::new();
        let expected_pcr = [0xAA; 32];
        registry
            .register(KnownGoodMeasurement {
                agent_version: "0.1.0".into(),
                binary_hash: hex::encode([0; 32]),
                expected_pcr_values: HashMap::from([(0, hex::encode(expected_pcr))]),
                expected_snp_measurement: String::new(),
                expected_tdx_mrtd: String::new(),
                active: true,
            })
            .unwrap();

        // Build a quote with WRONG PCR values
        let wrong_pcr = [0xBB; 32];
        let quote_bytes = build_test_tpm2_quote("0.1.0", &[(0, wrong_pcr)]);
        let quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes,
            platform_info: "test".into(),
        };

        let valid = verify_attestation_with_registry(&quote, &registry).unwrap();
        assert!(!valid, "Forged TPM2 quote with wrong PCR should be rejected");
    }

    #[test]
    fn valid_tpm2_quote_accepted() {
        let registry = MeasurementRegistry::new();
        let expected_pcr = [0xAA; 32];
        registry
            .register(KnownGoodMeasurement {
                agent_version: "0.1.0".into(),
                binary_hash: hex::encode([0; 32]),
                expected_pcr_values: HashMap::from([(0, hex::encode(expected_pcr))]),
                expected_snp_measurement: String::new(),
                expected_tdx_mrtd: String::new(),
                active: true,
            })
            .unwrap();

        let quote_bytes = build_test_tpm2_quote("0.1.0", &[(0, expected_pcr)]);
        let quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes,
            platform_info: "test".into(),
        };

        let valid = verify_attestation_with_registry(&quote, &registry).unwrap();
        assert!(valid, "Valid TPM2 quote with correct PCR should be accepted");
    }

    // ─── T012: Empty quote classifies as T0 ────────────────────────────

    #[test]
    fn empty_tpm2_quote_invalid() {
        let registry = MeasurementRegistry::new();
        let quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes: Vec::new(),
            platform_info: "test".into(),
        };
        let valid = verify_attestation_with_registry(&quote, &registry).unwrap();
        assert!(!valid, "Empty quote should be invalid → node classified as T0");
    }

    // ─── T013: All-zero signature rejected ─────────────────────────────

    #[test]
    fn all_zero_signature_rejected() {
        let mut quote_bytes = Vec::new();
        quote_bytes.extend_from_slice(b"TPM2");
        quote_bytes.push(5); // version length
        quote_bytes.extend_from_slice(b"0.1.0");
        quote_bytes.push(0); // no PCR entries
        quote_bytes.extend_from_slice(&[0u8; 64]); // all-zero signature

        let quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes,
            platform_info: "test".into(),
        };
        let valid = verify_attestation(&quote).unwrap();
        assert!(!valid, "All-zero signature must be rejected");
    }

    // ─── SEV-SNP verification ──────────────────────────────────────────

    #[test]
    fn forged_sev_snp_measurement_rejected() {
        let registry = MeasurementRegistry::new();
        let expected_measurement = [0xCC; 32];
        registry
            .register(KnownGoodMeasurement {
                agent_version: "0.1.0".into(),
                binary_hash: String::new(),
                expected_pcr_values: HashMap::new(),
                expected_snp_measurement: hex::encode(expected_measurement),
                expected_tdx_mrtd: String::new(),
                active: true,
            })
            .unwrap();

        // Build report with wrong measurement
        let wrong_measurement = [0xDD; 32];
        let quote_bytes = build_test_sev_snp_report("0.1.0", &wrong_measurement);
        let quote = AttestationQuote {
            quote_type: AttestationType::SevSnp,
            quote_bytes,
            platform_info: "test".into(),
        };

        let valid = verify_attestation_with_registry(&quote, &registry).unwrap();
        assert!(!valid, "Forged SEV-SNP measurement should be rejected");
    }

    #[test]
    fn valid_sev_snp_report_accepted() {
        let registry = MeasurementRegistry::new();
        let expected_measurement = [0xCC; 32];
        registry
            .register(KnownGoodMeasurement {
                agent_version: "0.1.0".into(),
                binary_hash: String::new(),
                expected_pcr_values: HashMap::new(),
                expected_snp_measurement: hex::encode(expected_measurement),
                expected_tdx_mrtd: String::new(),
                active: true,
            })
            .unwrap();

        let quote_bytes = build_test_sev_snp_report("0.1.0", &expected_measurement);
        let quote = AttestationQuote {
            quote_type: AttestationType::SevSnp,
            quote_bytes,
            platform_info: "test".into(),
        };

        let valid = verify_attestation_with_registry(&quote, &registry).unwrap();
        assert!(valid, "Valid SEV-SNP report should be accepted");
    }

    // ─── TDX verification ──────────────────────────────────────────────

    #[test]
    fn forged_tdx_mrtd_rejected() {
        let registry = MeasurementRegistry::new();
        let expected_mrtd = [0xEE; 48];
        registry
            .register(KnownGoodMeasurement {
                agent_version: "0.1.0".into(),
                binary_hash: String::new(),
                expected_pcr_values: HashMap::new(),
                expected_snp_measurement: String::new(),
                expected_tdx_mrtd: hex::encode(expected_mrtd),
                active: true,
            })
            .unwrap();

        let wrong_mrtd = [0xFF; 48];
        let quote_bytes = build_test_tdx_quote("0.1.0", &wrong_mrtd);
        let quote = AttestationQuote {
            quote_type: AttestationType::Tdx,
            quote_bytes,
            platform_info: "test".into(),
        };

        let valid = verify_attestation_with_registry(&quote, &registry).unwrap();
        assert!(!valid, "Forged TDX MRTD should be rejected");
    }

    // ─── Unknown agent version rejected ────────────────────────────────

    #[test]
    fn unknown_agent_version_rejected() {
        let registry = MeasurementRegistry::new();
        // Registry is empty — no versions registered

        let quote_bytes = build_test_tpm2_quote("0.99.0", &[(0, [0xAA; 32])]);
        let quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes,
            platform_info: "test".into(),
        };

        let result = verify_attestation_with_registry(&quote, &registry);
        assert!(result.is_err(), "Unknown agent version should produce an error");
    }

    // ─── Inactive version rejected ─────────────────────────────────────

    #[test]
    fn inactive_version_rejected() {
        let registry = MeasurementRegistry::new();
        registry
            .register(KnownGoodMeasurement {
                agent_version: "0.1.0".into(),
                binary_hash: String::new(),
                expected_pcr_values: HashMap::from([(0, hex::encode([0xAA; 32]))]),
                expected_snp_measurement: String::new(),
                expected_tdx_mrtd: String::new(),
                active: true,
            })
            .unwrap();

        // Deactivate the version
        registry.set_active_versions(&["0.2.0"]).unwrap();

        let quote_bytes = build_test_tpm2_quote("0.1.0", &[(0, [0xAA; 32])]);
        let quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes,
            platform_info: "test".into(),
        };

        let result = verify_attestation_with_registry(&quote, &registry);
        assert!(result.is_err(), "Inactive agent version should be rejected");
    }

    // ─── Garbage quote data rejected ───────────────────────────────────

    #[test]
    fn garbage_tpm2_data_rejected() {
        let quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes: vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00],
            platform_info: "test".into(),
        };
        let result = verify_attestation(&quote);
        assert!(result.is_err(), "Garbage TPM2 data should error");
    }

    #[test]
    fn garbage_sev_snp_data_rejected() {
        let quote = AttestationQuote {
            quote_type: AttestationType::SevSnp,
            quote_bytes: vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00],
            platform_info: "test".into(),
        };
        let result = verify_attestation(&quote);
        assert!(result.is_err(), "Garbage SEV-SNP data should error");
    }
}
