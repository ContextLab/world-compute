//! Safety tiers and governance kill switch (FR-125).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Safety tier governing what actions the mesh may take autonomously.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionTier {
    /// Read cluster state only; no writes.
    ReadOnly,
    /// Surface suggestions to operators; no automatic application.
    Suggest,
    /// Apply minor, low-risk parameter changes automatically.
    ModifyMinor,
    /// Apply major structural changes; requires governance approval.
    ModifyMajor,
    /// Deploy new software or configuration to production nodes.
    Deploy,
}

/// Returns `true` when the tier requires explicit governance approval before
/// the action may proceed.
pub fn requires_governance_approval(tier: ActionTier) -> bool {
    matches!(tier, ActionTier::ModifyMajor | ActionTier::Deploy)
}

/// Shared safety state for the mesh cluster.
#[derive(Debug, Default)]
pub struct MeshSafetyState {
    killed: AtomicBool,
}

impl MeshSafetyState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap in an `Arc` for convenient sharing across threads.
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

/// Engage the governance kill switch — permanently halts mesh autonomy until
/// the process is restarted.
pub fn kill_switch(state: &MeshSafetyState) {
    state.killed.store(true, Ordering::SeqCst);
}

/// Returns `true` if the kill switch has been engaged.
pub fn is_killed(state: &MeshSafetyState) -> bool {
    state.killed.load(Ordering::SeqCst)
}

/// Keyword-based action tier classifier.
///
/// Matches the first applicable keyword in descending tier order so that a
/// description containing "deploy" is classified as `Deploy` even if it also
/// contains "modify".
pub fn classify_action(description: &str) -> ActionTier {
    let lower = description.to_lowercase();
    if lower.contains("deploy") || lower.contains("release") || lower.contains("publish") {
        ActionTier::Deploy
    } else if lower.contains("modify major")
        || lower.contains("restructure")
        || lower.contains("refactor")
        || lower.contains("migration")
        || lower.contains("upgrade")
    {
        ActionTier::ModifyMajor
    } else if lower.contains("modify") || lower.contains("update") || lower.contains("patch") {
        ActionTier::ModifyMinor
    } else if lower.contains("suggest") || lower.contains("recommend") || lower.contains("propose")
    {
        ActionTier::Suggest
    } else {
        ActionTier::ReadOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_switch_works() {
        let state = MeshSafetyState::new();
        assert!(!is_killed(&state));
        kill_switch(&state);
        assert!(is_killed(&state));
    }

    #[test]
    fn kill_switch_idempotent() {
        let state = MeshSafetyState::new();
        kill_switch(&state);
        kill_switch(&state);
        assert!(is_killed(&state));
    }

    #[test]
    fn governance_approval_tiers() {
        assert!(!requires_governance_approval(ActionTier::ReadOnly));
        assert!(!requires_governance_approval(ActionTier::Suggest));
        assert!(!requires_governance_approval(ActionTier::ModifyMinor));
        assert!(requires_governance_approval(ActionTier::ModifyMajor));
        assert!(requires_governance_approval(ActionTier::Deploy));
    }

    #[test]
    fn classify_deploy() {
        assert_eq!(classify_action("deploy new model version"), ActionTier::Deploy);
        assert_eq!(classify_action("release the update"), ActionTier::Deploy);
    }

    #[test]
    fn classify_modify_major() {
        assert_eq!(classify_action("restructure the scheduler"), ActionTier::ModifyMajor);
        assert_eq!(classify_action("database migration"), ActionTier::ModifyMajor);
    }

    #[test]
    fn classify_modify_minor() {
        assert_eq!(classify_action("modify timeout value"), ActionTier::ModifyMinor);
        assert_eq!(classify_action("update config flag"), ActionTier::ModifyMinor);
        assert_eq!(classify_action("patch the configuration"), ActionTier::ModifyMinor);
    }

    #[test]
    fn classify_suggest() {
        assert_eq!(classify_action("suggest a better queue depth"), ActionTier::Suggest);
        assert_eq!(classify_action("recommend new parameters"), ActionTier::Suggest);
    }

    #[test]
    fn classify_read_only() {
        assert_eq!(classify_action("read cluster metrics"), ActionTier::ReadOnly);
        assert_eq!(classify_action("analyze logs"), ActionTier::ReadOnly);
    }

    #[test]
    fn deploy_beats_modify_in_same_description() {
        // "deploy" should win over "modify" when both are present.
        assert_eq!(classify_action("deploy and modify config"), ActionTier::Deploy);
    }

    #[test]
    fn shared_state_across_threads() {
        let state = MeshSafetyState::shared();
        let state2 = Arc::clone(&state);
        let handle = std::thread::spawn(move || {
            kill_switch(&state2);
        });
        handle.join().unwrap();
        assert!(is_killed(&state));
    }
}
