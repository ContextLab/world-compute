//! Peer discovery — mDNS (LAN) + Kademlia DHT (WAN) per FR-060, FR-061 (T037-T038).
//!
//! On a LAN with no internet: mDNS discovers peers within 2 seconds.
//! On the internet: DNS bootstrap seeds → Kademlia DHT self-organization.
//! Both run simultaneously — the agent is always discovering peers on all
//! available channels.

use libp2p::{
    kad, mdns,
    swarm::NetworkBehaviour,
    PeerId,
};
use std::time::Duration;

/// Combined network behaviour for peer discovery.
/// mDNS for LAN (zero-config, <2s) and Kademlia for WAN.
#[derive(NetworkBehaviour)]
pub struct DiscoveryBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

/// Configuration for the discovery subsystem.
pub struct DiscoveryConfig {
    /// Enable mDNS for LAN peer discovery (default: true).
    pub mdns_enabled: bool,
    /// Enable Kademlia DHT for internet peer discovery (default: true).
    pub kademlia_enabled: bool,
    /// DNS bootstrap seed addresses for initial WAN contact.
    pub bootstrap_seeds: Vec<String>,
    /// Kademlia query timeout.
    pub kad_query_timeout: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            mdns_enabled: true,
            kademlia_enabled: true,
            bootstrap_seeds: vec![
                // TODO: Replace with real World Compute DNS seeds at launch.
                // These are placeholder seeds for development.
                "/dnsaddr/bootstrap1.worldcompute.org".into(),
                "/dnsaddr/bootstrap2.worldcompute.org".into(),
            ],
            kad_query_timeout: Duration::from_secs(30),
        }
    }
}

/// Create the discovery behaviour for a given local peer.
pub fn build_discovery_behaviour(
    local_peer_id: PeerId,
    config: &DiscoveryConfig,
) -> Result<DiscoveryBehaviour, Box<dyn std::error::Error>> {
    // mDNS: discovers peers on the local network via multicast DNS.
    // Fires DiscoveredEvent within ~1-2 seconds on most platforms.
    let mdns = mdns::tokio::Behaviour::new(
        mdns::Config::default(),
        local_peer_id,
    )?;

    // Kademlia: distributed hash table for WAN peer routing.
    // Nodes self-organize into a DHT; queries find peers by ID.
    let store = kad::store::MemoryStore::new(local_peer_id);
    let mut kademlia = kad::Behaviour::new(local_peer_id, store);

    // Set Kademlia to server mode so we both provide and consume records.
    kademlia.set_mode(Some(kad::Mode::Server));

    Ok(DiscoveryBehaviour { mdns, kademlia })
}

/// Bootstrap Kademlia by connecting to known seed peers.
/// Called once at agent startup when internet is available.
pub fn bootstrap_kademlia(
    kademlia: &mut kad::Behaviour<kad::store::MemoryStore>,
    seeds: &[String],
) {
    for seed in seeds {
        if let Ok(addr) = seed.parse() {
            kademlia.add_address(&PeerId::random(), addr);
        }
    }
    if let Err(e) = kademlia.bootstrap() {
        tracing::warn!("Kademlia bootstrap failed (may be offline): {e}");
    }
}

/// Count of currently known peers across both discovery methods.
pub struct PeerCounts {
    pub mdns_peers: usize,
    pub kademlia_peers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity;

    #[test]
    fn discovery_config_has_sane_defaults() {
        let config = DiscoveryConfig::default();
        assert!(config.mdns_enabled);
        assert!(config.kademlia_enabled);
        assert!(!config.bootstrap_seeds.is_empty());
    }

    #[test]
    fn build_discovery_behaviour_succeeds() {
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let config = DiscoveryConfig::default();
        let behaviour = build_discovery_behaviour(peer_id, &config);
        assert!(behaviour.is_ok());
    }
}
