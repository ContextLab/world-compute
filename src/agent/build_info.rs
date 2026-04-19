//! Reproducible build metadata and supply chain verification per FR-006.

use crate::error::{ErrorCode, WcError};

// ---------------------------------------------------------------------------
// T095: Build provenance constants and accessors
// ---------------------------------------------------------------------------

/// Compile-time build timestamp (Unix epoch seconds), set by build.rs.
pub const BUILD_TIMESTAMP: &str = env!("WC_BUILD_TIMESTAMP");

/// Git commit hash at build time, set by build.rs.
/// Falls back to "unknown" if not available.
pub const GIT_COMMIT: &str = match option_env!("WC_GIT_COMMIT") {
    Some(v) => v,
    None => "unknown",
};

/// Rustc version or wrapper used for the build.
pub const RUSTC_VERSION: &str = match option_env!("WC_RUSTC_VERSION") {
    Some(v) => v,
    None => "unknown",
};

/// Compile-time build information for reproducibility and auditability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    /// Semantic version from Cargo.toml.
    pub version: &'static str,
    /// Git SHA of the commit this binary was built from.
    pub git_sha: &'static str,
    /// Build timestamp (Unix epoch seconds).
    pub build_timestamp: &'static str,
    /// Rustc version or wrapper.
    pub rustc_version: &'static str,
    /// Whether the binary was built with a reproducible signed build.
    pub is_signed: bool,
}

/// Return the build info for this binary, populated from compile-time env vars.
pub fn get_build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        git_sha: GIT_COMMIT,
        build_timestamp: BUILD_TIMESTAMP,
        rustc_version: RUSTC_VERSION,
        is_signed: matches!(option_env!("SIGNED_BUILD"), Some("true")),
    }
}

// ---------------------------------------------------------------------------
// T096: Binary signature verification
// ---------------------------------------------------------------------------

/// Verify an Ed25519 signature over the SHA-256 hash of a binary file.
///
/// Reads the file at `binary_path`, computes its SHA-256 digest, and verifies
/// the provided `signature` against the given 32-byte Ed25519 `public_key`.
pub fn verify_binary_signature(
    binary_path: &str,
    signature: &[u8],
    public_key: &[u8; 32],
) -> Result<bool, WcError> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use sha2::{Digest, Sha256};

    // Read the binary file
    let binary_data = std::fs::read(binary_path).map_err(|e| {
        WcError::new(ErrorCode::NotFound, format!("Cannot read binary at {binary_path}: {e}"))
    })?;

    // Compute SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    let hash = hasher.finalize();

    // Parse the public key
    let verifying_key = VerifyingKey::from_bytes(public_key).map_err(|e| {
        WcError::new(ErrorCode::AttestationFailed, format!("Invalid public key: {e}"))
    })?;

    // Parse the signature (must be 64 bytes)
    let sig = Signature::from_slice(signature).map_err(|e| {
        WcError::new(ErrorCode::AttestationFailed, format!("Invalid signature: {e}"))
    })?;

    // Verify signature over the hash
    match verifying_key.verify(hash.as_slice(), &sig) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// T097: Version checking logic
// ---------------------------------------------------------------------------

/// Check if a version string is present in a list of known versions.
pub fn is_known_version(version: &str, known_versions: &[&str]) -> bool {
    known_versions.contains(&version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_info_has_non_empty_version() {
        let info = get_build_info();
        assert!(!info.version.is_empty(), "version should not be empty");
    }

    #[test]
    fn build_info_version_matches_cargo() {
        let info = get_build_info();
        assert!(info.version.contains('.'), "version '{}' should be semver", info.version);
    }

    #[test]
    fn build_info_git_sha_is_present() {
        let info = get_build_info();
        assert!(!info.git_sha.is_empty());
    }

    #[test]
    fn build_info_timestamp_is_present() {
        let info = get_build_info();
        assert!(!info.build_timestamp.is_empty());
        // Should be a numeric Unix timestamp
        assert!(info.build_timestamp.parse::<u64>().is_ok() || info.build_timestamp == "unknown");
    }

    #[test]
    fn build_constants_match_accessors() {
        let info = get_build_info();
        assert_eq!(info.git_sha, GIT_COMMIT);
        assert_eq!(info.build_timestamp, BUILD_TIMESTAMP);
    }

    #[test]
    fn verify_binary_signature_rejects_nonexistent_file() {
        let result = verify_binary_signature("/nonexistent/path", &[0u8; 64], &[0u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn verify_binary_signature_with_valid_signature() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        // Create a temp file
        let dir = std::env::temp_dir().join("wc_test_binary_sig");
        std::fs::create_dir_all(&dir).unwrap();
        let binary_path = dir.join("test_binary");
        let content = b"Hello World Compute binary content";
        std::fs::write(&binary_path, content).unwrap();

        // Sign the SHA-256 hash of the content
        let signing_key = SigningKey::generate(&mut rand::thread_rng());
        let verifying_key = signing_key.verifying_key();

        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        let signature = signing_key.sign(hash.as_slice());

        let result = verify_binary_signature(
            binary_path.to_str().unwrap(),
            signature.to_bytes().as_slice(),
            verifying_key.as_bytes(),
        );
        assert!(result.is_ok());
        assert!(result.unwrap(), "valid signature should verify");

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_binary_signature_rejects_wrong_key() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        let dir = std::env::temp_dir().join("wc_test_binary_sig2");
        std::fs::create_dir_all(&dir).unwrap();
        let binary_path = dir.join("test_binary2");
        let content = b"Some binary content";
        std::fs::write(&binary_path, content).unwrap();

        // Sign with one key
        let signing_key = SigningKey::generate(&mut rand::thread_rng());
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        let signature = signing_key.sign(hash.as_slice());

        // Verify with a different key
        let wrong_key = SigningKey::generate(&mut rand::thread_rng());
        let result = verify_binary_signature(
            binary_path.to_str().unwrap(),
            signature.to_bytes().as_slice(),
            wrong_key.verifying_key().as_bytes(),
        );
        assert!(result.is_ok());
        assert!(!result.unwrap(), "wrong key should fail verification");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_known_version_matches() {
        let known = &["0.1.0", "0.2.0", "1.0.0"];
        assert!(is_known_version("0.1.0", known));
        assert!(is_known_version("1.0.0", known));
    }

    #[test]
    fn is_known_version_rejects_unknown() {
        let known = &["0.1.0", "0.2.0"];
        assert!(!is_known_version("9.9.9", known));
        assert!(!is_known_version("", known));
    }
}
