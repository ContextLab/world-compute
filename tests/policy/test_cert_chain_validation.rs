//! T040: Integration tests for CertificateChainValidator trait.
//!
//! Tests structural validation of certificate chains for TPM2, SEV-SNP, and TDX.
//! Uses rcgen to generate real X.509 test certificates at runtime.

use worldcompute::verification::attestation::{
    CertificateChainValidator, SevSnpChainValidator, TdxChainValidator, Tpm2ChainValidator,
};

/// Generate a self-signed root CA certificate (DER-encoded).
fn generate_root_ca() -> (rcgen::CertifiedKey, Vec<u8>) {
    let mut params = rcgen::CertificateParams::new(vec![]).unwrap();
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Test Root CA");
    params
        .distinguished_name
        .push(rcgen::DnType::OrganizationName, "Test Org");

    let key_pair = rcgen::KeyPair::generate().unwrap();
    let cert = params.self_signed(&key_pair).unwrap();
    let der = cert.der().to_vec();
    (
        rcgen::CertifiedKey {
            cert,
            key_pair,
        },
        der,
    )
}

/// Generate an intermediate CA certificate signed by the given issuer.
fn generate_intermediate_ca(
    issuer_cert: &rcgen::Certificate,
    issuer_key: &rcgen::KeyPair,
) -> (rcgen::CertifiedKey, Vec<u8>) {
    let mut params = rcgen::CertificateParams::new(vec![]).unwrap();
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Test Intermediate CA");
    params
        .distinguished_name
        .push(rcgen::DnType::OrganizationName, "Test Org");

    let key_pair = rcgen::KeyPair::generate().unwrap();
    let cert = params.signed_by(&key_pair, issuer_cert, issuer_key).unwrap();
    let der = cert.der().to_vec();
    (
        rcgen::CertifiedKey {
            cert,
            key_pair,
        },
        der,
    )
}

/// Generate a leaf (end-entity) certificate signed by the given issuer.
fn generate_leaf_cert(
    issuer_cert: &rcgen::Certificate,
    issuer_key: &rcgen::KeyPair,
) -> Vec<u8> {
    let mut params = rcgen::CertificateParams::new(vec!["localhost".into()]).unwrap();
    params.is_ca = rcgen::IsCa::NoCa;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Test Leaf Cert");

    let key_pair = rcgen::KeyPair::generate().unwrap();
    let cert = params.signed_by(&key_pair, issuer_cert, issuer_key).unwrap();
    cert.der().to_vec()
}

/// Build a valid 3-cert chain: leaf -> intermediate -> root.
fn build_valid_chain() -> Vec<Vec<u8>> {
    let (root, root_der) = generate_root_ca();
    let (intermediate, intermediate_der) =
        generate_intermediate_ca(&root.cert, &root.key_pair);
    let leaf_der = generate_leaf_cert(&intermediate.cert, &intermediate.key_pair);
    vec![leaf_der, intermediate_der, root_der]
}

// ─── Empty chain rejection ─────────────────────────────────────────────

#[test]
fn tpm2_empty_chain_rejected() {
    let validator = Tpm2ChainValidator;
    let result = validator.validate_chain(b"dummy-quote", &[]);
    let valid = result.unwrap();
    assert!(!valid, "Empty cert chain must be rejected for TPM2");
}

#[test]
fn sev_snp_empty_chain_rejected() {
    let validator = SevSnpChainValidator;
    let result = validator.validate_chain(b"dummy-quote", &[]);
    let valid = result.unwrap();
    assert!(!valid, "Empty cert chain must be rejected for SEV-SNP");
}

#[test]
fn tdx_empty_chain_rejected() {
    let validator = TdxChainValidator;
    let result = validator.validate_chain(b"dummy-quote", &[]);
    let valid = result.unwrap();
    assert!(!valid, "Empty cert chain must be rejected for TDX");
}

// ─── Single cert rejection (needs at least leaf + CA) ───────────────────

#[test]
fn tpm2_single_cert_rejected() {
    let (_, root_der) = generate_root_ca();
    let validator = Tpm2ChainValidator;
    let result = validator.validate_chain(b"dummy-quote", &[root_der]);
    let valid = result.unwrap();
    assert!(!valid, "Single cert chain must be rejected (need >= 2)");
}

#[test]
fn sev_snp_single_cert_rejected() {
    let (_, root_der) = generate_root_ca();
    let validator = SevSnpChainValidator;
    let result = validator.validate_chain(b"dummy-quote", &[root_der]);
    let valid = result.unwrap();
    assert!(!valid, "Single cert chain must be rejected (need >= 2)");
}

#[test]
fn tdx_single_cert_rejected() {
    let (_, root_der) = generate_root_ca();
    let validator = TdxChainValidator;
    let result = validator.validate_chain(b"dummy-quote", &[root_der]);
    let valid = result.unwrap();
    assert!(!valid, "Single cert chain must be rejected (need >= 2)");
}

// ─── Valid chain accepted ───────────────────────────────────────────────

#[test]
fn tpm2_valid_chain_accepted() {
    let chain = build_valid_chain();
    let validator = Tpm2ChainValidator;
    let valid = validator.validate_chain(b"dummy-quote", &chain).unwrap();
    assert!(valid, "Valid 3-cert chain should be accepted for TPM2");
}

#[test]
fn sev_snp_valid_chain_accepted() {
    let chain = build_valid_chain();
    let validator = SevSnpChainValidator;
    let valid = validator.validate_chain(b"dummy-quote", &chain).unwrap();
    assert!(valid, "Valid 3-cert chain should be accepted for SEV-SNP");
}

#[test]
fn tdx_valid_chain_accepted() {
    let chain = build_valid_chain();
    let validator = TdxChainValidator;
    let valid = validator.validate_chain(b"dummy-quote", &chain).unwrap();
    assert!(valid, "Valid 3-cert chain should be accepted for TDX");
}

// ─── Misordered chain rejected ─────────────────────────────────────────

#[test]
fn tpm2_misordered_chain_rejected() {
    let chain = build_valid_chain();
    // Reverse the chain: root first, leaf last (wrong order)
    let reversed: Vec<Vec<u8>> = chain.into_iter().rev().collect();
    let validator = Tpm2ChainValidator;
    let valid = validator.validate_chain(b"dummy-quote", &reversed).unwrap();
    assert!(
        !valid,
        "Misordered chain (root-first) should be rejected"
    );
}

// ─── Garbage DER rejected with error ────────────────────────────────────

#[test]
fn tpm2_garbage_der_returns_error() {
    let garbage = vec![vec![0xFF, 0xFE, 0xFD], vec![0x00, 0x01, 0x02]];
    let validator = Tpm2ChainValidator;
    let result = validator.validate_chain(b"dummy-quote", &garbage);
    assert!(
        result.is_err(),
        "Garbage DER bytes should return an error"
    );
}

// ─── Two unrelated certs (issuer mismatch) ──────────────────────────────

#[test]
fn sev_snp_unrelated_certs_rejected() {
    // Two independent CAs with different subjects — issuer/subject won't chain
    let (_, root1_der) = generate_root_ca();
    // Generate a second root CA with a different distinguished name
    let mut params2 = rcgen::CertificateParams::new(vec![]).unwrap();
    params2.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params2.distinguished_name = rcgen::DistinguishedName::new();
    params2
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Different Root CA");
    params2
        .distinguished_name
        .push(rcgen::DnType::OrganizationName, "Other Org");
    let key2 = rcgen::KeyPair::generate().unwrap();
    let cert2 = params2.self_signed(&key2).unwrap();
    let root2_der = cert2.der().to_vec();
    let validator = SevSnpChainValidator;
    let valid = validator
        .validate_chain(b"dummy-quote", &[root1_der, root2_der])
        .unwrap();
    assert!(
        !valid,
        "Two unrelated certs should fail chain ordering check"
    );
}

// ─── Platform name correctness ──────────────────────────────────────────

#[test]
fn platform_names_correct() {
    assert_eq!(Tpm2ChainValidator.platform_name(), "TPM 2.0");
    assert_eq!(SevSnpChainValidator.platform_name(), "AMD SEV-SNP");
    assert_eq!(TdxChainValidator.platform_name(), "Intel TDX");
}
