//! Regional broker scaffold — node roster management and task matching (T078).
//!
//! The broker is the regional intermediary between job submitters and worker nodes.
//! It maintains a roster of available nodes and matches task requirements to
//! eligible nodes based on declared capabilities.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::scheduler::ResourceEnvelope;
use crate::types::{AttestationQuote, PeerIdStr};
use crate::verification::attestation::{self, MeasurementRegistry};
use serde::{Deserialize, Serialize};

/// Information about a node registered with the broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Peer ID string for this node.
    pub peer_id: PeerIdStr,
    /// Human-readable region tag (e.g., "us-east-1", "eu-west-2").
    pub region_code: String,
    /// Declared available capacity.
    pub capacity: ResourceEnvelope,
    /// Trust tier (1 = basic, 2 = attested, 3 = TEE).
    pub trust_tier: u8,
    /// Whether this node's attestation has been verified.
    pub attestation_verified: bool,
    /// When attestation was last verified (microseconds since epoch).
    pub attestation_verified_at: Option<u64>,
}

/// Minimum resource requirements for task placement.
#[derive(Debug, Clone)]
pub struct TaskRequirements {
    /// Minimum CPU millicores needed.
    pub min_cpu_millicores: u64,
    /// Minimum RAM bytes needed.
    pub min_ram_bytes: u64,
    /// Minimum scratch storage bytes needed.
    pub min_scratch_bytes: u64,
    /// Minimum trust tier required.
    pub min_trust_tier: u8,
}

/// Regional broker — manages a roster of worker nodes and matches tasks to nodes.
#[derive(Debug)]
pub struct Broker {
    /// Unique identifier for this broker instance.
    pub broker_id: String,
    /// Geographic/cloud region this broker manages.
    pub region_code: String,
    /// Active node roster — nodes that have registered and are eligible.
    pub node_roster: Vec<NodeInfo>,
    /// Standby pool — nodes registered but currently unavailable (draining, etc.).
    pub standby_pool: Vec<NodeInfo>,
    /// Frozen hosts — removed from scheduling due to incident response.
    pub frozen_hosts: Vec<PeerIdStr>,
}

impl Broker {
    /// Create a new broker for the given region.
    pub fn new(broker_id: impl Into<String>, region_code: impl Into<String>) -> Self {
        Self {
            broker_id: broker_id.into(),
            region_code: region_code.into(),
            node_roster: Vec::new(),
            standby_pool: Vec::new(),
            frozen_hosts: Vec::new(),
        }
    }

    /// Register a node into the active roster.
    ///
    /// Returns `AlreadyExists` if a node with the same peer_id is already registered.
    pub fn register_node(&mut self, node_info: NodeInfo) -> WcResult<()> {
        let already_active = self.node_roster.iter().any(|n| n.peer_id == node_info.peer_id);
        let already_standby = self.standby_pool.iter().any(|n| n.peer_id == node_info.peer_id);
        if already_active || already_standby {
            return Err(WcError::new(
                ErrorCode::AlreadyExists,
                format!("Node {} is already registered", node_info.peer_id),
            ));
        }
        self.node_roster.push(node_info);
        Ok(())
    }

    /// Deregister a node, removing it from both the active roster and standby pool.
    ///
    /// Returns `NotFound` if the peer_id is not registered anywhere.
    pub fn deregister_node(&mut self, peer_id: &PeerIdStr) -> WcResult<()> {
        let before = self.node_roster.len() + self.standby_pool.len();
        self.node_roster.retain(|n| &n.peer_id != peer_id);
        self.standby_pool.retain(|n| &n.peer_id != peer_id);
        let after = self.node_roster.len() + self.standby_pool.len();
        if before == after {
            return Err(WcError::new(
                ErrorCode::NotFound,
                format!("Node {peer_id} is not registered"),
            ));
        }
        Ok(())
    }

    /// Match a task's requirements against the active node roster.
    ///
    /// Returns the peer IDs of all nodes that meet the requirements.
    /// Returns `NoEligibleNodes` if no nodes qualify.
    pub fn match_task(&self, requirements: &TaskRequirements) -> WcResult<Vec<PeerIdStr>> {
        let eligible: Vec<PeerIdStr> = self
            .node_roster
            .iter()
            .filter(|node| {
                node.capacity.cpu_millicores >= requirements.min_cpu_millicores
                    && node.capacity.ram_bytes >= requirements.min_ram_bytes
                    && node.capacity.scratch_bytes >= requirements.min_scratch_bytes
                    && node.trust_tier >= requirements.min_trust_tier
            })
            .map(|node| node.peer_id.clone())
            .collect();

        // Exclude frozen hosts (incident response)
        let eligible: Vec<PeerIdStr> =
            eligible.into_iter().filter(|p| !self.frozen_hosts.contains(p)).collect();

        if eligible.is_empty() {
            return Err(WcError::new(
                ErrorCode::NoEligibleNodes,
                "No nodes meet the task requirements",
            ));
        }

        Ok(eligible)
    }

    /// Register a node with attestation verification (T041).
    ///
    /// Per FR-S010/FR-S011: verifies the node's attestation quote against
    /// the measurement registry before admitting it to the active roster.
    /// Nodes with invalid attestation are rejected. Nodes with no attestation
    /// (empty quote) are classified as T0 (trust_tier = 0).
    pub fn register_node_with_attestation(
        &mut self,
        mut node_info: NodeInfo,
        quote: &AttestationQuote,
        registry: &MeasurementRegistry,
    ) -> WcResult<()> {
        // Verify attestation
        let verified =
            attestation::verify_attestation_with_registry(quote, registry).unwrap_or(false);

        if !verified {
            // If quote is non-empty but invalid, reject entirely
            if !quote.quote_bytes.is_empty() {
                return Err(WcError::new(
                    ErrorCode::AttestationFailed,
                    format!(
                        "Node {} presented invalid attestation — rejected (not downgraded to T0)",
                        node_info.peer_id
                    ),
                ));
            }
            // Empty quote → classify as T0
            node_info.trust_tier = 0;
            node_info.attestation_verified = false;
        } else {
            node_info.attestation_verified = true;
            node_info.attestation_verified_at = Some(crate::types::Timestamp::now().0);
        }

        self.register_node(node_info)
    }

    /// Freeze a host — remove from scheduling pool (incident response).
    pub fn freeze_host(&mut self, peer_id: &PeerIdStr) {
        if !self.frozen_hosts.contains(peer_id) {
            self.frozen_hosts.push(peer_id.clone());
        }
    }

    /// Unfreeze a host — restore to scheduling pool.
    pub fn unfreeze_host(&mut self, peer_id: &PeerIdStr) {
        self.frozen_hosts.retain(|p| p != peer_id);
    }

    /// Check if a host is frozen.
    pub fn is_host_frozen(&self, peer_id: &PeerIdStr) -> bool {
        self.frozen_hosts.contains(peer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_envelope(cpu: u64, ram: u64) -> ResourceEnvelope {
        ResourceEnvelope {
            cpu_millicores: cpu,
            ram_bytes: ram,
            gpu_class: None,
            gpu_vram_bytes: 0,
            scratch_bytes: 10 * 1024 * 1024 * 1024, // 10 GiB
            network_egress_bytes: 0,
            walltime_budget_ms: 3_600_000,
        }
    }

    fn test_node(peer_id: &str, cpu: u64, ram: u64) -> NodeInfo {
        NodeInfo {
            peer_id: peer_id.to_string(),
            region_code: "us-east-1".to_string(),
            capacity: test_envelope(cpu, ram),
            trust_tier: 1,
            attestation_verified: false,
            attestation_verified_at: None,
        }
    }

    #[test]
    fn register_node_success() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        let node = test_node("peer-aaa", 4000, 8 * 1024 * 1024 * 1024);
        assert!(broker.register_node(node).is_ok());
        assert_eq!(broker.node_roster.len(), 1);
    }

    #[test]
    fn register_duplicate_node_fails() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        let node1 = test_node("peer-aaa", 4000, 8 * 1024 * 1024 * 1024);
        let node2 = test_node("peer-aaa", 4000, 8 * 1024 * 1024 * 1024);
        assert!(broker.register_node(node1).is_ok());
        let err = broker.register_node(node2).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::AlreadyExists));
    }

    #[test]
    fn deregister_node_success() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        let node = test_node("peer-bbb", 4000, 8 * 1024 * 1024 * 1024);
        broker.register_node(node).unwrap();
        assert!(broker.deregister_node(&"peer-bbb".to_string()).is_ok());
        assert!(broker.node_roster.is_empty());
    }

    #[test]
    fn deregister_missing_node_fails() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        let err = broker.deregister_node(&"peer-zzz".to_string()).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::NotFound));
    }

    #[test]
    fn match_task_returns_eligible_nodes() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        broker.register_node(test_node("peer-big", 8000, 16 * 1024 * 1024 * 1024)).unwrap();
        broker.register_node(test_node("peer-small", 1000, 1024 * 1024 * 1024)).unwrap();

        let reqs = TaskRequirements {
            min_cpu_millicores: 4000,
            min_ram_bytes: 8 * 1024 * 1024 * 1024,
            min_scratch_bytes: 1,
            min_trust_tier: 1,
        };
        let matched = broker.match_task(&reqs).unwrap();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0], "peer-big");
    }

    #[test]
    fn match_task_no_eligible_returns_error() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        broker.register_node(test_node("peer-tiny", 500, 512 * 1024 * 1024)).unwrap();

        let reqs = TaskRequirements {
            min_cpu_millicores: 4000,
            min_ram_bytes: 8 * 1024 * 1024 * 1024,
            min_scratch_bytes: 1,
            min_trust_tier: 1,
        };
        let err = broker.match_task(&reqs).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::NoEligibleNodes));
    }

    #[test]
    fn match_task_trust_tier_filter() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        let mut node = test_node("peer-t1", 8000, 16 * 1024 * 1024 * 1024);
        node.trust_tier = 1;
        broker.register_node(node).unwrap();

        let reqs = TaskRequirements {
            min_cpu_millicores: 1000,
            min_ram_bytes: 1,
            min_scratch_bytes: 1,
            min_trust_tier: 3, // requires TEE
        };
        let err = broker.match_task(&reqs).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::NoEligibleNodes));
    }

    #[test]
    fn frozen_host_excluded_from_matching() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        broker.register_node(test_node("peer-frozen", 8000, 16 * 1024 * 1024 * 1024)).unwrap();
        broker.register_node(test_node("peer-active", 8000, 16 * 1024 * 1024 * 1024)).unwrap();

        broker.freeze_host(&"peer-frozen".to_string());

        let reqs = TaskRequirements {
            min_cpu_millicores: 1000,
            min_ram_bytes: 1,
            min_scratch_bytes: 1,
            min_trust_tier: 1,
        };
        let matched = broker.match_task(&reqs).unwrap();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0], "peer-active");
    }

    #[test]
    fn unfreeze_host_restores_matching() {
        let mut broker = Broker::new("broker-001", "us-east-1");
        broker.register_node(test_node("peer-1", 8000, 16 * 1024 * 1024 * 1024)).unwrap();
        broker.freeze_host(&"peer-1".to_string());
        assert!(broker.is_host_frozen(&"peer-1".to_string()));

        broker.unfreeze_host(&"peer-1".to_string());
        assert!(!broker.is_host_frozen(&"peer-1".to_string()));

        let reqs = TaskRequirements {
            min_cpu_millicores: 1000,
            min_ram_bytes: 1,
            min_scratch_bytes: 1,
            min_trust_tier: 1,
        };
        assert_eq!(broker.match_task(&reqs).unwrap().len(), 1);
    }

    #[test]
    fn attestation_with_empty_quote_classifies_t0() {
        use crate::types::{AttestationQuote, AttestationType};
        use crate::verification::attestation::MeasurementRegistry;

        let mut broker = Broker::new("broker-001", "us-east-1");
        let registry = MeasurementRegistry::new();
        let mut node = test_node("peer-noattest", 8000, 16 * 1024 * 1024 * 1024);
        node.trust_tier = 2; // claims T2

        let empty_quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes: Vec::new(),
            platform_info: "test".into(),
        };

        broker.register_node_with_attestation(node, &empty_quote, &registry).unwrap();
        // Should have been downgraded to T0
        assert_eq!(broker.node_roster[0].trust_tier, 0);
        assert!(!broker.node_roster[0].attestation_verified);
    }

    #[test]
    fn attestation_with_invalid_quote_rejected() {
        use crate::types::{AttestationQuote, AttestationType};
        use crate::verification::attestation::MeasurementRegistry;

        let mut broker = Broker::new("broker-001", "us-east-1");
        let registry = MeasurementRegistry::new();
        let node = test_node("peer-bad", 8000, 16 * 1024 * 1024 * 1024);

        // Non-empty but garbage quote
        let bad_quote = AttestationQuote {
            quote_type: AttestationType::Tpm2,
            quote_bytes: vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00],
            platform_info: "test".into(),
        };

        let result = broker.register_node_with_attestation(node, &bad_quote, &registry);
        assert!(result.is_err(), "Invalid attestation should reject the node");
    }
}
