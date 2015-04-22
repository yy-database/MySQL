use uuid::Uuid;
use yykv_event::*;
use yykv_types::{DsValue, Redundancy};

#[tokio::test]
async fn test_wae_flow() {
    let wal = WalManager::new(100);
    let mut rx = wal.subscribe();

    let tenant_id = Uuid::new_v4();
    let data = DsValue::Text("test data".into());
    wal.write(
        tenant_id,
        "users".into(),
        OpType::Insert,
        Redundancy::TRUTH,
        data,
    )
    .await
    .unwrap();

    let event = rx.recv().await.unwrap();
    assert_eq!(event.tenant_id, tenant_id);
    assert_eq!(event.table, "users");
    assert_eq!(event.op_type, OpType::Insert);
    assert_eq!(event.sdr_level, Redundancy::TRUTH);
}
