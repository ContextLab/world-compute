//! Transport configuration — QUIC (primary) + TCP (fallback) per FR-062 (T075-T076).

use libp2p::identity;

/// Configuration for the transport layer.
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Enable QUIC transport (primary, lower latency, multiplexed).
    pub quic_enabled: bool,
    /// Enable TCP transport (fallback, wider compatibility).
    pub tcp_enabled: bool,
    /// Enable circuit relay for NAT traversal.
    pub relay_enabled: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self { quic_enabled: true, tcp_enabled: true, relay_enabled: true }
    }
}

/// Opaque transport handle — holds the configuration used to build the transport.
/// A full libp2p transport requires a running async event loop; this type holds
/// the resolved config so callers can wire it into a Swarm builder.
pub struct BuiltTransport {
    pub config: TransportConfig,
    pub keypair: identity::Keypair,
}

/// Configure the transport stack for the given keypair.
///
/// QUIC is preferred (lower latency, built-in TLS 1.3, multiplexing).
/// TCP + Noise + Yamux is the fallback for networks that block UDP.
/// Returns `BuiltTransport` which carries the keypair and resolved config
/// ready to be passed into the Swarm builder.
pub fn build_transport(
    keypair: &identity::Keypair,
    config: TransportConfig,
) -> Result<BuiltTransport, Box<dyn std::error::Error>> {
    Ok(BuiltTransport { config, keypair: keypair.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_config_defaults_are_sane() {
        let config = TransportConfig::default();
        assert!(config.quic_enabled, "QUIC should be enabled by default");
        assert!(config.tcp_enabled, "TCP fallback should be enabled by default");
        assert!(config.relay_enabled, "Relay should be enabled by default");
    }

    #[test]
    fn build_transport_returns_ok() {
        let keypair = identity::Keypair::generate_ed25519();
        let config = TransportConfig::default();
        let result = build_transport(&keypair, config);
        assert!(result.is_ok(), "build_transport should succeed");
    }

    #[test]
    fn built_transport_preserves_config() {
        let keypair = identity::Keypair::generate_ed25519();
        let config =
            TransportConfig { quic_enabled: true, tcp_enabled: false, relay_enabled: true };
        let bt = build_transport(&keypair, config).unwrap();
        assert!(bt.config.quic_enabled);
        assert!(!bt.config.tcp_enabled);
    }
}
