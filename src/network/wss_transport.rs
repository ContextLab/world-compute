//! WebSocket-over-TLS-443 fallback transport per spec 005 US1 T018 / FR-003.
//!
//! Operators behind hostile institutional firewalls that allow only HTTPS
//! traffic can still join the mesh via this transport. libp2p's websocket
//! transport negotiates the wire protocol over TLS on port 443 — the same
//! port browsers use — so virtually every firewall allows it.
//!
//! **Security considerations**:
//! - Connections are end-to-end Noise-encrypted over libp2p regardless, so
//!   middleboxes cannot inspect payload.
//! - TLS pin-mismatch detection: when a middlebox does SSL inspection, the
//!   outer TLS cert will not match the known-relay fingerprint. By default
//!   we refuse; opt-in via `--allow-ssl-inspection` (see
//!   `WssTransportConfig::allow_ssl_inspection`). When opt-in, the
//!   connection is marked `Inspected` and the trust tier is capped.

use crate::types::TransportKind;
use serde::{Deserialize, Serialize};

/// Configuration for the WSS-443 fallback transport (data-model A.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WssTransportConfig {
    /// Master switch. Default true — always available as a fallback.
    pub enabled: bool,
    /// If this node should listen on 443 for inbound WSS circuits (typically
    /// only dedicated relays). Default false.
    pub listen_on_443: bool,
    /// Order in the fallback chain (QUIC=0, TCP=1, WSS=2).
    pub fallback_priority: u8,
    /// Enforce TLS pin-match against known relay fingerprints. Default true.
    pub middlebox_pin_check: bool,
    /// Allow SSL-inspecting middlebox to MITM the connection; requires
    /// `middlebox_pin_check == false` and downgrades the connection trust tier.
    pub allow_ssl_inspection: bool,
}

impl Default for WssTransportConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            listen_on_443: false,
            fallback_priority: 2,
            middlebox_pin_check: true,
            allow_ssl_inspection: false,
        }
    }
}

impl WssTransportConfig {
    /// Config preset for a project-operated public fallback relay.
    /// Listens on 443, keeps pin check on.
    pub fn for_relay() -> Self {
        Self { listen_on_443: true, ..Default::default() }
    }

    /// Config preset for a donor behind an SSL-inspecting firewall.
    /// Must be explicitly opted in via `--allow-ssl-inspection`.
    pub fn with_ssl_inspection_allowed() -> Self {
        Self {
            enabled: true,
            listen_on_443: false,
            fallback_priority: 2,
            middlebox_pin_check: false,
            allow_ssl_inspection: true,
        }
    }

    /// Returns true iff this transport configuration would downgrade the
    /// resulting connection's trust tier (SSL inspection allowed).
    pub fn produces_inspected_tier(&self) -> bool {
        self.allow_ssl_inspection
    }

    /// Expose the transport kind for telemetry / dial-logging.
    pub fn kind(&self) -> TransportKind {
        TransportKind::Wss
    }

    /// Validate invariant: SSL-inspection allowed requires middlebox_pin_check off.
    pub fn validate(&self) -> Result<(), String> {
        if self.allow_ssl_inspection && self.middlebox_pin_check {
            return Err("allow_ssl_inspection=true requires middlebox_pin_check=false \
                 (cannot both pin-check and allow inspection)"
                .into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_fallback_not_listener() {
        let cfg = WssTransportConfig::default();
        assert!(cfg.enabled);
        assert!(!cfg.listen_on_443);
        assert!(cfg.middlebox_pin_check);
        assert!(!cfg.allow_ssl_inspection);
        assert_eq!(cfg.kind(), TransportKind::Wss);
    }

    #[test]
    fn relay_preset_listens_on_443() {
        let cfg = WssTransportConfig::for_relay();
        assert!(cfg.listen_on_443);
        assert!(cfg.middlebox_pin_check);
    }

    #[test]
    fn ssl_inspection_preset_downgrades_tier() {
        let cfg = WssTransportConfig::with_ssl_inspection_allowed();
        assert!(cfg.produces_inspected_tier());
        assert!(!cfg.middlebox_pin_check);
        cfg.validate().expect("config should be valid");
    }

    #[test]
    fn invalid_combination_rejected() {
        let cfg = WssTransportConfig {
            enabled: true,
            listen_on_443: false,
            fallback_priority: 2,
            middlebox_pin_check: true,  // conflicts
            allow_ssl_inspection: true, // conflicts
        };
        assert!(cfg.validate().is_err());
    }
}
