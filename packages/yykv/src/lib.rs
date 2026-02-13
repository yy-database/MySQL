#![warn(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
pub use yykv_compute::{ComputeEngine, ComputeOperator, VectorMetric, VectorSearchOperator};
use yykv_tiering::TieredStorage;
use yykv_types::{DsError, DsResult, DsValue, PageId, Redundancy, StorageConfig};
use yykv_wal::{WalEntry, WalManager};

/// YYKV Engine: The high-level entry point for the storage engine.
/// It integrates I/O scheduling, tiered storage, and physical layout management.
pub struct YyKv {
    tiering: Arc<RwLock<TieredStorage>>,
    /// Metadata Index: Key -> List of PageId for redundancy
    index: Arc<RwLock<HashMap<String, Vec<PageId>>>>,
    wal: Option<Arc<RwLock<WalManager>>>,
    compute: Arc<ComputeEngine>,
}

impl YyKv {
    pub async fn new(config: StorageConfig) -> DsResult<Self> {
        info!(
            "Initializing YYKV engine with {} devices",
            config.devices.len()
        );
        let mut tiering = TieredStorage::new();

        // Register devices from config
        for dev_cfg in config.devices {
            let device = Arc::new(yykv_hal::MemDevice::new(
                format!("dev_{}", dev_cfg.id),
                dev_cfg.medium,
                1024 * 1024 * 10, // 10MB mock size
            ));
            tiering.add_device(dev_cfg.medium, device);
        }

        if let Some(path) = config.wal_path {
            info!("WAL enabled, loading from {:?}", path);
            let mut wal = WalManager::new(PathBuf::from(path))
                .await
                .map_err(|e| DsError::storage_with_op("wal_init", e.to_string()))?;

            // Recovery
            info!("Replaying WAL...");
            let entries = wal
                .replay()
                .await
                .map_err(|e| DsError::storage_with_op("wal_replay", e.to_string()))?;

            info!("Recovered {} entries from WAL", entries.len());

            let kv = YyKv {
                tiering: Arc::new(RwLock::new(tiering)),
                index: Arc::new(RwLock::new(HashMap::new())),
                wal: None, // Temporarily None during replay
                compute: Arc::new(ComputeEngine::new()),
            };

            for entry in entries {
                match entry {
                    WalEntry::Put {
                        tenant_id,
                        key,
                        value,
                        sdr,
                    } => {
                        kv.put_internal(tenant_id, key, value, sdr).await?;
                    }
                    WalEntry::Delete { tenant_id, key } => {
                        kv.delete_internal(tenant_id, &key).await?;
                    }
                }
            }

            // After replay, we can set the real tiering and index if we were replaying into a fresh state
            // In this simple implementation, YyKv::new is called on startup, so we replay into the fresh tiering/index.
            tiering = match Arc::try_unwrap(kv.tiering) {
                Ok(lock) => lock.into_inner(),
                Err(_) => panic!("Failed to unwrap tiering Arc"),
            };
            let index = match Arc::try_unwrap(kv.index) {
                Ok(lock) => lock.into_inner(),
                Err(_) => panic!("Failed to unwrap index Arc"),
            };

            let wal_manager = Some(Arc::new(RwLock::new(wal)));

            return Ok(YyKv {
                tiering: Arc::new(RwLock::new(tiering)),
                index: Arc::new(RwLock::new(index)),
                wal: wal_manager,
                compute: Arc::new(ComputeEngine::new()),
            });
        }

        Ok(YyKv {
            tiering: Arc::new(RwLock::new(tiering)),
            index: Arc::new(RwLock::new(HashMap::new())),
            wal: None,
            compute: Arc::new(ComputeEngine::new()),
        })
    }

    /// Import data from a .yydb file (JSON format for prototype) into a tenant's namespace.
    pub async fn import_from_file(&self, tenant_id: u64, path: PathBuf) -> DsResult<()> {
        if !path.exists() {
            return Err(DsError::storage(format!("File not found: {:?}", path)));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| DsError::storage_with_op("fs_read", e.to_string()))?;

        // Format: List of (key, value, sdr)
        // Since we removed serde_json, we use oak-json's parser.
        // For now, we only support a very limited set of types during import via oak-json
        // unless we implement a full Deserializer for JsonValue.
        let val = oak_json::parse(&content).map_err(|e| {
            DsError::storage(format!("Failed to parse .yydb file with oak-json: {}", e))
        })?;

        let array = val
            .as_array()
            .ok_or_else(|| DsError::storage("Expected array of entries in .yydb file"))?;

        for entry_val in &array.elements {
            let entry = entry_val
                .as_array()
                .ok_or_else(|| DsError::storage("Expected entry to be an array"))?;

            if entry.elements.len() != 3 {
                return Err(DsError::storage(
                    "Expected entry to have 3 elements: [key, value, sdr]",
                ));
            }

            let key = entry.elements[0]
                .as_str()
                .ok_or_else(|| DsError::storage("Expected key to be a string"))?
                .to_string();

            // For now, we only support basic types during import if we don't have a full deserializer
            let value = match &entry.elements[1] {
                oak_json::JsonValue::String(s) => DsValue::Text(s.value.clone()),
                oak_json::JsonValue::Number(n) => DsValue::Float(n.value),
                oak_json::JsonValue::Boolean(b) => DsValue::Bool(b.value),
                oak_json::JsonValue::Null(_) => DsValue::Null,
                _ => {
                    return Err(DsError::storage(
                        "Complex DsValue types are not yet supported in oak-json import",
                    ));
                }
            };

            let sdr_val = entry.elements[2]
                .as_f64()
                .ok_or_else(|| DsError::storage("Expected sdr to be a number"))?
                as u8;
            let sdr = Redundancy::from_u8(sdr_val);

            self.put(tenant_id, key, value, sdr).await?;
        }

        Ok(())
    }

    /// Internal put that doesn't write to WAL (used during recovery)
    async fn put_internal(
        &self,
        tenant_id: u64,
        key: String,
        value: DsValue,
        sdr: Redundancy,
    ) -> DsResult<()> {
        let replicas = sdr.replicas() as usize;

        let mut page_ids = Vec::with_capacity(replicas);

        for _ in 0..replicas {
            let mut tier = self.tiering.write().await;
            let page_id = tier
                .write_value(&value)
                .await
                .map_err(|e| DsError::storage(e.to_string()))?;
            page_ids.push(page_id);
        }

        let mut index = self.index.write().await;
        let tenant_key = format!("{}:{}", tenant_id, key);
        index.insert(tenant_key, page_ids);

        Ok(())
    }

    /// Internal delete that doesn't write to WAL (used during recovery)
    async fn delete_internal(&self, tenant_id: u64, key: &str) -> DsResult<()> {
        let mut index = self.index.write().await;
        let tenant_key = format!("{}:{}", tenant_id, key);
        index.remove(&tenant_key);
        Ok(())
    }

    /// Put a value with a specific SDR (Software-Defined Redundancy) level and tenant context.
    pub async fn put(
        &self,
        tenant_id: u64,
        key: String,
        value: DsValue,
        sdr: Redundancy,
    ) -> DsResult<()> {
        // 1. Write to WAL first
        if let Some(ref wal) = self.wal {
            let mut wal_lock = wal.write().await;
            wal_lock
                .append(WalEntry::Put {
                    tenant_id,
                    key: key.clone(),
                    value: value.clone(),
                    sdr,
                })
                .await
                .map_err(|e| DsError::storage(format!("WAL write failed: {}", e)))?;
        }

        // 2. Apply to memory/storage
        self.put_internal(tenant_id, key, value, sdr).await
    }

    /// Delete a value by key and tenant context.
    pub async fn delete(&self, tenant_id: u64, key: String) -> DsResult<()> {
        // 1. Write to WAL first
        if let Some(ref wal) = self.wal {
            let mut wal_lock = wal.write().await;
            wal_lock
                .append(WalEntry::Delete {
                    tenant_id,
                    key: key.clone(),
                })
                .await
                .map_err(|e| DsError::storage(format!("WAL write failed: {}", e)))?;
        }

        // 2. Apply to memory/storage
        self.delete_internal(tenant_id, &key).await
    }

    /// Get a value by key and tenant context.
    pub async fn get(&self, tenant_id: u64, key: &str) -> DsResult<Option<DsValue>> {
        let index = self.index.read().await;
        let tenant_key = format!("{}:{}", tenant_id, key);
        let page_ids = match index.get(&tenant_key) {
            Some(ids) => ids,
            None => return Ok(None),
        };

        // Try reading from replicas
        for page_id in page_ids {
            let mut tier = self.tiering.write().await;
            match tier.read_value(*page_id).await {
                Ok(val) => return Ok(Some(val)),
                Err(_) => continue, // Try next replica
            }
        }

        Err(DsError::storage("Failed to read value from any replica"))
    }

    /// Perform a vector search with pushdown to the storage layer.
    pub async fn vector_search(
        &self,
        tenant_id: u64,
        query: Vec<f32>,
        metric: VectorMetric,
        top_k: usize,
    ) -> DsResult<Vec<(f32, String)>> {
        let index = self.index.read().await;
        let mut results = Vec::new();

        let prefix = format!("{}:", tenant_id);

        for (tenant_key, page_ids) in index.iter() {
            if !tenant_key.starts_with(&prefix) {
                continue;
            }

            let key = tenant_key
                .strip_prefix(&prefix)
                .unwrap_or(tenant_key)
                .to_string();

            // Try reading from replicas
            for page_id in page_ids {
                let mut tier = self.tiering.write().await;
                // Get raw data for pushdown
                if let Ok(raw_data) = tier.read_raw(*page_id).await {
                    let op = VectorSearchOperator {
                        query_vector: query.clone(),
                        metric,
                        top_k,
                    };

                    if let Ok(result_bytes) = self.compute.pushdown(&op, &raw_data).await {
                        // For VectorSearchOperator, result_bytes is f32 similarity
                        if result_bytes.len() == 4 {
                            let sim = f32::from_le_bytes(result_bytes.try_into().unwrap());
                            results.push((sim, key.clone()));
                        }
                    }
                    break; // Successfully processed this key
                }
            }
        }

        // Sort and take top_k
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);

        Ok(results)
    }

    /// Batch put multiple values with tenant context.
    pub async fn batch_put(
        &self,
        tenant_id: u64,
        entries: Vec<(String, DsValue)>,
        sdr: Redundancy,
    ) -> DsResult<()> {
        for (key, value) in entries {
            self.put(tenant_id, key, value, sdr).await?;
        }
        Ok(())
    }

    /// Checkpoint the WAL: ensure all data is flushed and truncate the WAL.
    pub async fn checkpoint(&self) -> DsResult<()> {
        if let Some(ref wal) = self.wal {
            // In our current implementation, put_internal already writes to tiered storage.
            // So we just need to ensure the WAL is truncated.
            let mut wal_lock = wal.write().await;
            wal_lock
                .checkpoint()
                .await
                .map_err(|e| DsError::storage(format!("WAL checkpoint failed: {}", e)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yykv_types::{DeviceConfig, MediumType};

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
}
