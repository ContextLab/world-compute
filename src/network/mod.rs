//! Network module — P2P discovery, transport, gossip per FR-060–063.

pub mod discovery;
pub mod dispatch;
pub mod gossip;
pub mod nat;
pub mod rate_limit;
pub mod rest_gateway;
pub mod tls;
pub mod transport;
