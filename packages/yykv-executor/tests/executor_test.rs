use yykv_executor::*;
use yykv_event::{WalManager, OpType};
use yykv_types::{DsResult, DsValue, DsError};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

struct MockUdf;
#[async_trait]
impl UdfEngine for MockUdf {
    async fn call(&self, name: &str, input: Vec<u8>) -> DsResult<DsValue> {
        use yykv_types::layout::YYValueDecoder;
        let val = YYValueDecoder::decode(&input.into())
            .map_err(|e| DsError::internal(format!("UDF decode failed: {:?}", e)))?;

        if name == "to_upper"
            && let DsValue::Text(s) = val
        {
            return Ok(DsValue::Text(s.to_uppercase()));
        }
        Ok(val)
    }
}

#[tokio::test]
async fn test_wae_near_data_computing_distributed() {
    let wal = Arc::new(WalManager::new(100));
    let compute = Arc::new(MockUdf);
    let executor = YykvExecutor::new(Uuid::new_v4(), compute, wal.clone());

    let tenant_id = Uuid::new_v4();
    let worker = executor.create_wae_worker(tenant_id);

    let mut results = worker.start("users".into(), "to_upper".into()).await;

    // 写入数据触发事件
    wal.write(
        tenant_id,
        "users".into(),
        OpType::Insert,
        yykv_types::Redundancy::SINGLE,
        DsValue::Text("yykv".into()),
    )
    .await
    .unwrap();

    // 验证计算结果
    let res = results.recv().await.unwrap();
    assert_eq!(res, DsValue::Text("YYKV".into()));

    // 验证多租户隔离 (另一个租户的数据不应被处理)
    let other_tenant = Uuid::new_v4();
    wal.write(
        other_tenant,
        "users".into(),
        OpType::Insert,
        yykv_types::Redundancy::SINGLE,
        DsValue::Text("hidden".into()),
    )
    .await
    .unwrap();

    // 我们不应该收到 "HIDDEN"
    // 实际上由于 results.recv() 是阻塞的，我们需要一种非阻塞的方式来验证
    // 这里简单起见，我们再次写入当前租户的数据并确保顺序正确
    wal.write(
        tenant_id,
        "users".into(),
        OpType::Insert,
        yykv_types::Redundancy::SINGLE,
        DsValue::Text("visible".into()),
    )
    .await
    .unwrap();

    let res = results.recv().await.unwrap();
    assert_eq!(res, DsValue::Text("VISIBLE".into()));
}
