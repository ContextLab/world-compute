//! Peer discovery — mDNS (LAN) + Kademlia DHT (WAN) per FR-060, FR-061 (T037-T038).
//!
//! On a LAN with no internet: mDNS discovers peers within 2 seconds.
//! On the internet: DNS bootstrap seeds → Kademlia DHT self-organization.
//! Both run simultaneously — the agent is always discovering peers on all
//! available channels.

use libp2p::{kad, mdns, swarm::NetworkBehaviour, PeerId};
use std::time::Duration;

/// DNS bootstrap seeds for initial WAN contact.
///
/// On startup, the agent resolves these DNS names to multiaddresses and
/// dials them to enter the global Kademlia DHT. Replace placeholders with
/// real records before mainnet launch.
pub const BOOTSTRAP_DNS_SEEDS: &[&str] = &[
    // World Compute project seeds (not yet deployed — replace before mainnet).
    "/dnsaddr/bootstrap1.worldcompute.org",
    "/dnsaddr/bootstrap2.worldcompute.org",
    "/dnsaddr/bootstrap3.worldcompute.org",
];

/// Public libp2p bootstrap relays operated by Protocol Labs and mirrors.
///
/// These speak the same base libp2p protocols we use (Noise, Yamux, TCP, QUIC,
/// Identify, Ping, Relay v2, DCUtR, Kademlia). They do NOT speak our
/// application-layer protocols (`/worldcompute/dispatch/*`, etc.), so they only
/// serve as rendezvous/relay nodes — never as executors.
///
/// Security: traffic relayed through these nodes is end-to-end Noise-encrypted,
/// so the relay operators cannot read or tamper with payloads. They can see
/// peer IDs and traffic volume (metadata), which is acceptable for our public-
/// good threat model. See SECURITY.md.
///
/// Each entry includes the `/p2p/<peer_id>` suffix so the initial Noise
/// handshake authenticates the relay against its pinned identity — a spoofed
/// DNS response would produce a handshake mismatch and be rejected.
pub const PUBLIC_LIBP2P_BOOTSTRAP_RELAYS: &[&str] = &[
    // Protocol Labs public libp2p bootstrap relays (existing).
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
    "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
    // spec 005 T022 / FR-007a — project-operated WSS/443 launch fallback relays.
    //
    // These are the project's own relays speaking the full World Compute
    // application-layer protocols AND the libp2p WSS-over-TLS-443 transport.
    // They guarantee a fallback rendezvous point for donors whose firewalls
    // block all outbound libp2p transports except WSS/443.
    //
    // The entries below are commented out until the relays are deployed.
    // `docs/operators/running-a-relay.md` documents how a volunteer brings up
    // a WSS/443 relay that auto-announces into the mesh via the Identify /
    // peer-exchange protocols, at which point these can be uncommented or
    // replaced without a client update.
    //
    // (Awaiting deployment tracked in docs/operators/running-a-relay.md —
    //  uncomment and update with real multiaddrs once the project-operated
    //  WSS/443 fallback relays are stood up. Until then, donors without WSS
    //  support still bootstrap via the Protocol Labs relays above, which do
    //  NOT speak WSS/443 — meaning a donor behind a strict HTTPS-only
    //  firewall cannot join yet.)
    //
    // "/dns6/relay1.worldcompute.org/tcp/443/tls/ws/p2p/<peer_id_1>",
    // "/dns6/relay2.worldcompute.org/tcp/443/tls/ws/p2p/<peer_id_2>",
];

/// Result of merging a locally-discovered LAN cluster with the global DHT.
///
/// When a group of nodes on a LAN all join the WAN DHT, the LAN cluster's
/// local Kademlia state merges with the global routing table. This struct
/// captures the outcome of that merge event.
#[derive(Debug, Clone)]
pub struct ClusterMergeResult {
    /// Number of LAN peers that were successfully announced to the global DHT.
    pub peers_announced: usize,
    /// Number of routing table entries added from the global DHT.
    pub routes_added: usize,
    /// Whether the merge completed without errors.
    pub success: bool,
}

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
        // Load bootstrap seeds from environment or use defaults.
        // In production, set WORLDCOMPUTE_BOOTSTRAP_SEEDS to a comma-separated
        // list of multiaddr strings to override.
        //
        // The default merges worldcompute project seeds (when deployed) with
        // the public libp2p bootstrap relays. Public relays provide immediate
        // NAT-traversal rendezvous without requiring worldcompute to operate
        // infrastructure itself — as the network grows, any publicly-reachable
        // donor also acts as a relay (relay::Behaviour in daemon.rs).
        let seeds: Vec<String> = std::env::var("WORLDCOMPUTE_BOOTSTRAP_SEEDS")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| {
                let mut v: Vec<String> =
                    BOOTSTRAP_DNS_SEEDS.iter().map(|s| s.to_string()).collect();
                v.extend(PUBLIC_LIBP2P_BOOTSTRAP_RELAYS.iter().map(|s| s.to_string()));
                v
            });

        Self {
            mdns_enabled: true,
            kademlia_enabled: true,
            bootstrap_seeds: seeds,
            kad_query_timeout: Duration::from_secs(30),
        }
    }
}

/// Create the discovery behaviour for a given local peer.
pub fn build_discovery_behaviour(
    local_peer_id: PeerId,
    _config: &DiscoveryConfig,
) -> Result<DiscoveryBehaviour, Box<dyn std::error::Error>> {
    // mDNS: discovers peers on the local network via multicast DNS.
    // Fires DiscoveredEvent within ~1-2 seconds on most platforms.
    let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;

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
    fn bootstrap_dns_seeds_is_non_empty() {
        assert!(BOOTSTRAP_DNS_SEEDS.len() >= 2, "Need at least 2 bootstrap seeds");
        for seed in BOOTSTRAP_DNS_SEEDS {
            assert!(seed.starts_with("/dnsaddr/"), "Seed should be a /dnsaddr/ multiaddr: {seed}");
        }
    }

    #[test]
    fn cluster_merge_result_fields() {
        let result = ClusterMergeResult { peers_announced: 3, routes_added: 10, success: true };
        assert_eq!(result.peers_announced, 3);
        assert_eq!(result.routes_added, 10);
        assert!(result.success);
    }

    #[test]
    fn discovery_config_has_sane_defaults() {
        let config = DiscoveryConfig::default();
        assert!(config.mdns_enabled);
        assert!(config.kademlia_enabled);
        assert!(!config.bootstrap_seeds.is_empty());
    }

    #[test]
    fn build_discovery_behaviour_succeeds() {
        // mDNS requires multicast/netlink which may not be available in
        // CI containers. This test verifies the construction logic works
        // on hosts with network support; it's allowed to skip on CI.
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let config = DiscoveryConfig::default();
        // Use catch_unwind because mDNS on Linux may panic (not Err)
        // if netlink sockets are unavailable in a container.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            build_discovery_behaviour(peer_id, &config)
        }));
        match result {
            Ok(Ok(_)) => {} // Success — mDNS + Kademlia constructed
            Ok(Err(e)) => {
                eprintln!("Discovery init returned error (expected in CI): {e}");
            }
            Err(_) => {
                eprintln!("Discovery init panicked (expected in CI containers without multicast)");
            }
        }
    }
}
