# CLI Contract: worldcompute

**Branch**: `003-stub-replacement` | **Date**: 2026-04-16

This documents the CLI interface contract after stub replacement. All commands below must produce meaningful output (not "not yet implemented").

## Commands

### worldcompute donor

| Subcommand | Arguments | Output |
|-|-|-|
| join | --consent \<classes\> | Confirmation of enrollment with consent classes |
| status | (none) | Resource usage, trust score, credit balance, uptime |
| pause | (none) | Confirmation agent paused, active work checkpointed |
| resume | (none) | Confirmation agent resumed |
| leave | (none) | Confirmation of withdrawal, host state cleanup |
| credits | --verify | Credit balance, history; optional ledger verification |
| logs | --lines \<n\> | Recent agent log lines |

### worldcompute job

| Subcommand | Arguments | Output |
|-|-|-|
| submit | \<manifest\> | Job ID, validation result, dispatch status |
| status | \<job-id\> | Job state, assigned donors, progress |
| results | \<job-id\> | Output artifacts or download location |
| cancel | \<job-id\> | Cancellation confirmation |
| list | (none) | Table of submitted jobs with status |

### worldcompute cluster

| Subcommand | Arguments | Output |
|-|-|-|
| status | (none) | Cluster health, node count, coordinator status |
| peers | (none) | Connected peer list with trust scores |
| ledger-head | (none) | Current ledger head hash and height |

### worldcompute governance

| Subcommand | Arguments | Output |
|-|-|-|
| propose | \<title\> | Proposal ID, voting period, quorum requirement |
| list | (none) | Active proposals with status and vote counts |
| vote | \<proposal-id\> --position \<yes/no\> | Vote confirmation |
| report | \<proposal-id\> | Detailed proposal report with vote breakdown |

### worldcompute admin

| Subcommand | Arguments | Output |
|-|-|-|
| halt | (none) | Emergency halt confirmation (requires OnCallResponder role) |
| resume | (none) | Resume confirmation |
| ban | \<peer-id\> | Ban confirmation with audit record |
| audit | --since \<time\> | Audit log entries |

## Error Contract

All commands follow a consistent error format:
- **Missing role**: "Error: this command requires {role} role. Current roles: {roles}"
- **Not connected**: "Error: not connected to cluster. Run 'worldcompute donor join' first."
- **Invalid input**: "Error: {specific validation failure}"
- **Exit codes**: 0 = success, 1 = error, 2 = usage error
