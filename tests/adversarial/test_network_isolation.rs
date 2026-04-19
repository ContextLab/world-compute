//! Adversarial test: network isolation — workload cannot reach host or private networks.
//!
//! T079: network_escape_via_host_bridge — verify RFC1918 and metadata ranges blocked
//! T080: network_escape_via_dns_intercept — verify DNS resolution controls

use std::net::{IpAddr, Ipv4Addr};
use worldcompute::sandbox::egress::{is_blocked_destination, EgressPolicy};

/// T079: Verify that egress rules block all private/RFC1918 ranges and cloud metadata.
///
/// Tests that the egress filter correctly identifies and blocks:
/// - 10.0.0.0/8 (RFC1918 Class A private)
/// - 172.16.0.0/12 (RFC1918 Class B private)
/// - 192.168.0.0/16 (RFC1918 Class C private)
/// - 169.254.169.254 (cloud metadata endpoint)
/// - 127.0.0.0/8 (loopback)
/// - 169.254.0.0/16 (link-local)
/// - 224.0.0.0/4 (multicast)
/// - 255.255.255.255 (broadcast)
#[test]
fn network_escape_via_host_bridge() {
    // RFC1918 Class A: 10.0.0.0/8
    let rfc1918_a = [
        Ipv4Addr::new(10, 0, 0, 1),
        Ipv4Addr::new(10, 255, 255, 255),
        Ipv4Addr::new(10, 100, 50, 25),
    ];
    for addr in &rfc1918_a {
        assert!(
            is_blocked_destination(&IpAddr::V4(*addr)),
            "10.x.x.x ({addr}) must be blocked (RFC1918 Class A)"
        );
    }

    // RFC1918 Class B: 172.16.0.0/12
    let rfc1918_b = [
        Ipv4Addr::new(172, 16, 0, 1),
        Ipv4Addr::new(172, 31, 255, 255),
        Ipv4Addr::new(172, 20, 10, 5),
    ];
    for addr in &rfc1918_b {
        assert!(
            is_blocked_destination(&IpAddr::V4(*addr)),
            "172.16-31.x.x ({addr}) must be blocked (RFC1918 Class B)"
        );
    }

    // RFC1918 Class C: 192.168.0.0/16
    let rfc1918_c = [
        Ipv4Addr::new(192, 168, 0, 1),
        Ipv4Addr::new(192, 168, 255, 255),
        Ipv4Addr::new(192, 168, 1, 100),
    ];
    for addr in &rfc1918_c {
        assert!(
            is_blocked_destination(&IpAddr::V4(*addr)),
            "192.168.x.x ({addr}) must be blocked (RFC1918 Class C)"
        );
    }

    // Cloud metadata endpoint: 169.254.169.254
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))),
        "169.254.169.254 must be blocked (cloud metadata endpoint)"
    );

    // Loopback: 127.0.0.0/8
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        "127.0.0.1 must be blocked (loopback)"
    );
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))),
        "127.255.255.255 must be blocked (loopback)"
    );

    // Link-local: 169.254.0.0/16
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))),
        "169.254.1.1 must be blocked (link-local)"
    );

    // Multicast: 224.0.0.0/4
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1))),
        "224.0.0.1 must be blocked (multicast)"
    );
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(239, 255, 255, 255))),
        "239.255.255.255 must be blocked (multicast)"
    );

    // Broadcast
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))),
        "255.255.255.255 must be blocked (broadcast)"
    );

    // Verify public IPs are NOT blocked (positive control)
    let public_addrs = [
        Ipv4Addr::new(8, 8, 8, 8),       // Google DNS
        Ipv4Addr::new(1, 1, 1, 1),       // Cloudflare DNS
        Ipv4Addr::new(93, 184, 216, 34), // example.com
        Ipv4Addr::new(204, 13, 164, 50), // arbitrary public IP
    ];
    for addr in &public_addrs {
        assert!(
            !is_blocked_destination(&IpAddr::V4(*addr)),
            "Public IP {addr} must NOT be blocked"
        );
    }

    // Verify edge cases at RFC1918 boundaries
    // 172.15.x.x is NOT private (just below 172.16.0.0/12)
    assert!(
        !is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(172, 15, 255, 255))),
        "172.15.255.255 is NOT RFC1918 and must be allowed"
    );
    // 172.32.x.x is NOT private (just above 172.31.255.255)
    assert!(
        !is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))),
        "172.32.0.1 is NOT RFC1918 and must be allowed"
    );
}

/// T080: Verify DNS resolution goes through approved channels only.
///
/// Since we cannot intercept actual DNS queries in a unit-style test, we verify
/// the policy and configuration controls that enforce DNS isolation:
/// 1. Default egress policy is deny-all (blocks all DNS).
/// 2. If endpoints are explicitly allowed, only those pass.
/// 3. Standard DNS port (53) traffic to private IPs is blocked.
/// 4. Non-standard DNS ports to any address are blocked by default-deny.
#[test]
fn network_escape_via_dns_intercept() {
    // 1. Default-deny policy blocks ALL outbound traffic including DNS
    let deny_policy = EgressPolicy::deny_all();
    assert!(!deny_policy.egress_allowed, "Default policy must block all egress including DNS");
    assert!(deny_policy.approved_endpoints.is_empty(), "No endpoints should be pre-approved");
    assert_eq!(deny_policy.max_egress_bytes, 0, "Zero egress bytes in deny-all mode");

    // 2. Explicitly allowing specific endpoints does NOT include DNS servers
    use worldcompute::sandbox::egress::{ApprovedEndpoint, EgressProtocol};
    let allowed = EgressPolicy::allow_endpoints(
        vec![ApprovedEndpoint {
            host: "api.example.com".to_string(),
            port: 443,
            protocol: EgressProtocol::Https,
        }],
        1_000_000,
    );
    assert!(allowed.egress_allowed, "Policy with endpoints should allow egress");
    assert_eq!(allowed.approved_endpoints.len(), 1);
    // The approved endpoint is HTTPS on 443, not DNS on 53
    assert_eq!(allowed.approved_endpoints[0].port, 443);
    assert_ne!(allowed.approved_endpoints[0].port, 53, "DNS port should not be in approved list");

    // 3. DNS servers at private IPs are blocked by the egress filter
    // Common private DNS: 10.0.0.2, 192.168.1.1, 172.16.0.1
    let private_dns_servers =
        [Ipv4Addr::new(10, 0, 0, 2), Ipv4Addr::new(192, 168, 1, 1), Ipv4Addr::new(172, 16, 0, 1)];
    for dns_ip in &private_dns_servers {
        assert!(
            is_blocked_destination(&IpAddr::V4(*dns_ip)),
            "Private DNS server at {dns_ip} must be blocked"
        );
    }

    // 4. Cloud metadata DNS (169.254.169.253 on some clouds) is also blocked
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 253))),
        "Cloud metadata DNS (169.254.169.253) must be blocked"
    );

    // 5. Loopback DNS (127.0.0.53 — systemd-resolved) is blocked
    assert!(
        is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 53))),
        "Loopback DNS (127.0.0.53) must be blocked"
    );

    // 6. Public DNS servers are not blocked at IP level (but still blocked by
    //    default-deny egress policy at the sandbox level)
    assert!(
        !is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))),
        "Public DNS IP is not blocked at IP level (blocked by egress policy instead)"
    );
}
