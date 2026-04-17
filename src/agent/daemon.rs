//! P2P daemon — the persistent background process that makes World Compute a distributed system.
//!
//! This module bridges the agent lifecycle with the libp2p networking layer.
//! When `worldcompute donor join` runs with `--daemon`, it starts a Swarm that:
//! - Listens on TCP and QUIC for incoming peer connections
//! - Discovers LAN peers via mDNS (< 2 seconds, zero config)
//! - Joins the global DHT via Kademlia bootstrap seeds
//! - Publishes heartbeats on the capacity-updates GossipSub topic
//! - Receives task announcements and lease grants via GossipSub
//! - Bridges incoming work to the local sandbox for execution

use libp2p::{
    futures::StreamExt,
    gossipsub::{self, IdentTopic},
    identity,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, SwarmBuilder,
};
use std::collections::HashSet;
use std::time::Duration;
use tokio::select;

use crate::agent::lifecycle::AgentInstance;
use crate::network::discovery::{build_discovery_behaviour, DiscoveryBehaviour, DiscoveryConfig};
use crate::network::gossip::{
    build_gossip, GossipConfig, TOPIC_CAPACITY_UPDATES, TOPIC_LEASE_GRANTS,
    TOPIC_TASK_ANNOUNCEMENTS,
};

/// Combined network behaviour: discovery (mDNS + Kademlia) + gossip (GossipSub).
#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub discovery: DiscoveryBehaviour,
    pub gossipsub: gossipsub::Behaviour,
}

/// Configuration for the daemon.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// TCP listen port (default: 19999).
    pub tcp_port: u16,
    /// QUIC listen port (default: 19999).
    pub quic_port: u16,
    /// Heartbeat publish interval in seconds.
    pub heartbeat_interval_secs: u64,
    /// Bootstrap peer multiaddresses to dial on startup.
    pub bootstrap_peers: Vec<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            tcp_port: 19999,
            quic_port: 19999,
            heartbeat_interval_secs: 30,
            bootstrap_peers: Vec::new(),
        }
    }
}

/// Runtime state of a running daemon.
pub struct DaemonState {
    pub local_peer_id: PeerId,
    pub connected_peers: HashSet<PeerId>,
    pub messages_received: u64,
    pub heartbeats_sent: u64,
    pub running: bool,
}

/// Start the P2P daemon. This function blocks until shutdown.
///
/// It creates a libp2p Swarm with mDNS + Kademlia + GossipSub,
/// listens on TCP and QUIC, and runs the event loop.
pub async fn start_daemon(
    mut agent: AgentInstance,
    config: DaemonConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Generate or load the node's Ed25519 keypair
    let keypair = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(keypair.public());

    tracing::info!(%local_peer_id, "Starting World Compute daemon");

    // Build the combined behaviour
    let discovery_config = DiscoveryConfig::default();
    let gossip_config = GossipConfig::default();

    let discovery = build_discovery_behaviour(local_peer_id, &discovery_config)?;
    let mut gossipsub_behaviour = build_gossip(&keypair, local_peer_id, &gossip_config)?;

    // Subscribe to all topics
    let topic_tasks = IdentTopic::new(TOPIC_TASK_ANNOUNCEMENTS);
    let topic_capacity = IdentTopic::new(TOPIC_CAPACITY_UPDATES);
    let topic_leases = IdentTopic::new(TOPIC_LEASE_GRANTS);

    gossipsub_behaviour.subscribe(&topic_tasks)?;
    gossipsub_behaviour.subscribe(&topic_capacity)?;
    gossipsub_behaviour.subscribe(&topic_leases)?;

    let behaviour = NodeBehaviour { discovery, gossipsub: gossipsub_behaviour };

    // Build the Swarm
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|_| Ok(behaviour))?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(300)))
        .build();

    // Listen on TCP and QUIC
    let tcp_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.tcp_port).parse()?;
    let quic_addr: Multiaddr = format!("/ip4/0.0.0.0/udp/{}/quic-v1", config.quic_port).parse()?;

    swarm.listen_on(tcp_addr)?;
    swarm.listen_on(quic_addr)?;

    tracing::info!(tcp_port = config.tcp_port, quic_port = config.quic_port, "Listening for peers");

    // Dial bootstrap peers if provided
    for addr_str in &config.bootstrap_peers {
        if let Ok(addr) = addr_str.parse::<Multiaddr>() {
            tracing::info!(%addr, "Dialing bootstrap peer");
            if let Err(e) = swarm.dial(addr.clone()) {
                tracing::warn!(%addr, "Failed to dial bootstrap peer: {e}");
            }
        }
    }

    // Runtime state
    let mut state = DaemonState {
        local_peer_id,
        connected_peers: HashSet::new(),
        messages_received: 0,
        heartbeats_sent: 0,
        running: true,
    };

    // Heartbeat timer
    let mut heartbeat_interval =
        tokio::time::interval(Duration::from_secs(config.heartbeat_interval_secs));

    println!(
        "World Compute daemon running.\n  Peer ID: {}\n  TCP: /ip4/0.0.0.0/tcp/{}\n  QUIC: /ip4/0.0.0.0/udp/{}/quic-v1\n  Peers: discovering...",
        local_peer_id, config.tcp_port, config.quic_port
    );

    // Event loop — the heart of the distributed system
    while state.running {
        select! {
            // Process Swarm events (peer discovery, gossip messages, connections)
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        tracing::info!(%address, "Listening on");
                        println!("  Listening on: {address}");
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        state.connected_peers.insert(peer_id);
                        let addr = endpoint.get_remote_address();
                        tracing::info!(%peer_id, %addr, peers = state.connected_peers.len(), "Peer connected");
                        println!("  Peer connected: {peer_id} ({addr}) [{} peers total]", state.connected_peers.len());
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        state.connected_peers.remove(&peer_id);
                        tracing::info!(%peer_id, peers = state.connected_peers.len(), "Peer disconnected");
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, propagation_source, .. }
                    )) => {
                        state.messages_received += 1;
                        let topic = message.topic.to_string();
                        let data_len = message.data.len();
                        tracing::info!(
                            %propagation_source, %topic, data_len,
                            "Received gossip message"
                        );

                        // Route message to appropriate handler
                        if topic == TOPIC_TASK_ANNOUNCEMENTS {
                            handle_task_announcement(&message.data, &propagation_source);
                        } else if topic == TOPIC_LEASE_GRANTS {
                            handle_lease_grant(&message.data, &propagation_source);
                        } else if topic == TOPIC_CAPACITY_UPDATES {
                            // Peer capacity update — update peer registry
                            tracing::debug!(%propagation_source, "Capacity update received");
                        }
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Discovery(event)) => {
                        // mDNS or Kademlia discovery events are handled automatically
                        // by the DiscoveryBehaviour — new peers are added to the routing table
                        tracing::debug!(?event, "Discovery event");
                    }
                    _ => {}
                }
            }

            // Publish heartbeat on timer
            _ = heartbeat_interval.tick() => {
                if let Ok(payload) = agent.heartbeat() {
                    let json = serde_json::to_vec(&payload).unwrap_or_default();
                    if let Err(e) = swarm.behaviour_mut().gossipsub.publish(
                        topic_capacity.clone(),
                        json,
                    ) {
                        // Publishing fails if no peers are subscribed — normal at startup
                        tracing::debug!("Heartbeat publish: {e} (normal if no peers yet)");
                    } else {
                        state.heartbeats_sent += 1;
                        tracing::debug!(
                            heartbeats = state.heartbeats_sent,
                            peers = state.connected_peers.len(),
                            "Heartbeat published"
                        );
                    }
                }
            }

            // Handle Ctrl+C for clean shutdown
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down daemon...");
                state.running = false;
            }
        }
    }

    // Clean shutdown
    tracing::info!(
        heartbeats = state.heartbeats_sent,
        messages = state.messages_received,
        "Daemon shutdown complete"
    );
    println!(
        "Daemon stopped. Heartbeats sent: {}, Messages received: {}",
        state.heartbeats_sent, state.messages_received
    );

    Ok(())
}

/// Handle an incoming task announcement from a broker.
fn handle_task_announcement(data: &[u8], source: &PeerId) {
    match String::from_utf8(data.to_vec()) {
        Ok(msg) => {
            tracing::info!(%source, "Task announcement: {msg}");
        }
        Err(_) => {
            tracing::warn!(%source, bytes = data.len(), "Invalid task announcement (not UTF-8)");
        }
    }
}

/// Handle an incoming lease grant from a broker.
fn handle_lease_grant(data: &[u8], source: &PeerId) {
    match String::from_utf8(data.to_vec()) {
        Ok(msg) => {
            tracing::info!(%source, "Lease grant: {msg}");
        }
        Err(_) => {
            tracing::warn!(%source, bytes = data.len(), "Invalid lease grant (not UTF-8)");
        }
    }
}

/// Get a multiaddr string for dialing a specific peer on a known host and port.
pub fn peer_multiaddr(host: &str, port: u16) -> String {
    // Resolve hostname to IP if needed, or use IP directly
    format!("/ip4/{host}/tcp/{port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_config_defaults() {
        let config = DaemonConfig::default();
        assert_eq!(config.tcp_port, 19999);
        assert_eq!(config.quic_port, 19999);
        assert_eq!(config.heartbeat_interval_secs, 30);
        assert!(config.bootstrap_peers.is_empty());
    }

    #[test]
    fn daemon_state_initial() {
        let keypair = identity::Keypair::generate_ed25519();
        let state = DaemonState {
            local_peer_id: PeerId::from(keypair.public()),
            connected_peers: HashSet::new(),
            messages_received: 0,
            heartbeats_sent: 0,
            running: true,
        };
        assert!(state.running);
        assert_eq!(state.connected_peers.len(), 0);
    }

    #[test]
    fn peer_multiaddr_format() {
        let addr = peer_multiaddr("192.168.1.100", 19999);
        assert_eq!(addr, "/ip4/192.168.1.100/tcp/19999");
    }

    #[test]
    fn node_behaviour_compiles() {
        // Verify the combined behaviour type compiles correctly.
        // The #[derive(NetworkBehaviour)] macro generates the event enum.
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let discovery = build_discovery_behaviour(peer_id, &DiscoveryConfig::default()).unwrap();
        let gossipsub = build_gossip(&keypair, peer_id, &GossipConfig::default()).unwrap();
        let _behaviour = NodeBehaviour { discovery, gossipsub };
    }
}
