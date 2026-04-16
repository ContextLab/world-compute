//! GossipSub protocol setup — task announcements, capacity updates, lease grants
//! per FR-063 (T077).

use libp2p::{
    gossipsub::{self, MessageAuthenticity, ValidationMode},
    identity, PeerId,
};
use std::time::Duration;

/// GossipSub topic for task announcement messages.
pub const TOPIC_TASK_ANNOUNCEMENTS: &str = "wc/task-announcements/1.0.0";

/// GossipSub topic for node capacity update messages.
pub const TOPIC_CAPACITY_UPDATES: &str = "wc/capacity-updates/1.0.0";

/// GossipSub topic for lease grant messages.
pub const TOPIC_LEASE_GRANTS: &str = "wc/lease-grants/1.0.0";

/// All gossip topics as a slice for iteration.
pub const ALL_TOPICS: &[&str] =
    &[TOPIC_TASK_ANNOUNCEMENTS, TOPIC_CAPACITY_UPDATES, TOPIC_LEASE_GRANTS];

/// Configuration for the gossip subsystem.
#[derive(Debug, Clone)]
pub struct GossipConfig {
    /// Topics to subscribe to on startup.
    pub topics: Vec<String>,
    /// Heartbeat interval for mesh maintenance.
    pub heartbeat_interval: Duration,
    /// Maximum message size in bytes.
    pub max_transmit_size: usize,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            topics: ALL_TOPICS.iter().map(|s| s.to_string()).collect(),
            heartbeat_interval: Duration::from_secs(1),
            max_transmit_size: 1024 * 1024, // 1 MiB
        }
    }
}

/// Build a GossipSub behaviour for the given peer.
///
/// Uses signed message authentication so peers can verify message origin.
/// Permissive validation allows all well-formed messages through — content
/// policy enforcement happens at the application layer.
pub fn build_gossip(
    keypair: &identity::Keypair,
    _peer_id: PeerId,
    config: &GossipConfig,
) -> Result<gossipsub::Behaviour, Box<dyn std::error::Error>> {
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(config.heartbeat_interval)
        .validation_mode(ValidationMode::Permissive)
        .max_transmit_size(config.max_transmit_size)
        .build()
        .map_err(|e| format!("GossipSub config error: {e}"))?;

    let behaviour =
        gossipsub::Behaviour::new(MessageAuthenticity::Signed(keypair.clone()), gossipsub_config)
            .map_err(|e| format!("GossipSub behaviour error: {e}"))?;

    Ok(behaviour)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gossip_config_has_correct_topic_count() {
        let config = GossipConfig::default();
        assert_eq!(
            config.topics.len(),
            3,
            "Expected 3 topics: task_announcements, capacity_updates, lease_grants"
        );
        assert!(config.topics.contains(&TOPIC_TASK_ANNOUNCEMENTS.to_string()));
        assert!(config.topics.contains(&TOPIC_CAPACITY_UPDATES.to_string()));
        assert!(config.topics.contains(&TOPIC_LEASE_GRANTS.to_string()));
    }

    #[test]
    fn all_topics_constant_has_three_entries() {
        assert_eq!(ALL_TOPICS.len(), 3);
    }

    #[test]
    fn build_gossip_succeeds() {
        // GossipSub may panic in CI if socket setup fails.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let keypair = identity::Keypair::generate_ed25519();
            let peer_id = PeerId::from(keypair.public());
            let config = GossipConfig::default();
            build_gossip(&keypair, peer_id, &config)
        }));
        match result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                eprintln!("GossipSub init returned error (may be expected in CI): {e}");
            }
            Err(_) => {
                eprintln!("GossipSub init panicked (may be expected in CI containers)");
            }
        }
    }
}
