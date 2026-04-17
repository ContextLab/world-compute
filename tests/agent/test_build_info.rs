//! Integration tests for build info, binary signature verification, and version checking (T098).

use worldcompute::agent::build_info::{
    get_build_info, is_known_version, verify_binary_signature, BUILD_TIMESTAMP, GIT_COMMIT,
};

#[test]
fn build_info_returns_valid_struct() {
    let info = get_build_info();
    assert!(!info.version.is_empty(), "version must not be empty");
    assert!(info.version.contains('.'), "version must be semver-like");
    assert!(!info.build_timestamp.is_empty(), "build_timestamp must be set");
}

#[test]
fn build_constants_are_accessible() {
    // These are compile-time constants; they must be non-empty strings
    assert!(!BUILD_TIMESTAMP.is_empty());
    assert!(!GIT_COMMIT.is_empty());
}

#[test]
fn version_checking_accepts_known() {
    let known = &["0.1.0", "0.2.0", "1.0.0"];
    assert!(is_known_version("0.1.0", known));
    assert!(is_known_version("1.0.0", known));
}

#[test]
fn version_checking_rejects_unknown() {
    let known = &["0.1.0", "0.2.0"];
    assert!(!is_known_version("99.0.0", known));
}

#[test]
fn binary_signature_roundtrip() {
    use ed25519_dalek::{Signer, SigningKey};
    use sha2::{Digest, Sha256};

    let dir = std::env::temp_dir().join("wc_integ_binary_sig");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("testbin");
    let content = b"integration test binary payload";
    std::fs::write(&path, content).unwrap();

    let signing_key = SigningKey::generate(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();

    let mut hasher = Sha256::new();
    hasher.update(content);
    let hash = hasher.finalize();
    let signature = signing_key.sign(hash.as_slice());

    let result = verify_binary_signature(
        path.to_str().unwrap(),
        signature.to_bytes().as_slice(),
        verifying_key.as_bytes(),
    )
    .expect("verification should not error");
    assert!(result, "valid signature should verify");

    let _ = std::fs::remove_dir_all(&dir);
}
