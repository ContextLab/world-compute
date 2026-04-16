//! T026 [US1]: ARP/mDNS discovery packets blocked.
//!
//! ARP and mDNS use multicast/broadcast addresses which are blocked
//! by the egress filter.

use std::net::{IpAddr, Ipv4Addr};
use worldcompute::sandbox::egress::is_blocked_destination;

#[test]
fn mdns_multicast_address_blocked() {
    // mDNS uses 224.0.0.251
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(224, 0, 0, 251))));
}

#[test]
fn broadcast_address_blocked() {
    // ARP uses broadcast 255.255.255.255
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))));
}

#[test]
fn ssdp_multicast_blocked() {
    // SSDP/UPnP uses 239.255.255.250
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250))));
}

#[test]
fn all_multicast_range_blocked() {
    // Entire 224.0.0.0/4 range
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(224, 0, 0, 0))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(239, 255, 255, 255))));
}
