# Data Model: Replace Implementation Stubs

**Branch**: `003-stub-replacement` | **Date**: 2026-04-16

This document covers the data entities introduced or modified by stub replacement. Most entities already exist in the codebase — this documents their current shape and any additions needed.

## Existing Entities (no changes)

These entities are already fully defined and are not modified by stub replacement:

| Entity | Location | Purpose |
|-|-|-|
| DonorCommand | src/cli/donor.rs | CLI subcommand enum (Join, Status, Pause, Resume, Leave, Credits, Logs) |
| SubmitterCommand | src/cli/submitter.rs | CLI subcommand enum (Submit, Status, Results, Cancel, List) |
| GovernanceCommand | src/cli/governance.rs | CLI subcommand enum (Propose, List, Vote, Report) |
| AdminCommand | src/cli/admin.rs | CLI subcommand enum (Halt, Resume, Ban, Audit) |
| PersonhoodResult | src/identity/personhood.rs | Enum: Verified, Pending, Failed, ProviderUnavailable |
| BrightIdVerification | src/identity/personhood.rs | Struct: verified, unique, context_id, error |
| OAuth2Result | src/identity/oauth2.rs | Enum: Verified{provider, account_id}, Failed, ProviderUnavailable |
| PhoneResult | src/identity/phone.rs | Enum: Verified{phone_hash}, CodeExpired, InvalidCode, ProviderUnavailable |
| NatStatus | src/network/nat.rs | Enum: Direct, FullCone, RestrictedCone, PortRestricted, Symmetric, Unknown |
| TransparencyLogResult | src/registry/transparency.rs | Enum for Rekor log submission results |
| MerkleRootAnchor | src/ledger/transparency.rs | Struct for Rekor anchoring results |

## New/Modified Entities

### FirecrackerVmConfig

**Purpose**: Structured configuration for Firecracker API socket calls. Currently these values are inline strings; extracting them into a struct enables validation and testing.

```
FirecrackerVmConfig
├── vcpu_count: u8                    # Number of vCPUs
├── mem_size_mib: u32                 # Memory in MiB
├── kernel_image_path: String         # Path to guest kernel
├── rootfs_path: String               # Path to rootfs image
├── boot_args: String                 # Kernel boot arguments
└── network_interfaces: Vec<NetworkInterface>
    ├── iface_id: String
    ├── host_dev_name: String
    └── guest_mac: Option<String>
```

**State transitions**: None (configuration, not stateful).

### CertificateChainValidator (trait)

**Purpose**: Pluggable certificate chain validation for attestation platforms.

```
trait CertificateChainValidator
├── validate_chain(quote: &[u8], certs: &[Certificate]) → Result<bool>
└── root_ca() → &Certificate

Implementations:
├── Tpm2ChainValidator          # EK → AIK → quote
├── SevSnpChainValidator        # ARK → ASK → VCEK → report
├── TdxChainValidator           # Intel DCAP root → PCK → quote
└── AppleSeValidator            # Remote validation via Apple API
```

**Relationships**: Used by `src/verification/attestation.rs` verification functions.

### OtlpConfig

**Purpose**: Configuration for OTLP exporter wiring. Extracted from the `otel_endpoint` parameter.

```
OtlpConfig
├── endpoint: String                  # OTLP collector URL
├── service_name: String              # Service identifier (default: "worldcompute")
├── batch_size: usize                 # Span batch size (default: 512)
└── export_interval_secs: u64         # Export interval (default: 5)
```

### RaftCoordinatorStorage

**Purpose**: openraft-compatible storage adapter for coordinator state.

```
RaftCoordinatorStorage
├── log: BTreeMap<u64, Entry>         # In-memory Raft log
├── state_machine: CoordinatorState   # Applied state
├── vote: Option<Vote>                # Current vote
├── snapshot: Option<Snapshot>        # Latest snapshot
└── wal_path: Option<PathBuf>        # Optional WAL file path

Entry
├── term: u64
├── index: u64
└── payload: CoordinatorAction        # Job assignment, status change, etc.
```

**State transitions**: Follower → Candidate → Leader (managed by openraft).

### OAuth2ProviderConfig

**Purpose**: Per-provider OAuth2 configuration loaded from environment.

```
OAuth2ProviderConfig
├── provider: String                  # "github", "google", "twitter", "email"
├── client_id: String                 # From env var
├── client_secret: String             # From env var
├── auth_url: String                  # Provider's authorization endpoint
├── token_url: String                 # Provider's token endpoint
├── redirect_uri: String              # Callback URL
└── scopes: Vec<String>               # Required scopes
```

### SmsProviderConfig

**Purpose**: SMS verification provider configuration.

```
SmsProviderConfig
├── provider: String                  # "twilio", "vonage"
├── account_sid: String               # From env var
├── auth_token: String                # From env var
├── verify_service_sid: String        # Twilio Verify service ID
└── from_number: Option<String>       # Sender number (if not using Verify)
```

## Validation Rules

| Entity | Rule |
|-|-|
| FirecrackerVmConfig | vcpu_count ≥ 1, mem_size_mib ≥ 128, kernel_image_path must exist |
| OAuth2ProviderConfig | client_id and client_secret must be non-empty, URLs must be valid |
| SmsProviderConfig | account_sid, auth_token, verify_service_sid must be non-empty |
| OtlpConfig | endpoint must be valid URL, batch_size ≥ 1 |
| CertificateChainValidator | Root CA certificate must be parseable and not expired |
