# Contract: Evidence artifact format

**Scope**: The directory structure and files produced by every real-hardware test run that generates evidence (FR-015, FR-016, FR-020a, FR-028a, plus SC-001 through SC-010 where real-hardware evidence is required).

## Directory layout

```
evidence/
└── phase<N>/                  # N matches the project phase; 1 for spec 005
    └── <area>/                # One of: firewall-traversal, attestation, diffusion-mesh, cloud-adapter, churn, quickstart, firecracker-rootfs
        └── <UTC-timestamp>/   # ISO 8601 basic, e.g., 20260419T142030Z
            ├── run.log
            ├── metadata.json
            ├── results.json
            ├── trace.jsonl            # optional
            ├── screenshots/           # optional directory
            │   └── *.png
            └── index.md
```

## File contracts

### `run.log`

Plain text. Full combined stdout+stderr of the test run. UTF-8. No rotation — one file per run. Size target < 10 MB; if larger, the run is atypical and the driver SHOULD investigate before committing.

### `metadata.json`

```json
{
  "run_id": "<uuid>",
  "area": "firewall-traversal",
  "spec": "005-production-readiness",
  "git_sha": "abc123...",
  "software_version": "0.5.0-rc1",
  "started_at": "2026-04-19T14:20:30Z",
  "ended_at": "2026-04-19T14:35:42Z",
  "machines": [
    {
      "hostname": "tensor02.dartmouth.edu",
      "os": "Rocky Linux 9.3",
      "kernel": "5.14.0-362.24.2.el9_3.x86_64",
      "cpu_model": "Intel Xeon Gold 6338",
      "gpus": ["NVIDIA H100 80GB PCIe", "NVIDIA H100 80GB PCIe"],
      "memory_gb": 1024,
      "network_profile": "institutional-firewall"
    }
  ],
  "env": {
    "RUST_LOG": "info,libp2p_swarm=debug",
    "any_other_relevant_env": "value"
  }
}
```

### `results.json`

```json
{
  "overall": "pass",
  "assertions": [
    {
      "name": "SC-001: 10-minute continuous relay connection",
      "expected": "connection_holds_seconds >= 600",
      "observed": {"connection_holds_seconds": 812},
      "pass": true
    },
    {
      "name": "FR-006: reservation reacquire after loss",
      "expected": "reacquire_seconds <= 60",
      "observed": {"reacquire_seconds": 23, "trigger": "relay_reboot_simulated"},
      "pass": true
    }
  ]
}
```

### `trace.jsonl` (optional)

NDJSON event trace. One JSON object per line. Used for post-hoc replay. Format:

```
{"ts": "2026-04-19T14:20:31.123Z", "event": "dial_attempt", "target": "/ip4/...", "transport": "tcp", "outcome": "timeout"}
{"ts": "2026-04-19T14:20:33.567Z", "event": "dial_attempt", "target": "/ip4/.../wss/...", "transport": "wss", "outcome": "success"}
```

### `index.md`

Human-readable summary. Template:

```markdown
# Evidence: <area> / <timestamp>

**Run ID**: <uuid>
**Git SHA**: <sha>
**Outcome**: ✅ PASS / ❌ FAIL
**Duration**: <ended - started>

## Machines

(table of machines, one row each)

## Assertions

(table of assertions with pass/fail)

## Key artifacts

- [run.log](./run.log)
- [metadata.json](./metadata.json)
- [results.json](./results.json)
- [trace.jsonl](./trace.jsonl)
- [screenshots/](./screenshots/)

## Notes

(freeform operator notes)
```

## Validation

A helper `scripts/validate-evidence.sh <dir>` checks:
- All required files present.
- `metadata.json.git_sha` matches a valid commit in the repo.
- `results.json.overall` is one of `pass | fail | partial`.
- Filesystem total size ≤ 10 MB (soft warn at 5 MB).

## Release gate

A release tag MAY be cut only if every SC with a real-hardware evidence requirement has at least one `overall: pass` artifact committed under `evidence/phase1/<area>/` on the release branch, for the commit being tagged.
