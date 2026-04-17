//! T069-T070: Integration tests for NAT detection and DNS seed configuration.
//!
//! T069: classify_nat_type with various address patterns.
//! T070: DiscoveryConfig::default() returns valid seed addresses.

use worldcompute::network::discovery::{DiscoveryConfig, BOOTSTRAP_DNS_SEEDS};
use worldcompute::network::nat::{detect_nat_status_with_config, NatConfig, NatStatus};

// --- T069: NAT detection ---

#[test]
fn nat_detection_with_no_stun_servers_returns_unknown() {
    let config = NatConfig { stun_servers: vec![], ..NatConfig::default() };
    assert_eq!(detect_nat_status_with_config(&config), NatStatus::Unknown);
}

#[test]
fn nat_detection_with_unreachable_stun_returns_unknown() {
    // Point at a non-routable STUN server — should return Unknown, not panic.
    let config = NatConfig { stun_servers: vec!["127.0.0.1:1".into()], ..NatConfig::default() };
    let status = detect_nat_status_with_config(&config);
    // Will be Unknown because the STUN binding request will fail/timeout
    assert_eq!(status, NatStatus::Unknown);
}

#[test]
fn nat_config_default_has_known_stun_servers() {
    let config = NatConfig::default();
    assert!(config.stun_servers.len() >= 2);
    assert!(config.stun_servers.iter().any(|s| s.contains("google")));
    assert!(config.stun_servers.iter().any(|s| s.contains("cloudflare")));
}

#[test]
fn nat_status_all_variants_are_distinct() {
    let variants = [
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
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "Variants at index {i} and {j} should differ");
            }
        }
    }
}

// --- T070: DNS seed config ---

#[test]
fn discovery_config_default_returns_valid_seeds() {
    let config = DiscoveryConfig::default();
    assert!(config.mdns_enabled, "mDNS should be on by default");
    assert!(config.kademlia_enabled, "Kademlia should be on by default");
    assert!(!config.bootstrap_seeds.is_empty(), "Default seeds must be non-empty");
    for seed in &config.bootstrap_seeds {
        assert!(seed.starts_with("/dnsaddr/"), "Seed should be a /dnsaddr/ multiaddr, got: {seed}");
        assert!(seed.contains("worldcompute"), "Seed should reference worldcompute domain: {seed}");
    }
}

#[test]
fn bootstrap_dns_seeds_constant_matches_config() {
    let config = DiscoveryConfig::default();
    assert_eq!(
        config.bootstrap_seeds.len(),
        BOOTSTRAP_DNS_SEEDS.len(),
        "Config seeds and constant seeds should match in count"
    );
    for (i, seed) in BOOTSTRAP_DNS_SEEDS.iter().enumerate() {
        assert_eq!(
            *seed,
            config.bootstrap_seeds[i].as_str(),
            "Seed {i} mismatch between constant and config default"
        );
    }
}

#[test]
fn discovery_config_query_timeout_is_reasonable() {
    let config = DiscoveryConfig::default();
    assert!(config.kad_query_timeout.as_secs() >= 5, "Kademlia timeout should be at least 5s");
    assert!(
        config.kad_query_timeout.as_secs() <= 120,
        "Kademlia timeout should not exceed 2 minutes"
    );
}
