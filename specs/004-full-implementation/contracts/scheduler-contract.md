# Scheduler Contract

## Broker Matchmaking

```
match_task(task: &TaskTemplate, nodes: &[NodeCapability]) -> Vec<LeaseOffer>
```

### Input
- `task`: Requirements (CPU, GPU, memory, trust tier, workload class, allowed regions)
- `nodes`: Available node capabilities (CPU, GPU, memory, trust tier, AS number, region)

### Output
- `Vec<LeaseOffer>` — up to R (replication factor) nodes from disjoint autonomous systems

### Behavior
- ClassAd-style bilateral match: task requirements ↔ node capabilities
- Disjoint AS enforcement for R=3 replicas
- Lease TTL configurable (default: 300s)
- Lease renewed on heartbeat
- Expired lease → task rescheduled from last checkpoint

## Lease Lifecycle

| State | Transition | Trigger |
|-|-|-|
| Active | → Active (renewed) | Heartbeat received within TTL |
| Active | → Expired | TTL exceeded without heartbeat |
| Active | → Released | Task completed or cancelled |
| Expired | → (rescheduled) | Broker finds new match |

## Graceful Degradation

When coordinator quorum is lost:
- Local broker continues dispatching from cached lease offers
- New lease requests queue locally
- Ledger writes queue locally
- On quorum restoration: CRDT merge reconciles all queued state
