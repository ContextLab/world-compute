//! NAT traversal configuration and status detection per FR-062 (T079).

/// Configuration for NAT traversal methods.
#[derive(Debug, Clone)]
pub struct NatConfig {
    /// Enable UPnP IGD port mapping (works on many home routers).
    pub upnp_enabled: bool,
    /// Enable Direct Connection Upgrade Through Relay (dcutr / hole-punching).
    pub dcutr_enabled: bool,
    /// Enable circuit relay v2 as a fallback when direct connection fails.
    pub relay_enabled: bool,
}

impl Default for NatConfig {
    fn default() -> Self {
        Self { upnp_enabled: true, dcutr_enabled: true, relay_enabled: true }
    }
}

/// NAT traversal status for a peer connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NatStatus {
    /// Direct TCP/UDP reachability — no NAT or fully open firewall.
    Direct,
    /// Hole-punching via dcutr succeeded.
    HolePunched,
    /// Reachable only via circuit relay (worst-case fallback).
    Relayed,
    /// Peer is unreachable via all methods.
    Unreachable,
}

/// Detect the NAT status for the local node.
///
/// This is a stub that returns `Direct`. Full detection requires an active
/// Swarm with AutoNAT behaviour and an observed external address — that
/// integration happens at the Swarm event loop level.
pub fn detect_nat_status() -> NatStatus {
    NatStatus::Direct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_all_methods_enabled() {
        let config = NatConfig::default();
        assert!(config.upnp_enabled, "UPnP should be enabled by default");
        assert!(config.dcutr_enabled, "dcutr should be enabled by default");
        assert!(config.relay_enabled, "Relay should be enabled by default");
    }

    #[test]
    fn detect_nat_status_returns_direct_stub() {
        assert_eq!(detect_nat_status(), NatStatus::Direct);
    }

    #[test]
    fn nat_status_variants_are_distinct() {
        assert_ne!(NatStatus::Direct, NatStatus::Relayed);
        assert_ne!(NatStatus::HolePunched, NatStatus::Unreachable);
    }
}
