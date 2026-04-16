//! mTLS configuration stub per FR-060 security transport requirements.

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
}
