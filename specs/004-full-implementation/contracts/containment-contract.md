# Containment Contract

## execute_containment(action: ContainmentAction) -> Result<AuditRecord>

### Actions and Effects

| Action | Target | Effect | Timeout |
|-|-|-|-|
| FreezeHost | host_id | SIGSTOP all sandbox PIDs on host, block new leases | 60s |
| QuarantineWorkloadClass | class_name | Update policy engine rejection list | 5s |
| BlockSubmitter | submitter_id | Cancel in-flight jobs, add to ban list | 30s |
| RevokeArtifact | artifact_cid | Remove from registry, halt jobs using it | 30s |
| DrainHostPool | pool_id | Migrate workloads, block new assignments | 300s |

### Input
- `action`: ContainmentAction with actor, target, justification
- Actor must hold OnCallResponder governance role

### Output
- `AuditRecord { action, actor, timestamp, result, reversible }`

### Behavior
- Verify actor has OnCallResponder role
- Execute enforcement effect (not just log)
- Produce immutable audit record
- All actions reversible except RevokeArtifact (requires re-signing)
- Actions complete within specified timeout or fail with error
