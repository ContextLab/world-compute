//! T027 [US1]: RFC1918/link-local/metadata endpoints blocked.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use worldcompute::sandbox::egress::is_blocked_destination;

#[test]
fn rfc1918_10_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
}

#[test]
fn rfc1918_172_16_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
    // 172.15 and 172.32 should NOT be blocked
    assert!(!is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(172, 15, 0, 1))));
    assert!(!is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));
}

#[test]
fn rfc1918_192_168_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));
}

#[test]
fn cloud_metadata_169_254_169_254_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
}

#[test]
fn link_local_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 255, 254))));
}

#[test]
fn loopback_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    assert!(is_blocked_destination(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
}

#[test]
fn multicast_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(239, 255, 255, 255))));
}

#[test]
fn broadcast_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))));
}

#[test]
fn public_ips_allowed() {
    assert!(!is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    assert!(!is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    assert!(!is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(142, 250, 80, 46))));
}
