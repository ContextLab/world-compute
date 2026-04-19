//! Integration tests for PII redaction (T110).

use worldcompute::telemetry::redaction::redact;

#[test]
fn redaction_masks_hostnames_in_paths() {
    let input = "/Users/jmanning/world-compute/data";
    let output = redact(input);
    assert!(output.contains("[REDACTED_USER]"), "Should redact username from path, got: {output}");
    assert!(!output.contains("jmanning"), "Username should not appear in output");
}

#[test]
fn redaction_masks_private_ips() {
    let input = "connecting to node at 192.168.1.42 on port 8080";
    let output = redact(input);
    assert!(output.contains("[REDACTED_IP]"), "Should redact private IP, got: {output}");
    assert!(!output.contains("192.168.1.42"), "Private IP should not appear in output");

    // Also test 10.x.x.x range
    let input2 = "host 10.0.0.1 is up";
    let output2 = redact(input2);
    assert!(output2.contains("[REDACTED_IP]"));
}

#[test]
fn clean_data_passes_through() {
    let input = "job completed successfully with 42 results on port 8080";
    let output = redact(input);
    assert_eq!(input, output, "Clean data should pass through unchanged");
}

#[test]
fn public_ip_not_redacted() {
    let input = "connecting to 8.8.8.8 for DNS";
    let output = redact(input);
    assert_eq!(input, output, "Public IPs should not be redacted");
}

#[test]
fn mac_address_redacted() {
    let input = "interface aa:bb:cc:dd:ee:ff is up";
    let output = redact(input);
    assert!(output.contains("[REDACTED_MAC]"));
    assert!(!output.contains("aa:bb:cc:dd:ee:ff"));
}
