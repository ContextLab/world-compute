//! T060: Integration test for OTLP telemetry initialization.
//!
//! Verifies that telemetry::OtlpConfig can be constructed and that
//! the module's types are usable. We cannot call telemetry::init()
//! in integration tests because tracing subscriber can only be
//! initialized once per process and other tests may also initialize it.
//! Instead we test the configuration types and verify the module compiles
//! and exports correctly.

use worldcompute::telemetry::OtlpConfig;

#[test]
fn otlp_config_construction() {
    let config = OtlpConfig::new("http://localhost:9999");
    assert_eq!(config.endpoint, "http://localhost:9999");
    assert_eq!(config.service_name, "worldcompute");
}

#[test]
fn otlp_config_accepts_various_endpoints() {
    let endpoints = [
        "http://localhost:4317",
        "https://otel-collector.example.com:4317",
        "http://127.0.0.1:9999",
        "grpc://collector:4317",
    ];
    for ep in endpoints {
        let config = OtlpConfig::new(ep);
        assert_eq!(config.endpoint, ep);
        assert_eq!(config.service_name, "worldcompute");
    }
}

#[test]
fn otlp_config_with_string_types() {
    // Verify Into<String> works with String, &str, and String literals
    let _c1 = OtlpConfig::new("http://example.com");
    let _c2 = OtlpConfig::new(String::from("http://example.com"));
    let s = String::from("http://example.com");
    let _c3 = OtlpConfig::new(s);
}
