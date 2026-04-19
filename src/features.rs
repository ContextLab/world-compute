//! Compile-time feature-gate assertions for spec 005 (FR-008, FR-010, FR-011a).
//!
//! Under `--features production`, the build fails if any pinned root-of-trust
//! constant is still the zero placeholder. This closes the safety gap where
//! spec-004 validators silently entered permissive bypass mode when pins were
//! `[0u8; 32]`.
//!
//! The non-production (default) build intentionally allows the bypass so that
//! development and unit tests can exercise attestation-pipeline code paths
//! without requiring live AMD/Intel hardware. Operators MUST build release
//! binaries with `cargo build --release --features production` or equivalent.

#[cfg(feature = "production")]
const _: () = {
    use crate::verification::attestation::{
        AMD_ARK_SHA256_FINGERPRINT, INTEL_ROOT_CA_SHA256_FINGERPRINT,
    };
    use crate::ledger::transparency::REKOR_PUBLIC_KEY;

    assert!(
        !is_all_zero(&AMD_ARK_SHA256_FINGERPRINT),
        "production build: AMD_ARK_SHA256_FINGERPRINT must not be zero — pin real value at release cut time (FR-008, FR-011a)"
    );
    assert!(
        !is_all_zero(&INTEL_ROOT_CA_SHA256_FINGERPRINT),
        "production build: INTEL_ROOT_CA_SHA256_FINGERPRINT must not be zero — pin real value at release cut time (FR-008, FR-011a)"
    );
    assert!(
        !is_all_zero(&REKOR_PUBLIC_KEY),
        "production build: REKOR_PUBLIC_KEY must not be zero — pin real value at release cut time (FR-010, FR-011a)"
    );
};

#[cfg(feature = "production")]
const fn is_all_zero(bytes: &[u8; 32]) -> bool {
    let mut i = 0;
    while i < 32 {
        if bytes[i] != 0 {
            return false;
        }
        i += 1;
    }
    true
}

/// Returns true if this build is configured for production deployment.
///
/// Production builds enforce non-zero pinned fingerprints at compile time
/// (see the `const _: () = { ... }` block above). Test / dev builds return
/// false and may run with zero pins in permissive bypass mode.
pub const fn is_production_build() -> bool {
    cfg!(feature = "production")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_production_build_reports_false() {
        // In the default test harness, the `production` feature is off.
        assert!(!is_production_build());
    }
}
