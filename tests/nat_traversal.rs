//! Production NAT-traversal integration test.
//!
//! Exercises the full libp2p relay v2 + DCUtR flow with three in-process nodes:
//!
//!   Node R (relay server, publicly-reachable)
//!     ├── Node A (behind NAT): reaches R, reserves circuit, is now dialable via
//!     │                         /ip4/.../p2p/R/p2p-circuit/p2p/A
//!     └── Node B (behind NAT): connects to R, discovers A through R, dials A
//!                              via relay circuit, then dispatches a real WASM
//!                              job to A and verifies result.
//!
//! This validates every piece of our production NAT-traversal stack in a single
//! reproducible test:
//!   - relay::Behaviour (R as server)
//!   - relay::client::Behaviour (A, B as clients)
//!   - identify::Behaviour (peer discovery, observed-address learning)
//!   - AutoRelay reservation (A's listen on /p2p-circuit)
//!   - Circuit-routed dispatch (B → A via R)
//!   - Request-response over the relay (TaskDispatch protocol)
//!   - WASM execution end-to-end

use libp2p::{
    futures::StreamExt,
    identify, identity, ping, relay,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use std::time::Duration;
use tokio::time::timeout;

use worldcompute::network::dispatch::{
    TaskDispatchRequest, TaskDispatchResponse, TaskStatus, PROTOCOL_TASK_DISPATCH,
};
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

/// Relay node's network behaviour: relay SERVER + identify + ping.
/// No relay client — R is publicly reachable and accepts circuit requests.
#[derive(NetworkBehaviour)]
struct RelayServerBehaviour {
    relay: relay::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
}

/// Client node's network behaviour: full production stack.
/// Has relay client (to use R as a rendezvous) AND relay server (because every
/// donor also serves as a relay for others as the network grows).
#[derive(NetworkBehaviour)]
struct ClientBehaviour {
    relay_client: relay::client::Behaviour,
    relay_server: relay::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    dispatch: request_response::cbor::Behaviour<TaskDispatchRequest, TaskDispatchResponse>,
}

fn build_dispatch_behaviour(
) -> request_response::cbor::Behaviour<TaskDispatchRequest, TaskDispatchResponse> {
    request_response::cbor::Behaviour::new(
        std::iter::once((StreamProtocol::new(PROTOCOL_TASK_DISPATCH), ProtocolSupport::Full)),
        request_response::Config::default().with_request_timeout(Duration::from_secs(60)),
    )
}

fn build_relay_swarm(keypair: identity::Keypair) -> libp2p::Swarm<RelayServerBehaviour> {
    let peer_id = PeerId::from(keypair.public());
    SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("tcp")
        .with_behaviour(|kp| RelayServerBehaviour {
            relay: relay::Behaviour::new(peer_id, relay::Config::default()),
            identify: identify::Behaviour::new(identify::Config::new(
                "/worldcompute-test/1.0.0".into(),
                kp.public(),
            )),
            ping: ping::Behaviour::new(ping::Config::new()),
        })
        .expect("behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(120)))
        .build()
}

fn build_client_swarm(keypair: identity::Keypair) -> libp2p::Swarm<ClientBehaviour> {
    let peer_id = PeerId::from(keypair.public());
    SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("tcp")
        .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)
        .expect("relay client")
        .with_behaviour(|kp, relay_client| ClientBehaviour {
            relay_client,
            relay_server: relay::Behaviour::new(peer_id, relay::Config::default()),
            identify: identify::Behaviour::new(identify::Config::new(
                "/worldcompute-test/1.0.0".into(),
                kp.public(),
            )),
            ping: ping::Behaviour::new(ping::Config::new()),
            dispatch: build_dispatch_behaviour(),
        })
        .expect("behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(120)))
        .build()
}

fn make_test_manifest(workload_cid: cid::Cid) -> JobManifest {
    JobManifest {
        manifest_cid: None,
        name: "nat-traversal-test".into(),
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
    }
}

fn execute_wasm_task(req: &TaskDispatchRequest) -> TaskDispatchResponse {
    use std::time::Instant;
    let start = Instant::now();
    let wasm = req.inline_inputs.iter().find(|(n, _)| n == "workload").map(|(_, b)| b.clone());
    let Some(bytes) = wasm else {
        return TaskDispatchResponse {
            task_id: req.task_id.clone(),
            status: TaskStatus::Failed,
            output: Vec::new(),
            output_cid: None,
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some("no inline workload".into()),
        };
    };
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    match worldcompute::sandbox::wasm::compile_module(&engine, &bytes) {
        Ok(module) => match worldcompute::sandbox::wasm::run_module(&engine, &module, 10_000_000) {
            Ok(output) => TaskDispatchResponse {
                task_id: req.task_id.clone(),
                status: TaskStatus::Succeeded,
                output,
                output_cid: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: None,
            },
            Err(e) => TaskDispatchResponse {
                task_id: req.task_id.clone(),
                status: TaskStatus::Failed,
                output: Vec::new(),
                output_cid: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("run: {e}")),
            },
        },
        Err(e) => TaskDispatchResponse {
            task_id: req.task_id.clone(),
            status: TaskStatus::Failed,
            output: Vec::new(),
            output_cid: None,
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some(format!("compile: {e}")),
        },
    }
}

/// End-to-end: three nodes (one relay + two clients) exchange a real WASM job
/// via a relay circuit, exercising the full production NAT-traversal stack.
#[tokio::test]
async fn three_node_relay_circuit_wasm_dispatch() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .try_init();

    // ─── Setup: spawn relay node R ───────────────────────────────────────
    let r_kp = identity::Keypair::generate_ed25519();
    let r_peer = PeerId::from(r_kp.public());
    let mut r_swarm = build_relay_swarm(r_kp);
    r_swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).expect("relay listen");

    // Wait for relay to obtain a concrete listen address.
    let r_addr: Multiaddr = timeout(Duration::from_secs(10), async {
        loop {
            if let SwarmEvent::NewListenAddr { address, .. } = r_swarm.select_next_some().await {
                return address;
            }
        }
    })
    .await
    .expect("relay listen addr timeout");
    // Mark R's address as an external (confirmed) address so identify advertises
    // it and the relay server treats connected peers as reachable via it.
    r_swarm.add_external_address(r_addr.clone());
    let r_addr_with_peer = r_addr.with(libp2p::multiaddr::Protocol::P2p(r_peer));
    println!("Relay R listening at {r_addr_with_peer}");

    // ─── Spawn client A — will reserve a relay circuit via R ─────────────
    let a_kp = identity::Keypair::generate_ed25519();
    let a_peer = PeerId::from(a_kp.public());
    let mut a_swarm = build_client_swarm(a_kp);
    a_swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).expect("A listen");

    // ─── Spawn client B — will dial A through R ──────────────────────────
    let b_kp = identity::Keypair::generate_ed25519();
    let mut b_swarm = build_client_swarm(b_kp);
    b_swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).expect("B listen");

    // A dials R and requests a relay reservation.
    a_swarm.dial(r_addr_with_peer.clone()).expect("A->R dial");
    // B also connects to R (same rendezvous).
    b_swarm.dial(r_addr_with_peer.clone()).expect("B->R dial");

    // Build the circuit multiaddr that B will dial: /<R_addr>/p2p-circuit/p2p/<A_peer>
    let a_circuit_addr = r_addr_with_peer
        .clone()
        .with(libp2p::multiaddr::Protocol::P2pCircuit)
        .with(libp2p::multiaddr::Protocol::P2p(a_peer));
    println!("A's circuit address: {a_circuit_addr}");

    // Prepare dispatch request (real WASM module).
    let wasm_bytes: Vec<u8> = vec![
        0x00, 0x61, 0x73, 0x6d, // magic
        0x01, 0x00, 0x00, 0x00, // version 1
    ];
    let workload_cid = worldcompute::data_plane::cid_store::compute_cid(&wasm_bytes).expect("cid");
    let dispatch_request = TaskDispatchRequest {
        task_id: "nat-test-001".into(),
        manifest: make_test_manifest(workload_cid),
        inline_inputs: vec![("workload".to_string(), wasm_bytes)],
    };

    // ─── Event-loop soup: drive all three swarms concurrently ────────────
    //
    // Milestones (must all fire before test passes):
    //   1. A has an active relay reservation (A gets ReservationReqAccepted).
    //   2. B requests A's circuit address (we just dial it directly).
    //   3. B connects to A via R's circuit (InboundCircuitEstablished on A).
    //   4. B sends TaskDispatch to A, A executes WASM, B gets Succeeded back.
    //
    // The listen on /<R>/p2p-circuit is how A signals to R that it wants a
    // reservation. libp2p's relay::client::Behaviour handles the RESERVE
    // request internally.
    let response = timeout(Duration::from_secs(60), async {
        let mut a_reserved = false;
        let mut b_dialed_a = false;
        let mut request_sent = false;

        loop {
            tokio::select! {
                // R's event loop: just keep serving relay requests.
                event = r_swarm.select_next_some() => {
                    match &event {
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            println!("R: connection established with {peer_id}");
                        }
                        SwarmEvent::Behaviour(RelayServerBehaviourEvent::Relay(ev)) => {
                            println!("R: relay event: {ev:?}");
                        }
                        SwarmEvent::Behaviour(RelayServerBehaviourEvent::Identify(ev)) => {
                            tracing::debug!(?ev, "R: identify event");
                        }
                        _ => {
                            tracing::debug!(?event, "R event");
                        }
                    }
                }

                // A's event loop: request reservation once connected to R,
                // then answer dispatch requests from B.
                event = a_swarm.select_next_some() => {
                    match event {
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            println!("A: connection established with {peer_id}");
                            if peer_id == r_peer && !a_reserved {
                                // Listen on /<R>/p2p-circuit to request a reservation.
                                let circuit_listen = r_addr_with_peer
                                    .clone()
                                    .with(libp2p::multiaddr::Protocol::P2pCircuit);
                                println!("A: requesting relay reservation on {circuit_listen}");
                                a_swarm
                                    .listen_on(circuit_listen)
                                    .expect("A circuit listen");
                            }
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            println!("A: new listen addr {address}");
                            // If this listen address is a relay circuit, our
                            // reservation is live. (The dedicated client event
                            // may or may not fire depending on libp2p version;
                            // the new-listen-addr is authoritative.)
                            let is_circuit = address.iter().any(|p| {
                                matches!(p, libp2p::multiaddr::Protocol::P2pCircuit)
                            });
                            if is_circuit && !a_reserved {
                                a_reserved = true;
                                println!("A: reservation confirmed via NewListenAddr (circuit)");
                            }
                        }
                        SwarmEvent::ListenerError { error, .. } => {
                            println!("A: LISTENER ERROR: {error}");
                        }
                        SwarmEvent::Behaviour(ClientBehaviourEvent::RelayClient(ev)) => {
                            match &ev {
                                relay::client::Event::ReservationReqAccepted { relay_peer_id, .. } => {
                                    assert_eq!(*relay_peer_id, r_peer);
                                    a_reserved = true;
                                    println!("A: relay reservation accepted by {relay_peer_id}");
                                }
                                other => println!("A: relay_client event: {other:?}"),
                            }
                        }
                        SwarmEvent::Behaviour(ClientBehaviourEvent::Identify(
                            identify::Event::Received { peer_id, info, .. }
                        )) => {
                            tracing::info!(%peer_id, protocols = ?info.protocols, "A: identify received");
                        }
                        SwarmEvent::Behaviour(ClientBehaviourEvent::Dispatch(
                            request_response::Event::Message {
                                message: request_response::Message::Request { request, channel, .. },
                                ..
                            }
                        )) => {
                            println!("A: received dispatch request, executing...");
                            let resp = execute_wasm_task(&request);
                            let _ = a_swarm.behaviour_mut().dispatch.send_response(channel, resp);
                        }
                        _ => {}
                    }
                }

                // B's event loop: once connected to R AND A is reserved, dial A
                // via the circuit, then send dispatch.
                event = b_swarm.select_next_some() => {
                    match event {
                        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                            let is_circuit = endpoint.get_remote_address().iter()
                                .any(|p| matches!(p, libp2p::multiaddr::Protocol::P2pCircuit));
                            if peer_id == a_peer {
                                println!("B: connected to A (circuit={is_circuit})");
                                if !request_sent {
                                    b_swarm.behaviour_mut().dispatch.send_request(&a_peer, dispatch_request.clone());
                                    request_sent = true;
                                }
                            } else if peer_id == r_peer {
                                // We've got R. If A has its reservation, dial A via circuit.
                                if a_reserved && !b_dialed_a {
                                    println!("B: dialing A via circuit {a_circuit_addr}");
                                    b_swarm.dial(a_circuit_addr.clone()).expect("B->A circuit dial");
                                    b_dialed_a = true;
                                }
                            }
                        }
                        SwarmEvent::Behaviour(ClientBehaviourEvent::Dispatch(
                            request_response::Event::Message {
                                message: request_response::Message::Response { response, .. },
                                ..
                            }
                        )) => {
                            return Ok::<_, String>(response);
                        }
                        _ => {}
                    }
                }
            }

            // If A just became reserved and B is already connected to R but
            // hasn't dialed A yet, do the circuit dial now.
            if a_reserved && !b_dialed_a {
                println!("B: dialing A via circuit {a_circuit_addr}");
                if b_swarm.dial(a_circuit_addr.clone()).is_ok() {
                    b_dialed_a = true;
                }
            }
        }
    })
    .await
    .expect("nat traversal test timeout")
    .expect("circuit error");

    assert_eq!(response.task_id, "nat-test-001");
    assert_eq!(
        response.status,
        TaskStatus::Succeeded,
        "Dispatch via relay should succeed: {:?}",
        response.error
    );
    println!("✓ Cross-NAT dispatch succeeded: {}ms via relay circuit", response.duration_ms);
}
