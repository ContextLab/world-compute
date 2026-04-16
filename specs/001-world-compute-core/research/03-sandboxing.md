# 03 — Sandboxing and Host Integrity Architecture

**Stage**: Research  
**Date**: 2026-04-15  
**Author**: Scientist (automated research stage)  
**Constitution anchor**: Principle I — Safety First

---

## [OBJECTIVE]

Identify a defense-in-depth sandbox architecture for World Compute that:
- Isolates cluster workloads completely from donor host filesystems, credentials, network state, and peripherals
- Operates on donor-class hardware spanning personal laptops, phones, Raspberry Pis, gaming rigs, Macs (Intel + Apple Silicon), Linux servers, Windows desktops, and cloud VMs
- Supports GPU/accelerator passthrough for ML workloads
- Provides cryptographic attestation of what is running
- Imposes overhead low enough that idle donors do not notice (Principle IV)

---

## [FINDING] Process-level sandboxing alone is never sufficient

No process-level isolation mechanism — seccomp-bpf, Landlock, Linux namespaces, bubblewrap, AppArmor, SELinux, V8 isolates, or Deno permissions — provides an adequate boundary for untrusted cluster workloads running on donor machines.

[EVIDENCE] The attack surface of a process sharing the host kernel is the entire kernel syscall table. Kernel exploits (e.g., Dirty Pipe CVE-2022-0847, Dirty COW CVE-2016-5195, io_uring privilege escalation classes) cross process boundaries because the kernel is shared. Container escapes in runc, Docker, and Kubernetes have been reported repeatedly; CVE-2019-5736 (runc exec), CVE-2020-15257 (Containerd), and similar demonstrate this class.

[EVIDENCE] seccomp-bpf and Landlock reduce but do not eliminate the kernel attack surface. A single unfiltered syscall with an exploitable implementation terminates isolation. In a volunteer cluster context where workloads are submitted by arbitrary third parties, reduction is insufficient — the boundary must be a hardware-enforced privilege boundary (ring 0 / hypervisor).

[EVIDENCE] gVisor (user-space kernel) adds a second kernel layer but is still susceptible to Sentry (the gVisor kernel process) exploits; the Sentry runs in the host kernel's address space and still exposes a syscall surface (ptrace mode) or a KVM exit surface (KVM mode). gVisor's KVM mode approximates but does not equal full VM isolation.

[CONFIDENCE] HIGH. This is the consensus position of security researchers, cloud providers (AWS, Google, Azure all use VM-level isolation for multi-tenant untrusted code), and the academic literature on isolation.

**Answer**: No. We cannot trust process-level sandboxing alone on any platform.

---

## 1. Per-Platform Recommendation

### 1.1 Linux (x86-64, ARM64) — Primary Tier

**Recommended primary**: Firecracker microVM + KVM

[FINDING] Firecracker provides the best balance of isolation strength, boot speed, and resource overhead for Linux hosts.

[EVIDENCE] Firecracker's VMM threads impose ≤5 MiB memory overhead for a 1-vCPU, 128 MiB RAM microVM. Boot-to-userspace latency is ≤125 ms from API call. Guest CPU performance is >95% of bare-metal equivalent. Virtualization adds an average of 0.06 ms latency overhead. (Source: Firecracker SPECIFICATION.md, continuously CI-enforced.)

[EVIDENCE] Firecracker's minimal device model — no USB, no PCI bus, no BIOS, no legacy hardware emulation — reduces the VMM attack surface to a small, auditable set of virtio devices. The Firecracker codebase is written in Rust with an explicit goal of minimal attack surface.

[EVIDENCE] AWS Lambda, Fly.io, and other multi-tenant serverless platforms run Firecracker in production at scale, providing real-world validation of its isolation model.

Architecture:
- KVM hypervisor boundary (ring-0 isolation, hardware-enforced on Intel VT-x / AMD-V / ARM VHE)
- Firecracker VMM as the sole guest interface
- Linux namespaces + seccomp-bpf inside the VMM process (defense-in-depth layer 2)
- Ephemeral rootfs image (read-only base + size-capped writable overlay, wiped on job exit)
- Network: dedicated veth pair into an isolated bridge; no access to host LAN segments

**Fallback (if KVM unavailable)**: Kata Containers with QEMU-lite or cloud-hypervisor backend. Kata adds 10–30% startup overhead vs Firecracker but still provides full VM isolation. Accept for hosts without KVM acceleration (nested virt, some cloud VMs).

**Do not use as primary**: gVisor alone, runc/containerd alone, bubblewrap alone.

[CONFIDENCE] HIGH.

---

### 1.2 macOS — Intel + Apple Silicon

**Recommended**: Apple Virtualization.framework (VZVirtualMachine)

[FINDING] Apple's Virtualization.framework is the only supported path to full VM-boundary isolation on macOS. It is hardware-accelerated on both Intel (Hypervisor.framework / VT-x) and Apple Silicon (ARM virtualization extensions), runs as a sandboxed App Sandbox process, and is the foundation Docker Desktop uses as of mid-2025 (deprecating QEMU on Apple Silicon July 2025).

[EVIDENCE] Virtualization.framework VMs run in a dedicated hypervisor entitlement sandbox. The guest kernel runs in EL2 (ARM) or VMX root mode (Intel), providing hardware ring-level isolation from the host. Guest memory is mapped outside the host process address space.

[EVIDENCE] Projects like Tart (macOS CI VMs) and UTM run production workloads on Virtualization.framework, confirming stability and performance on both Intel and M-series chips.

Practical architecture:
- VZVirtualMachine with a minimal Linux guest (Alpine or stripped Ubuntu)
- VZVirtioBlockDevice for ephemeral storage (size-capped)
- VZVirtioNetworkDevice with isolated NAT; no host bridge access
- No VZUSBController, no VZGraphicsDevice exposed to workload
- Agent daemon running outside the VM as host-side manager; workload runner inside VM only

**GPU on macOS**: Apple Silicon GPU is not expose-able to guest VMs via Virtualization.framework as of 2026 — Metal is host-only. GPU workloads on Mac donors are CPU-fallback only, or must wait for Apple to expose paravirtual GPU. This is a known limitation. macOS donors should be classified as CPU-only nodes unless Apple changes this.

[CONFIDENCE] HIGH for CPU isolation; MEDIUM for GPU timeline.

---

### 1.3 Windows — x86-64

**Recommended**: Hyper-V isolation via WSL2 VM infrastructure or direct Hyper-V guest

[FINDING] Hyper-V provides hardware-enforced VM isolation on Windows and is available on Windows 10/11 Pro/Enterprise and Windows Server. Windows Home users require a fallback.

Architecture:
- Primary: Hyper-V Type-1 hypervisor boundary (available when "Virtual Machine Platform" feature is enabled)
- Use Windows Subsystem for Linux 2 (WSL2) infrastructure as the managed VM layer; WSL2 VMs run inside a real Hyper-V partition
- Alternatively, use containerd + Windows HCS (Host Compute Service) with Hyper-V isolation mode, which spins an isolated utility VM per container

[EVIDENCE] WSL2 uses a Hyper-V lightweight VM (the "Utility VM") to run Linux workloads, providing a ring-0 boundary. Microsoft's own documentation describes WSL2 as running "within a lightweight utility virtual machine (VM)."

**Windows Home fallback**: Windows Home lacks Hyper-V. Use QEMU with WHPX (Windows Hypervisor Platform) acceleration. WHPX is available on Windows Home via the "Windows Hypervisor Platform" optional feature. Overhead is higher (~15% vs Hyper-V native) but still VM-boundary isolated.

**GPU on Windows**: Windows VMs can expose GPU via GPU-P (GPU Paravirtualization, available for DirectX workloads) or via RemoteFX-successor APIs. For CUDA workloads, NVIDIA GRID / vGPU driver stack inside a Hyper-V VM is the supported path but requires GRID licensing. Practical recommendation: Windows GPU donors contribute via a CUDA-enabled Docker container inside a Hyper-V isolated container — this is supported in Windows 11 via WSL2 + CUDA.

[CONFIDENCE] MEDIUM-HIGH. WSL2 path is well-validated; WHPX fallback has less public security scrutiny.

---

### 1.4 Mobile (Android, iOS)

**Recommended**: Browser-based WASM execution only (see Section 2); no native VM layer

[FINDING] Neither Android nor iOS provides public APIs for running arbitrary hypervisors or KVM-equivalent isolation from user-space apps. Android does expose KVM to privileged system apps but not to third-party APKs. iOS prohibits JIT compilation for App Store apps (except via entitlements), making even WASM JIT impractical.

[EVIDENCE] Android's `/dev/kvm` is accessible to select system apps (Android Emulator, Chrome for ARCVM) but requires system-level privileges unavailable to a Play Store app. iOS has no KVM equivalent.

Practical consequence: Mobile donors are **browser-WASM donors only**. They run a web page (PWA) that executes WASM workloads inside the browser sandbox. This is the zero-install path described in Section 2. Mobile nodes are CPU-only, low-power contributors — suitable for small embarrassingly parallel tasks (parameter sweeps, inference on small models), not GPU training.

[CONFIDENCE] HIGH. This is a platform constraint, not a design choice.

---

### 1.5 Browser (opted-in zero-install donors)

**Recommended**: Browser WASM sandbox + SharedArrayBuffer + WebGPU

[FINDING] The browser provides a meaningful but not VM-equivalent sandbox. Its value is zero-install reach — any device with a modern browser becomes a potential donor.

Architecture:
- Workload compiled to WASM, executed in a Worker thread (isolated from DOM)
- SharedArrayBuffer for multi-threaded workloads (requires COOP/COEP headers)
- WebGPU for GPU acceleration (available in Chrome, Firefox, Safari as of 2025/2026)
- Origin isolation via separate cross-origin iframe or dedicated origin for the worker host

**Limitations**: Browser WASM does not provide VM-boundary isolation. The browser's sandbox (process isolation + site isolation) is meaningful but has had escape CVEs. Browser donors are **explicitly lower-trust nodes**; they should receive only workloads that are safe to run on lower-trust infrastructure (e.g., inference tasks where model weights are public, embarrassingly parallel compute).

[EVIDENCE] WASM can be up to 45% slower than native code in compute-heavy scenarios due to SIMD and parallelism limitations. WebGPU is now production-viable for inference workloads (used by Transformers.js, Candle-WASM) but not for full CUDA-equivalent training.

[CONFIDENCE] HIGH for use case scoping; MEDIUM for security boundary strength vs VM-level.

---

## 2. WASM as Universal Lowest-Common-Denominator Layer

[FINDING] WASM is viable as a portability and distribution layer for CPU workloads on non-VM-capable hosts, but is NOT a security-equivalent replacement for VM isolation on capable hardware.

**What WASM is good for**:
- Zero-install browser donors (see 1.5)
- Mobile donors where no VM path exists (see 1.4)
- Portable workload packaging format: compile once, run on any WASM runtime (Wasmtime, WasmEdge, WAMR, browser)
- Fine-grained capability control via WASI (WebAssembly System Interface) — filesystem access, network, clocks are all opt-in imports. A WASM module with no imported capabilities has zero host access by construction.

[EVIDENCE] Wasmtime's security model: modules execute in isolated linear memories; JIT bounds-check elimination bugs have been found and patched, but the model is significantly narrower than a full kernel syscall interface. Wasmtime is written in Rust and undergoes regular fuzzing.

[EVIDENCE] The 2025 "Wasm Breach" research identified JIT compiler logic bugs as escape vectors — these are real but patchable, unlike the structural shared-kernel problem with process isolation. The WASM sandbox has a much smaller attack surface than the Linux kernel syscall table.

**What WASM is NOT good for**:
- GPU/ML training workloads at scale (WebGPU covers inference, not CUDA-equivalent training)
- Workloads requiring POSIX semantics, large file I/O, or low-latency IPC
- Replacing VM isolation on Linux/macOS/Windows hosts that have hypervisor capability

**Verdict**: WASM is the mandatory transport format for browser and mobile donors. On VM-capable hosts, WASM may be used as an inner workload format inside the VM (defense-in-depth: WASM in Firecracker), but is not the outer security boundary.

[CONFIDENCE] HIGH.

---

## 3. GPU Passthrough: Exposing Accelerators Without Exposing the Host

[FINDING] GPU passthrough is achievable but introduces significant complexity and platform-specific constraints. The core mechanism is VFIO + IOMMU on Linux, with platform-specific equivalents elsewhere.

### 3.1 Linux: VFIO + IOMMU

Architecture:
- IOMMU (Intel VT-d or AMD-Vi) must be enabled and configured to place the GPU in its own IOMMU group
- GPU bound to the `vfio-pci` driver rather than the host driver (nvidia, amdgpu, etc.)
- VFIO device passed into the Firecracker or QEMU VM; guest driver loads inside the VM

[EVIDENCE] The critical security requirement: the GPU **must** be in its own IOMMU group. If it shares an IOMMU group with other devices (common on budget motherboards), DMA isolation is not guaranteed — a malicious guest driver could issue DMA reads/writes to host memory via the shared group. The ACS Override Patch bypasses this requirement and is explicitly unsafe for multi-tenant use.

[EVIDENCE] Interrupt remapping is mandatory; without it, a guest can inject interrupts into the host kernel, creating a privilege escalation path. Modern hardware (post-2016) generally supports interrupt remapping, but it must be verified per-host.

**Practical constraints for donor hardware**:
- Consumer GPUs (NVIDIA RTX, AMD RX) support VFIO passthrough but may have issues with NVIDIA's driver detecting virtualization and refusing to load (the "Error 43" problem for gaming GPUs) — resolvable with vendor-id spoofing in QEMU, but this is per-model fragile
- NVIDIA datacenter GPUs (A100, H100) support vGPU (GRID), providing proper multi-tenant GPU virtualization with mediated device (mdev) support
- AMD ROCm GPUs generally have fewer passthrough driver friction points than NVIDIA consumer cards

**Recommended approach for GPU donors**:
1. Verify IOMMU groups at registration time (agent checks `/sys/kernel/iommu_groups/`)
2. Verify interrupt remapping support
3. Verify GPU is in a singleton IOMMU group; reject passthrough if not
4. Use VFIO passthrough for NVIDIA/AMD GPUs inside Firecracker with VFIO device support (Firecracker added VFIO support; verify version)
5. Alternatively for NVIDIA: use the [NVIDIA Container Toolkit](https://github.com/NVIDIA/nvidia-container-toolkit) inside a Kata Containers VM with the NVIDIA vGPU or CDI path — this is the production path used by GPU cloud providers

### 3.2 macOS: No GPU Passthrough

As noted in Section 1.2, Apple Silicon's Metal GPU is not accessible from guest VMs via Virtualization.framework. Mac donors are CPU-only for clustered workloads.

### 3.3 Windows: GPU-P / WSL2 CUDA

NVIDIA CUDA is available inside WSL2 on Windows 11 via the CUDA on WSL2 driver stack. The guest (Linux inside WSL2) sees a `/dev/nvidia*` device via a paravirtual path that goes through the Hyper-V GPU Paravirtualization (GPU-P) layer. This provides CUDA workloads without full VFIO passthrough, and Microsoft + NVIDIA maintain this stack. This is the recommended Windows GPU path.

### 3.4 GPU-specific security threats

[FINDING] GPU passthrough introduces unique threat vectors beyond standard VM escape:
- **GPU firmware attacks**: Guest can load custom GPU microcode in some configurations, potentially persisting across VM resets
- **VRAM inspection**: Without proper IOMMU group isolation, DMA from GPU to host memory is possible
- **P2P DMA**: NVLink and PCIe P2P DMA between GPUs can bypass IOMMU isolation on some configurations
- **Driver surface**: The guest GPU driver (large, complex, closed-source for NVIDIA) is fully trusted inside the guest; bugs in it cannot escape the VM but can corrupt the guest

[CONFIDENCE] HIGH for threat identification; MEDIUM for completeness (GPU security research is an active area).

---

## 4. Attestation: Proving What's Actually Running

[FINDING] A two-layer attestation strategy is required: (a) agent binary attestation and (b) workload/sandbox attestation.

### 4.1 Agent Binary Attestation

The World Compute agent binary (running on the donor host) must be:
- **Code-signed**: macOS (Notarized + Hardened Runtime), Windows (Authenticode), Linux (GPG-signed packages + reproducible builds)
- **Reproducibly built**: Binary output is deterministic from source; any third party can verify the hash
- **Measured**: On platforms with TPM 2.0 (most x86 hosts post-2016), the agent binary hash should be extended into a PCR at launch, enabling remote attestation via TPM quote

Architecture for TPM-based agent attestation:
1. Control plane generates a challenge nonce
2. Donor TPM signs a quote containing PCR values (which include the agent binary measurement) using the Attestation Key (AK), chained to the Endorsement Key (EK) certificate
3. Control plane verifies the EK certificate chain (manufacturer root), verifies the AK binding, verifies PCR values match expected measurement for the current signed agent version
4. Only then does the control plane dispatch jobs to that node

[EVIDENCE] This is the same model used by Google's Confidential VMs and Azure Attestation. A 2025 CNCF blog post describes a combined TPM+TEE attestation protocol showing 24% efficiency improvement over sequential attestation, validating the hybrid approach.

### 4.2 Workload/Sandbox Attestation

For Linux with KVM/Firecracker:
- The microVM kernel and rootfs images are content-addressed (SHA-256 hash in the job spec)
- The Firecracker VMM binary itself is measured in the host TPM PCR alongside the agent
- The control plane verifies image hashes before dispatch; workload images are signed by the cluster's signing key

For confidential compute hardware (AMD SEV-SNP, Intel TDX — available on some cloud VMs and newer consumer hardware):
- SEV-SNP measurement covers the guest memory pages at launch (firmware, kernel, initrd)
- TDX provides a Trust Domain Report signed by the CPU hardware
- These provide cryptographic proof that a specific, unmodified workload is running, even against a potentially compromised hypervisor

[EVIDENCE] Google Cloud and Azure both use SEV-SNP/TDX for their Confidential VM products, providing production validation of these attestation paths.

**Practical recommendation**: Require TPM attestation for all x86 Linux/Windows hosts. Use SEV-SNP/TDX where available as an upgrade. For macOS, use Apple's Secure Enclave-based code signing chain (Notarization + hardened runtime prevents unsigned code injection). Mobile/browser donors receive only pre-validated WASM bundles signed by the cluster; browser-side attestation is not possible beyond the HTTPS/TLS channel.

[CONFIDENCE] HIGH for the overall model; MEDIUM for mobile/browser attestation completeness.

---

## 5. Escape Surface Analysis and Red-Team Test Plan

[FINDING] The top escape vectors, ranked by likelihood × impact for World Compute's threat model:

| Rank | Vector | Mechanism | Severity |
|-|-|-|-|
| 1 | VMM/hypervisor CVE | Bug in Firecracker/KVM/Hyper-V VMM code exploited by malicious guest | Critical |
| 2 | IOMMU misconfiguration | GPU in shared IOMMU group; DMA to host memory | Critical |
| 3 | Host kernel upgrade without agent restart | New kernel syscall or driver loaded post-attestation changes PCR values silently | High |
| 4 | Supply chain: agent binary | Compromised agent binary distributed to donors | High |
| 5 | Covert channel via shared hardware | CPU cache timing, rowhammer, DRAM bus contention | Medium |
| 6 | Network misconfiguration | VM gets access to host LAN rather than isolated bridge | High |
| 7 | WASM JIT bug | JIT bounds-check bypass in Wasmtime/browser for WASM-tier donors | Medium |
| 8 | Disk I/O covert channel | Workload infers host filesystem content via I/O timing | Low |

### Red-Team Test Plan (per Principle V requirement)

The following adversarial tests MUST be run before production deployment and on every major release:

**T1 — VM escape attempt**: Run a suite of known VM escape CVE proof-of-concepts against the Firecracker/KVM/Hyper-V stack on representative donor hardware. Use Metasploit's virtualization escape modules + custom tests targeting the specific VMM version. Expected result: all attempts fail; any success is a P0 blocker.

**T2 — IOMMU isolation**: On a GPU-capable Linux host, verify that a malicious guest driver cannot perform DMA reads of host memory outside its IOMMU group. Use the `iommu_test` utility and a custom PCIe DMA test harness. Verify that hosts failing the singleton IOMMU group check are correctly rejected by the agent.

**T3 — Network isolation**: From inside a guest VM, attempt to reach: (a) host localhost services, (b) LAN peers, (c) donor's router admin interface. All must be unreachable. Use nmap + curl from inside the VM.

**T4 — Filesystem isolation**: Attempt to access host filesystem paths from inside the guest via /proc/sysfs, virtio-fs mount attempts, plan 9 protocol abuse, and known container escape techniques adapted for VM context. No host path should be readable.

**T5 — Peripheral isolation**: Verify camera, microphone, clipboard, GPS, and USB devices are not accessible from the guest. On Linux, verify `/dev/video*`, `/dev/audio*` are absent in the guest. On macOS, verify Virtualization.framework does not forward host VZUSBController.

**T6 — Attestation bypass**: Attempt to run a modified agent binary on a TPM-attested host and verify the control plane rejects the node (PCR mismatch). Attempt to replay an old attestation quote and verify the nonce challenge defeats the replay.

**T7 — Resource exhaustion**: Verify the guest VM cannot exhaust host memory (balloon driver limits), CPU (cgroup + vcpu pinning limits), or disk (overlay quota). Host should remain responsive during maximum guest resource consumption.

**T8 — WASM escape**: For browser-tier donors, run the Wasm-breach proof-of-concept test suite against the target runtime (Wasmtime version, Chrome version). Verify no escape; update runtime if any PoC succeeds.

[CONFIDENCE] HIGH for completeness of threat enumeration; tests themselves require implementation and direct hardware execution per Principle V.

---

## 6. Performance Overhead Budget

[FINDING] The isolation stack must fit within a donor-unnoticeable overhead budget during idle or low-activity periods.

[EVIDENCE] Firecracker: ≤5 MiB VMM overhead, ≤125 ms boot latency, >95% CPU performance vs bare metal, 0.06 ms added latency average. These are CI-enforced specifications.

[EVIDENCE] gVisor: ~10–20% overhead on typical workloads, up to 30% on I/O-heavy syscall-intensive workloads. Startup in milliseconds (no boot).

[EVIDENCE] Kata Containers: 12–16% I/O overhead in IOZone benchmarks; startup 150–300 ms.

[EVIDENCE] WASM vs native: up to 45% slower for SIMD/parallel workloads; closer to native for simple compute.

**Proposed overhead budget**:

| Platform | Isolation | Idle memory overhead | Boot latency | CPU overhead |
|-|-|-|-|-|
| Linux (Firecracker) | VM+KVM | ≤5 MiB per VM slot | ≤125 ms | <5% |
| macOS (Virt.framework) | VM | ~50 MiB (Linux guest kernel) | ~500 ms | <8% |
| Windows (Hyper-V) | VM | ~64 MiB (utility VM) | ~300 ms | <8% |
| Browser (WASM) | Browser sandbox | ~5 MiB Worker | Immediate | 20–45% |
| Mobile (WASM) | Browser sandbox | ~5 MiB | Immediate | 20–45% |

The agent MUST enforce Principle III: when the donor's machine becomes active (keyboard/mouse input, thermal pressure, battery below threshold), all VMs MUST be suspended or terminated. Suspension (SIGSTOP to the VMM process) achieves this in <100 ms on all platforms without losing workload state.

[CONFIDENCE] HIGH for Linux numbers (CI-enforced specs); MEDIUM for macOS/Windows (platform-specific, less publicly benchmarked).

---

## [LIMITATION]

1. **Firecracker VFIO GPU support**: Firecracker's VFIO device passthrough support requires verification against the current release version; the feature has been in development and may require a specific build flag or version constraint.

2. **NVIDIA consumer GPU passthrough**: "Error 43" driver detection and per-model workarounds are fragile and may break across driver updates. A robust solution requires NVIDIA's cooperation or use of open-source nouveau driver (limited performance).

3. **macOS GPU**: No path to GPU passthrough in Virtualization.framework as of 2026. This limits Mac donor utility for GPU workloads.

4. **Mobile security**: Mobile donors are browser-WASM only; this is significantly weaker than VM isolation. Mobile nodes should receive only lowest-sensitivity workloads with public data.

5. **Attestation for browser donors**: No hardware attestation is possible. HTTPS channel + WASM bundle code signing is the only verification available. Browser donors must be explicitly lower-trust in the scheduling model.

6. **AMD SEV-SNP / Intel TDX availability**: Confidential compute hardware is not available on most consumer donor machines (2026). It is available on cloud VMs and newer AMD Zen 3+/4 systems. Plan for gradual adoption as hardware generational turnover occurs.

7. **This document is architecture research, not a direct test result**: Per Principle V, all recommendations require direct adversarial testing on real representative hardware before production deployment. The red-team test plan in Section 5 must be executed and evidence artifacts produced.

---

## Summary Recommendation

A tiered sandbox strategy with a single non-negotiable rule: every host platform that supports a hypervisor boundary MUST use one. The tiers are:

- **Tier 1 (Full trust, GPU eligible)**: Linux with KVM → Firecracker microVM + verified IOMMU GPU passthrough
- **Tier 2 (Full trust, CPU only)**: macOS (Virtualization.framework), Windows (Hyper-V/WSL2), Linux without KVM (Kata+QEMU)
- **Tier 3 (Reduced trust, CPU only)**: Browser WASM, Mobile WASM — scheduler assigns only public-data, loss-tolerant workloads

WASM is mandatory as the workload distribution format for Tier 3 and optional as an inner defense-in-depth layer inside VMs on Tier 1/2.

Attestation uses TPM 2.0 quotes for x86 hosts, Apple Notarization chain for macOS, Authenticode for Windows, and WASM bundle signing for browser/mobile.

---

*Research conducted 2026-04-15. Sources: Firecracker SPECIFICATION.md (CI-enforced specs); Northflank blog comparative analysis; KubeBlocks IOZone benchmark data; Level1Techs IOMMU security forum analysis; CNCF TEE+TPM attestation blog; WebAssembly security documentation; Wasmtime security model; Scrumlaunch WASM 2025 analysis.*
