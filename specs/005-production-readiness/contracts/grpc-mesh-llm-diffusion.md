# Contract: `MeshLlmDiffusion` gRPC service

**Scope**: Replaces the existing `MeshLLM` service (AR-ensemble) with a diffusion-native service. The existing service is deleted; there is no compatibility shim. `proto/mesh_llm_diffusion.proto` is added; `proto/mesh_llm.proto` is removed.

## Service definition (proto)

```proto
syntax = "proto3";
package worldcompute.mesh_llm_diffusion.v1;

service MeshLlmDiffusion {
  // Inference — streaming response carries per-step telemetry and final output
  rpc Infer(InferRequest) returns (stream InferResponse);

  // Register a backbone on this node (called by the daemon at startup)
  rpc RegisterBackbone(RegisterBackboneRequest) returns (RegisterBackboneResponse);

  // Register a specialized expert on this node
  rpc RegisterExpert(RegisterExpertRequest) returns (RegisterExpertResponse);

  // Poll the governance kill-switch state (workers poll before each denoising step)
  rpc PollKillSwitch(PollKillSwitchRequest) returns (PollKillSwitchResponse);

  // List currently-loaded backbones + experts on this node
  rpc ListLoaded(ListLoadedRequest) returns (ListLoadedResponse);
}

message InferRequest {
  string request_id = 1;           // UUID
  string prompt = 2;
  string backbone_model_id = 3;    // e.g., "GSAI-ML/LLaDA-8B-Instruct"
  repeated ExpertSelection experts = 4;
  uint32 denoising_steps = 5;      // default 64
  uint32 paradigms_block_size = 6; // default 4
  uint32 distrifusion_staleness = 7; // default 1
  float clipping_tau = 8;          // PCG clipping bound, default 10.0
  SafetyTier safety_tier = 9;
}

message ExpertSelection {
  string expert_id = 1;
  float guidance_weight = 2; // default 1.0
}

message InferResponse {
  oneof payload {
    DenoisingStepTelemetry step_telemetry = 1;
    ParaDiGMSBlockReport paradigms_block = 2;
    DistriFusionPipelineReport distrifusion_report = 3;
    InferComplete complete = 4;
    InferHalted halted = 5;
    InferError error = 6;
  }
}

message DenoisingStepTelemetry {
  uint32 step_index = 1;
  repeated ExpertScore per_expert = 2;
  float composed_score_norm = 3;
  repeated string clipping_activated_for = 4; // expert_ids that were clipped
}

message ExpertScore {
  string expert_id = 1;
  float score_norm = 2;
  float applied_weight = 3;
}

message ParaDiGMSBlockReport {
  uint32 block_start = 1;
  uint32 block_size = 2;
  uint32 iterations_used = 3;
  bool converged = 4;
  uint32 wall_clock_ms = 5;
}

message DistriFusionPipelineReport {
  uint32 step_index = 1;
  uint32 rtt_ms_masked = 2;
  uint32 rtt_ms_total = 3;
}

message InferComplete {
  string output = 1;
  SafetyTier classified_tier = 2;
  uint32 total_wall_clock_ms = 3;
  bytes coordinator_receipt = 4; // signed blob, verifiable via verify_receipt
}

message InferHalted {
  string reason = 1;            // typically "kill_switch_fired"
  uint32 halted_at_step = 2;
}

message InferError {
  string code = 1;              // e.g., "paradigms_nonconvergence"
  string detail = 2;
}

message PollKillSwitchRequest { string worker_id = 1; }
message PollKillSwitchResponse {
  bool active = 1;
  string reason = 2; // populated if active
}

enum SafetyTier {
  SAFETY_TIER_UNSPECIFIED = 0;
  SAFETY_TIER_PUBLIC = 1;
  SAFETY_TIER_INTERNAL = 2;
  SAFETY_TIER_RESTRICTED = 3;
}

message RegisterBackboneRequest {
  string model_id = 1;
  bytes weights_cid = 2;     // raw CID bytes
  string device = 3;         // e.g., "cuda:0"
  Quantization quantization = 4;
}
enum Quantization {
  QUANT_NONE = 0;
  QUANT_INT8 = 1;
  QUANT_INT4_AWQ = 2;
  QUANT_GGUF = 3;
}
message RegisterBackboneResponse { bool success = 1; string detail = 2; }

message RegisterExpertRequest {
  string expert_id = 1;
  string specialization_domain = 2;
  bytes weights_cid = 3;
  string backbone_model_id = 4;
}
message RegisterExpertResponse { bool success = 1; string detail = 2; }

message ListLoadedRequest {}
message ListLoadedResponse {
  repeated string backbone_ids = 1;
  repeated string expert_ids = 2;
}
```

## Semantics

- `Infer` is a **server-streaming** RPC. The client receives zero or more telemetry messages followed by exactly one terminal message (`complete`, `halted`, or `error`).
- `PollKillSwitch` is called by every worker **before every denoising step** per FR-029.
- The coordinator's receipt in `InferComplete.coordinator_receipt` MUST verify via `src/verification/receipt.rs::verify_receipt` using the wired coordinator public key per FR-032.

## Error model

- Any `InferError` with `code == "paradigms_nonconvergence"` means ParaDiGMS hit its max-iterations budget AND sequential fallback also failed; the request is then retried under the submitter's per-request retry budget per Edge Cases.
- Any `InferError` with `code == "expert_compatibility_mismatch"` means an expert's `backbone_compat_version` doesn't match the selected backbone; the client MUST re-dispatch with compatible experts.

## Transport

- Over libp2p, the service is invoked via gRPC-over-libp2p-stream using the existing tonic + libp2p transport integration (from spec 004). No new transport is introduced for the control plane.
- Activation tensors for DistriFusion (FR-026) use a *separate* libp2p request-response protocol `/worldcompute/diffusion-activation/1.0.0` — see `data-model.md` E.6 — NOT this gRPC service.
