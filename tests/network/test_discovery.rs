//! Integration tests for network discovery types (T106).

use worldcompute::network::discovery::{ClusterMergeResult, DiscoveryConfig, BOOTSTRAP_DNS_SEEDS};
use worldcompute::network::nat::{NatConfig, NatStatus};

#[test]
fn peer_record_creation_cluster_merge() {
    let result = ClusterMergeResult { peers_announced: 5, routes_added: 12, success: true };
    assert_eq!(result.peers_announced, 5);
    assert_eq!(result.routes_added, 12);
    assert!(result.success);
}

#[test]
fn dns_seed_parsing() {
    assert!(BOOTSTRAP_DNS_SEEDS.len() >= 2);
    for seed in BOOTSTRAP_DNS_SEEDS {
        assert!(
            seed.starts_with("/dnsaddr/"),
            "Bootstrap seed must be /dnsaddr/ multiaddr: {seed}"
        );
    }
    // DiscoveryConfig also picks up seeds
    let config = DiscoveryConfig::default();
    assert!(!config.bootstrap_seeds.is_empty());
    for seed in &config.bootstrap_seeds {
        assert!(seed.starts_with("/dnsaddr/"));
    }
}

#[test]
fn nat_type_classification_variants() {
    // All NAT status variants should be distinct
    let statuses = [
        NatStatus::Direct,
        NatStatus::FullCone,
        NatStatus::RestrictedCone,
        NatStatus::PortRestricted,
        NatStatus::Symmetric,
        NatStatus::HolePunched,
        NatStatus::Relayed,
        NatStatus::Unreachable,
        NatStatus::Unknown,
    ];
    for i in 0..statuses.len() {
        for j in (i + 1)..statuses.len() {
            assert_ne!(statuses[i], statuses[j]);
        }
    }
}

#[test]
fn nat_config_defaults() {
    let config = NatConfig::default();
    assert!(config.upnp_enabled);
    assert!(config.dcutr_enabled);
    assert!(config.relay_enabled);
    assert!(!config.stun_servers.is_empty());
}
