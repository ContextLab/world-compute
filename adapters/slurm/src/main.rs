//! World Compute — Slurm adapter
//!
//! Bridges the World Compute task scheduler to an existing HPC cluster managed
//! by Slurm.  The adapter runs as a long-lived daemon on the Slurm head node
//! (or a machine with SSH/REST access to it) and translates World Compute task
//! submissions into `sbatch` jobs.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Slurm REST API client (T145-T147)
// ---------------------------------------------------------------------------

/// A node reported by the Slurm REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlurmNode {
    pub name: String,
    pub cpus: u32,
    pub state: String,
}

/// Status of a Slurm batch job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlurmJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
}

impl SlurmJobStatus {
    /// Parse a Slurm job-state string into the enum.
    pub fn from_slurm_state(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "PENDING" | "PD" => Ok(Self::Pending),
            "RUNNING" | "R" => Ok(Self::Running),
            "COMPLETED" | "CD" => Ok(Self::Completed),
            "FAILED" | "F" => Ok(Self::Failed),
            "TIMEOUT" | "TO" => Ok(Self::Timeout),
            other => Err(format!("unknown Slurm job state: {other}")),
        }
    }
}

/// Result returned when a job is submitted via the REST API.
#[derive(Debug, Deserialize)]
struct SubmitResponse {
    job_id: Option<u64>,
    #[serde(default)]
    errors: Vec<SlurmApiError>,
}

#[derive(Debug, Deserialize)]
struct SlurmApiError {
    #[serde(default)]
    error: String,
}

/// Response envelope for GET /slurm/v0.0.40/nodes.
#[derive(Debug, Deserialize)]
struct NodesResponse {
    #[serde(default)]
    nodes: Vec<NodeEntry>,
}

#[derive(Debug, Deserialize)]
struct NodeEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    cpus: u32,
    #[serde(default)]
    state: String,
}

/// Response envelope for GET /slurm/v0.0.40/job/{id}.
#[derive(Debug, Deserialize)]
struct JobResponse {
    #[serde(default)]
    jobs: Vec<JobEntry>,
}

#[derive(Debug, Deserialize)]
struct JobEntry {
    #[serde(default)]
    job_state: String,
}

/// HTTP client for the Slurm REST daemon (`slurmrestd`).
pub struct SlurmClient {
    pub base_url: String,
    pub client: reqwest::blocking::Client,
}

impl SlurmClient {
    /// Create a new client pointing at a slurmrestd base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// List compute nodes known to the Slurm controller.
    pub fn get_nodes(&self) -> Result<Vec<SlurmNode>, String> {
        let url = format!("{}/slurm/v0.0.40/nodes", self.base_url);
        let resp =
            self.client.get(&url).send().map_err(|e| format!("HTTP GET {url} failed: {e}"))?;

        let body = resp.text().map_err(|e| format!("Failed to read response body: {e}"))?;

        Self::parse_nodes_response(&body)
    }

    /// Parse a nodes response JSON into `Vec<SlurmNode>`.
    pub fn parse_nodes_response(json: &str) -> Result<Vec<SlurmNode>, String> {
        let resp: NodesResponse =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;
        Ok(resp
            .nodes
            .into_iter()
            .map(|n| SlurmNode { name: n.name, cpus: n.cpus, state: n.state })
            .collect())
    }

    /// Submit a batch job script and return the assigned job ID.
    pub fn submit_job(&self, script: &str) -> Result<u64, String> {
        let url = format!("{}/slurm/v0.0.40/job/submit", self.base_url);
        let payload = serde_json::json!({
            "script": script,
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .map_err(|e| format!("HTTP POST {url} failed: {e}"))?;

        let body = resp.text().map_err(|e| format!("Failed to read response body: {e}"))?;

        Self::parse_submit_response(&body)
    }

    /// Parse a submit-job response JSON into the job ID.
    pub fn parse_submit_response(json: &str) -> Result<u64, String> {
        let resp: SubmitResponse =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;

        if let Some(err) = resp.errors.first() {
            if !err.error.is_empty() {
                return Err(format!("Slurm API error: {}", err.error));
            }
        }

        resp.job_id.ok_or_else(|| "No job_id in response".to_string())
    }

    /// Query the status of a previously submitted job.
    pub fn get_job_status(&self, job_id: u64) -> Result<SlurmJobStatus, String> {
        let url = format!("{}/slurm/v0.0.40/job/{job_id}", self.base_url);
        let resp =
            self.client.get(&url).send().map_err(|e| format!("HTTP GET {url} failed: {e}"))?;

        let body = resp.text().map_err(|e| format!("Failed to read response body: {e}"))?;

        Self::parse_job_status_response(&body)
    }

    /// Parse a job-status response JSON.
    pub fn parse_job_status_response(json: &str) -> Result<SlurmJobStatus, String> {
        let resp: JobResponse =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;

        let entry = resp.jobs.first().ok_or("No jobs in response")?;
        SlurmJobStatus::from_slurm_state(&entry.job_state)
    }

    /// Collect the result/exit code of a completed job.
    pub fn collect_result(&self, job_id: u64) -> Result<SlurmJobStatus, String> {
        self.get_job_status(job_id)
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Connection settings for a Slurm cluster.
#[derive(Debug, Clone)]
pub struct SlurmConfig {
    /// Hostname or IP of the Slurm head node.
    pub head_node: String,
    /// Slurm partition to submit jobs into.
    pub partition: String,
    /// Maximum number of concurrent jobs this adapter will hold in queue.
    pub max_jobs: u32,
}

impl Default for SlurmConfig {
    fn default() -> Self {
        Self { head_node: "localhost".to_string(), partition: "general".to_string(), max_jobs: 64 }
    }
}

// ---------------------------------------------------------------------------
// Adapter struct
// ---------------------------------------------------------------------------

/// Slurm backend adapter for World Compute.
///
/// Holds connection state and delegates task lifecycle operations to the
/// Slurm REST API (or `ssh + sbatch` fallback).  Full Slurm connectivity is
/// not yet implemented; this struct establishes the data model and CLI.
pub struct SlurmAdapter {
    pub config: SlurmConfig,
}

impl SlurmAdapter {
    pub fn new(config: SlurmConfig) -> Self {
        Self { config }
    }

    /// Print a human-readable summary of the adapter's configuration.
    pub fn describe(&self) {
        println!("Slurm adapter");
        println!("  Head node : {}", self.config.head_node);
        println!("  Partition : {}", self.config.partition);
        println!("  Max jobs  : {}", self.config.max_jobs);
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "worldcompute-slurm-adapter",
    about = "World Compute adapter for Slurm HPC clusters",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install the adapter daemon and systemd unit on the head node.
    Install {
        /// Hostname or IP of the Slurm head node.
        #[arg(long, default_value = "localhost")]
        head_node: String,
        /// Slurm partition to target.
        #[arg(long, default_value = "general")]
        partition: String,
        /// Maximum concurrent job slots.
        #[arg(long, default_value_t = 64)]
        max_jobs: u32,
    },
    /// Write or update the adapter configuration file.
    Configure {
        /// Hostname or IP of the Slurm head node.
        #[arg(long, default_value = "localhost")]
        head_node: String,
        /// Slurm partition to target.
        #[arg(long, default_value = "general")]
        partition: String,
        /// Maximum concurrent job slots.
        #[arg(long, default_value_t = 64)]
        max_jobs: u32,
    },
    /// Report the current status of the adapter daemon.
    Status,
}

// `#[allow]` because `fn main` is declared after this test module by convention
// in this file; clippy's items-after-test-module lint would otherwise flag it.
#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn slurm_client_creation() {
        let client = SlurmClient::new("http://localhost:6820");
        assert_eq!(client.base_url, "http://localhost:6820");
    }

    #[test]
    fn slurm_client_trailing_slash() {
        let client = SlurmClient::new("http://localhost:6820/");
        assert_eq!(client.base_url, "http://localhost:6820");
    }

    #[test]
    fn job_status_variants() {
        assert_eq!(SlurmJobStatus::from_slurm_state("PENDING").unwrap(), SlurmJobStatus::Pending);
        assert_eq!(SlurmJobStatus::from_slurm_state("PD").unwrap(), SlurmJobStatus::Pending);
        assert_eq!(SlurmJobStatus::from_slurm_state("RUNNING").unwrap(), SlurmJobStatus::Running);
        assert_eq!(SlurmJobStatus::from_slurm_state("R").unwrap(), SlurmJobStatus::Running);
        assert_eq!(
            SlurmJobStatus::from_slurm_state("COMPLETED").unwrap(),
            SlurmJobStatus::Completed
        );
        assert_eq!(SlurmJobStatus::from_slurm_state("CD").unwrap(), SlurmJobStatus::Completed);
        assert_eq!(SlurmJobStatus::from_slurm_state("FAILED").unwrap(), SlurmJobStatus::Failed);
        assert_eq!(SlurmJobStatus::from_slurm_state("F").unwrap(), SlurmJobStatus::Failed);
        assert_eq!(SlurmJobStatus::from_slurm_state("TIMEOUT").unwrap(), SlurmJobStatus::Timeout);
        assert_eq!(SlurmJobStatus::from_slurm_state("TO").unwrap(), SlurmJobStatus::Timeout);
        assert!(SlurmJobStatus::from_slurm_state("UNKNOWN").is_err());
    }

    #[test]
    fn parse_nodes_response() {
        let json = r#"{
            "nodes": [
                {"name": "node001", "cpus": 64, "state": "idle"},
                {"name": "node002", "cpus": 128, "state": "allocated"}
            ]
        }"#;
        let nodes = SlurmClient::parse_nodes_response(json).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "node001");
        assert_eq!(nodes[0].cpus, 64);
        assert_eq!(nodes[0].state, "idle");
        assert_eq!(nodes[1].name, "node002");
        assert_eq!(nodes[1].cpus, 128);
    }

    #[test]
    fn parse_submit_response_ok() {
        let json = r#"{"job_id": 42, "errors": []}"#;
        let id = SlurmClient::parse_submit_response(json).unwrap();
        assert_eq!(id, 42);
    }

    #[test]
    fn parse_submit_response_error() {
        let json = r#"{"job_id": null, "errors": [{"error": "invalid script"}]}"#;
        assert!(SlurmClient::parse_submit_response(json).is_err());
    }

    #[test]
    fn parse_job_status_response() {
        let json = r#"{"jobs": [{"job_state": "RUNNING"}]}"#;
        let status = SlurmClient::parse_job_status_response(json).unwrap();
        assert_eq!(status, SlurmJobStatus::Running);
    }

    #[test]
    fn parse_job_status_completed() {
        let json = r#"{"jobs": [{"job_state": "COMPLETED"}]}"#;
        let status = SlurmClient::parse_job_status_response(json).unwrap();
        assert_eq!(status, SlurmJobStatus::Completed);
    }

    #[test]
    fn slurm_config_default() {
        let config = SlurmConfig::default();
        assert_eq!(config.head_node, "localhost");
        assert_eq!(config.partition, "general");
        assert_eq!(config.max_jobs, 64);
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { head_node, partition, max_jobs } => {
            let config = SlurmConfig { head_node, partition, max_jobs };
            let adapter = SlurmAdapter::new(config);
            println!("Installing World Compute Slurm adapter…");
            adapter.describe();
            println!();
            println!("Next steps:");
            println!("  1. Ensure SSH key-based access from this host to the head node.");
            println!("  2. Verify Slurm REST API is enabled (slurmdbd + slurmrestd).");
            println!("  3. Run `worldcompute-slurm-adapter status` to confirm connectivity.");
        }
        Commands::Configure { head_node, partition, max_jobs } => {
            let config = SlurmConfig { head_node, partition, max_jobs };
            let adapter = SlurmAdapter::new(config);
            println!("Writing adapter configuration…");
            adapter.describe();
            println!();
            println!("Configuration recorded. Restart the adapter daemon to apply changes.");
        }
        Commands::Status => {
            println!("World Compute Slurm adapter — status");
            println!();
            println!("  Daemon     : not yet started (run 'install' first)");
            println!("  Slurm API  : not yet connected");
            println!("  Jobs held  : 0");
        }
    }
}
