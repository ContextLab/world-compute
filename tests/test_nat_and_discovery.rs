//! T069-T070: Integration tests for NAT detection and DNS seed configuration.
//!
//! T069: classify_nat_type with various address patterns.
//! T070: DiscoveryConfig::default() returns valid seed addresses.

use worldcompute::network::discovery::{
    DiscoveryConfig, BOOTSTRAP_DNS_SEEDS, PUBLIC_LIBP2P_BOOTSTRAP_RELAYS,
};
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
    // Seeds may be either /dnsaddr/, /ip4/, or /ip6/ (public libp2p bootstrap
    // relays are included as the default fallback rendezvous until the
    // worldcompute project runs its own bootstrap servers).
    for seed in &config.bootstrap_seeds {
        assert!(
            seed.starts_with("/dnsaddr/") || seed.starts_with("/ip4/") || seed.starts_with("/ip6/"),
            "Seed should be a /dnsaddr/, /ip4/, or /ip6/ multiaddr: {seed}"
        );
    }
    // At least one seed should reference worldcompute or a known public relay.
    let has_wc = config.bootstrap_seeds.iter().any(|s| s.contains("worldcompute"));
    let has_public = config.bootstrap_seeds.iter().any(|s| s.contains("bootstrap.libp2p.io"));
    assert!(
        has_wc || has_public,
        "Expected worldcompute or public libp2p bootstrap seeds in defaults"
    );
}

#[test]
fn bootstrap_dns_seeds_constant_matches_config_prefix() {
    // The default config seeds should start with all worldcompute project
    // seeds (BOOTSTRAP_DNS_SEEDS) followed by the public libp2p bootstrap
    // relays (PUBLIC_LIBP2P_BOOTSTRAP_RELAYS).
    let config = DiscoveryConfig::default();
    let expected_total = BOOTSTRAP_DNS_SEEDS.len() + PUBLIC_LIBP2P_BOOTSTRAP_RELAYS.len();
    assert_eq!(
        config.bootstrap_seeds.len(),
        expected_total,
        "Config seeds should be project seeds + public libp2p relays"
    );
    // Project seeds come first.
    for (i, seed) in BOOTSTRAP_DNS_SEEDS.iter().enumerate() {
        assert_eq!(
            *seed,
            config.bootstrap_seeds[i].as_str(),
            "Project seed {i} mismatch"
        );
    }
    // Public libp2p relays follow.
    for (i, seed) in PUBLIC_LIBP2P_BOOTSTRAP_RELAYS.iter().enumerate() {
        let config_idx = BOOTSTRAP_DNS_SEEDS.len() + i;
        assert_eq!(
            *seed,
            config.bootstrap_seeds[config_idx].as_str(),
            "Public relay {i} mismatch"
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
