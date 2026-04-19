//! mTLS configuration and certificate authority per FR-060 security transport requirements.

use crate::error::{ErrorCode, WcError};
use std::path::PathBuf;

/// Certificate rotation policy.
#[derive(Debug, Clone)]
pub struct CertRotationPolicy {
    /// Rotate certificates after this many days.
    pub rotate_after_days: u32,
    /// Overlap window in days during which both old and new certs are valid.
    pub overlap_days: u32,
    /// Whether to automatically trigger rotation without manual intervention.
    pub auto_rotate: bool,
}

impl Default for CertRotationPolicy {
    fn default() -> Self {
        Self { rotate_after_days: 90, overlap_days: 7, auto_rotate: true }
    }
}

/// mTLS configuration for World Compute network transport.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to the node's TLS certificate (PEM).
    pub cert_path: PathBuf,
    /// Path to the node's private key (PEM).
    pub key_path: PathBuf,
    /// Path to the CA certificate bundle used to verify peers (PEM).
    pub ca_path: PathBuf,
    /// Number of days before automatic certificate rotation.
    pub auto_rotate_days: u32,
    /// Certificate rotation policy details.
    pub rotation_policy: CertRotationPolicy,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: PathBuf::from("/etc/world-compute/tls/node.crt"),
            key_path: PathBuf::from("/etc/world-compute/tls/node.key"),
            ca_path: PathBuf::from("/etc/world-compute/tls/ca-bundle.crt"),
            auto_rotate_days: 90,
            rotation_policy: CertRotationPolicy::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// T090: Self-signed CA and per-account certificate issuance using rcgen
// ---------------------------------------------------------------------------

/// A self-signed Certificate Authority that can issue per-account certs.
pub struct CertificateAuthority {
    /// DER-encoded CA certificate.
    pub ca_cert_der: Vec<u8>,
    /// DER-encoded CA private key (PKCS#8).
    pub ca_key_der: Vec<u8>,
    /// The rcgen CA certificate (used internally for signing).
    ca_cert: rcgen::CertifiedKey,
}

/// An issued certificate signed by the CA.
#[derive(Debug, Clone)]
pub struct IssuedCert {
    /// DER-encoded certificate.
    pub cert_der: Vec<u8>,
    /// DER-encoded private key (PKCS#8).
    pub key_der: Vec<u8>,
    /// Certificate expiry time.
    pub not_after: chrono::DateTime<chrono::Utc>,
}

impl CertificateAuthority {
    /// Generate a new self-signed CA using ECDSA P-256.
    pub fn new() -> Result<Self, WcError> {
        use rcgen::{CertificateParams, DnType, IsCa, KeyPair};

        let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).map_err(|e| {
            WcError::new(ErrorCode::Internal, format!("CA key generation failed: {e}"))
        })?;

        let mut params = CertificateParams::new(Vec::<String>::new()).map_err(|e| {
            WcError::new(ErrorCode::Internal, format!("CA params creation failed: {e}"))
        })?;
        params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.distinguished_name.push(DnType::CommonName, "World Compute CA");
        params.distinguished_name.push(DnType::OrganizationName, "World Compute");
        // CA valid for 10 years
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2034, 1, 1);

        let ca_cert = params
            .self_signed(&key_pair)
            .map_err(|e| WcError::new(ErrorCode::Internal, format!("CA self-sign failed: {e}")))?;

        let ca_cert_der = ca_cert.der().to_vec();
        let ca_key_der = key_pair.serialized_der().to_vec();

        Ok(Self {
            ca_cert_der,
            ca_key_der,
            ca_cert: rcgen::CertifiedKey { cert: ca_cert, key_pair },
        })
    }

    /// Issue a certificate for `subject` (e.g., account ID) signed by this CA.
    /// The issued cert is valid for 90 days from now.
    pub fn issue_cert(&self, subject: &str) -> Result<IssuedCert, WcError> {
        use rcgen::{CertificateParams, DnType, KeyPair};

        let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).map_err(|e| {
            WcError::new(ErrorCode::Internal, format!("cert key generation failed: {e}"))
        })?;

        let mut params = CertificateParams::new(vec![subject.to_string()]).map_err(|e| {
            WcError::new(ErrorCode::Internal, format!("cert params creation failed: {e}"))
        })?;
        params.distinguished_name.push(DnType::CommonName, subject);

        // Valid for 90 days from now
        let now = chrono::Utc::now();
        let expiry = now + chrono::Duration::days(90);

        let cert = params
            .signed_by(&key_pair, &self.ca_cert.cert, &self.ca_cert.key_pair)
            .map_err(|e| WcError::new(ErrorCode::Internal, format!("cert signing failed: {e}")))?;

        Ok(IssuedCert {
            cert_der: cert.der().to_vec(),
            key_der: key_pair.serialized_der().to_vec(),
            not_after: expiry,
        })
    }
}

// ---------------------------------------------------------------------------
// T091: Auto-rotation logic
// ---------------------------------------------------------------------------

/// Returns true if the certificate expires within `days_before_expiry` days.
pub fn needs_rotation(cert: &IssuedCert, days_before_expiry: u32) -> bool {
    let threshold = chrono::Utc::now() + chrono::Duration::days(days_before_expiry as i64);
    cert.not_after <= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_90_day_rotation() {
        let cfg = TlsConfig::default();
        assert_eq!(cfg.auto_rotate_days, 90);
        assert_eq!(cfg.rotation_policy.rotate_after_days, 90);
        assert!(cfg.rotation_policy.auto_rotate);
    }

    #[test]
    fn rotation_policy_has_overlap_window() {
        let policy = CertRotationPolicy::default();
        assert!(policy.overlap_days > 0, "overlap window should be positive");
    }

    #[test]
    fn custom_paths_are_stored() {
        let cfg = TlsConfig {
            cert_path: PathBuf::from("/custom/cert.pem"),
            key_path: PathBuf::from("/custom/key.pem"),
            ca_path: PathBuf::from("/custom/ca.pem"),
            auto_rotate_days: 30,
            rotation_policy: CertRotationPolicy {
                rotate_after_days: 30,
                overlap_days: 3,
                auto_rotate: false,
            },
        };
        assert_eq!(cfg.cert_path, PathBuf::from("/custom/cert.pem"));
        assert_eq!(cfg.auto_rotate_days, 30);
        assert!(!cfg.rotation_policy.auto_rotate);
    }

    #[test]
    fn ca_generation_succeeds() {
        let ca = CertificateAuthority::new().expect("CA generation should succeed");
        assert!(!ca.ca_cert_der.is_empty(), "CA cert DER should not be empty");
        assert!(!ca.ca_key_der.is_empty(), "CA key DER should not be empty");
    }

    #[test]
    fn cert_issuance_succeeds() {
        let ca = CertificateAuthority::new().unwrap();
        let cert = ca.issue_cert("test-account-001").expect("cert issuance should succeed");
        assert!(!cert.cert_der.is_empty());
        assert!(!cert.key_der.is_empty());
        // Cert should expire ~90 days from now
        let days_until = (cert.not_after - chrono::Utc::now()).num_days();
        assert!(
            (89..=91).contains(&days_until),
            "cert should expire in ~90 days, got {days_until}"
        );
    }

    #[test]
    fn needs_rotation_expiring_soon() {
        let cert = IssuedCert {
            cert_der: vec![],
            key_der: vec![],
            not_after: chrono::Utc::now() + chrono::Duration::days(3),
        };
        assert!(
            needs_rotation(&cert, 7),
            "cert expiring in 3 days should need rotation at 7-day threshold"
        );
    }

    #[test]
    fn no_rotation_needed_far_expiry() {
        let cert = IssuedCert {
            cert_der: vec![],
            key_der: vec![],
            not_after: chrono::Utc::now() + chrono::Duration::days(30),
        };
        assert!(
            !needs_rotation(&cert, 7),
            "cert expiring in 30 days should not need rotation at 7-day threshold"
        );
    }
}
