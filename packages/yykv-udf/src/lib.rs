#![warn(missing_docs)]

use async_trait::async_trait;
use tracing::info;
use wasmtime::{Config, Engine, component::Linker};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use yykv_types::{DsError, DsResult, DsValue};

#[async_trait]
pub trait UdfEngine: Send + Sync {
    /// 执行 UDF
    async fn call(&self, name: &str, input: DsValue) -> DsResult<DsValue>;
}

/// UdfRuntime manages the execution of Wasm-based User Defined Functions
/// directly within the storage engine layer (YYKV).
pub struct UdfRuntime {
    _engine: Engine,
    _linker: Linker<UdfState>,
}

#[async_trait]
impl UdfEngine for UdfRuntime {
    async fn call(&self, name: &str, _input: DsValue) -> DsResult<DsValue> {
        // Implementation
        Err(DsError::internal(format!(
            "UDF {} execution not fully implemented",
            name
        )))
    }
}

struct UdfState {
    wasi_ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for UdfState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

impl UdfRuntime {
    pub fn new() -> DsResult<Self> {
        info!("Initializing UDF runtime (Wasmtime)");
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config)
            .map_err(|e| DsError::internal(format!("Failed to create Wasm engine: {}", e)))?;

        let linker = Linker::new(&engine);

        Ok(Self {
            _engine: engine,
            _linker: linker,
        })
    }

    // Additional methods to load and call UDFs would go here
}
