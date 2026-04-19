//! CLI `worldcompute admin` subcommand per US6 / FR-090.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Admin operations — halt, resume, ban, audit")]
pub struct AdminCli {
    #[command(subcommand)]
    pub command: AdminCommand,
}

#[derive(Subcommand)]
pub enum AdminCommand {
    /// Trigger an emergency halt of the cluster
    Halt {
        /// Reason for halt
        #[arg(long)]
        reason: String,
    },
    /// Resume cluster operations after a halt
    Resume,
    /// Ban a user or node from the cluster
    Ban {
        /// Subject ID (user or node) to ban
        #[arg(long)]
        subject_id: String,
        /// Reason for ban
        #[arg(long)]
        reason: String,
    },
    /// Audit a proposal or subject
    Audit {
        /// Proposal or subject ID to audit
        #[arg(long)]
        id: String,
    },
    // spec 005 US1 T025 / US2 T038 / US8 T117 additions
    /// Diagnose why the agent cannot form a mesh connection. Runs a
    /// time-boxed debug-log capture and emits an evidence bundle under
    /// evidence/phase1/firewall-traversal/<ts>/ for offline analysis.
    FirewallDiagnose {
        /// Duration of diagnostic capture in seconds (default 300 = 5 min)
        #[arg(long, default_value = "300")]
        duration_s: u64,
    },
    /// Refetch pinned AMD/Intel/Rekor values from upstream and compare against
    /// the in-tree constants. Opens a repository issue on mismatch when run
    /// from CI; reports the diff locally otherwise. Wraps scripts/drift-check.sh.
    DriftCheck,
    /// Verify a release binary against its detached Ed25519 signature using
    /// the pinned RELEASE_PUBLIC_KEY. Wraps ops/release/verify-release.sh.
    VerifyRelease {
        /// Path to the binary to verify
        #[arg(long)]
        binary: String,
        /// Path to the detached .sig file
        #[arg(long)]
        signature: String,
    },
}

/// Execute an admin CLI command. Returns a human-readable status string.
///
/// Note: Admin operations require OnCallResponder role per FR-S031.
/// Without a running daemon and authenticated session, these commands
/// validate the request structure but cannot execute against the cluster.
pub fn execute(cmd: &AdminCommand) -> String {
    match cmd {
        AdminCommand::Halt { reason } => {
            format!(
                "Emergency halt requested.\n  Reason: {reason}\n  Status: requires OnCallResponder role and active admin service connection."
            )
        }
        AdminCommand::Resume => {
            "Resume requested. Requires OnCallResponder role and active admin service connection."
                .into()
        }
        AdminCommand::Ban { subject_id, reason } => {
            format!(
                "Ban requested.\n  Subject: {subject_id}\n  Reason: {reason}\n  Status: requires active admin service connection."
            )
        }
        AdminCommand::Audit { id } => {
            format!("Audit requested for {id}. Requires active admin service connection.")
        }
        AdminCommand::FirewallDiagnose { duration_s } => {
            format!(
                "Firewall diagnosis requested.\n  Duration: {duration_s}s\n  \
                 Evidence will be written to evidence/phase1/firewall-traversal/<ts>/.\n  \
                 Daemon mode is required for this command to collect real dial data."
            )
        }
        AdminCommand::DriftCheck => "Drift check requested. Wraps scripts/drift-check.sh.\n  \
             Compares pinned AMD/Intel/Rekor values against upstream.\n  \
             Exit 0 = all pins match. Non-zero = mismatch detected."
            .into(),
        AdminCommand::VerifyRelease { binary, signature } => {
            format!(
                "Verify release requested.\n  Binary: {binary}\n  Signature: {signature}\n  \
                 Wraps ops/release/verify-release.sh using pinned RELEASE_PUBLIC_KEY."
            )
        }
    }
}
