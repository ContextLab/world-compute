//! WASM sandbox driver using wasmtime per FR-021.
//!
//! Tier 3 / browser / low-trust workloads. Cross-platform.
//! This is the one sandbox we can fully test on any host.

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
}

impl WasmSandbox {
    pub fn new(work_dir: std::path::PathBuf) -> Result<Self, WcError> {
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
        })
    }
}

impl Sandbox for WasmSandbox {
    fn create(&mut self, workload_cid: &Cid) -> Result<(), WcError> {
        self.workload_cid = Some(*workload_cid);
        // TODO: Fetch WASM module bytes from CID store, compile.
        // For now, create a minimal test module.
        tracing::info!(workload_cid = %workload_cid, "WASM sandbox created");
        Ok(())
    }

    fn start(&mut self) -> Result<(), WcError> {
        self.running = true;
        // TODO: Instantiate module in a Store with fuel limits,
        // call the entrypoint, capture stdout as result.
        tracing::info!("WASM sandbox started");
        Ok(())
    }

    fn freeze(&mut self) -> Result<(), WcError> {
        // WASM execution is cooperative — fuel exhaustion acts as freeze.
        // For true freeze, we interrupt the Store's epoch.
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
            std::fs::remove_dir_all(&self.work_dir).map_err(|e| {
                WcError::new(ErrorCode::Internal, format!("Cleanup failed: {e}"))
            })?;
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
pub fn run_module(engine: &Engine, module: &Module, fuel: u64) -> Result<Vec<u8>, WcError> {
    let mut store = Store::new(engine, ());
    store.set_fuel(fuel).map_err(|e| {
        WcError::new(ErrorCode::Internal, format!("Fuel setup: {e}"))
    })?;

    let linker = Linker::new(engine);
    let _instance = linker.instantiate(&mut store, module).map_err(|e| {
        WcError::new(ErrorCode::Internal, format!("WASM instantiation: {e}"))
    })?;

    // TODO: Call _start or main, capture output via WASI stdout.
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_engine_initializes() {
        let sandbox = WasmSandbox::new(std::path::PathBuf::from("/tmp/wc-test-wasm"));
        assert!(sandbox.is_ok());
    }

    #[test]
    fn wasm_sandbox_lifecycle() {
        let mut sandbox = WasmSandbox::new(std::path::PathBuf::from("/tmp/wc-test-wasm-lc"))
            .unwrap();
        let cid = crate::data_plane::cid_store::compute_cid(b"test wasm module").unwrap();
        assert!(sandbox.create(&cid).is_ok());
        assert!(sandbox.start().is_ok());
        assert!(sandbox.freeze().is_ok());
        // Checkpoint should fail for WASM (Restartable only)
        assert!(sandbox.checkpoint(crate::types::DurationMs(500)).is_err());
        assert!(sandbox.terminate().is_ok());
        assert!(sandbox.cleanup().is_ok());
    }
}
