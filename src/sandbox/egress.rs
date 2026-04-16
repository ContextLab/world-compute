//! Network egress enforcement — per-sandbox firewall rules.
//!
//! Per FR-S002/FR-S020: default-deny all outbound traffic from sandboxes.
//! Per FR-S022: block RFC1918, link-local, cloud metadata, donor LAN.
//! Per FR-S021: only declared+approved endpoints pass the firewall.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// An approved egress endpoint that a job is allowed to contact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedEndpoint {
    pub host: String,
    pub port: u16,
    pub protocol: EgressProtocol,
}

/// Supported egress protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EgressProtocol {
    Https,
    Http,
}

/// Egress policy for a sandbox instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressPolicy {
    /// Whether any egress is allowed (false = default-deny).
    pub egress_allowed: bool,
    /// Approved endpoints if egress is allowed.
    pub approved_endpoints: Vec<ApprovedEndpoint>,
    /// Maximum egress bytes (from ResourceEnvelope).
    pub max_egress_bytes: u64,
}

impl EgressPolicy {
    /// Create a default-deny policy (no egress).
    pub fn deny_all() -> Self {
        Self {
            egress_allowed: false,
            approved_endpoints: Vec::new(),
            max_egress_bytes: 0,
        }
    }

    /// Create a policy allowing specific endpoints.
    pub fn allow_endpoints(endpoints: Vec<ApprovedEndpoint>, max_bytes: u64) -> Self {
        Self {
            egress_allowed: !endpoints.is_empty(),
            approved_endpoints: endpoints,
            max_egress_bytes: max_bytes,
        }
    }
}

/// Check if an IP address is in a blocked range per FR-S022.
///
/// Blocked ranges: RFC1918 private, link-local, loopback, cloud metadata,
/// multicast, broadcast.
pub fn is_blocked_destination(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            // RFC1918 private ranges
            if octets[0] == 10 {
                return true;
            }
            if octets[0] == 172 && (16..=31).contains(&octets[1]) {
                return true;
            }
            if octets[0] == 192 && octets[1] == 168 {
                return true;
            }
            // Loopback
            if octets[0] == 127 {
                return true;
            }
            // Link-local
            if octets[0] == 169 && octets[1] == 254 {
                return true;
            }
            // Cloud metadata endpoint (169.254.169.254)
            if octets == [169, 254, 169, 254] {
                return true;
            }
            // Multicast
            if (224..=239).contains(&octets[0]) {
                return true;
            }
            // Broadcast
            if octets == [255, 255, 255, 255] {
                return true;
            }
            false
        }
        IpAddr::V6(v6) => {
            // Loopback (::1)
            if v6.is_loopback() {
                return true;
            }
            // Link-local (fe80::/10)
            let segments = v6.segments();
            if segments[0] & 0xffc0 == 0xfe80 {
                return true;
            }
            // Multicast (ff00::/8)
            if segments[0] & 0xff00 == 0xff00 {
                return true;
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn rfc1918_blocked() {
        assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    }

    #[test]
    fn loopback_blocked() {
        assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_blocked_destination(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn cloud_metadata_blocked() {
        assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
    }

    #[test]
    fn link_local_blocked() {
        assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
    }

    #[test]
    fn public_ip_allowed() {
        assert!(!is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }

    #[test]
    fn default_deny_policy() {
        let policy = EgressPolicy::deny_all();
        assert!(!policy.egress_allowed);
        assert!(policy.approved_endpoints.is_empty());
    }
}
