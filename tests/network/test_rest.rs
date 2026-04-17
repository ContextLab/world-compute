//! Integration tests for REST gateway (T174).

use worldcompute::network::rest_gateway::{verify_auth_token, RestGateway, RestResponse};

#[test]
fn rest_gateway_creation() {
    let gw = RestGateway::new(8080);
    assert_eq!(gw.port, 8080);
}

#[test]
fn rest_gateway_creation_different_port() {
    let gw = RestGateway::new(3000);
    assert_eq!(gw.port, 3000);
}

#[test]
fn known_donor_path_returns_200() {
    let gw = RestGateway::new(8080);
    let resp = gw.handle_request("GET", "/api/v1/donor/status", "");
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("trust_score"));
}

#[test]
fn known_cluster_path_returns_200() {
    let gw = RestGateway::new(8080);
    let resp = gw.handle_request("GET", "/api/v1/cluster/status", "");
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("nodes_online"));
}

#[test]
fn known_governance_path_returns_200() {
    let gw = RestGateway::new(8080);
    let resp = gw.handle_request("GET", "/api/v1/governance/proposals", "");
    assert_eq!(resp.status, 200);
}

#[test]
fn known_mesh_path_returns_200() {
    let gw = RestGateway::new(8080);
    let resp = gw.handle_request("GET", "/api/v1/mesh/status", "");
    assert_eq!(resp.status, 200);
}

#[test]
fn unknown_path_returns_404() {
    let gw = RestGateway::new(8080);
    let resp = gw.handle_request("GET", "/api/v1/nonexistent", "");
    assert_eq!(resp.status, 404);
    assert!(resp.body.contains("not found"));
}

#[test]
fn unknown_root_path_returns_404() {
    let gw = RestGateway::new(8080);
    let resp = gw.handle_request("GET", "/", "");
    assert_eq!(resp.status, 404);
}

#[test]
fn rate_limiting_blocks_excess_admin_requests() {
    let gw = RestGateway::new(8080);
    // AdminAction allows 1/min — first succeeds
    let resp1 = gw.handle_request("GET", "/api/v1/admin/health", "");
    assert_eq!(resp1.status, 200);
    // Second should be rate-limited
    let resp2 = gw.handle_request("GET", "/api/v1/admin/health", "");
    assert_eq!(resp2.status, 429);
}

#[test]
fn job_submit_with_valid_body_returns_200() {
    let gw = RestGateway::new(8080);
    let resp = gw.handle_request("POST", "/api/v1/job/submit", r#"{"manifest":"test"}"#);
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("job_id"));
}

#[test]
fn job_submit_empty_body_returns_400() {
    let gw = RestGateway::new(8080);
    let resp = gw.handle_request("POST", "/api/v1/job/submit", "");
    assert_eq!(resp.status, 400);
}

#[test]
fn rest_response_ok_has_json_content_type() {
    let resp = RestResponse::ok(r#"{"ok":true}"#);
    assert_eq!(resp.status, 200);
    assert!(resp.headers.iter().any(|(k, v)| k == "content-type" && v == "application/json"));
}

#[test]
fn rest_response_error_includes_status_in_body() {
    let resp = RestResponse::error(500, "internal");
    assert_eq!(resp.status, 500);
    assert!(resp.body.contains("500"));
    assert!(resp.body.contains("internal"));
}

#[test]
fn all_six_api_prefixes_routable() {
    let gw = RestGateway::new(9090);
    let routes = [
        ("GET", "/api/v1/donor/status"),
        ("GET", "/api/v1/job/list"),
        ("GET", "/api/v1/cluster/status"),
        ("GET", "/api/v1/governance/proposals"),
        ("GET", "/api/v1/admin/health"),
        ("GET", "/api/v1/mesh/status"),
    ];
    for (method, path) in routes {
        let resp = gw.handle_request(method, path, "");
        assert!(
            resp.status == 200 || resp.status == 429,
            "expected 200 or 429 for {method} {path}, got {}",
            resp.status
        );
    }
}

#[test]
fn auth_token_rejects_invalid_base64() {
    let result = verify_auth_token("not-valid!!!");
    assert!(result.is_err());
}

#[test]
fn auth_token_accepts_well_formed() {
    use base64::Engine;
    let sig_hex = "00".repeat(64);
    let raw = format!("peer-test:1713300000:{sig_hex}");
    let token = base64::engine::general_purpose::STANDARD.encode(&raw);
    let result = verify_auth_token(&token);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "peer-test");
}
