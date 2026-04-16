//! Expert node registry — each GPU donor runs one ExpertNode (FR-120, FR-121).

use std::collections::HashMap;

use crate::error::{ErrorCode, WcError, WcResult};

/// Operational status of an expert node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpertStatus {
    Online,
    Offline,
    Busy,
}

/// A single expert node participating in the mesh LLM.
///
/// All nodes MUST use the LLaMA-3 tokenizer with 128K vocab (FR-121).
#[derive(Debug, Clone)]
pub struct ExpertNode {
    pub expert_id: String,
    /// Name/path of the small model running on this node.
    pub model_name: String,
    /// Tokenizer family — always "llama3" per FR-121.
    pub tokenizer: String,
    pub status: ExpertStatus,
    /// Throughput in tokens per second.
    pub capacity_tokens_per_sec: f64,
}

impl ExpertNode {
    pub fn new(
        expert_id: impl Into<String>,
        model_name: impl Into<String>,
        capacity_tokens_per_sec: f64,
    ) -> Self {
        Self {
            expert_id: expert_id.into(),
            model_name: model_name.into(),
            tokenizer: "llama3".to_string(),
            status: ExpertStatus::Online,
            capacity_tokens_per_sec,
        }
    }
}

/// Registry of all known expert nodes.
#[derive(Debug, Default)]
pub struct ExpertRegistry {
    experts: HashMap<String, ExpertNode>,
}

impl ExpertRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new expert. Returns `AlreadyExists` if the ID is taken.
    pub fn register_expert(&mut self, node: ExpertNode) -> WcResult<()> {
        if self.experts.contains_key(&node.expert_id) {
            return Err(WcError::new(
                ErrorCode::AlreadyExists,
                format!("expert '{}' already registered", node.expert_id),
            ));
        }
        self.experts.insert(node.expert_id.clone(), node);
        Ok(())
    }

    /// Remove an expert. Returns `NotFound` if the ID is unknown.
    pub fn deregister_expert(&mut self, expert_id: &str) -> WcResult<ExpertNode> {
        self.experts.remove(expert_id).ok_or_else(|| {
            WcError::new(ErrorCode::NotFound, format!("expert '{expert_id}' not found"))
        })
    }

    /// Return IDs of all experts currently `Online`.
    pub fn list_online_experts(&self) -> Vec<String> {
        self.experts
            .values()
            .filter(|n| n.status == ExpertStatus::Online)
            .map(|n| n.expert_id.clone())
            .collect()
    }

    /// Look up a single expert by ID.
    pub fn get_expert(&self, expert_id: &str) -> Option<&ExpertNode> {
        self.experts.get(expert_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str) -> ExpertNode {
        ExpertNode::new(id, "meta-llama/Llama-3.2-1B", 100.0)
    }

    #[test]
    fn register_and_retrieve() {
        let mut reg = ExpertRegistry::new();
        reg.register_expert(make_node("a")).unwrap();
        let node = reg.get_expert("a").unwrap();
        assert_eq!(node.expert_id, "a");
        assert_eq!(node.tokenizer, "llama3");
    }

    #[test]
    fn duplicate_register_fails() {
        let mut reg = ExpertRegistry::new();
        reg.register_expert(make_node("x")).unwrap();
        let err = reg.register_expert(make_node("x")).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::AlreadyExists));
    }

    #[test]
    fn deregister_removes_expert() {
        let mut reg = ExpertRegistry::new();
        reg.register_expert(make_node("b")).unwrap();
        let removed = reg.deregister_expert("b").unwrap();
        assert_eq!(removed.expert_id, "b");
        assert!(reg.get_expert("b").is_none());
    }

    #[test]
    fn deregister_missing_fails() {
        let mut reg = ExpertRegistry::new();
        let err = reg.deregister_expert("ghost").unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::NotFound));
    }

    #[test]
    fn list_online_filters_offline() {
        let mut reg = ExpertRegistry::new();
        let mut offline = make_node("offline");
        offline.status = ExpertStatus::Offline;
        reg.register_expert(make_node("online")).unwrap();
        reg.register_expert(offline).unwrap();

        let online = reg.list_online_experts();
        assert_eq!(online, vec!["online"]);
    }

    #[test]
    fn list_online_empty_registry() {
        let reg = ExpertRegistry::new();
        assert!(reg.list_online_experts().is_empty());
    }
}
