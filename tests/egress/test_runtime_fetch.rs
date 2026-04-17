//! T027a [US1]: Attempt pip install, curl, secondary payload download
//! from within sandbox — all must fail per FR-S023.
//!
//! Since we can't run real pip/curl inside a test, we verify that the
//! egress policy would block all outbound connections that these tools need.

use std::net::{IpAddr, Ipv4Addr};
use worldcompute::sandbox::egress::{is_blocked_destination, EgressPolicy};

#[test]
fn pypi_server_blocked_by_default_deny() {
    // pip install connects to pypi.org (151.101.0.223)
    // Under default-deny, even public IPs are blocked because
    // egress_allowed = false means no outbound connections at all
    let policy = EgressPolicy::deny_all();
    assert!(!policy.egress_allowed, "Default deny blocks all outbound");
    assert_eq!(policy.max_egress_bytes, 0);
}

#[test]
fn curl_to_any_host_blocked_by_default_deny() {
    let policy = EgressPolicy::deny_all();
    assert!(!policy.egress_allowed);
}

#[test]
fn secondary_payload_download_blocked() {
    // Even if a workload tries to reach a public IP, the sandbox
    // network namespace has no route out under default-deny
    let policy = EgressPolicy::deny_all();
    assert_eq!(policy.approved_endpoints.len(), 0);
    assert!(!policy.egress_allowed);
}

#[test]
fn private_package_registries_also_blocked() {
    // Internal registries on private IPs are doubly blocked
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(10, 0, 1, 50))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))));
}
