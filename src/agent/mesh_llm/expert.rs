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

    /// Return references to all experts currently `Online` (healthy).
    pub fn get_healthy(&self) -> Vec<&ExpertNode> {
        self.experts.values().filter(|n| n.status == ExpertStatus::Online).collect()
    }

    /// Update the health/status of an expert by ID.
    pub fn update_health(&mut self, expert_id: &str, status: ExpertStatus) {
        if let Some(node) = self.experts.get_mut(expert_id) {
            node.status = status;
        }
    }

    /// Number of registered experts.
    pub fn len(&self) -> usize {
        self.experts.len()
    }

    /// Returns true if no experts are registered.
    pub fn is_empty(&self) -> bool {
        self.experts.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Model loading (T191)
// ---------------------------------------------------------------------------

/// Configuration for loading a model onto an expert node.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Path to the .gguf model file.
    pub model_path: String,
    /// Path to the tokenizer file.
    pub tokenizer_path: String,
    /// Maximum number of tokens the model can generate.
    pub max_tokens: usize,
}

/// A loaded model ready for inference.
#[derive(Debug, Clone)]
pub struct LoadedModel {
    pub name: String,
    pub vocab_size: usize,
    pub loaded: bool,
}

/// Attempt to load a model from the given configuration.
///
/// This is a placeholder — in production this would use
/// `candle_transformers::models::llama::Llama::load(...)`.
/// Returns `Err` if the model file does not exist.
pub fn load_model(config: &ModelConfig) -> Result<LoadedModel, String> {
    let path = std::path::Path::new(&config.model_path);
    if !path.exists() {
        return Err(format!("model file not found: {}", config.model_path));
    }
    Ok(LoadedModel {
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        vocab_size: 128_000, // LLaMA-3 128K vocab
        loaded: true,
    })
}

/// Health information for an expert node.
#[derive(Debug, Clone)]
pub struct ExpertHealth {
    pub expert_id: String,
    pub status: ExpertStatus,
    pub latency_ms: u32,
    pub tokens_per_sec: f64,
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

    #[test]
    fn get_healthy_returns_online_only() {
        let mut reg = ExpertRegistry::new();
        let mut offline = make_node("off");
        offline.status = ExpertStatus::Offline;
        reg.register_expert(make_node("on1")).unwrap();
        reg.register_expert(make_node("on2")).unwrap();
        reg.register_expert(offline).unwrap();
        assert_eq!(reg.get_healthy().len(), 2);
    }

    #[test]
    fn update_health_changes_status() {
        let mut reg = ExpertRegistry::new();
        reg.register_expert(make_node("x")).unwrap();
        reg.update_health("x", ExpertStatus::Busy);
        assert_eq!(reg.get_expert("x").unwrap().status, ExpertStatus::Busy);
    }

    #[test]
    fn load_model_missing_file() {
        let cfg = ModelConfig {
            model_path: "/nonexistent/model.gguf".to_string(),
            tokenizer_path: "/nonexistent/tokenizer.json".to_string(),
            max_tokens: 1024,
        };
        assert!(load_model(&cfg).is_err());
    }

    #[test]
    fn registry_len_and_empty() {
        let mut reg = ExpertRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        reg.register_expert(make_node("a")).unwrap();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
    }
}
