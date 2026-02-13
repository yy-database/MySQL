use yykv::*;
use yykv_types::{DsResult, DsValue, StorageConfig, DeviceConfig, MediumType, Redundancy};

#[tokio::test]
async fn test_yykv_value_storage_with_sdr() -> DsResult<()> {
    let config = StorageConfig {
        devices: vec![DeviceConfig {
            id: 0,
            path: "dev0.db".into(),
            medium: MediumType::NvmeSSD,
        }],
        page_size: 4096,
        wal_path: None,
    };

    let kv = YyKv::new(config).await?;
    let key = "test_key".to_string();
    let value = DsValue::Text("Hello YYKV".into());
    let tenant_id = 1;

    kv.put(tenant_id, key.clone(), value.clone(), Redundancy::EDGE)
        .await?;
    let result = kv.get(tenant_id, &key).await?;
    assert_eq!(result, Some(value));
    Ok(())
}

#[tokio::test]
async fn test_yykv_wal_recovery() -> DsResult<()> {
    let wal_path = "test_wal.log";
    // Clean up previous test
    let _ = std::fs::remove_file(wal_path);

    let config = StorageConfig {
        devices: vec![DeviceConfig {
            id: 0,
            path: "dev0.db".into(),
            medium: MediumType::NvmeSSD,
        }],
        page_size: 4096,
        wal_path: Some(wal_path.into()),
    };

    let tenant_id = 1;
    let key = "wal_test_key".to_string();
    let value = DsValue::Text("WAL Data".into());

    {
        let kv = YyKv::new(config.clone()).await?;
        kv.put(tenant_id, key.clone(), value.clone(), Redundancy::SINGLE)
            .await?;
        // Simulate crash - kv is dropped without checkpoint
    }

    // Re-open and check if data is recovered
    let kv = YyKv::new(config.clone()).await?;
    let result = kv.get(tenant_id, &key).await?;
    assert_eq!(result, Some(value.clone()));

    // Test checkpoint
    kv.checkpoint().await?;

    // After checkpoint, WAL should be empty but data still in storage
    let result = kv.get(tenant_id, &key).await?;
    assert_eq!(result, Some(value));

    // Clean up
    let _ = std::fs::remove_file(wal_path);
    Ok(())
}
