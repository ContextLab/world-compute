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
/// contains "modify". When keywords from multiple tiers are present, the
/// highest (most restrictive) tier wins per contract.
pub fn classify_action(description: &str) -> ActionTier {
    let lower = description.to_lowercase();

    // Deploy tier — highest restriction
    if lower.contains("deploy")
        || lower.contains("release")
        || lower.contains("publish")
        || lower.contains("change:")
        || lower.contains("replace:")
        || lower.contains("deploy:")
    {
        ActionTier::Deploy
    // ModifyMajor
    } else if lower.contains("modify major")
        || lower.contains("restructure")
        || lower.contains("refactor")
        || lower.contains("migration")
        || lower.contains("upgrade")
    {
        ActionTier::ModifyMajor
    // ModifyMinor
    } else if lower.contains("modify")
        || lower.contains("update")
        || lower.contains("update:")
        || lower.contains("set:")
        || lower.contains("configure:")
        || lower.contains("patch")
    {
        ActionTier::ModifyMinor
    // Suggest
    } else if lower.contains("suggest")
        || lower.contains("suggest:")
        || lower.contains("recommend")
        || lower.contains("recommend:")
        || lower.contains("propose")
        || lower.contains("experiment:")
        || lower.contains("test:")
        || lower.contains("sandbox")
    {
        ActionTier::Suggest
    // ReadOnly — default / observation keywords
    } else {
        // "analyze", "report", "observe" all fall here
        ActionTier::ReadOnly
    }
}

// ---------------------------------------------------------------------------
// KillSwitch struct (T196)
// ---------------------------------------------------------------------------

/// A kill switch that can halt mesh operations and track changes to revert.
#[derive(Debug, Clone, Default)]
pub struct KillSwitch {
    /// Whether the kill switch is currently active.
    pub active: bool,
    /// Identity of the actor who triggered the kill switch.
    pub triggered_by: Option<String>,
    /// List of change descriptions to revert when the switch is triggered.
    pub changes_to_revert: Vec<String>,
}

impl KillSwitch {
    /// Create a new inactive kill switch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Trigger the kill switch, recording the actor and recent changes.
    pub fn trigger(&mut self, actor: &str, recent_changes: &[String]) {
        self.active = true;
        self.triggered_by = Some(actor.to_string());
        self.changes_to_revert = recent_changes.to_vec();
    }

    /// Returns `true` if the kill switch has been triggered.
    pub fn is_halted(&self) -> bool {
        self.active
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

    // --- KillSwitch tests (T196) ---

    #[test]
    fn kill_switch_struct_default_inactive() {
        let ks = KillSwitch::new();
        assert!(!ks.is_halted());
        assert!(ks.triggered_by.is_none());
        assert!(ks.changes_to_revert.is_empty());
    }

    #[test]
    fn kill_switch_struct_trigger() {
        let mut ks = KillSwitch::new();
        let changes = vec!["config-change-1".to_string(), "deploy-2".to_string()];
        ks.trigger("admin-alice", &changes);
        assert!(ks.is_halted());
        assert_eq!(ks.triggered_by.as_deref(), Some("admin-alice"));
        assert_eq!(ks.changes_to_revert.len(), 2);
    }

    #[test]
    fn kill_switch_struct_trigger_twice() {
        let mut ks = KillSwitch::new();
        ks.trigger("alice", &["c1".to_string()]);
        ks.trigger("bob", &["c2".to_string(), "c3".to_string()]);
        assert!(ks.is_halted());
        assert_eq!(ks.triggered_by.as_deref(), Some("bob"));
        assert_eq!(ks.changes_to_revert.len(), 2);
    }

    #[test]
    fn classify_set_configure() {
        assert_eq!(classify_action("set: new timeout"), ActionTier::ModifyMinor);
        assert_eq!(classify_action("configure: logging level"), ActionTier::ModifyMinor);
    }

    #[test]
    fn classify_change_replace() {
        assert_eq!(classify_action("change: scheduler algorithm"), ActionTier::Deploy);
        assert_eq!(classify_action("replace: old module"), ActionTier::Deploy);
    }
}
