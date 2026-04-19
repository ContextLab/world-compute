//! gRPC stub handler for MeshLLMService (T118).
//!
//! Generated code lives in the `mesh_llm` proto package; this module wires
//! the hand-written scaffold types to the tonic service trait stubs.

use tonic::{Request, Response, Status};

// Bring in the generated proto types for mesh_llm.
pub mod proto {
    tonic::include_proto!("worldcompute.v1");
}

use proto::{
    mesh_llm_service_server::MeshLlmService, GetRouterStatusRequest, GetRouterStatusResponse,
    HaltMeshRequest, HaltMeshResponse, RegisterExpertRequest, RegisterExpertResponse,
    SubmitSelfTaskRequest, SubmitSelfTaskResponse,
};

use crate::agent::mesh_llm::{
    expert::{ExpertNode, ExpertRegistry},
    safety::{kill_switch, MeshSafetyState},
    self_prompt::generate_task_prompt,
};

use std::sync::{Arc, Mutex};

/// Concrete service implementation (stub — no real inference yet).
pub struct MeshLlmServiceImpl {
    registry: Arc<Mutex<ExpertRegistry>>,
    safety: Arc<MeshSafetyState>,
}

impl MeshLlmServiceImpl {
    pub fn new(registry: Arc<Mutex<ExpertRegistry>>, safety: Arc<MeshSafetyState>) -> Self {
        Self { registry, safety }
    }
}

#[tonic::async_trait]
impl MeshLlmService for MeshLlmServiceImpl {
    async fn register_expert(
        &self,
        request: Request<RegisterExpertRequest>,
    ) -> Result<Response<RegisterExpertResponse>, Status> {
        let req = request.into_inner();
        let node = ExpertNode::new(
            req.expert_id.clone(),
            req.model_name.clone(),
            req.capacity_tokens_per_sec,
        );
        let mut reg =
            self.registry.lock().map_err(|_| Status::internal("registry lock poisoned"))?;
        match reg.register_expert(node) {
            Ok(()) => Ok(Response::new(RegisterExpertResponse {
                success: true,
                message: format!("registered {}", req.expert_id),
            })),
            Err(e) => {
                Ok(Response::new(RegisterExpertResponse { success: false, message: e.to_string() }))
            }
        }
    }

    async fn get_router_status(
        &self,
        _request: Request<GetRouterStatusRequest>,
    ) -> Result<Response<GetRouterStatusResponse>, Status> {
        let reg = self.registry.lock().map_err(|_| Status::internal("registry lock poisoned"))?;
        let online = reg.list_online_experts();
        Ok(Response::new(GetRouterStatusResponse {
            online_expert_count: online.len() as u32,
            online_expert_ids: online,
        }))
    }

    async fn submit_self_task(
        &self,
        request: Request<SubmitSelfTaskRequest>,
    ) -> Result<Response<SubmitSelfTaskResponse>, Status> {
        use crate::agent::mesh_llm::self_prompt::SelfPromptTask;
        let req = request.into_inner();
        let task = match req.task_type.as_str() {
            "scheduler_optimization" => SelfPromptTask::SchedulerOptimization,
            "security_log_analysis" => SelfPromptTask::SecurityLogAnalysis,
            "test_generation" => SelfPromptTask::TestGeneration,
            "config_tuning" => SelfPromptTask::ConfigTuning,
            "governance_proposal_draft" => SelfPromptTask::GovernanceProposalDraft,
            other => return Err(Status::invalid_argument(format!("unknown task_type: {other}"))),
        };
        let prompt = generate_task_prompt(task);
        Ok(Response::new(SubmitSelfTaskResponse {
            accepted: true,
            prompt_preview: prompt[..prompt.len().min(200)].to_string(),
        }))
    }

    async fn halt_mesh(
        &self,
        _request: Request<HaltMeshRequest>,
    ) -> Result<Response<HaltMeshResponse>, Status> {
        kill_switch(&self.safety);
        Ok(Response::new(HaltMeshResponse { halted: true }))
    }
}

// ---------------------------------------------------------------------------
// Higher-level service handler (T197-T198)
// ---------------------------------------------------------------------------

use crate::agent::mesh_llm::{router::MeshRouter, safety::KillSwitch, self_prompt::SelfTask};

/// Status snapshot of the router.
#[derive(Debug, Clone)]
pub struct RouterStatus {
    pub expert_count: usize,
    pub active_streams: usize,
    pub health: String,
}

/// Result of a halt operation.
#[derive(Debug, Clone)]
pub struct HaltResult {
    pub halted: bool,
    pub streams_stopped: usize,
    pub changes_reverted: usize,
}

/// High-level service handler that combines the router, kill switch, and
/// self-task submission into a single facade.
pub struct MeshLLMServiceHandler {
    pub router: MeshRouter,
    pub kill_switch: KillSwitch,
    active_streams: usize,
}

/// Minimum expert count below which a degradation warning is issued (T198).
pub const MIN_EXPERT_COUNT: usize = 280;

impl MeshLLMServiceHandler {
    pub fn new(router: MeshRouter) -> Self {
        Self { router, kill_switch: KillSwitch::new(), active_streams: 0 }
    }

    /// Register an expert and return its ID on success.
    pub fn register_expert(&mut self, expert: ExpertNode) -> Result<String, String> {
        if self.kill_switch.is_halted() {
            return Err("mesh is halted — cannot register experts".to_string());
        }
        let id = expert.expert_id.clone();
        self.router.register_expert(expert);
        Ok(id)
    }

    /// Return the current router status, including a degradation warning
    /// when the expert count is below [`MIN_EXPERT_COUNT`].
    pub fn get_router_status(&self) -> RouterStatus {
        let count = self.router.expert_count();
        let health = if self.kill_switch.is_halted() {
            "halted".to_string()
        } else if count < MIN_EXPERT_COUNT {
            format!("degraded — only {count} experts (minimum {MIN_EXPERT_COUNT})")
        } else {
            "healthy".to_string()
        };
        RouterStatus { expert_count: count, active_streams: self.active_streams, health }
    }

    /// Submit a self-generated task. Returns a task ID on acceptance.
    pub fn submit_self_task(&self, task: SelfTask) -> Result<String, String> {
        if self.kill_switch.is_halted() {
            return Err("mesh is halted — cannot accept tasks".to_string());
        }
        Ok(format!("task-{:?}-accepted", task.domain))
    }

    /// Halt the mesh, reverting any tracked changes.
    pub fn halt_mesh(&mut self, actor: &str) -> HaltResult {
        let changes = self.kill_switch.changes_to_revert.len();
        self.kill_switch.trigger(actor, &self.kill_switch.changes_to_revert.clone());
        let streams = self.active_streams;
        self.active_streams = 0;
        HaltResult { halted: true, streams_stopped: streams, changes_reverted: changes }
    }

    /// Increment the active stream count (for testing / bookkeeping).
    pub fn add_stream(&mut self) {
        self.active_streams += 1;
    }
}

#[cfg(test)]
mod handler_tests {
    use super::*;
    use crate::agent::mesh_llm::expert::ExpertNode;
    use crate::agent::mesh_llm::router::MeshRouter;
    use crate::agent::mesh_llm::safety::ActionTier;
    use crate::agent::mesh_llm::self_prompt::{SelfTask, TaskDomain};

    #[test]
    fn handler_register_and_status() {
        let mut h = MeshLLMServiceHandler::new(MeshRouter::new(4));
        h.register_expert(ExpertNode::new("e1", "m", 100.0)).unwrap();
        let status = h.get_router_status();
        assert_eq!(status.expert_count, 1);
        assert!(status.health.contains("degraded"));
    }

    #[test]
    fn handler_degradation_warning() {
        let h = MeshLLMServiceHandler::new(MeshRouter::new(4));
        let status = h.get_router_status();
        assert!(status.health.contains("degraded"));
    }

    #[test]
    fn handler_halt() {
        let mut h = MeshLLMServiceHandler::new(MeshRouter::new(4));
        h.add_stream();
        h.add_stream();
        let result = h.halt_mesh("admin");
        assert!(result.halted);
        assert_eq!(result.streams_stopped, 2);
        assert!(h.kill_switch.is_halted());
    }

    #[test]
    fn handler_reject_after_halt() {
        let mut h = MeshLLMServiceHandler::new(MeshRouter::new(4));
        h.halt_mesh("admin");
        assert!(h.register_expert(ExpertNode::new("e1", "m", 100.0)).is_err());
        let task = SelfTask {
            description: "test".to_string(),
            domain: TaskDomain::SecurityAudit,
            priority: 1,
            action_tier: ActionTier::ReadOnly,
        };
        assert!(h.submit_self_task(task).is_err());
    }

    #[test]
    fn handler_submit_task() {
        let h = MeshLLMServiceHandler::new(MeshRouter::new(4));
        let task = SelfTask {
            description: "optimize".to_string(),
            domain: TaskDomain::SchedulerOptimization,
            priority: 2,
            action_tier: ActionTier::Suggest,
        };
        let result = h.submit_self_task(task);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("accepted"));
    }

    #[test]
    fn handler_halted_status() {
        let mut h = MeshLLMServiceHandler::new(MeshRouter::new(4));
        h.halt_mesh("admin");
        let status = h.get_router_status();
        assert_eq!(status.health, "halted");
    }
}
