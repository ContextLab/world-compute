//! Canonical error model — 20 error codes per contracts/errors.md.
//!
//! Each variant maps to a gRPC status code and HTTP status code for the
//! REST gateway. Error codes are stable; new codes are additive-only.

use thiserror::Error;

/// Canonical World Compute error codes (WC-001 through WC-026).
///
/// Codes 001-020 are from spec 001 (core). Codes 021-026 added by spec 005 (T009).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
#[allow(dead_code)]
pub enum ErrorCode {
    InvalidManifest = 1,
    InsufficientCredits = 2,
    AcceptableUseViolation = 3,
    NoEligibleNodes = 4,
    QuorumFailure = 5,
    TrustTierMismatch = 6,
    SandboxUnavailable = 7,
    PreemptionTimeout = 8,
    LedgerVerificationFailed = 9,
    CoordinatorUnreachable = 10,
    ResidencyConstraintViolation = 11,
    AttestationFailed = 12,
    RateLimited = 13,
    Unauthorized = 14,
    Internal = 15,
    Unavailable = 16,
    DeadlineExceeded = 17,
    NotFound = 18,
    AlreadyExists = 19,
    PermissionDenied = 20,

    // spec 005 additions (T009) — covers new error surfaces introduced in
    // cross-firewall mesh, deep attestation, diffusion mesh-LLM, and
    // placeholder-elimination sweeps.
    /// Feature/function called on a platform where it is not implemented
    /// (e.g., Apple VF helper on non-macOS). Not a bug — a clean platform refusal.
    UnsupportedPlatform = 21,
    /// libp2p dial failure with full root-cause context; per FR-004.
    DialFailureWithDetail = 22,
    /// Relay reservation could not be acquired from any bootstrap relay
    /// after fallback transport exhaustion; per FR-006, FR-007.
    ReservationAcquisitionFailed = 23,
    /// ParaDiGMS Picard iteration failed to converge within its budget AND
    /// strict-sequential fallback also failed; per FR-025 edge case.
    ParaDiGMSNonconvergence = 24,
    /// An attestation chain validated structurally but did not match any
    /// pinned manufacturer root (no bypass); per FR-008, FR-009.
    AttestationRootMismatch = 25,
    /// A production invariant (placeholder detected, allowlist non-empty at
    /// completion gate, etc.) was violated; per FR-038, SC-006.
    PlaceholderDetected = 26,
}

impl ErrorCode {
    /// gRPC status code mapping.
    pub fn grpc_code(self) -> i32 {
        match self {
            Self::InvalidManifest => 3,              // INVALID_ARGUMENT
            Self::InsufficientCredits => 9,          // FAILED_PRECONDITION
            Self::AcceptableUseViolation => 9,       // FAILED_PRECONDITION
            Self::NoEligibleNodes => 9,              // FAILED_PRECONDITION
            Self::QuorumFailure => 10,               // ABORTED
            Self::TrustTierMismatch => 9,            // FAILED_PRECONDITION
            Self::SandboxUnavailable => 14,          // UNAVAILABLE
            Self::PreemptionTimeout => 4,            // DEADLINE_EXCEEDED
            Self::LedgerVerificationFailed => 10,    // ABORTED
            Self::CoordinatorUnreachable => 14,      // UNAVAILABLE
            Self::ResidencyConstraintViolation => 9, // FAILED_PRECONDITION
            Self::AttestationFailed => 16,           // UNAUTHENTICATED
            Self::RateLimited => 8,                  // RESOURCE_EXHAUSTED
            Self::Unauthorized => 16,                // UNAUTHENTICATED
            Self::Internal => 13,                    // INTERNAL
            Self::Unavailable => 14,                 // UNAVAILABLE
            Self::DeadlineExceeded => 4,             // DEADLINE_EXCEEDED
            Self::NotFound => 5,                     // NOT_FOUND
            Self::AlreadyExists => 6,                // ALREADY_EXISTS
            Self::PermissionDenied => 7,             // PERMISSION_DENIED
            // spec 005 additions
            Self::UnsupportedPlatform => 12,         // UNIMPLEMENTED
            Self::DialFailureWithDetail => 14,       // UNAVAILABLE
            Self::ReservationAcquisitionFailed => 14, // UNAVAILABLE
            Self::ParaDiGMSNonconvergence => 10,     // ABORTED
            Self::AttestationRootMismatch => 16,     // UNAUTHENTICATED
            Self::PlaceholderDetected => 9,          // FAILED_PRECONDITION
        }
    }

    /// HTTP status code mapping for REST gateway.
    pub fn http_status(self) -> u16 {
        match self {
            Self::InvalidManifest => 400,
            Self::InsufficientCredits => 402,
            Self::AcceptableUseViolation => 403,
            Self::NoEligibleNodes => 503,
            Self::QuorumFailure => 409,
            Self::TrustTierMismatch => 422,
            Self::SandboxUnavailable => 503,
            Self::PreemptionTimeout => 504,
            Self::LedgerVerificationFailed => 409,
            Self::CoordinatorUnreachable => 503,
            Self::ResidencyConstraintViolation => 422,
            Self::AttestationFailed => 401,
            Self::RateLimited => 429,
            Self::Unauthorized => 401,
            Self::Internal => 500,
            Self::Unavailable => 503,
            Self::DeadlineExceeded => 504,
            Self::NotFound => 404,
            Self::AlreadyExists => 409,
            Self::PermissionDenied => 403,
            // spec 005 additions
            Self::UnsupportedPlatform => 501,           // Not Implemented
            Self::DialFailureWithDetail => 503,         // Service Unavailable
            Self::ReservationAcquisitionFailed => 503,  // Service Unavailable
            Self::ParaDiGMSNonconvergence => 409,       // Conflict (convergence)
            Self::AttestationRootMismatch => 401,       // Unauthorized
            Self::PlaceholderDetected => 422,           // Unprocessable Entity
        }
    }
}

/// Top-level error type for World Compute operations.
#[derive(Debug, Error)]
pub enum WcError {
    #[error("WC-{:03}: {message}", *code as u16)]
    Application { code: ErrorCode, message: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl WcError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Application { code, message: message.into() }
    }

    pub fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::Application { code, .. } => Some(*code),
            _ => None,
        }
    }
}

/// Convenience result type.
pub type WcResult<T> = Result<T, WcError>;
