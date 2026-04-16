//! T023 [US1]: Outbound connection from sandbox must be refused.
//!
//! Verifies that the default-deny egress policy blocks all outbound traffic.

use worldcompute::sandbox::egress::EgressPolicy;

#[test]
fn default_deny_policy_blocks_all_egress() {
    let policy = EgressPolicy::deny_all();
    assert!(!policy.egress_allowed);
    assert!(policy.approved_endpoints.is_empty());
    assert_eq!(policy.max_egress_bytes, 0);
}

#[test]
fn default_deny_is_the_default_for_firecracker() {
    use worldcompute::sandbox::firecracker::FirecrackerConfig;
    let config = FirecrackerConfig::default();
    assert!(!config.egress_policy.egress_allowed);
}

#[test]
fn default_deny_is_the_default_for_apple_vf() {
    use worldcompute::sandbox::apple_vf::AppleVfConfig;
    let config = AppleVfConfig::default();
    assert!(!config.egress_policy.egress_allowed);
}

#[test]
fn default_deny_is_the_default_for_hyperv() {
    use worldcompute::sandbox::hyperv::HyperVConfig;
    let config = HyperVConfig::default();
    assert!(!config.egress_policy.egress_allowed);
}
