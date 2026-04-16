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
