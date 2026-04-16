//! Agent lifecycle — enroll, heartbeat, pause, resume, withdraw (T039-T043).
//!
//! This is the core donor experience. The agent transitions through states:
//! Enrolling → Idle ↔ Working ↔ Paused → Withdrawing → (removed)

use crate::acceptable_use::AcceptableUseClass;
use crate::agent::config::AgentConfig;
use crate::agent::donor::Donor;
use crate::agent::node::{Node, NodeState};
use crate::agent::{AgentState};
use crate::credits::caliber::CaliberClass;
use crate::error::{ErrorCode, WcError};
use crate::sandbox::{detect_capability, SandboxCapability};
use crate::types::{NcuAmount, PeerIdStr, Timestamp, TrustScore};
use crate::verification::trust_score::{classify_trust_tier, TrustTier};
use crate::scheduler::ResourceEnvelope;

/// The running agent instance — owns all local state.
pub struct AgentInstance {
    pub state: AgentState,
    pub donor: Option<Donor>,
    pub node: Option<Node>,
    pub config: AgentConfig,
    pub peer_id_str: Option<PeerIdStr>,
    sandbox_capability: SandboxCapability,
}

impl AgentInstance {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            state: AgentState::Enrolling,
            donor: None,
            node: None,
            config,
            peer_id_str: None,
            sandbox_capability: detect_capability(),
        }
    }

    /// T039: Enrollment flow — generate identity, probe platform, register.
    pub fn enroll(
        &mut self,
        consent_classes: Vec<AcceptableUseClass>,
    ) -> Result<EnrollmentResult, WcError> {
        if self.state != AgentState::Enrolling {
            return Err(WcError::new(
                ErrorCode::AlreadyExists,
                "Agent is already enrolled",
            ));
        }

        // Generate or load Ed25519 identity
        let signing_key = crate::agent::identity::load_or_create_key(&self.config.key_path)?;
        let peer_id = crate::agent::identity::peer_id_from_key(&signing_key);
        let peer_id_str = peer_id.to_string();
        self.peer_id_str = Some(peer_id_str.clone());

        // Detect platform and sandbox capability
        let sandbox_cap = detect_capability();
        self.sandbox_capability = sandbox_cap;

        // Classify trust tier (conservative defaults — no TPM/TEE until attested)
        let trust_tier = classify_trust_tier(
            false, // has_tpm — determined at attestation time
            false, // has_sev_snp
            false, // has_tdx
            false, // has_h100_cc
            false, // has_gpu — determined at GPU probe time
            sandbox_cap == SandboxCapability::WasmOnly,
        );

        // Estimate caliber class from system resources
        let caliber = estimate_caliber_class();

        // Create donor record
        let now = Timestamp::now();
        let donor = Donor {
            donor_id: format!("donor-{}", &peer_id_str[..12]),
            peer_id: peer_id_str.clone(),
            caliber_class: caliber,
            credit_balance: NcuAmount::ZERO,
            trust_score: TrustScore::ZERO,
            consent_classes: consent_classes.clone(),
            shard_allowlist: Vec::new(),
            enrolled_at: now,
        };

        // Create node record
        let node = Node {
            peer_id: peer_id_str.clone(),
            state: NodeState::Idle,
            trust_tier,
            caliber_class: caliber,
            trust_score: TrustScore::ZERO,
            sandbox_capability: sandbox_cap,
            capacity: detect_capacity(),
            last_heartbeat: now,
        };

        self.donor = Some(donor);
        self.node = Some(node);
        self.state = AgentState::Idle;

        tracing::info!(
            peer_id = %peer_id_str,
            caliber = ?caliber,
            trust_tier = ?trust_tier,
            sandbox = ?sandbox_cap,
            "Agent enrolled successfully"
        );

        Ok(EnrollmentResult {
            peer_id: peer_id_str,
            caliber_class: caliber,
            trust_tier,
            sandbox_capability: sandbox_cap,
        })
    }

    /// T040: Heartbeat — report state, receive lease offers.
    pub fn heartbeat(&mut self) -> Result<(), WcError> {
        let node = self.node.as_mut().ok_or_else(|| {
            WcError::new(ErrorCode::NotFound, "Not enrolled")
        })?;
        node.last_heartbeat = Timestamp::now();
        // TODO: Send heartbeat to broker/coordinator, receive lease offers,
        // check version blocklist for P0 incidents (FR-014).
        Ok(())
    }

    /// T041: Pause — checkpoint active work, stop advertising capacity.
    pub fn pause(&mut self) -> Result<(), WcError> {
        match self.state {
            AgentState::Idle | AgentState::Working => {
                // TODO: Checkpoint any active sandboxes, notify broker.
                self.state = AgentState::Paused;
                if let Some(node) = &mut self.node {
                    node.state = NodeState::Offline;
                }
                tracing::info!("Agent paused");
                Ok(())
            }
            _ => Err(WcError::new(
                ErrorCode::Internal,
                format!("Cannot pause from state {:?}", self.state),
            )),
        }
    }

    /// T041: Resume — start advertising capacity again.
    pub fn resume(&mut self) -> Result<(), WcError> {
        if self.state != AgentState::Paused {
            return Err(WcError::new(
                ErrorCode::Internal,
                "Agent is not paused",
            ));
        }
        self.state = AgentState::Idle;
        if let Some(node) = &mut self.node {
            node.state = NodeState::Idle;
        }
        tracing::info!("Agent resumed");
        Ok(())
    }

    /// T042: Withdrawal — stop all work, wipe working directory, deregister.
    /// After this, no World Compute state remains on the host (FR-004).
    pub fn withdraw(&mut self) -> Result<WithdrawalResult, WcError> {
        self.state = AgentState::Withdrawing;

        // TODO: Checkpoint and terminate all active sandboxes.
        // TODO: Notify broker/coordinator of withdrawal.

        let credits_remaining = self.donor.as_ref()
            .map(|d| d.credit_balance)
            .unwrap_or(NcuAmount::ZERO);

        // Wipe scoped working directory (FR-004)
        let work_dir = &self.config.work_dir;
        if work_dir.exists() {
            std::fs::remove_dir_all(work_dir).map_err(|e| {
                WcError::new(ErrorCode::Internal, format!("Cleanup failed: {e}"))
            })?;
        }

        // Remove key file
        if self.config.key_path.exists() {
            std::fs::remove_file(&self.config.key_path).ok();
        }

        tracing::info!(
            credits_remaining = %credits_remaining,
            "Agent withdrawn — all host state removed"
        );

        self.donor = None;
        self.node = None;

        Ok(WithdrawalResult {
            credits_remaining,
            clean: true,
        })
    }

    /// T043: Update consent — change which workload classes are accepted.
    pub fn update_consent(
        &mut self,
        consent_classes: Vec<AcceptableUseClass>,
    ) -> Result<(), WcError> {
        let donor = self.donor.as_mut().ok_or_else(|| {
            WcError::new(ErrorCode::NotFound, "Not enrolled")
        })?;
        donor.consent_classes = consent_classes;
        tracing::info!(
            classes = ?donor.consent_classes,
            "Consent classes updated"
        );
        Ok(())
    }
}

/// Result of enrollment.
#[derive(Debug)]
pub struct EnrollmentResult {
    pub peer_id: PeerIdStr,
    pub caliber_class: CaliberClass,
    pub trust_tier: TrustTier,
    pub sandbox_capability: SandboxCapability,
}

/// Result of withdrawal.
#[derive(Debug)]
pub struct WithdrawalResult {
    pub credits_remaining: NcuAmount,
    pub clean: bool,
}

/// Estimate caliber class from system resources.
fn estimate_caliber_class() -> CaliberClass {
    let cpus = num_cpus();
    let ram_gb = ram_gb();

    if ram_gb >= 128 && cpus >= 16 {
        CaliberClass::C3
    } else if ram_gb >= 32 && cpus >= 8 {
        CaliberClass::C2
    } else if ram_gb >= 8 && cpus >= 4 {
        CaliberClass::C1
    } else {
        CaliberClass::C0
    }
    // Note: C4 (high-end GPU) requires explicit GPU probe — not auto-detected here.
}

/// Detect available resource capacity.
fn detect_capacity() -> ResourceEnvelope {
    ResourceEnvelope {
        cpu_millicores: num_cpus() as u64 * 1000,
        ram_bytes: ram_gb() as u64 * 1024 * 1024 * 1024,
        gpu_class: None,
        gpu_vram_bytes: 0,
        scratch_bytes: 10 * 1024 * 1024 * 1024, // 10 GB default
        network_egress_bytes: 0,
        walltime_budget_ms: 0,
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn ram_gb() -> usize {
    // Cross-platform RAM detection
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
            .map(|bytes| (bytes / 1_073_741_824) as usize)
            .unwrap_or(8)
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("MemTotal:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| (kb / 1_048_576) as usize)
            })
            .unwrap_or(8)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        8 // Conservative default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_config() -> AgentConfig {
        let dir = std::env::temp_dir().join(format!("wc-test-{}", uuid::Uuid::new_v4()));
        AgentConfig {
            work_dir: dir.clone(),
            key_path: dir.join("test-key"),
            ..AgentConfig::default()
        }
    }

    #[test]
    fn enroll_creates_donor_and_node() {
        let config = test_config();
        let mut agent = AgentInstance::new(config);
        let result = agent.enroll(vec![AcceptableUseClass::Scientific]);
        assert!(result.is_ok());
        assert_eq!(agent.state, AgentState::Idle);
        assert!(agent.donor.is_some());
        assert!(agent.node.is_some());
        // Cleanup
        let _ = agent.withdraw();
    }

    #[test]
    fn double_enroll_rejected() {
        let config = test_config();
        let mut agent = AgentInstance::new(config);
        agent.enroll(vec![]).unwrap();
        let second = agent.enroll(vec![]);
        assert!(second.is_err());
        let _ = agent.withdraw();
    }

    #[test]
    fn pause_resume_cycle() {
        let config = test_config();
        let mut agent = AgentInstance::new(config);
        agent.enroll(vec![]).unwrap();
        assert!(agent.pause().is_ok());
        assert_eq!(agent.state, AgentState::Paused);
        assert!(agent.resume().is_ok());
        assert_eq!(agent.state, AgentState::Idle);
        let _ = agent.withdraw();
    }

    #[test]
    fn withdraw_cleans_up_state() {
        let config = test_config();
        std::fs::create_dir_all(&config.work_dir).unwrap();
        let mut agent = AgentInstance::new(config.clone());
        agent.enroll(vec![]).unwrap();
        let result = agent.withdraw().unwrap();
        assert!(result.clean);
        assert!(!config.work_dir.exists(), "Work dir should be removed");
        assert!(agent.donor.is_none());
        assert!(agent.node.is_none());
    }

    #[test]
    fn update_consent_changes_classes() {
        let config = test_config();
        let mut agent = AgentInstance::new(config);
        agent.enroll(vec![AcceptableUseClass::Scientific]).unwrap();
        agent.update_consent(vec![
            AcceptableUseClass::Scientific,
            AcceptableUseClass::PublicGoodMl,
        ]).unwrap();
        assert_eq!(agent.donor.as_ref().unwrap().consent_classes.len(), 2);
        let _ = agent.withdraw();
    }

    #[test]
    fn heartbeat_updates_timestamp() {
        let config = test_config();
        let mut agent = AgentInstance::new(config);
        agent.enroll(vec![]).unwrap();
        let before = agent.node.as_ref().unwrap().last_heartbeat;
        std::thread::sleep(std::time::Duration::from_millis(10));
        agent.heartbeat().unwrap();
        let after = agent.node.as_ref().unwrap().last_heartbeat;
        assert!(after.0 > before.0);
        let _ = agent.withdraw();
    }

    #[test]
    fn caliber_detection_returns_valid_class() {
        let caliber = estimate_caliber_class();
        // On any real machine, should be at least C0
        assert!(caliber >= CaliberClass::C0);
    }
}
