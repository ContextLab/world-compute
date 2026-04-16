# World Compute v1 Planning Quickstart

**Document type**: Direct-test walkthrough (planning artifact)
**Spec**: `001-world-compute-core`
**Date**: 2026-04-15
**Status**: Planning — no binaries exist. Every command in this document is
target-syntax. Steps that require an unbuilt component are marked
**[REQUIRES UNBUILT COMPONENT]**.

---

## 1. Purpose

This document is the canonical direct-test walkthrough for World Compute v1.
It describes, step by step, how a test operator will verify that the three
P1 user stories — US1 (donor joins and contributes), US2 (submitter runs a
job and gets a correct result), and US3 (zero-config LAN cluster) — work
correctly on real hardware once binaries exist. It is the Principle V
enforcement harness for the planning phase: every P1 acceptance scenario in
`spec.md` must be walkable through one or more sections below.

This document is not end-user installation documentation. It does not describe
how to install World Compute for personal use. It describes what the
implementation team will exercise, in order, to demonstrate that the system
meets its own requirements on real donor-class machines. Evidence artifacts
produced by following this walkthrough are the direct-test artifacts required
by Principle V of the constitution and SC-010.

---

## 2. Prerequisites

### 2.1 Hardware

The operator must provision the following machines. All must be bare-metal or
real hardware; VMs hosting the agent are not acceptable for Principle V
compliance (the sandbox driver runs VMs inside the agent — nested
virtualization under an uncontrolled outer hypervisor is not representative of
a donor machine).

| Role | Count | Minimum spec | Notes |
|-|-|-|-|
| Primary Linux test node | 1 | x86-64, KVM-capable, 8 GB RAM, 100 GB NVMe | Phase 0 + Phase 1 donor |
| Secondary Linux test node | 1 | x86-64, KVM-capable, 8 GB RAM | Phase 1 donor |
| Tertiary Linux test node | 1 | x86-64 or ARM64, KVM-capable, 4 GB RAM | Phase 1 donor; ARM64 preferred for diversity |
| macOS test node | 1 | Apple Silicon (M1+) or Intel, 8 GB RAM | US1 sandbox-coverage requirement |
| Windows test node | 1 | Windows 10/11 Pro, Hyper-V enabled, 8 GB RAM | Optional at Phase 1; required at Phase 2 |
| GPU Linux test node | 1 | x86-64, KVM-capable, NVIDIA GPU in a verified singleton IOMMU group | Optional at Phase 1; required for FR-012 GPU passthrough test |
| Slurm test node | 2 | Any Linux, 4 GB RAM | US4 adapter smoke test; optional at Phase 1 |
| K8s test cluster | 3 | Any Linux, can be lightweight (k3s) | US4 adapter smoke test; optional at Phase 1 |

**Verifying singleton IOMMU group (GPU node)**: Before any GPU passthrough
test, confirm the GPU occupies its own IOMMU group and no ACS-override patch
is in use. **[REQUIRES UNBUILT COMPONENT: agent registration check]** The
agent will refuse GPU passthrough if IOMMU isolation is not verified; this
must be independently confirmed by the operator before starting GPU tests.

### 2.2 Network topology

Two isolated network segments are required:

- **Isolated LAN switch**: a dedicated switch connecting only the three Linux
  test nodes, with no upstream route to the internet. This switch is used for
  Phase 0 and Phase 1 air-gap tests.
- **Internet-connected segment**: a separate network with NAT gateway used for
  Phase 1 DHT-merge tests and Phase 2 onward.

The two segments must be physically switchable. The LAN nodes must be
physically disconnected from the internet segment during air-gap tests, not
merely firewalled. The adversarial tests in Section 5 require the ability to
run packet-injection tools on the isolated LAN switch.

### 2.3 Accounts

| Account | Purpose |
|-|-|
| Test coordinator quorum operator account | Signs evidence artifacts; required for threshold-signed ledger witness in Phase 1 |
| Submitter account | Used in US2 and US4 tests |
| Adversarial donor account | Used in adversarial wrong-result injection test (Section 5.4) |

### 2.4 Pre-built binaries

**Not yet available.** This quickstart is the target. When binaries exist,
they must be:

- Reproducibly built from source (deterministic `cargo build --release` output
  with locked `Cargo.lock`).
- Code-signed: macOS Notarization, Windows Authenticode, Linux GPG signature
  from the project's published signing key.
- Attested: the coordinator must verify TPM 2.0 PCR quotes before dispatching
  any job to a test node.

The single binary is `worldcompute`. All subcommands used in this document
(`donor`, `job`, `cluster`, `admin`) are planned subcommands of that binary.

---

## 3. Phase 0: Single-Machine Smoke Test

**Exercises**: US1 (acceptance scenarios 1, 2, 4), US2 (acceptance scenario 1)
**Success criteria verified**: SC-001, SC-010
**Constitution principles**: I, III, V

This phase runs on a single machine with no networking. Its sole purpose is to
confirm that the agent installs, sandboxes a trivial workload, returns a
correct result, and leaves no residue on uninstall. All three adversarial cases
below are mandatory before Phase 1 begins.

### 3.1 Install and enroll

**[REQUIRES UNBUILT COMPONENT: `worldcompute` binary, installer]**

```
# Download and verify the agent binary
curl -fsSL https://releases.worldcompute.org/v1/worldcompute-linux-x86_64 \
  -o worldcompute
sha256sum worldcompute  # verify against published checksum
gpg --verify worldcompute.sig worldcompute  # verify code signature

# Install
sudo install -m 0755 worldcompute /usr/local/bin/worldcompute

# Enroll as donor
worldcompute donor join --cpu-cap 50% --no-network
```

Expected output includes a Peer ID, confirmation that mDNS is disabled
(single-machine mode), and an idle-threshold message. The agent must drop to
an unprivileged UID after initialization; verify with:

```
ps aux | grep worldcompute
# UID field must not be root after the first line (initialization)
```

**Pass criterion**: Agent running as unprivileged user within 5 seconds of
`donor join`. **Fail criterion**: Agent runs as root beyond initialization.

### 3.2 Run a trivial echo workload

**[REQUIRES UNBUILT COMPONENT: local job submission path, Firecracker
integration]**

Take a filesystem snapshot before submission:

```
# Capture host state before job
find / -maxdepth 4 -not -path '/proc/*' -not -path '/sys/*' \
  2>/dev/null | sort > /tmp/fs-before.txt
```

Submit a known-output job:

```
# Known input
echo -n "worldcompute-phase0-test-2026-04-15" > /tmp/test-input.txt
sha256sum /tmp/test-input.txt
# Expected: record this hash as EXPECTED_HASH

worldcompute job submit --local-only --image oci+cid:sha256utils-v1 \
  --command "sha256sum /input/data.txt" \
  --input /tmp/test-input.txt \
  --output /tmp/test-output.txt
```

Verify the result:

```
cat /tmp/test-output.txt
# Must match EXPECTED_HASH
```

Run 100 consecutive times and record each result. All 100 must match
EXPECTED_HASH. **Pass criterion**: 100/100 correct. **Fail criterion**: Any
incorrect result, any sandbox timeout, any agent crash.

### 3.3 Verify no host residue after uninstall

**[REQUIRES UNBUILT COMPONENT: `donor withdraw` command, uninstaller]**

```
worldcompute donor withdraw
# Expected: "All World Compute state removed."

# Compare filesystem state
find / -maxdepth 4 -not -path '/proc/*' -not -path '/sys/*' \
  2>/dev/null | sort > /tmp/fs-after.txt

diff /tmp/fs-before.txt /tmp/fs-after.txt
```

The diff must show zero files outside the scoped working directory remaining.
The scoped working directory itself must be absent. No startup hooks, cron
entries, systemd units, or launchd plists may remain.

**Pass criterion**: Empty diff (allowing for normal OS churn like `/tmp`
entries unrelated to World Compute). **Fail criterion**: Any World Compute
file, process, or service entry persists after `withdraw`.

### 3.4 Phase 0 adversarial tests

These three tests are mandatory before Phase 1 is unlocked. Each must be run
after the main smoke test, on the same machine.

**AT-0.1 — Sandbox escape: read /etc/passwd**
**[REQUIRES UNBUILT COMPONENT: sandbox driver, adversarial workload image]**

```
worldcompute job submit --local-only \
  --image oci+cid:adversarial-read-host-v1 \
  --command "cat /etc/passwd" \
  --output /tmp/at01-output.txt
```

The adversarial image attempts to read `/etc/passwd` from inside the sandbox.

**Pass criterion**: Job exits with a non-zero code or produces empty output;
agent logs record a blocked host-file read attempt; `/etc/passwd` content does
not appear in `/tmp/at01-output.txt`. **Fail criterion (kill condition)**: Any
content from `/etc/passwd` appears in the output — this is a sandbox escape
and blocks all further testing until the vulnerability is remediated.

**AT-0.2 — Sandbox escape: write outside scoped directory**
**[REQUIRES UNBUILT COMPONENT: sandbox driver]**

```
worldcompute job submit --local-only \
  --image oci+cid:adversarial-write-host-v1 \
  --command "touch /tmp/worldcompute-escape-marker" \
  --output /tmp/at02-output.txt
```

**Pass criterion**: `/tmp/worldcompute-escape-marker` does not exist after job
completion; agent logs record a blocked write attempt. **Fail criterion**: The
marker file exists — sandbox escape, blocks all further testing.

**AT-0.3 — Non-zero exit code cleanup**
**[REQUIRES UNBUILT COMPONENT: agent cleanup logic]**

```
worldcompute job submit --local-only \
  --image oci+cid:sha256utils-v1 \
  --command "exit 1" \
  --output /tmp/at03-output.txt
```

**Pass criterion**: Agent reports job failure; sandbox working directory is
wiped; no zombie processes remain (`ps aux | grep worldcompute-sandbox` returns
nothing). **Fail criterion**: Any sandbox process or directory persists.

### 3.5 Phase 0 evidence artifact

The operator must produce the following file before Phase 1 begins:

```
evidence/phase0/smoke-test-YYYY-MM-DD.json
```

See Section 8 for the required artifact structure.

**Phase 0 kill conditions**: Any sandbox escape (AT-0.1 or AT-0.2), any
privileged-process persistence, any host-file read confirmed in output. A
Phase 0 kill condition halts work on Phase 1 until the root cause is identified
and AT-0.1/AT-0.2 are re-run cleanly.

---

## 4. Phase 1: Three-Machine LAN Cluster

**Exercises**: US1 (all acceptance scenarios), US2 (acceptance scenarios 1–3),
US3 (all acceptance scenarios)
**Success criteria verified**: SC-001, SC-002, SC-003, SC-006, SC-010, SC-012
**Constitution principles**: I, II, III, V

### 4.1 Air-gap the LAN

Connect only the three Linux test nodes to the isolated LAN switch. Physically
disconnect all three from any internet-connected interface. Verify isolation:

```
# On each node
ping 8.8.8.8  # must time out
ip route show  # must show only the LAN subnet; no default route to internet
```

### 4.2 Install and start the agent on all three nodes

**[REQUIRES UNBUILT COMPONENT: `worldcompute` binary]**

Repeat the install steps from Section 3.1 on all three nodes. On each:

```
worldcompute donor join --cpu-cap 50%
```

### 4.3 Verify cluster formation (US3, SC-003)

**[REQUIRES UNBUILT COMPONENT: mDNS discovery, Kademlia DHT]**

Record the time from `donor join` on the third node to the point where
`worldcompute cluster status` on any node shows all three peers:

```
worldcompute cluster status
# Expected output:
# Cluster: wc-lan-<id> (3 nodes)
# Peers: <node-a-id>, <node-b-id>, <node-c-id>
# DHT: island (no internet connectivity)
```

**Pass criterion**: All three peers visible and cluster marked ready within 5
seconds of the last `donor join` (SC-003). **Fail criterion**: Any peer missing
after 10 seconds, or cluster status not reachable.

### 4.4 Submit a test job and verify replicated execution (US2, US3)

**[REQUIRES UNBUILT COMPONENT: job scheduler, R=3 replica placement, quorum
logic]**

From node 1, submit a job whose correct output is known:

```
# Known input — record this hash before submission
echo -n "worldcompute-phase1-lan-2026-04-15" | sha256sum
# EXPECTED_HASH = <record this>

worldcompute job submit ./jobs/sha256-test.yaml
# Job ID returned: JOB_ID
```

The `sha256-test.yaml` manifest **[REQUIRES UNBUILT COMPONENT: job manifest
schema]**:

```yaml
apiVersion: worldcompute/v1
kind: Job
metadata:
  name: phase1-sha256-test
spec:
  image: oci+cid:sha256utils-v1
  command: ["sha256sum", "/input/data.txt"]
  inputs:
    - cid: <CID of test-input.txt>
      mount: /input/data.txt
  outputs:
    - name: result
      path: /output/result.txt
  replicas: 3
  job_class: scientific
  max_wall_time: 5m
```

Monitor job status:

```
worldcompute job status JOB_ID --watch
# Observe: queued → leased (3 nodes) → running → verifying → verified
```

Fetch the result:

```
worldcompute job results JOB_ID --output ./phase1-result.txt
cat ./phase1-result.txt
# Must equal EXPECTED_HASH
```

**Pass criterion**: Result matches EXPECTED_HASH; job transitions through all
states; job status shows R=3 replicas on three distinct nodes; a signed
`WorkUnitReceipt` is present. **Fail criterion**: Wrong result, fewer than 3
replicas, no signed receipt.

### 4.5 Verify the signed WorkUnitReceipt (US2, SC-012)

**[REQUIRES UNBUILT COMPONENT: `donor credits --verify`, Merkle ledger]**

```
worldcompute donor credits --verify
# Expected output includes:
# Ledger root:    sha256:<hash>
# Proof depth:    <N> nodes
# Verification:   PASS  (locally verified against published Merkle root)
```

**Pass criterion**: `Verification: PASS`. **Fail criterion**: Any
`FAIL` or cryptographic error.

### 4.6 Preemption test (US1, acceptance scenario 3)

**[REQUIRES UNBUILT COMPONENT: preemption supervisor, keyboard event
detection]**

Start a long-running job on the cluster. While it is in state `running` on
node 1, use an input-injection tool to simulate keyboard activity:

```
# On node 1, in a separate terminal, while the job is running:
# Linux: use evemu-play or xdotool to inject a keypress
xdotool key Return
```

Observe agent logs on node 1:

```
worldcompute donor logs --follow
# Must show: "Sovereignty event: keyboard; SIGSTOP sent; elapsed: <N>ms"
# N must be <= 10
```

After the SIGSTOP, observe that the standby replica (pre-warmed on node 2 or
node 3) continues the job and it completes:

```
worldcompute job status JOB_ID --watch
# Must transition from "preempted on node-1" to "running on node-2" (or node-3)
# and then to "verified"
```

**Pass criterion**: SIGSTOP within 10 ms of simulated keyboard event (logged
timestamp); job completes from a different node; result is correct.
**Fail criterion**: SIGSTOP latency > 10 ms, job does not recover, or result
is wrong after recovery.

### 4.7 Node failure and reschedule from checkpoint (US1, SC, Principle II)

**[REQUIRES UNBUILT COMPONENT: heartbeat detection, checkpoint resume]**

Start a new long-running job. Wait until at least one checkpoint has been
written (60 seconds after job enters `running` state). Then hard-power-off
node 2 by pulling the power cable (not a graceful shutdown — a real failure).

Observe the remaining two nodes:

```
worldcompute cluster status
# Node 2 must transition to "failed" within the heartbeat timeout window
# (target: 10–30 seconds, per two missed heartbeat intervals)

worldcompute job status JOB_ID --watch
# Must show: reschedule from last checkpoint CID onto surviving node
# Must eventually show: verified
```

**Pass criterion**: Node failure detected within 30 seconds; job resumes from
checkpoint on a surviving node; result is correct and matches EXPECTED_HASH.
**Fail criterion**: Job does not recover, result is wrong, or the system does
not detect the failure within 60 seconds.

Power node 2 back on and let it rejoin before continuing.

### 4.8 Internet merge (US3, acceptance scenario 3)

**[REQUIRES UNBUILT COMPONENT: DHT merge, credit CRDT reconciliation]**

While the cluster is running and has a non-zero credit ledger (run several jobs
to accumulate ledger entries), enable internet connectivity on the LAN gateway:

```
# On the gateway: add default route to internet
ip route add default via <gateway-ip>
```

Observe the cluster:

```
worldcompute cluster status
# Must transition from: DHT: island
# To:                   DHT: merged (global)
# Within: 60 seconds of internet becoming reachable
```

After merge, verify credit ledger integrity:

```
worldcompute donor credits --verify
# Verification: PASS
# No credit entries may be duplicated or missing
```

**Pass criterion**: DHT merge completes without error; credit ledger passes
verification; no in-flight jobs are dropped during the merge transition.
**Fail criterion**: Credit duplication, job loss, or ledger verification
failure.

---

## 5. Phase 1 Adversarial Tests

**Exercises**: US1, US2, US3
**Success criteria verified**: SC-006
**Constitution principles**: I, II, V

All four tests below are mandatory. They must be run after Section 4 passes
and before Phase 2 is unlocked.

### 5.1 Adversarial test: sandbox escape attempt (AT-1.1)

**[REQUIRES UNBUILT COMPONENT: adversarial workload image, sandbox driver]**

From node 1, submit a workload that explicitly attempts to read a host file:

```
worldcompute job submit --image oci+cid:adversarial-read-host-v1 \
  --command "cat /etc/passwd" \
  --output ./at11-result.txt
```

**Pass criterion**: Output file is empty or contains only an error message;
agent logs on the executing donor node record a blocked read attempt; no
content from the host `/etc/passwd` appears in the output.
**Fail criterion (kill condition)**: Any `/etc/passwd` content in output. This
is a Principle I P0 incident. Halt all testing. Do not proceed to Phase 2.

### 5.2 Adversarial test: host-network probe attempt (AT-1.2)

**[REQUIRES UNBUILT COMPONENT: sandbox network isolation, adversarial workload]**

Submit a workload that attempts to connect to a LAN peer's IP from inside the
sandbox:

```
worldcompute job submit --image oci+cid:adversarial-network-probe-v1 \
  --command "curl http://<node-2-lan-ip>:22" \
  --output ./at12-result.txt
```

**Pass criterion**: Connection refused or timed out from inside the sandbox;
output contains no successful response from the LAN peer; agent logs record the
blocked outbound connection. **Fail criterion**: The workload successfully
connects to a LAN peer — sandbox network isolation has failed.

### 5.3 Adversarial test: malformed-peer flood (AT-1.3)

**[REQUIRES UNBUILT COMPONENT: libp2p stack, adversarial flood tool]**

Using a separate machine or a network tool on the isolated switch, flood the
libp2p listen port of node 1 with malformed protocol messages for 60 seconds:

```
# Example using a custom flood tool (to be developed alongside the agent)
worldcompute-test-tools flood-libp2p \
  --target <node-1-ip>:<libp2p-port> \
  --rate 10000/s \
  --duration 60s \
  --payload random-malformed
```

During the flood, verify that node 1 remains functional:

```
worldcompute cluster status   # must show all 3 peers
worldcompute job submit ./jobs/sha256-test.yaml   # must complete correctly
```

**Pass criterion**: Cluster remains operational during and after the flood; no
node crash; submitted job completes correctly. **Fail criterion**: Any node
crashes, drops from the cluster, or produces wrong results during the flood.

### 5.4 Adversarial test: wrong-result injection (AT-1.4, SC-006)

**[REQUIRES UNBUILT COMPONENT: compromised-agent mode, audit / quorum
detection]**

This test requires a modified agent binary that deliberately returns an
incorrect result for every task (specifically: `sha256(garbage)` instead of
the correct SHA-256 of the input). Install this compromised agent on node 3.
Submit 200 tasks:

```
worldcompute job submit --replicas 3 --count 200 ./jobs/sha256-test.yaml
```

Observe audit behavior:

```
worldcompute admin audit --node <node-3-id>
# Must show: node-3 flagged as Byzantine within the first 100 audited tasks
# with >=95% probability (SC-006)
```

After flagging, submit one more job:

```
worldcompute job submit ./jobs/sha256-test.yaml
# Must complete correctly without using node-3 as a replica
```

**Pass criterion**: Node 3 is detected and quarantined within 100 audited
tasks; all 200 jobs return correct results (quorum overrides the wrong
result); post-quarantine job completes correctly on the remaining two nodes
plus an auto-recruited replacement. **Fail criterion**: Wrong result accepted
by the system, or node 3 not quarantined within 100 tasks.

Restore node 3 to a clean agent before Phase 2.

---

## 6. Phase 1 Submitter Flow (US2)

**Exercises**: US2 (all acceptance scenarios)
**Success criteria verified**: SC-010, SC-012

### 6.1 Submit a minimal ML matmul job

**[REQUIRES UNBUILT COMPONENT: WASM or OCI matmul workload, CID-based input
staging]**

Prepare a known-correct matrix multiplication:

```
# 4x4 identity * 4x4 test matrix = test matrix (trivially verifiable)
# Pre-compute EXPECTED_RESULT locally
python3 -c "import numpy as np; A=np.eye(4); B=np.arange(16).reshape(4,4); print((A@B).tolist())"
# EXPECTED_RESULT = [[0.0,1.0,2.0,3.0],[4.0,5.0,6.0,7.0],[8.0,9.0,10.0,11.0],[12.0,13.0,14.0,15.0]]
```

Submit:

```
worldcompute job submit ./jobs/matmul-test.yaml
```

The `matmul-test.yaml` manifest **[REQUIRES UNBUILT COMPONENT]**:

```yaml
apiVersion: worldcompute/v1
kind: Job
metadata:
  name: phase1-matmul-test
spec:
  image: oci+cid:numpy-matmul-v1
  command: ["python3", "/job/matmul.py"]
  inputs:
    - cid: <CID of matrix inputs>
      mount: /input/
  outputs:
    - name: result
      path: /output/result.json
  replicas: 3
  job_class: scientific
  checkpointing: enabled
  max_wall_time: 10m
```

Observe staging, replication, and result fetch:

```
worldcompute job status JOB_ID --watch
worldcompute job results JOB_ID --output ./matmul-result.json
cat ./matmul-result.json
# Must equal EXPECTED_RESULT
```

**Pass criterion**: Result matches EXPECTED_RESULT; `--watch` shows staging,
replication across 3 distinct nodes, and verified status; a signed receipt is
present. **Fail criterion**: Wrong result, missing receipt, or fewer than 3
replicas.

### 6.2 Verify the signed WorkUnitReceipt cryptographically

**[REQUIRES UNBUILT COMPONENT: receipt verification command, Merkle ledger]**

```
worldcompute donor credits --verify
```

**Pass criterion**: Output shows `Verification: PASS` and a Rekor entry ID
(or local Merkle proof during Phase 1 before Rekor integration is live).
**Fail criterion**: Any cryptographic verification failure.

### 6.3 Confidential job rejection (US2, acceptance scenario 4)

**[REQUIRES UNBUILT COMPONENT: confidential job classification, T3+ node
detection]**

Submit a job marked `confidential`:

```yaml
# confidential-test.yaml
apiVersion: worldcompute/v1
kind: Job
metadata:
  name: phase1-confidential-test
spec:
  image: oci+cid:sha256utils-v1
  command: ["sha256sum", "/input/data.txt"]
  inputs:
    - cid: <CID of test-input.txt>
      mount: /input/data.txt
  outputs:
    - name: result
      path: /output/result.txt
  confidentiality: confidential
  replicas: 1
```

```
worldcompute job submit ./confidential-test.yaml
```

**Pass criterion**: Job is rejected immediately with an error such as:
`No eligible T3+ nodes (SEV-SNP/TDX/H100 Confidential Compute) available in
this cluster.` The job must never be dispatched to the T1 Phase 1 test nodes.
**Fail criterion**: The job is accepted or dispatched to a T1 node.

---

## 7. Phase 1 Adapter Smoke Tests (US4, Optional)

**Exercises**: US4 (acceptance scenarios 1 and 2)
**Priority**: Optional at Phase 1; required before Phase 2 GA gate

### 7.1 Slurm adapter

**[REQUIRES UNBUILT COMPONENT: Slurm pilot-job gateway adapter]**

On the two-node Slurm testbed, install the adapter:

```
worldcompute admin slurm-adapter install \
  --slurm-host <slurm-head-node> \
  --partition worldcompute-test \
  --cpu-cap 50%
```

Verify the adapter appears as a node in the cluster:

```
worldcompute cluster status
# Must show: <slurm-adapter-id> as a node with correct aggregate capacity
```

Submit a pilot job:

```
worldcompute job submit ./jobs/sha256-test.yaml
worldcompute job results JOB_ID --output ./slurm-result.txt
cat ./slurm-result.txt
# Must equal EXPECTED_HASH
```

**Pass criterion**: Adapter visible as a cluster node; job dispatched via
Slurm's normal scheduler; result correct. **Fail criterion**: Wrong result or
adapter not visible.

### 7.2 Kubernetes adapter

**[REQUIRES UNBUILT COMPONENT: Kubernetes CRD operator]**

On the k3s test cluster, install the World Compute operator:

```
kubectl apply -f https://releases.worldcompute.org/v1/k8s-operator.yaml
```

Apply a `ClusterDonation` CRD:

```yaml
# cluster-donation.yaml
apiVersion: worldcompute.org/v1
kind: ClusterDonation
metadata:
  name: phase1-k8s-test
spec:
  cpuCap: "50%"
  memoryCap: "4Gi"
  jobClasses: ["scientific", "public-good"]
  namespace: worldcompute-jobs
```

```
kubectl apply -f cluster-donation.yaml
```

Verify a Pod is created and returns a correct result:

```
worldcompute job submit ./jobs/sha256-test.yaml
worldcompute job results JOB_ID --output ./k8s-result.txt
cat ./k8s-result.txt
# Must equal EXPECTED_HASH
```

**Pass criterion**: CRD applied; Pod created in `worldcompute-jobs` namespace;
result correct. **Fail criterion**: No Pod created, or wrong result.

---

## 8. Direct-Test Evidence Artifact

Every phase must produce a signed evidence artifact before the next phase is
unlocked. This is the Principle V / SC-010 compliance record.

### 8.1 Required structure

Each evidence artifact is a JSON file at the path:

```
evidence/<phase>/<test-id>-YYYY-MM-DD.json
```

The required fields are:

```json
{
  "schema_version": "1",
  "phase": "phase0 | phase1 | phase2 | phase3",
  "test_id": "smoke-test | lan-cluster | adversarial-AT-1.1 | ...",
  "date_utc": "2026-04-15T00:00:00Z",
  "operator": {
    "name": "<operator name>",
    "signature": "<GPG or SSH signature over the sha256 of this file>"
  },
  "environment": {
    "host_os": "Linux 6.x.x x86_64",
    "agent_version": "0.1.0-dev+<git-sha>",
    "agent_sha256": "<sha256 of worldcompute binary>",
    "hardware": "<CPU model, RAM, storage>",
    "network": "isolated-lan | lan-with-internet | internet"
  },
  "inputs": [
    {
      "name": "test-input.txt",
      "sha256": "<hash of input>",
      "cid": "<CIDv1>"
    }
  ],
  "expected_outputs": [
    {
      "name": "result.txt",
      "sha256": "<expected hash>"
    }
  ],
  "observed_outputs": [
    {
      "name": "result.txt",
      "sha256": "<observed hash>",
      "match": true
    }
  ],
  "adversarial_results": [
    {
      "test_id": "AT-0.1",
      "description": "Workload attempts to read /etc/passwd",
      "pass": true,
      "evidence": "Agent log excerpt: 'blocked host read /etc/passwd'"
    }
  ],
  "pass": true,
  "fail_reason": null,
  "notes": "Free text. Record any deviations from the steps above."
}
```

### 8.2 Signing the artifact

**[REQUIRES UNBUILT COMPONENT: `worldcompute admin evidence sign` command, or
use manual GPG]**

```
sha256sum evidence/phase0/smoke-test-2026-04-15.json > \
  evidence/phase0/smoke-test-2026-04-15.json.sha256
gpg --sign --detach evidence/phase0/smoke-test-2026-04-15.json
```

The operator's GPG key must be the same key registered in the coordinator
quorum for Phase 1 and beyond. For Phase 0, any GPG key controlled by the
operator is sufficient; record the key fingerprint in the `operator` field.

### 8.3 Mandatory evidence files per phase

| Phase | Mandatory artifact files |
|-|-|
| Phase 0 | `evidence/phase0/smoke-test-YYYY-MM-DD.json` (includes AT-0.1, AT-0.2, AT-0.3) |
| Phase 1 | `evidence/phase1/lan-cluster-YYYY-MM-DD.json` (includes cluster formation, preemption, node failure, DHT merge), `evidence/phase1/adversarial-YYYY-MM-DD.json` (AT-1.1 through AT-1.4), `evidence/phase1/submitter-flow-YYYY-MM-DD.json` (US2 matmul, receipt verification, confidential rejection) |
| Phase 2 | One artifact per 72-hour testnet run; one per Sybil/flooding adversarial test |
| Phase 3 | Monthly aggregate artifact; external penetration test report; red-team exercise report |

---

## 9. Phase Gating Table

This table defines the concrete pass criteria and kill conditions for each
phase transition. No phase may be skipped. All criteria must be met
simultaneously before the next phase begins.

| Phase | Pass criteria | Kill conditions |
|-|-|-|
| Phase 0 → Phase 1 | 100/100 trivial workloads correct; zero host-file reads; zero privileged-process persistence after withdraw; all three AT-0.x adversarial tests pass; evidence artifact signed and filed | Any sandbox escape (AT-0.1 or AT-0.2); any host-file read confirmed in output; any privileged process surviving withdraw |
| Phase 1 → Phase 2 | SC-003: cluster forms in <5s; SC-006: wrong-result donor quarantined within 100 tasks; preemption SIGSTOP within 10ms; node failure reschedules from checkpoint and completes correctly; DHT merge leaves ledger intact; all four AT-1.x adversarial tests pass; US2 matmul result correct; signed receipt verifies; confidential job rejected; evidence artifacts signed and filed | Any sandbox escape; any cross-node data leakage; host OOM on any machine from workload; data loss from simulated node failure; wrong result accepted by quorum |
| Phase 2 → Phase 3 | ≥80% job completion over 72h with 30% churn (SC-004); ≥20 nodes across ≥3 geographic regions and ≥3 autonomous systems; Sybil attack rate-limited; Byzantine node detected within N re-runs; network partition recovers without data loss; external security audit started; 501(c)(3) incorporation filed | Data loss from churn; verified Byzantine node undetected; any host machine affected outside scoped directory; external audit finds critical/high unresolved finding |
| Phase 3 → GA | ≥90% job completion over 30 days (SC-005); zero real-world Principle I incidents; resource yield <1s P99 across all donors (SC-002); external security audit cleared (critical and high findings remediated); 501(c)(3) fully operational; governance structure seated (TSC + board); incident-disclosure drill completed; energy/carbon footprint published; all production components have evidence artifacts (SC-010) | Any real-world sandbox escape or host-data exfiltration; external audit finds unresolved critical finding; governance structure not operational |

---

## 10. Operator Checklist

Before signing off on Phase 0 and unlocking Phase 1, the operator must be
able to check every box below. Sign and date the completed checklist and attach
it to the Phase 0 evidence artifact.

**Phase 0 sign-off**

- [ ] `worldcompute donor join` produces a non-root agent process within 5
  seconds on a clean machine.
- [ ] 100/100 runs of the trivial SHA-256 workload return the correct expected
  hash.
- [ ] Filesystem diff after `worldcompute donor withdraw` shows no World
  Compute files, processes, or service entries remaining.
- [ ] AT-0.1: workload cannot read `/etc/passwd` from inside the sandbox.
- [ ] AT-0.2: workload cannot write outside the scoped working directory.
- [ ] AT-0.3: non-zero exit code leaves no zombie processes or residual
  directories.
- [ ] Evidence artifact `evidence/phase0/smoke-test-YYYY-MM-DD.json` is
  complete and operator-signed.
- [ ] No kill condition was triggered during Phase 0.

**Phase 1 sign-off**

- [ ] Three-machine LAN cluster forms within 5 seconds of the last `donor join`
  with no internet, no manual configuration (SC-003).
- [ ] SHA-256 test job with R=3 replicas returns the correct expected hash.
- [ ] `worldcompute donor credits --verify` returns `Verification: PASS`.
- [ ] Keyboard-injection preemption test: SIGSTOP logged within 10 ms;
  standby replica completes the job correctly.
- [ ] Hard power-off of one node: job reschedules from checkpoint and completes
  correctly within 2x expected wall-clock time.
- [ ] Internet-enable DHT merge: credit ledger passes verification after
  merge; no in-flight jobs dropped.
- [ ] AT-1.1: workload cannot read `/etc/passwd` from a multi-node cluster
  environment.
- [ ] AT-1.2: workload cannot connect to a LAN peer from inside the sandbox.
- [ ] AT-1.3: malformed-peer flood for 60 seconds does not crash any node;
  a test job completes correctly during the flood.
- [ ] AT-1.4: compromised donor injecting wrong results is quarantined within
  100 audited tasks; all jobs return correct results throughout.
- [ ] Matmul job result matches expected output; signed receipt verifies.
- [ ] Confidential job is rejected with no dispatch to T1 nodes.
- [ ] Evidence artifacts for LAN cluster, adversarial tests, and submitter
  flow are complete and operator-signed.
- [ ] No kill condition was triggered during Phase 1.

**Operator name**: ___________________________

**Date**: ___________________________

**Signature**: ___________________________

---

*This document is a planning artifact. It describes the target behavior that
will be verified once the `worldcompute` binary and its dependencies are
implemented. All commands are target-syntax. All pass and fail criteria are
binding requirements derived from the feature specification
(`specs/001-world-compute-core/spec.md`), the ratified constitution
(`.specify/memory/constitution.md`), and the testing research
(`specs/001-world-compute-core/research/07-governance-testing-ux.md`).*
