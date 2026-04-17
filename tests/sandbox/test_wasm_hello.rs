//! T029: WASM "hello world" workload test.
//!
//! Creates a minimal WASM module, stores it in a CidStore,
//! creates a WasmSandbox, calls create() and start(), and
//! verifies it completes without error.

use worldcompute::data_plane::cid_store::CidStore;
use worldcompute::sandbox::wasm::WasmSandbox;
use worldcompute::sandbox::Sandbox;

/// Minimal valid WASM module (empty module, no imports/exports/start).
fn minimal_wasm_module() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, // magic: \0asm
        0x01, 0x00, 0x00, 0x00, // version: 1
    ]
}

#[test]
fn wasm_hello_world_lifecycle() {
    let store = CidStore::new();
    let wasm_bytes = minimal_wasm_module();
    let cid = store.put(&wasm_bytes).unwrap();

    let work_dir = std::env::temp_dir().join("wc-t029-wasm-hello");
    let _ = std::fs::remove_dir_all(&work_dir); // clean slate

    let mut sandbox = WasmSandbox::new(work_dir.clone(), store).expect("WasmSandbox::new should succeed");

    // create() loads the module from the CidStore
    sandbox.create(&cid).expect("create() should succeed for valid WASM module");

    // start() executes the module (no-op for empty module)
    sandbox.start().expect("start() should succeed for minimal WASM module");

    // Verify capability
    assert_eq!(sandbox.capability(), worldcompute::sandbox::SandboxCapability::WasmOnly);

    // Cleanup
    sandbox.terminate().expect("terminate() should succeed");
    sandbox.cleanup().expect("cleanup() should succeed");

    assert!(!work_dir.exists(), "Work dir should be removed after cleanup");
}

#[test]
fn wasm_create_fails_for_missing_cid_in_store() {
    let store = CidStore::new();
    let work_dir = std::env::temp_dir().join("wc-t029-wasm-missing-cid");
    let _ = std::fs::remove_dir_all(&work_dir);

    let mut sandbox = WasmSandbox::new(work_dir, store).unwrap();

    // Compute a CID for data that was never stored
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"not-in-store").unwrap();
    let result = sandbox.create(&cid);
    assert!(result.is_err(), "create() should fail when CID is not in store");
}

#[test]
fn wasm_create_fails_for_invalid_wasm_bytes() {
    let store = CidStore::new();
    let bad_bytes = b"this is not valid wasm bytecode";
    let cid = store.put(bad_bytes).unwrap();

    let work_dir = std::env::temp_dir().join("wc-t029-wasm-bad-bytes");
    let _ = std::fs::remove_dir_all(&work_dir);

    let mut sandbox = WasmSandbox::new(work_dir, store).unwrap();
    let result = sandbox.create(&cid);
    assert!(result.is_err(), "create() should fail for invalid WASM bytes");
    assert!(
        result.unwrap_err().to_string().contains("compilation failed"),
        "Error should mention compilation failure"
    );
}
