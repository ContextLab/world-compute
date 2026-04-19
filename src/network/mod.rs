//! Network module — P2P discovery, transport, gossip per FR-060–063.
//!
//! Spec 005 US1 additions: WSS-over-TLS-443 fallback transport (FR-003),
//! DoH resolver fallback (FR-005), relay-reservation lifecycle (FR-002, FR-006),
//! dial-attempt logging (FR-004).

pub mod dial_logging;
pub mod discovery;
pub mod dispatch;
pub mod doh_resolver;
pub mod gossip;
pub mod nat;
pub mod rate_limit;
pub mod relay_reservation;
pub mod rest_gateway;
pub mod tls;
pub mod transport;
pub mod wss_transport;
