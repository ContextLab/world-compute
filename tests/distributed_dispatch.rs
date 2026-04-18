//! End-to-end distributed job dispatch test.
//!
//! Spawns two daemons in-process, connects them via localhost, submits a real
//! WASM job from one to the other via the TaskDispatch request-response protocol,
//! and verifies the result is returned correctly.

use libp2p::{
    futures::StreamExt,
    identify, identity, ping,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use std::time::Duration;
use tokio::time::timeout;

use worldcompute::network::dispatch::{
    TaskDispatchRequest, TaskDispatchResponse, PROTOCOL_TASK_DISPATCH,
};
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

/// Minimal test behaviour: just dispatch + identify + ping. We don't need the
/// full NAT stack for an in-process test.
#[derive(NetworkBehaviour)]
struct TestBehaviour {
    dispatch: request_response::cbor::Behaviour<TaskDispatchRequest, TaskDispatchResponse>,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
}

fn build_swarm(keypair: identity::Keypair) -> libp2p::Swarm<TestBehaviour> {
    SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("tcp")
        .with_behaviour(|kp| TestBehaviour {
            dispatch: request_response::cbor::Behaviour::new(
                std::iter::once((
                    StreamProtocol::new(PROTOCOL_TASK_DISPATCH),
                    ProtocolSupport::Full,
                )),
                request_response::Config::default()
                    .with_request_timeout(Duration::from_secs(30)),
            ),
            identify: identify::Behaviour::new(identify::Config::new(
                "/test/1.0.0".into(),
                kp.public(),
            )),
            ping: ping::Behaviour::new(ping::Config::new()),
        })
        .expect("behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
        .build()
}

fn make_test_manifest(workload_cid: cid::Cid) -> JobManifest {
    JobManifest {
        manifest_cid: None,
        name: "distributed-test".into(),
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

#[tokio::test]
async fn distributed_wasm_job_dispatch_end_to_end() {
    // Minimal valid WASM module
    let wasm_bytes: Vec<u8> = vec![
        0x00, 0x61, 0x73, 0x6d, // magic
        0x01, 0x00, 0x00, 0x00, // version 1
    ];
    let workload_cid = worldcompute::data_plane::cid_store::compute_cid(&wasm_bytes)
        .expect("compute cid");

    // Executor daemon
    let executor_kp = identity::Keypair::generate_ed25519();
    let executor_peer = PeerId::from(executor_kp.public());
    let mut executor = build_swarm(executor_kp);
    let listen: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    executor.listen_on(listen).expect("executor listen");

    // Wait for executor to obtain a listen address
    let executor_addr = loop {
        match executor.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                break address.with(libp2p::multiaddr::Protocol::P2p(executor_peer));
            }
            _ => continue,
        }
    };

    // Broker daemon
    let broker_kp = identity::Keypair::generate_ed25519();
    let mut broker = build_swarm(broker_kp);

    // Dial executor
    broker.dial(executor_addr.clone()).expect("dial");

    // Build dispatch request
    let request = TaskDispatchRequest {
        task_id: "dist-test-001".into(),
        manifest: make_test_manifest(workload_cid),
        inline_inputs: vec![("workload".to_string(), wasm_bytes.clone())],
    };

    // Drive event loops until broker sends request + receives response, or timeout
    let result = timeout(Duration::from_secs(30), async {
        let mut request_sent = false;

        loop {
            tokio::select! {
                event = broker.select_next_some() => {
                    match event {
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            if peer_id == executor_peer && !request_sent {
                                broker.behaviour_mut().dispatch.send_request(&executor_peer, request.clone());
                                request_sent = true;
                            }
                        }
                        SwarmEvent::Behaviour(TestBehaviourEvent::Dispatch(
                            request_response::Event::Message {
                                message: request_response::Message::Response { response, .. },
                                ..
                            }
                        )) => {
                            return response;
                        }
                        _ => {}
                    }
                }
                event = executor.select_next_some() => {
                    if let SwarmEvent::Behaviour(TestBehaviourEvent::Dispatch(
                        request_response::Event::Message {
                            message: request_response::Message::Request { request, channel, .. },
                            ..
                        }
                    )) = event
                    {
                        // Actually run the task using the daemon's execution path logic
                        let resp = execute_task_for_test(&request);
                        let _ = executor.behaviour_mut().dispatch.send_response(channel, resp);
                    }
                }
            }
        }
    })
    .await;

    let response: TaskDispatchResponse =
        result.expect("timed out waiting for dispatch response");
    assert_eq!(response.task_id, "dist-test-001");
    assert_eq!(
        response.status,
        worldcompute::network::dispatch::TaskStatus::Succeeded,
        "error: {:?}",
        response.error
    );
}

/// Execute a task locally for the test — mirrors the daemon's logic but uses
/// the inline_inputs fallback so we don't need a shared CID store.
fn execute_task_for_test(req: &TaskDispatchRequest) -> TaskDispatchResponse {
    use std::time::Instant;
    use worldcompute::network::dispatch::TaskStatus;

    let start = Instant::now();
    let wasm_bytes = req
        .inline_inputs
        .iter()
        .find(|(n, _)| n == "workload")
        .map(|(_, b)| b.clone());

    let wasm_bytes = match wasm_bytes {
        Some(b) => b,
        None => {
            return TaskDispatchResponse {
                task_id: req.task_id.clone(),
                status: TaskStatus::Failed,
                output: Vec::new(),
                output_cid: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some("no inline workload".into()),
            };
        }
    };

    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");

    match worldcompute::sandbox::wasm::compile_module(&engine, &wasm_bytes) {
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
                error: Some(e.to_string()),
            },
        },
        Err(e) => TaskDispatchResponse {
            task_id: req.task_id.clone(),
            status: TaskStatus::Failed,
            output: Vec::new(),
            output_cid: None,
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some(e.to_string()),
        },
    }
}
