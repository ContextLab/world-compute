//! Integration tests for mTLS certificate authority and rotation (T093).

use worldcompute::network::tls::{needs_rotation, CertificateAuthority, IssuedCert};

#[test]
fn ca_generation_succeeds() {
    let ca = CertificateAuthority::new().expect("CA generation should succeed");
    assert!(!ca.ca_cert_der.is_empty(), "CA cert DER must not be empty");
    assert!(!ca.ca_key_der.is_empty(), "CA key DER must not be empty");
}

#[test]
fn cert_issuance_produces_valid_cert() {
    let ca = CertificateAuthority::new().unwrap();
    let cert = ca.issue_cert("account-integration-test").expect("cert issuance should succeed");
    assert!(!cert.cert_der.is_empty(), "issued cert DER must not be empty");
    assert!(!cert.key_der.is_empty(), "issued key DER must not be empty");
    // Cert should expire approximately 90 days from now
    let days_until = (cert.not_after - chrono::Utc::now()).num_days();
    assert!(
        days_until >= 89 && days_until <= 91,
        "cert should expire in ~90 days, got {days_until}"
    );
}

#[test]
fn rotation_needed_for_cert_expiring_in_3_days() {
    let cert = IssuedCert {
        cert_der: vec![1, 2, 3],
        key_der: vec![4, 5, 6],
        not_after: chrono::Utc::now() + chrono::Duration::days(3),
    };
    assert!(
        needs_rotation(&cert, 7),
        "cert expiring in 3 days should need rotation with 7-day threshold"
    );
}

#[test]
fn no_rotation_needed_for_cert_expiring_in_30_days() {
    let cert = IssuedCert {
        cert_der: vec![1, 2, 3],
        key_der: vec![4, 5, 6],
        not_after: chrono::Utc::now() + chrono::Duration::days(30),
    };
    assert!(
        !needs_rotation(&cert, 7),
        "cert expiring in 30 days should NOT need rotation with 7-day threshold"
    );
}

#[test]
fn multiple_certs_from_same_ca() {
    let ca = CertificateAuthority::new().unwrap();
    let cert1 = ca.issue_cert("account-001").unwrap();
    let cert2 = ca.issue_cert("account-002").unwrap();
    // Different certs should have different DER content (different keys)
    assert_ne!(cert1.key_der, cert2.key_der, "different accounts should get different keys");
}
