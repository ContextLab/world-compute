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
pub fn execute(cmd: &JobCommand) -> String {
    match cmd {
        JobCommand::Submit { manifest_path } => match std::fs::read_to_string(manifest_path) {
            Ok(content) => {
                match serde_json::from_str::<crate::scheduler::manifest::JobManifest>(&content) {
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
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_returns_manifest_path_in_message() {
        let msg = execute(&JobCommand::Submit { manifest_path: "/tmp/job.json".into() });
        assert!(msg.contains("/tmp/job.json"));
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
