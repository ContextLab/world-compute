//! Red Team Scenario 4: Sandbox escape attempt.
//!
//! Attack: Attempt to reach host resources from within the sandbox —
//! filesystem, network, LAN, metadata endpoints.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use worldcompute::sandbox::egress::is_blocked_destination;
use worldcompute::sandbox::Sandbox;

#[test]
fn attack_4a_host_filesystem_inaccessible_after_cleanup() {
    use worldcompute::sandbox::firecracker::FirecrackerSandbox;
    let tmp = std::env::temp_dir().join("wc-redteam-escape");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("host-secret.txt"), b"sensitive data").unwrap();

    let mut sandbox = FirecrackerSandbox::new(tmp.clone());
    sandbox.cleanup().unwrap();
    assert!(!tmp.exists(), "Host files must be completely removed — no residue");
}

#[test]
fn attack_4b_lan_scanning_blocked() {
    // Common LAN ranges an attacker would scan
    let lan_targets = [
        Ipv4Addr::new(192, 168, 1, 1),     // common router
        Ipv4Addr::new(10, 0, 0, 1),        // corporate gateway
        Ipv4Addr::new(172, 16, 0, 1),      // Docker default
        Ipv4Addr::new(169, 254, 169, 254), // cloud metadata
        Ipv4Addr::new(127, 0, 0, 1),       // localhost
    ];
    for target in &lan_targets {
        assert!(
            is_blocked_destination(&IpAddr::V4(*target)),
            "LAN target {target} must be blocked"
        );
    }
}

#[test]
fn attack_4c_ipv6_escape_routes_blocked() {
    assert!(is_blocked_destination(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
    // Link-local fe80::
    let link_local = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    assert!(is_blocked_destination(&IpAddr::V6(link_local)));
    // Multicast ff02::1
    let multicast = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 1);
    assert!(is_blocked_destination(&IpAddr::V6(multicast)));
}

#[test]
fn attack_4d_cloud_metadata_theft_blocked() {
    // AWS, GCP, Azure all use 169.254.169.254
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
    // Link-local range entirely
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
}

#[test]
fn attack_4e_broadcast_multicast_discovery_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(224, 0, 0, 251)))); // mDNS
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250))));
    // SSDP
}

#[test]
fn attack_4f_egress_policy_default_is_deny() {
    use worldcompute::sandbox::apple_vf::AppleVfConfig;
    use worldcompute::sandbox::firecracker::FirecrackerConfig;
    use worldcompute::sandbox::hyperv::HyperVConfig;

    assert!(!FirecrackerConfig::default().egress_policy.egress_allowed);
    assert!(!AppleVfConfig::default().egress_policy.egress_allowed);
    assert!(!HyperVConfig::default().egress_policy.egress_allowed);
}
