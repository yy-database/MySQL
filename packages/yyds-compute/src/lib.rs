pub use yykv_udf::{UdfEngine, UdfRuntime as WasmtimeUdfEngine};
use yyds_types::{DsValue, DsResult};
use async_trait::async_trait;

/// 模拟 Wasm UDF 运行时 (保留用于测试)
pub struct MockWasmRuntime;

#[async_trait]
impl UdfEngine for MockWasmRuntime {
    async fn call(&self, name: &str, input: DsValue) -> DsResult<DsValue> {
        match name {
            "to_upper" => {
                if let DsValue::Text(s) = input {
                    Ok(DsValue::Text(s.to_uppercase()))
                } else {
                    Ok(input)
                }
            }
            _ => Ok(input),
        }
    }
}
