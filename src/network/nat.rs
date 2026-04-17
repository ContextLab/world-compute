//! NAT traversal configuration and status detection per FR-062 (T079).

/// Configuration for NAT traversal methods.
#[derive(Debug, Clone)]
pub struct NatConfig {
    /// Enable UPnP IGD port mapping (works on many home routers).
    pub upnp_enabled: bool,
    /// Enable Direct Connection Upgrade Through Relay (dcutr / hole-punching).
    pub dcutr_enabled: bool,
    /// Enable circuit relay v2 as a fallback when direct connection fails.
    pub relay_enabled: bool,
    /// STUN server addresses for external address discovery.
    pub stun_servers: Vec<String>,
}

impl Default for NatConfig {
    fn default() -> Self {
        Self {
            upnp_enabled: true,
            dcutr_enabled: true,
            relay_enabled: true,
            stun_servers: vec!["stun.l.google.com:19302".into(), "stun.cloudflare.com:3478".into()],
        }
    }
}

/// NAT traversal status for a peer connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NatStatus {
    /// Direct TCP/UDP reachability — no NAT or fully open firewall.
    Direct,
    /// Full cone NAT — any external host can send to the mapped port.
    FullCone,
    /// Restricted cone NAT — only hosts the internal host has sent to can reply.
    RestrictedCone,
    /// Port-restricted cone NAT — restricted by both IP and port.
    PortRestricted,
    /// Symmetric NAT — different mapping for each destination.
    Symmetric,
    /// Hole-punching via dcutr succeeded.
    HolePunched,
    /// Reachable only via circuit relay (worst-case fallback).
    Relayed,
    /// Peer is unreachable via all methods.
    Unreachable,
    /// NAT type could not be determined.
    Unknown,
}

/// Detect the NAT status for the local node using STUN.
///
/// Sends a STUN binding request to the configured STUN servers to discover
/// the external address. Compares mapped addresses across multiple servers
/// to classify the NAT type.
///
/// Falls back to `Unknown` if no STUN servers are reachable.
pub fn detect_nat_status() -> NatStatus {
    detect_nat_status_with_config(&NatConfig::default())
}

/// Detect NAT status using the provided configuration.
pub fn detect_nat_status_with_config(config: &NatConfig) -> NatStatus {
    if config.stun_servers.is_empty() {
        return NatStatus::Unknown;
    }

    // Attempt STUN binding requests to discover external address
    let mut mapped_addresses: Vec<std::net::SocketAddr> = Vec::new();

    for server in &config.stun_servers {
        match stun_binding_request(server) {
            Ok(addr) => mapped_addresses.push(addr),
            Err(e) => {
                tracing::debug!(server = server, error = %e, "STUN binding request failed");
            }
        }
    }

    if mapped_addresses.is_empty() {
        return NatStatus::Unknown;
    }

    // Classify NAT type based on mapped addresses
    classify_nat_type(&mapped_addresses)
}

/// Send a STUN binding request to discover our external address.
///
/// Implements RFC 5389 STUN Binding Request over UDP.
fn stun_binding_request(server: &str) -> Result<std::net::SocketAddr, String> {
    use std::net::UdpSocket;
    use std::time::Duration;

    let socket =
        UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("Cannot bind UDP socket: {e}"))?;
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| format!("Cannot set timeout: {e}"))?;

    // STUN Binding Request: 20 bytes
    // Type: 0x0001 (Binding Request)
    // Length: 0x0000 (no attributes)
    // Magic cookie: 0x2112A442
    // Transaction ID: 12 random bytes
    let mut request = vec![
        0x00, 0x01, // Type: Binding Request
        0x00, 0x00, // Length: 0
        0x21, 0x12, 0xA4, 0x42, // Magic Cookie
    ];
    // Transaction ID (12 bytes)
    let txn_id: [u8; 12] = rand::random();
    request.extend_from_slice(&txn_id);

    socket.send_to(&request, server).map_err(|e| format!("Cannot send to {server}: {e}"))?;

    let mut buf = [0u8; 256];
    let (len, _) =
        socket.recv_from(&mut buf).map_err(|e| format!("No response from {server}: {e}"))?;

    if len < 20 {
        return Err("STUN response too short".into());
    }

    // Parse XOR-MAPPED-ADDRESS from response
    parse_xor_mapped_address(&buf[20..len], &buf[4..8])
        .ok_or_else(|| "No XOR-MAPPED-ADDRESS in STUN response".into())
}

/// Parse XOR-MAPPED-ADDRESS attribute from STUN response attributes.
fn parse_xor_mapped_address(attrs: &[u8], magic_cookie: &[u8]) -> Option<std::net::SocketAddr> {
    let mut offset = 0;
    while offset + 4 <= attrs.len() {
        let attr_type = u16::from_be_bytes([attrs[offset], attrs[offset + 1]]);
        let attr_len = u16::from_be_bytes([attrs[offset + 2], attrs[offset + 3]]) as usize;

        if offset + 4 + attr_len > attrs.len() {
            break;
        }

        // XOR-MAPPED-ADDRESS = 0x0020
        if attr_type == 0x0020 && attr_len >= 8 {
            let value = &attrs[offset + 4..offset + 4 + attr_len];
            let family = value[1];
            let xor_port = u16::from_be_bytes([value[2], value[3]]) ^ 0x2112; // XOR with magic cookie MSB

            if family == 0x01 && attr_len >= 8 {
                // IPv4
                let ip = [
                    value[4] ^ magic_cookie[0],
                    value[5] ^ magic_cookie[1],
                    value[6] ^ magic_cookie[2],
                    value[7] ^ magic_cookie[3],
                ];
                return Some(std::net::SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])),
                    xor_port,
                ));
            }
        }

        // Advance to next attribute (padded to 4-byte boundary)
        offset += 4 + ((attr_len + 3) & !3);
    }
    None
}

/// Classify NAT type based on mapped addresses from multiple STUN servers.
fn classify_nat_type(addresses: &[std::net::SocketAddr]) -> NatStatus {
    if addresses.is_empty() {
        return NatStatus::Unknown;
    }

    if addresses.len() == 1 {
        // Only one server responded — can detect direct but not NAT type
        return NatStatus::Direct; // Assume direct if reachable
    }

    // Compare IP addresses across servers
    let first_ip = addresses[0].ip();
    let same_ip = addresses.iter().all(|a| a.ip() == first_ip);

    if !same_ip {
        // Different external IPs for different destinations = Symmetric NAT
        return NatStatus::Symmetric;
    }

    // Same IP — check ports
    let first_port = addresses[0].port();
    let same_port = addresses.iter().all(|a| a.port() == first_port);

    if same_port {
        // Same IP and port for all destinations = Full Cone or Direct
        NatStatus::FullCone
    } else {
        // Same IP but different ports = Port-Restricted or Restricted
        NatStatus::PortRestricted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_all_methods_enabled() {
        let config = NatConfig::default();
        assert!(config.upnp_enabled, "UPnP should be enabled by default");
        assert!(config.dcutr_enabled, "dcutr should be enabled by default");
        assert!(config.relay_enabled, "Relay should be enabled by default");
    }

    #[test]
    fn default_config_has_stun_servers() {
        let config = NatConfig::default();
        assert!(!config.stun_servers.is_empty());
        assert!(config.stun_servers[0].contains("google"));
    }

    #[test]
    fn classify_single_address_as_direct() {
        let addrs = vec!["1.2.3.4:5000".parse().unwrap()];
        assert_eq!(classify_nat_type(&addrs), NatStatus::Direct);
    }

    #[test]
    fn classify_same_ip_same_port_as_full_cone() {
        let addrs = vec!["1.2.3.4:5000".parse().unwrap(), "1.2.3.4:5000".parse().unwrap()];
        assert_eq!(classify_nat_type(&addrs), NatStatus::FullCone);
    }

    #[test]
    fn classify_same_ip_diff_port_as_port_restricted() {
        let addrs = vec!["1.2.3.4:5000".parse().unwrap(), "1.2.3.4:6000".parse().unwrap()];
        assert_eq!(classify_nat_type(&addrs), NatStatus::PortRestricted);
    }

    #[test]
    fn classify_diff_ip_as_symmetric() {
        let addrs = vec!["1.2.3.4:5000".parse().unwrap(), "5.6.7.8:5000".parse().unwrap()];
        assert_eq!(classify_nat_type(&addrs), NatStatus::Symmetric);
    }

    #[test]
    fn empty_stun_servers_returns_unknown() {
        let config = NatConfig { stun_servers: vec![], ..NatConfig::default() };
        assert_eq!(detect_nat_status_with_config(&config), NatStatus::Unknown);
    }

    #[test]
    fn nat_status_variants_are_distinct() {
        assert_ne!(NatStatus::Direct, NatStatus::Relayed);
        assert_ne!(NatStatus::HolePunched, NatStatus::Unreachable);
        assert_ne!(NatStatus::FullCone, NatStatus::Symmetric);
    }

    #[test]
    fn parse_xor_mapped_address_valid() {
        // Construct a valid XOR-MAPPED-ADDRESS attribute
        // Type: 0x0020, Length: 8
        // Family: IPv4 (0x01), Port XOR'd, IP XOR'd with magic cookie
        let magic_cookie = [0x21, 0x12, 0xA4, 0x42];
        let port: u16 = 12345;
        let xor_port = port ^ 0x2112;
        let ip = [192u8, 168, 1, 100];
        let xor_ip = [
            ip[0] ^ magic_cookie[0],
            ip[1] ^ magic_cookie[1],
            ip[2] ^ magic_cookie[2],
            ip[3] ^ magic_cookie[3],
        ];

        let mut attr = vec![
            0x00, 0x20, // Type: XOR-MAPPED-ADDRESS
            0x00, 0x08, // Length: 8
            0x00, 0x01, // Family: IPv4
        ];
        attr.extend_from_slice(&xor_port.to_be_bytes());
        attr.extend_from_slice(&xor_ip);

        let result = parse_xor_mapped_address(&attr, &magic_cookie);
        assert!(result.is_some());
        let addr = result.unwrap();
        assert_eq!(addr.port(), port);
        assert_eq!(addr.ip(), std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 100)));
    }
}
