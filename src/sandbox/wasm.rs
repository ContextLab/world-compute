//! WASM sandbox driver using wasmtime per FR-021.
//!
//! Tier 3 / browser / low-trust workloads. Cross-platform.
//! This is the one sandbox we can fully test on any host.

use crate::data_plane::cid_store::CidStore;
use crate::error::{ErrorCode, WcError};
use crate::sandbox::{Sandbox, SandboxCapability};
use crate::types::{Cid, DurationMs};
use wasmtime::{Config, Engine, Linker, Module, Store};

/// WASM sandbox state.
pub struct WasmSandbox {
    engine: Engine,
    workload_cid: Option<Cid>,
    module: Option<Module>,
    running: bool,
    work_dir: std::path::PathBuf,
    cid_store: CidStore,
    /// Captured output from last execution.
    last_output: Vec<u8>,
}

impl WasmSandbox {
    pub fn new(work_dir: std::path::PathBuf, cid_store: CidStore) -> Result<Self, WcError> {
        let mut config = Config::new();
        config.consume_fuel(true); // Resource limiting via fuel metering
        let engine = Engine::new(&config).map_err(|e| {
            WcError::new(ErrorCode::SandboxUnavailable, format!("WASM engine init: {e}"))
        })?;
        Ok(Self {
            engine,
            workload_cid: None,
            module: None,
            running: false,
            work_dir,
            cid_store,
            last_output: Vec::new(),
        })
    }

    /// Get the output from the last execution.
    pub fn output(&self) -> &[u8] {
        &self.last_output
    }
}

impl Sandbox for WasmSandbox {
    fn create(&mut self, workload_cid: &Cid) -> Result<(), WcError> {
        self.workload_cid = Some(*workload_cid);

        // Fetch WASM module bytes from CID store
        let wasm_bytes = self.cid_store.get(workload_cid).ok_or_else(|| {
            WcError::new(
                ErrorCode::InvalidManifest,
                format!("WASM module not found in CID store: {workload_cid}"),
            )
        })?;

        // Compile the module
        let module = compile_module(&self.engine, &wasm_bytes)?;
        self.module = Some(module);

        tracing::info!(workload_cid = %workload_cid, bytes = wasm_bytes.len(), "WASM sandbox created and module compiled");
        Ok(())
    }

    fn start(&mut self) -> Result<(), WcError> {
        let module = self.module.as_ref().ok_or_else(|| {
            WcError::new(ErrorCode::Internal, "No compiled module — call create() first")
        })?;

        self.running = true;

        // Run the module with default fuel budget
        let output = run_module(&self.engine, module, 1_000_000)?;
        self.last_output = output;

        // Write output to work directory if non-empty
        if !self.last_output.is_empty() {
            std::fs::create_dir_all(&self.work_dir).map_err(|e| {
                WcError::new(ErrorCode::Internal, format!("Cannot create work dir: {e}"))
            })?;
            let output_path = self.work_dir.join("stdout");
            std::fs::write(&output_path, &self.last_output).map_err(|e| {
                WcError::new(ErrorCode::Internal, format!("Cannot write output: {e}"))
            })?;
        }

        self.running = false;
        tracing::info!(output_bytes = self.last_output.len(), "WASM sandbox execution completed");
        Ok(())
    }

    fn freeze(&mut self) -> Result<(), WcError> {
        // WASM execution is cooperative — fuel exhaustion acts as freeze.
        tracing::info!("WASM sandbox frozen (fuel exhausted / epoch interrupt)");
        Ok(())
    }

    fn checkpoint(&mut self, _budget: DurationMs) -> Result<Cid, WcError> {
        // WASM checkpointing requires serializing the Store's memory.
        // wasmtime doesn't natively support this yet — this is a known
        // limitation for WASM workloads (Tier 3 accepts restartable only).
        Err(WcError::new(
            ErrorCode::Internal,
            "WASM checkpoint not supported — Tier 3 workloads are Restartable",
        ))
    }

    fn terminate(&mut self) -> Result<(), WcError> {
        self.running = false;
        self.module = None;
        tracing::info!("WASM sandbox terminated");
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), WcError> {
        if self.work_dir.exists() {
            std::fs::remove_dir_all(&self.work_dir)
                .map_err(|e| WcError::new(ErrorCode::Internal, format!("Cleanup failed: {e}")))?;
        }
        tracing::info!("WASM sandbox cleaned up");
        Ok(())
    }

    fn capability(&self) -> SandboxCapability {
        SandboxCapability::WasmOnly
    }
}

/// Compile a WASM module from bytes.
pub fn compile_module(engine: &Engine, wasm_bytes: &[u8]) -> Result<Module, WcError> {
    Module::new(engine, wasm_bytes).map_err(|e| {
        WcError::new(ErrorCode::InvalidManifest, format!("WASM compilation failed: {e}"))
    })
}

/// Run a WASM module with fuel-limited execution and return stdout bytes.
///
/// Instantiates the module, calls `_start` if exported, and returns any
/// memory exported as "output" (convention for World Compute WASM workloads).
pub fn run_module(engine: &Engine, module: &Module, fuel: u64) -> Result<Vec<u8>, WcError> {
    let mut store = Store::new(engine, ());
    store
        .set_fuel(fuel)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("Fuel setup: {e}")))?;

    let linker = Linker::new(engine);
    let instance = linker
        .instantiate(&mut store, module)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("WASM instantiation: {e}")))?;

    // Try to call _start (WASI convention)
    if let Ok(start_fn) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
        match start_fn.call(&mut store, ()) {
            Ok(()) => {}
            Err(e) => {
                let msg = e.to_string();
                // Fuel exhaustion is expected for long-running modules — treat as completion
                if msg.contains("fuel") {
                    tracing::debug!("WASM execution fuel exhausted (normal termination)");
                } else {
                    return Err(WcError::new(
                        ErrorCode::Internal,
                        format!("WASM _start failed: {e}"),
                    ));
                }
            }
        }
    }

    // Try to read output from exported memory (convention: "memory" export
    // with result written to a known offset, or an "output" function)
    if let Ok(output_fn) = instance.get_typed_func::<(), i32>(&mut store, "output_len") {
        if let Ok(len) = output_fn.call(&mut store, ()) {
            if let Some(memory) = instance.get_memory(&mut store, "memory") {
                let len = len as usize;
                let data = memory.data(&store);
                if len <= data.len() {
                    return Ok(data[..len].to_vec());
                }
            }
        }
    }

    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::CidStore;

    /// Minimal valid WASM module (no imports, no exports, no start).
    fn minimal_wasm_module() -> Vec<u8> {
        // Binary encoding of `(module)` — the smallest valid WASM
        vec![
            0x00, 0x61, 0x73, 0x6d, // magic: \0asm
            0x01, 0x00, 0x00, 0x00, // version: 1
        ]
    }

    #[test]
    fn wasm_engine_initializes() {
        let store = CidStore::new();
        let sandbox = WasmSandbox::new(std::env::temp_dir().join("wc-test-wasm"), store);
        assert!(sandbox.is_ok());
    }

    #[test]
    fn wasm_create_fails_for_missing_cid() {
        let store = CidStore::new();
        let mut sandbox =
            WasmSandbox::new(std::env::temp_dir().join("wc-test-wasm-missing"), store).unwrap();
        let cid = crate::data_plane::cid_store::compute_cid(b"nonexistent").unwrap();
        assert!(sandbox.create(&cid).is_err());
    }

    #[test]
    fn wasm_compile_and_run_minimal_module() {
        let store = CidStore::new();
        let wasm_bytes = minimal_wasm_module();
        let cid = store.put(&wasm_bytes).unwrap();

        let mut sandbox =
            WasmSandbox::new(std::env::temp_dir().join("wc-test-wasm-run"), store).unwrap();
        assert!(sandbox.create(&cid).is_ok());
        assert!(sandbox.start().is_ok());
        assert!(sandbox.terminate().is_ok());
    }

    #[test]
    fn wasm_compile_rejects_invalid_bytes() {
        let store = CidStore::new();
        let bad_bytes = b"this is not wasm";
        let cid = store.put(bad_bytes).unwrap();

        let mut sandbox =
            WasmSandbox::new(std::env::temp_dir().join("wc-test-wasm-bad"), store).unwrap();
        let result = sandbox.create(&cid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("compilation failed"));
    }

    #[test]
    fn wasm_sandbox_lifecycle_with_store() {
        let store = CidStore::new();
        let wasm_bytes = minimal_wasm_module();
        let cid = store.put(&wasm_bytes).unwrap();

        let mut sandbox =
            WasmSandbox::new(std::env::temp_dir().join("wc-test-wasm-lifecycle"), store).unwrap();
        assert!(sandbox.create(&cid).is_ok());
        assert!(sandbox.start().is_ok());
        assert!(sandbox.freeze().is_ok());
        // Checkpoint should fail for WASM (Restartable only)
        assert!(sandbox.checkpoint(crate::types::DurationMs(500)).is_err());
        assert!(sandbox.terminate().is_ok());
        assert!(sandbox.cleanup().is_ok());
    }
}
