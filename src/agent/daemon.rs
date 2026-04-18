//! P2P daemon — the persistent background process that makes World Compute a distributed system.
//!
//! This is the production networking stack with full NAT traversal:
//! - **TCP + QUIC** transports (QUIC preferred for lower latency)
//! - **Noise protocol** for encrypted peer-to-peer handshake (no passwords)
//! - **mDNS** for zero-config LAN peer discovery (< 2 seconds)
//! - **Kademlia DHT** for WAN peer routing via bootstrap seeds
//! - **Identify** so peers learn each other's observed addresses
//! - **Ping** for liveness detection
//! - **GossipSub** for task announcements, capacity updates, lease grants
//! - **AutoNAT** to detect whether we're directly reachable
//! - **Relay v2** (server side) so public nodes relay traffic for NAT'd peers
//! - **Relay v2 client** so NAT'd nodes use relays as fallback
//! - **DCUtR** (Direct Connection Upgrade through Relay) for NAT hole-punching
//!
//! Flow for two nodes behind NAT:
//! 1. Both connect to a public relay node (listed in bootstrap seeds)
//! 2. The relay proxies their initial connection
//! 3. DCUtR coordinates simultaneous connection attempts
//! 4. If hole-punch succeeds, the connection upgrades to direct P2P
//! 5. Gossip + DHT + discovery then operate normally

use libp2p::{
    autonat, dcutr,
    futures::StreamExt,
    gossipsub::{self, IdentTopic},
    identify, identity, ping, relay,
    request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, SwarmBuilder,
};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::select;

use crate::agent::lifecycle::AgentInstance;
use crate::data_plane::cid_store::CidStore;
use crate::network::discovery::{build_discovery_behaviour, DiscoveryBehaviour, DiscoveryConfig};
use crate::network::dispatch::{
    build_dispatch_behaviour, build_offer_behaviour, TaskDispatchRequest, TaskDispatchResponse,
    TaskOffer, TaskOfferResponse, TaskStatus,
};
use crate::network::gossip::{
    build_gossip, GossipConfig, TOPIC_CAPACITY_UPDATES, TOPIC_LEASE_GRANTS,
    TOPIC_TASK_ANNOUNCEMENTS,
};
use crate::sandbox::wasm::{compile_module, run_module};

/// libp2p protocol name for identify.
pub const IDENTIFY_PROTOCOL: &str = "/worldcompute/1.0.0";

/// Combined network behaviour for production P2P with full NAT traversal + job dispatch.
#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub discovery: DiscoveryBehaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub autonat: autonat::Behaviour,
    pub relay_server: relay::Behaviour,
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    /// Lightweight capacity probe (broker → candidate executor).
    pub offer: request_response::cbor::Behaviour<TaskOffer, TaskOfferResponse>,
    /// Full job dispatch with result (broker → selected executor).
    pub dispatch: request_response::cbor::Behaviour<TaskDispatchRequest, TaskDispatchResponse>,
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
    ///
    /// These should be well-connected public nodes that serve as relays
    /// and DHT rendezvous points. Accepts either full multiaddrs with peer ID
    /// (`/ip4/.../tcp/.../p2p/<peer_id>`) or partial addresses which will be
    /// resolved via DHT peer-ID lookup.
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
    /// Peers we're connected to via a relay (rather than directly).
    pub relayed_peers: HashSet<PeerId>,
    /// Number of successful DCUtR hole-punches.
    pub holepunches_succeeded: u64,
    /// Number of task offers received (we were a candidate executor).
    pub offers_received: u64,
    /// Number of task offers we accepted.
    pub offers_accepted: u64,
    /// Number of task dispatches completed successfully.
    pub tasks_succeeded: u64,
    /// Number of task dispatches that failed.
    pub tasks_failed: u64,
}

/// Start the P2P daemon. This function blocks until shutdown.
pub async fn start_daemon(
    mut agent: AgentInstance,
    config: DaemonConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let keypair = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(keypair.public());

    tracing::info!(%local_peer_id, "Starting World Compute daemon");

    let discovery_config = DiscoveryConfig::default();
    let gossip_config = GossipConfig::default();

    let topic_tasks = IdentTopic::new(TOPIC_TASK_ANNOUNCEMENTS);
    let topic_capacity = IdentTopic::new(TOPIC_CAPACITY_UPDATES);
    let topic_leases = IdentTopic::new(TOPIC_LEASE_GRANTS);

    // SwarmBuilder with relay-client support. The relay client needs to wrap
    // the transport so that connections can be opened via relay circuits.
    let mut swarm = SwarmBuilder::with_existing_identity(keypair.clone())
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_quic()
        .with_dns()?
        .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)?
        .with_behaviour(|kp, relay_client| {
            let discovery = build_discovery_behaviour(local_peer_id, &discovery_config)
                .map_err(|e| format!("discovery: {e}"))?;
            let mut gossipsub_behaviour = build_gossip(kp, local_peer_id, &gossip_config)
                .map_err(|e| format!("gossip: {e}"))?;

            gossipsub_behaviour
                .subscribe(&topic_tasks)
                .map_err(|e| format!("subscribe tasks: {e}"))?;
            gossipsub_behaviour
                .subscribe(&topic_capacity)
                .map_err(|e| format!("subscribe capacity: {e}"))?;
            gossipsub_behaviour
                .subscribe(&topic_leases)
                .map_err(|e| format!("subscribe leases: {e}"))?;

            Ok(NodeBehaviour {
                discovery,
                gossipsub: gossipsub_behaviour,
                identify: identify::Behaviour::new(identify::Config::new(
                    IDENTIFY_PROTOCOL.into(),
                    kp.public(),
                )),
                ping: ping::Behaviour::new(ping::Config::new()),
                autonat: autonat::Behaviour::new(local_peer_id, autonat::Config::default()),
                relay_server: relay::Behaviour::new(local_peer_id, relay::Config::default()),
                relay_client,
                dcutr: dcutr::Behaviour::new(local_peer_id),
                offer: build_offer_behaviour(),
                dispatch: build_dispatch_behaviour(),
            })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(300)))
        .build();

    // Listen on TCP and QUIC. 0.0.0.0 means all interfaces.
    let tcp_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.tcp_port).parse()?;
    let quic_addr: Multiaddr = format!("/ip4/0.0.0.0/udp/{}/quic-v1", config.quic_port).parse()?;
    swarm.listen_on(tcp_addr)?;
    swarm.listen_on(quic_addr)?;

    // Dial bootstrap peers. These give us:
    // - Initial DHT contact (for routing table)
    // - Potential relay servers (for NAT traversal)
    // - Observed-address feedback (via identify, for autonat)
    for addr_str in &config.bootstrap_peers {
        match addr_str.parse::<Multiaddr>() {
            Ok(addr) => {
                tracing::info!(%addr, "Dialing bootstrap peer");
                if let Err(e) = swarm.dial(addr.clone()) {
                    tracing::warn!(%addr, "Failed to dial bootstrap: {e}");
                } else {
                    println!("  Dialing bootstrap: {addr}");
                }
            }
            Err(e) => {
                tracing::warn!(addr = %addr_str, "Invalid bootstrap multiaddr: {e}");
            }
        }
    }

    let mut state = DaemonState {
        local_peer_id,
        connected_peers: HashSet::new(),
        messages_received: 0,
        heartbeats_sent: 0,
        running: true,
        relayed_peers: HashSet::new(),
        holepunches_succeeded: 0,
        offers_received: 0,
        offers_accepted: 0,
        tasks_succeeded: 0,
        tasks_failed: 0,
    };

    // CID store for inline workload bytes (per-daemon, in-memory).
    let cid_store = CidStore::new();

    let mut heartbeat_interval =
        tokio::time::interval(Duration::from_secs(config.heartbeat_interval_secs));

    println!(
        "World Compute daemon running.\n  Peer ID: {}\n  TCP: /ip4/0.0.0.0/tcp/{}\n  QUIC: /ip4/0.0.0.0/udp/{}/quic-v1\n  NAT traversal: AutoNAT + Relay v2 + DCUtR enabled",
        local_peer_id, config.tcp_port, config.quic_port
    );

    // Event loop — the heart of the distributed system.
    while state.running {
        select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        tracing::info!(%address, "Listening on");
                        println!("  Listening on: {address}");
                    }
                    SwarmEvent::ExternalAddrConfirmed { address } => {
                        // AutoNAT confirmed we're reachable at this address.
                        tracing::info!(%address, "External address confirmed");
                        println!("  External address confirmed (directly reachable): {address}");
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        state.connected_peers.insert(peer_id);
                        let addr = endpoint.get_remote_address();
                        let is_relayed = addr.iter().any(|p| matches!(p, libp2p::multiaddr::Protocol::P2pCircuit));
                        if is_relayed {
                            state.relayed_peers.insert(peer_id);
                        }
                        let mode = if is_relayed { "via relay" } else { "direct" };
                        tracing::info!(%peer_id, %addr, %mode, peers = state.connected_peers.len(), "Peer connected");
                        println!("  Peer connected [{mode}]: {peer_id} ({addr}) [{} peers total]", state.connected_peers.len());
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        state.connected_peers.remove(&peer_id);
                        state.relayed_peers.remove(&peer_id);
                        tracing::info!(%peer_id, peers = state.connected_peers.len(), "Peer disconnected");
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, propagation_source, .. }
                    )) => {
                        state.messages_received += 1;
                        let topic = message.topic.to_string();
                        tracing::info!(%propagation_source, %topic, bytes = message.data.len(), "Received gossip");
                        if topic == TOPIC_TASK_ANNOUNCEMENTS {
                            handle_task_announcement(&message.data, &propagation_source);
                        } else if topic == TOPIC_LEASE_GRANTS {
                            handle_lease_grant(&message.data, &propagation_source);
                        }
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. })) => {
                        // Peer told us their observed address for us. Feed to autonat.
                        tracing::debug!(%peer_id, observed = %info.observed_addr, "Identify received");
                        // Add peer's listen addresses to kademlia so we can route to them.
                        for addr in info.listen_addrs {
                            swarm.behaviour_mut().discovery.kademlia.add_address(&peer_id, addr);
                        }
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Dcutr(dcutr::Event {
                        remote_peer_id, result,
                    })) => {
                        match result {
                            Ok(_) => {
                                state.holepunches_succeeded += 1;
                                tracing::info!(%remote_peer_id, "DCUtR hole-punch succeeded");
                                println!("  Hole-punch succeeded with {remote_peer_id} (upgraded to direct)");
                            }
                            Err(e) => {
                                tracing::warn!(%remote_peer_id, "DCUtR hole-punch failed: {e}");
                            }
                        }
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Autonat(autonat::Event::StatusChanged { new, .. })) => {
                        tracing::info!(?new, "AutoNAT status changed");
                        match new {
                            autonat::NatStatus::Public(addr) => {
                                println!("  NAT status: PUBLIC (reachable at {addr})");
                            }
                            autonat::NatStatus::Private => {
                                println!("  NAT status: PRIVATE (will use relay + DCUtR)");
                            }
                            autonat::NatStatus::Unknown => {
                                tracing::debug!("AutoNAT: status still unknown");
                            }
                        }
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::RelayServer(event)) => {
                        tracing::debug!(?event, "Relay server event");
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::RelayClient(event)) => {
                        tracing::debug!(?event, "Relay client event");
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Offer(
                        request_response::Event::Message {
                            peer,
                            message: request_response::Message::Request { request, channel, .. },
                            ..
                        }
                    )) => {
                        state.offers_received += 1;
                        let accepted = evaluate_offer(&request);
                        if accepted {
                            state.offers_accepted += 1;
                        }
                        let resp = TaskOfferResponse {
                            task_id: request.task_id.clone(),
                            accepted,
                            load: current_load(),
                            reason: if accepted { None } else { Some("insufficient capacity".into()) },
                        };
                        tracing::info!(%peer, task = %request.task_id, %accepted, "Offer received");
                        println!("  Offer from {peer}: task={} accepted={}", request.task_id, accepted);
                        if let Err(e) = swarm.behaviour_mut().offer.send_response(channel, resp) {
                            tracing::warn!(?e, "Failed to send offer response");
                        }
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Dispatch(
                        request_response::Event::Message {
                            peer,
                            message: request_response::Message::Request { request, channel, .. },
                            ..
                        }
                    )) => {
                        let task_id = request.task_id.clone();
                        tracing::info!(%peer, task = %task_id, "Dispatch received — executing");
                        println!("  Dispatch from {peer}: task={task_id} — executing in sandbox");
                        let resp = execute_dispatched_task(&request, &cid_store);
                        match resp.status {
                            TaskStatus::Succeeded => state.tasks_succeeded += 1,
                            _ => state.tasks_failed += 1,
                        }
                        println!(
                            "  Task {task_id} {:?} in {}ms, output={}B",
                            resp.status,
                            resp.duration_ms,
                            resp.output.len()
                        );
                        if let Err(e) = swarm.behaviour_mut().dispatch.send_response(channel, resp) {
                            tracing::warn!(?e, "Failed to send dispatch response");
                        }
                    }
                    SwarmEvent::Behaviour(NodeBehaviourEvent::Dispatch(
                        request_response::Event::Message {
                            peer,
                            message: request_response::Message::Response { response, .. },
                            ..
                        }
                    )) => {
                        // We were the broker — log the result.
                        println!(
                            "  Result from {peer}: task={} status={:?} {}B in {}ms",
                            response.task_id,
                            response.status,
                            response.output.len(),
                            response.duration_ms
                        );
                        tracing::info!(
                            %peer,
                            task = %response.task_id,
                            ?response.status,
                            "Dispatch completed"
                        );
                    }
                    _ => {}
                }
            }

            _ = heartbeat_interval.tick() => {
                if let Ok(payload) = agent.heartbeat() {
                    let json = serde_json::to_vec(&payload).unwrap_or_default();
                    match swarm.behaviour_mut().gossipsub.publish(topic_capacity.clone(), json) {
                        Ok(_) => {
                            state.heartbeats_sent += 1;
                            tracing::debug!(heartbeats = state.heartbeats_sent, peers = state.connected_peers.len(), "Heartbeat published");
                        }
                        Err(e) => {
                            tracing::debug!("Heartbeat publish: {e} (normal if no peers yet)");
                        }
                    }
                }
            }

            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down daemon...");
                state.running = false;
            }
        }
    }

    println!(
        "Daemon stopped.\n  Heartbeats sent: {}\n  Messages received: {}\n  Hole-punches succeeded: {}\n  Offers received: {}/{}accepted\n  Tasks succeeded/failed: {}/{}",
        state.heartbeats_sent,
        state.messages_received,
        state.holepunches_succeeded,
        state.offers_accepted,
        state.offers_received,
        state.tasks_succeeded,
        state.tasks_failed,
    );
    Ok(())
}

/// Evaluate a task offer against this node's capacity. Lightweight — no I/O.
fn evaluate_offer(offer: &TaskOffer) -> bool {
    // Real implementation would consult the scheduler's broker state; for now,
    // accept any task within reasonable bounds.
    offer.min_cpu_cores <= 64 && offer.min_memory_mb <= 512 * 1024 && offer.max_wallclock_ms <= 600_000
}

/// Report current load as a fraction 0.0–1.0. Stub returns 0.1 (mostly idle).
fn current_load() -> f32 {
    // Production: query system load avg, active leases, etc.
    0.1
}

/// Execute a dispatched task in a WASM sandbox and return the result.
/// This is the actual cross-node execution path.
fn execute_dispatched_task(
    request: &TaskDispatchRequest,
    cid_store: &CidStore,
) -> TaskDispatchResponse {
    let start = Instant::now();
    let task_id = request.task_id.clone();

    // Ingest inline inputs into the local CID store.
    for (_name, bytes) in &request.inline_inputs {
        let _ = cid_store.put(bytes);
    }

    // Fetch the workload bytes. For WASM, we require the bytes inlined as the
    // first inline_input OR already present in the store by workload CID.
    let workload_cid = request.manifest.workload_cid;
    let wasm_bytes = match cid_store.get(&workload_cid) {
        Some(b) => b,
        None => {
            // Fallback: look for a "workload" named inline input.
            match request.inline_inputs.iter().find(|(n, _)| n == "workload") {
                Some((_, b)) => b.clone(),
                None => {
                    return TaskDispatchResponse {
                        task_id,
                        status: TaskStatus::Failed,
                        output: Vec::new(),
                        output_cid: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error: Some(format!(
                            "workload CID {workload_cid} not available on executor"
                        )),
                    };
                }
            }
        }
    };

    // Compile and execute. Fuel must be enabled on the engine Config for
    // `run_module` to set fuel on its Store.
    let mut engine_config = wasmtime::Config::new();
    engine_config.consume_fuel(true);
    let engine = match wasmtime::Engine::new(&engine_config) {
        Ok(e) => e,
        Err(e) => {
            return TaskDispatchResponse {
                task_id,
                status: TaskStatus::Failed,
                output: Vec::new(),
                output_cid: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("engine init: {e}")),
            };
        }
    };
    match compile_module(&engine, &wasm_bytes) {
        Ok(module) => match run_module(&engine, &module, 10_000_000) {
            Ok(output) => TaskDispatchResponse {
                task_id,
                status: TaskStatus::Succeeded,
                output,
                output_cid: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: None,
            },
            Err(e) => TaskDispatchResponse {
                task_id,
                status: TaskStatus::Failed,
                output: Vec::new(),
                output_cid: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("run_module: {e}")),
            },
        },
        Err(e) => TaskDispatchResponse {
            task_id,
            status: TaskStatus::Failed,
            output: Vec::new(),
            output_cid: None,
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some(format!("compile_module: {e}")),
        },
    }
}

fn handle_task_announcement(data: &[u8], source: &PeerId) {
    if let Ok(msg) = String::from_utf8(data.to_vec()) {
        tracing::info!(%source, "Task announcement: {msg}");
    } else {
        tracing::warn!(%source, bytes = data.len(), "Invalid task announcement");
    }
}

fn handle_lease_grant(data: &[u8], source: &PeerId) {
    if let Ok(msg) = String::from_utf8(data.to_vec()) {
        tracing::info!(%source, "Lease grant: {msg}");
    } else {
        tracing::warn!(%source, bytes = data.len(), "Invalid lease grant");
    }
}

/// Get a multiaddr string for dialing a specific peer on a known host and port.
pub fn peer_multiaddr(host: &str, port: u16) -> String {
    format!("/ip4/{host}/tcp/{port}")
}

/// Get a relay reservation multiaddr for a peer behind NAT.
/// Format: `/ip4/<relay_ip>/tcp/<port>/p2p/<relay_peer_id>/p2p-circuit/p2p/<target_peer_id>`
pub fn relay_circuit_multiaddr(
    relay_host: &str,
    relay_port: u16,
    relay_peer_id: &str,
    target_peer_id: &str,
) -> String {
    format!(
        "/ip4/{relay_host}/tcp/{relay_port}/p2p/{relay_peer_id}/p2p-circuit/p2p/{target_peer_id}"
    )
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
            relayed_peers: HashSet::new(),
            holepunches_succeeded: 0,
            offers_received: 0,
            offers_accepted: 0,
            tasks_succeeded: 0,
            tasks_failed: 0,
        };
        assert!(state.running);
        assert_eq!(state.connected_peers.len(), 0);
        assert_eq!(state.relayed_peers.len(), 0);
        assert_eq!(state.tasks_succeeded, 0);
    }

    #[test]
    fn evaluate_offer_accepts_reasonable() {
        let offer = TaskOffer {
            task_id: "t1".into(),
            manifest_cid: "c1".into(),
            min_cpu_cores: 2,
            min_memory_mb: 1024,
            needs_gpu: false,
            max_wallclock_ms: 60_000,
        };
        assert!(evaluate_offer(&offer));
    }

    #[test]
    fn evaluate_offer_rejects_huge() {
        let offer = TaskOffer {
            task_id: "t2".into(),
            manifest_cid: "c2".into(),
            min_cpu_cores: 128,
            min_memory_mb: 1024 * 1024,
            needs_gpu: false,
            max_wallclock_ms: 86_400_000,
        };
        assert!(!evaluate_offer(&offer));
    }

    #[test]
    fn current_load_is_in_range() {
        let l = current_load();
        assert!((0.0..=1.0).contains(&l));
    }

    #[test]
    fn execute_dispatched_task_runs_real_wasm() {
        use crate::scheduler::manifest::JobManifest;
        use crate::scheduler::{
            ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
        };
        // Minimal valid WASM module (empty module — the smallest valid binary).
        let wasm_bytes: Vec<u8> = vec![
            0x00, 0x61, 0x73, 0x6d, // magic: \0asm
            0x01, 0x00, 0x00, 0x00, // version: 1
        ];

        let store = CidStore::new();
        let workload_cid = store.put(&wasm_bytes).expect("put wasm");

        let manifest = JobManifest {
            manifest_cid: None,
            name: "test-wasm".into(),
            workload_type: WorkloadType::WasmModule,
            workload_cid,
            command: vec!["_start".into()],
            inputs: Vec::new(),
            output_sink: "stdout".into(),
            resources: ResourceEnvelope {
                cpu_millicores: 1000,
                ram_bytes: 64 * 1024 * 1024,
                gpu_class: None,
                gpu_vram_bytes: 0,
                scratch_bytes: 0,
                network_egress_bytes: 0,
                walltime_budget_ms: 60_000,
            },
            category: JobCategory::PublicGood,
            confidentiality: ConfidentialityLevel::Public,
            verification: VerificationMethod::ReplicatedQuorum,
            acceptable_use_classes: Vec::new(),
            max_wallclock_ms: 60_000,
            allowed_endpoints: Vec::new(),
            confidentiality_level: None,
            submitter_signature: vec![1u8; 64],
        };

        let req = TaskDispatchRequest {
            task_id: "t-real".into(),
            manifest,
            inline_inputs: Vec::new(),
        };

        let resp = execute_dispatched_task(&req, &store);
        assert_eq!(resp.status, TaskStatus::Succeeded, "err={:?}", resp.error);
        assert!(resp.duration_ms < 60_000);
    }

    #[test]
    fn execute_dispatched_task_fails_missing_workload() {
        use crate::scheduler::manifest::JobManifest;
        use crate::scheduler::{
            ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
        };
        let store = CidStore::new();
        // Create a CID for bytes NOT added to the store.
        let other_store = CidStore::new();
        let phantom_cid = other_store.put(b"phantom-workload").expect("put");

        let manifest = JobManifest {
            manifest_cid: None,
            name: "missing".into(),
            workload_type: WorkloadType::WasmModule,
            workload_cid: phantom_cid,
            command: vec!["_start".into()],
            inputs: Vec::new(),
            output_sink: "stdout".into(),
            resources: ResourceEnvelope {
                cpu_millicores: 1000,
                ram_bytes: 64 * 1024 * 1024,
                gpu_class: None,
                gpu_vram_bytes: 0,
                scratch_bytes: 0,
                network_egress_bytes: 0,
                walltime_budget_ms: 60_000,
            },
            category: JobCategory::PublicGood,
            confidentiality: ConfidentialityLevel::Public,
            verification: VerificationMethod::ReplicatedQuorum,
            acceptable_use_classes: Vec::new(),
            max_wallclock_ms: 1_000,
            allowed_endpoints: Vec::new(),
            confidentiality_level: None,
            submitter_signature: vec![1u8; 64],
        };

        let req = TaskDispatchRequest {
            task_id: "t-missing".into(),
            manifest,
            inline_inputs: Vec::new(),
        };

        let resp = execute_dispatched_task(&req, &store);
        assert_eq!(resp.status, TaskStatus::Failed);
        assert!(resp.error.is_some());
    }

    #[test]
    fn peer_multiaddr_format() {
        let addr = peer_multiaddr("192.168.1.100", 19999);
        assert_eq!(addr, "/ip4/192.168.1.100/tcp/19999");
    }

    #[test]
    fn relay_circuit_multiaddr_format() {
        let addr = relay_circuit_multiaddr(
            "203.0.113.1",
            19999,
            "12D3KooWRelay",
            "12D3KooWTarget",
        );
        assert_eq!(
            addr,
            "/ip4/203.0.113.1/tcp/19999/p2p/12D3KooWRelay/p2p-circuit/p2p/12D3KooWTarget"
        );
    }

    #[test]
    fn identify_protocol_is_versioned() {
        assert!(IDENTIFY_PROTOCOL.starts_with("/worldcompute/"));
        assert!(IDENTIFY_PROTOCOL.contains("1.0"));
    }
}
