//! CLI `worldcompute job` subcommand per FR-090 (T073).

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Job operations — submit, status, results, cancel, list")]
pub struct JobCli {
    #[command(subcommand)]
    pub command: JobCommand,
}

#[derive(Subcommand)]
pub enum JobCommand {
    /// Submit a job from a manifest file
    Submit {
        /// Path to the job manifest JSON file
        #[arg(value_name = "MANIFEST_PATH")]
        manifest_path: String,
        /// Optional executor multiaddr for direct P2P dispatch
        /// (e.g., `/ip4/10.232.6.17/tcp/19999/p2p/12D3KooW...`)
        #[arg(long)]
        executor: Option<String>,
        /// Optional path to a WASM workload to dispatch inline with the job
        #[arg(long)]
        workload: Option<String>,
    },
    /// Show status of a submitted job
    Status {
        /// Job ID to query
        #[arg(value_name = "JOB_ID")]
        job_id: String,
    },
    /// Retrieve results for a completed job
    Results {
        /// Job ID whose results to fetch
        #[arg(value_name = "JOB_ID")]
        job_id: String,
    },
    /// Cancel a pending or running job
    Cancel {
        /// Job ID to cancel
        #[arg(value_name = "JOB_ID")]
        job_id: String,
    },
    /// List all jobs for the current submitter
    List,
}

/// Execute a job CLI command. Returns a human-readable status string.
/// For `--executor` mode, use [`execute_remote_submit`] instead.
pub fn execute(cmd: &JobCommand) -> String {
    match cmd {
        JobCommand::Submit { manifest_path, executor, workload: _ } => {
            if executor.is_some() {
                return format!(
                    "Remote dispatch requested (executor={}). Use execute_remote_submit() for P2P dispatch.",
                    executor.as_ref().unwrap()
                );
            }
            match std::fs::read_to_string(manifest_path) {
                Ok(content) => {
                    match serde_json::from_str::<crate::scheduler::manifest::JobManifest>(&content)
                    {
                        Ok(manifest) => {
                            format!(
                                "Job validated.\n  Name: {}\n  Workload: {:?}\n  Inputs: {}\n  Use classes: {:?}\n  Submitted (awaiting coordinator connection).",
                                manifest.name, manifest.workload_type, manifest.inputs.len(), manifest.acceptable_use_classes
                            )
                        }
                        Err(e) => format!("Error: invalid manifest JSON: {e}"),
                    }
                }
                Err(e) => format!("Error: cannot read manifest file '{manifest_path}': {e}"),
            }
        }
        JobCommand::Status { job_id } => {
            format!("Job {job_id}: no coordinator connection. Start a donor node first.")
        }
        JobCommand::Results { job_id } => {
            format!("Job {job_id}: no results available (no coordinator connection).")
        }
        JobCommand::Cancel { job_id } => {
            format!("Job {job_id}: cannot cancel (no coordinator connection).")
        }
        JobCommand::List => "No jobs found (no coordinator connection).".into(),
    }
}

/// Submit a job to a remote executor over the P2P network.
/// Opens a short-lived libp2p connection, sends a TaskDispatch, waits for the
/// result, prints it, and exits.
pub async fn execute_remote_submit(cmd: &JobCommand) -> Result<(), Box<dyn std::error::Error>> {
    use libp2p::{
        futures::StreamExt,
        identify, identity, ping,
        request_response::{self, ProtocolSupport},
        swarm::{NetworkBehaviour, SwarmEvent},
        Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
    };
    use std::time::Duration;
    use tokio::time::timeout;

    use crate::network::dispatch::{
        TaskDispatchRequest, TaskDispatchResponse, TaskStatus, PROTOCOL_TASK_DISPATCH,
    };
    use crate::scheduler::manifest::JobManifest;

    let (manifest_path, executor_addr, workload_path) = match cmd {
        JobCommand::Submit { manifest_path, executor: Some(e), workload } => {
            (manifest_path.clone(), e.clone(), workload.clone())
        }
        _ => return Err("remote submit requires --executor".into()),
    };

    // Parse the manifest from disk.
    let manifest_json = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("reading manifest '{manifest_path}': {e}"))?;
    let manifest: JobManifest =
        serde_json::from_str(&manifest_json).map_err(|e| format!("parsing manifest JSON: {e}"))?;

    // Optional inline workload.
    let inline_inputs: Vec<(String, Vec<u8>)> = if let Some(path) = workload_path {
        let bytes = std::fs::read(&path).map_err(|e| format!("reading workload '{path}': {e}"))?;
        println!("Loaded workload '{path}' ({} bytes)", bytes.len());
        vec![("workload".to_string(), bytes)]
    } else {
        Vec::new()
    };

    // Parse executor multiaddr.
    let executor: Multiaddr = executor_addr
        .parse()
        .map_err(|e| format!("invalid executor multiaddr '{executor_addr}': {e}"))?;
    // Extract the peer id from the multiaddr.
    let executor_peer: PeerId = executor
        .iter()
        .find_map(|p| match p {
            libp2p::multiaddr::Protocol::P2p(peer) => Some(peer),
            _ => None,
        })
        .ok_or("executor multiaddr must include /p2p/<peer_id>")?;

    #[derive(NetworkBehaviour)]
    struct ClientBehaviour {
        dispatch: request_response::cbor::Behaviour<TaskDispatchRequest, TaskDispatchResponse>,
        identify: identify::Behaviour,
        ping: ping::Behaviour,
    }

    let keypair = identity::Keypair::generate_ed25519();
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|kp| ClientBehaviour {
            dispatch: request_response::cbor::Behaviour::new(
                std::iter::once((
                    StreamProtocol::new(PROTOCOL_TASK_DISPATCH),
                    ProtocolSupport::Full,
                )),
                request_response::Config::default().with_request_timeout(Duration::from_secs(300)),
            ),
            identify: identify::Behaviour::new(identify::Config::new(
                "/worldcompute/1.0.0".into(),
                kp.public(),
            )),
            ping: ping::Behaviour::new(ping::Config::new()),
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    println!("Dialing executor: {executor}");
    swarm.dial(executor.clone())?;

    let task_id = format!("task-{}", uuid::Uuid::new_v4());
    let request = TaskDispatchRequest { task_id: task_id.clone(), manifest, inline_inputs };

    let response: TaskDispatchResponse = timeout(Duration::from_secs(300), async {
        let mut sent = false;
        loop {
            let event = swarm.select_next_some().await;
            match event {
                SwarmEvent::ConnectionEstablished { peer_id, .. }
                    if peer_id == executor_peer && !sent =>
                {
                    println!("Connected to executor {peer_id}. Sending dispatch...");
                    swarm.behaviour_mut().dispatch.send_request(&executor_peer, request.clone());
                    sent = true;
                }
                SwarmEvent::Behaviour(ClientBehaviourEvent::Dispatch(
                    request_response::Event::Message {
                        message: request_response::Message::Response { response, .. },
                        ..
                    },
                )) => {
                    return Ok::<_, String>(response);
                }
                SwarmEvent::OutgoingConnectionError { error, .. } => {
                    return Err(format!("connection error: {error}"));
                }
                _ => {}
            }
        }
    })
    .await
    .map_err(|_| "timed out waiting for executor response")??;

    println!("\nResult:");
    println!("  task_id: {}", response.task_id);
    println!("  status: {:?}", response.status);
    println!("  duration_ms: {}", response.duration_ms);
    println!("  output_bytes: {}", response.output.len());
    if let Some(err) = &response.error {
        println!("  error: {err}");
    }
    if response.status != TaskStatus::Succeeded {
        return Err(format!("task did not succeed: {:?}", response.status).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_returns_manifest_path_in_message() {
        let test_path = std::env::temp_dir().join("job.json");
        let test_path_str = test_path.to_string_lossy().to_string();
        let msg = execute(&JobCommand::Submit {
            manifest_path: test_path_str.clone(),
            executor: None,
            workload: None,
        });
        assert!(msg.contains(&test_path_str));
    }

    #[test]
    fn status_returns_job_id_in_message() {
        let msg = execute(&JobCommand::Status { job_id: "job-abc-123".into() });
        assert!(msg.contains("job-abc-123"));
    }

    #[test]
    fn results_returns_job_id_in_message() {
        let msg = execute(&JobCommand::Results { job_id: "job-xyz-456".into() });
        assert!(msg.contains("job-xyz-456"));
    }

    #[test]
    fn cancel_returns_job_id_in_message() {
        let msg = execute(&JobCommand::Cancel { job_id: "job-def-789".into() });
        assert!(msg.contains("job-def-789"));
    }

    #[test]
    fn list_returns_nonempty_message() {
        let msg = execute(&JobCommand::List);
        assert!(!msg.is_empty());
    }
}
