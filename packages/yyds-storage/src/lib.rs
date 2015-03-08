use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use yyds_types::{YYValue, Redundancy, Result, InternalId, YYError};
use tracing::{info, warn, debug, error};
use std::path::Path;
use std::fs;
use tokio::time::{self, Duration};
use bytes::{BytesMut, Buf, BufMut, Bytes};
use yykv_types::layout::{DsValueEncoder, YYValueDecoder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub devices: Vec<DeviceConfig>,
    pub page_size: usize,
    pub data_path: Option<String>,
    /// The specific shard this storage instance is responsible for.
    pub local_shard_id: Option<u16>,
    /// How often to checkpoint data to disk in seconds.
    pub checkpoint_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: u32,
    pub path: String,
    pub medium: MediumType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MediumType {
    NVM,
    NvmeSSD,
    SataSSD,
    HDD,
}

/// Helper for manual serialization of snapshots
struct SnapshotCodec;

impl SnapshotCodec {
    fn encode(snapshot: &HashMap<u64, HashMap<String, YYValue>>) -> Result<Vec<u8>> {
        let mut buf = BytesMut::new();
        // Version
        buf.put_u8(1);
        
        // Tenant Count
        buf.put_u32(snapshot.len() as u32);
        
        for (tenant_id, data) in snapshot {
            buf.put_u64(*tenant_id);
            // Item Count
            buf.put_u32(data.len() as u32);
            for (key, value) in data {
                let key_bytes = key.as_bytes();
                buf.put_u32(key_bytes.len() as u32);
                buf.put_slice(key_bytes);
                DsValueEncoder::encode_to_buf(value, &mut buf)
                    .map_err(|e| YYError::serialization(e.to_string()))?;
            }
        }
        
        Ok(buf.to_vec())
    }

    fn decode(data: &[u8]) -> Result<HashMap<u64, HashMap<String, YYValue>>> {
        let mut buf = Bytes::copy_from_slice(data);
        if buf.is_empty() {
            return Ok(HashMap::new());
        }
        
        let version = buf.get_u8();
        if version != 1 {
            return Err(YYError::serialization(format!("Unknown snapshot version: {}", version)));
        }

        let tenant_count = buf.get_u32();
        let mut snapshot = HashMap::with_capacity(tenant_count as usize);

        for _ in 0..tenant_count {
            let tenant_id = buf.get_u64();
            let item_count = buf.get_u32();
            let mut items = HashMap::with_capacity(item_count as usize);
            
            for _ in 0..item_count {
                let key_len = buf.get_u32() as usize;
                let key = String::from_utf8(buf.split_to(key_len).to_vec())
                    .map_err(|e| YYError::serialization(e.to_string()))?;
                let value = YYValueDecoder::decode_from_stream(&mut buf)
                    .map_err(|e| YYError::serialization(e.to_string()))?;
                items.insert(key, value);
            }
            snapshot.insert(tenant_id, items);
        }
        
        Ok(snapshot)
    }
}

/// Native storage engine for YYDS.
pub struct YydsStorage {
    /// TenantID -> Key -> Value
    /// Using nested locks for per-tenant concurrency.
    data: Arc<RwLock<HashMap<u64, Arc<RwLock<HashMap<String, YYValue>>>>>>,
    config: StorageConfig,
    /// Track if data has changed since last checkpoint.
    dirty: Arc<RwLock<bool>>,
}

impl YydsStorage {
    pub async fn new(config: StorageConfig) -> Result<Self> {
        info!("Initializing native YYDS storage");
        
        let mut initial_data = HashMap::new();
        
        if let Some(path) = &config.data_path {
            if Path::new(path).exists() {
                match fs::read(path) {
                    Ok(content) => {
                        match SnapshotCodec::decode(&content) {
                            Ok(data) => {
                                info!("Loaded storage data from {}", path);
                                for (tenant_id, tenant_map) in data {
                                    initial_data.insert(tenant_id, Arc::new(RwLock::new(tenant_map)));
                                }
                            }
                            Err(e) => warn!("Failed to parse storage data: {}. Starting fresh.", e),
                        }
                    }
                    Err(e) => warn!("Failed to read storage file: {}. Starting fresh.", e),
                }
            }
        }

        let storage = Self {
            data: Arc::new(RwLock::new(initial_data)),
            dirty: Arc::new(RwLock::new(false)),
            config,
        };

        // Start background checkpointing task
        storage.spawn_checkpoint_task();

        Ok(storage)
    }

    fn spawn_checkpoint_task(&self) {
        if self.config.data_path.is_none() {
            return;
        }

        let data_arc: Arc<RwLock<HashMap<u64, Arc<RwLock<HashMap<String, YYValue>>>>>> = Arc::clone(&self.data);
        let dirty_arc: Arc<RwLock<bool>> = Arc::clone(&self.dirty);
        let path = self.config.data_path.clone().unwrap();
        let interval = self.config.checkpoint_interval_secs;

        tokio::spawn(async move {
            let mut ticker = time::interval(Duration::from_secs(interval));
            loop {
                ticker.tick().await;
                
                let is_dirty = {
                    let lock = dirty_arc.read();
                    *lock
                };
                if !is_dirty {
                    continue;
                }

                debug!("Background checkpointing storage to {}", path);
                
                // Collect data for serialization
                let snapshot = {
                    let main_lock = data_arc.read();
                    let mut snapshot = HashMap::new();
                    for (&tenant_id, tenant_data_lock) in main_lock.iter() {
                        snapshot.insert(tenant_id, tenant_data_lock.read().clone());
                    }
                    snapshot
                };

                match SnapshotCodec::encode(&snapshot) {
                    Ok(bytes) => {
                        if let Err(e) = fs::write(&path, bytes) {
                            error!("Failed to write checkpoint to {}: {}", path, e);
                        } else {
                            *dirty_arc.write() = false;
                            debug!("Checkpoint completed successfully");
                        }
                    }
                    Err(e) => error!("Failed to serialize storage for checkpoint: {}", e),
                }
            }
        });
    }

    pub async fn put(&self, internal_id: u64, key: String, value: YYValue, _sdr: Redundancy) -> Result<()> {
        let id = InternalId(internal_id);
        
        // Optimization: Quick shard check using bitwise operations
        if let Some(local_shard) = self.config.local_shard_id {
            if id.shard_id() != local_shard {
                warn!("Data routing mismatch: Received data for shard {}, but local shard is {}", id.shard_id(), local_shard);
            }
        }

        // Get or create per-tenant map
        let tenant_map: Arc<RwLock<HashMap<String, YYValue>>> = {
            let data_lock = self.data.read();
            if let Some(map) = data_lock.get(&internal_id) {
                Arc::clone(map)
            } else {
                drop(data_lock);
                let mut data_lock = self.data.write();
                let map = data_lock.entry(internal_id).or_insert_with(|| Arc::new(RwLock::new(HashMap::new())));
                Arc::clone(map)
            }
        };

        // Write to tenant map
        {
            let mut tenant_lock = tenant_map.write();
            tenant_lock.insert(key, value);
        }
        
        // Mark as dirty for background checkpointing
        *self.dirty.write() = true;
        
        Ok(())
    }

    pub async fn get(&self, internal_id: u64, key: &str) -> Result<Option<YYValue>> {
        let tenant_map = {
            let data_lock = self.data.read();
            data_lock.get(&internal_id).cloned()
        };

        if let Some(map) = tenant_map {
            let tenant_lock = map.read();
            Ok(tenant_lock.get(key).cloned())
        } else {
            Ok(None)
        }
    }

    pub async fn delete(&self, internal_id: u64, key: String) -> Result<()> {
        let tenant_map = {
            let data_lock = self.data.read();
            data_lock.get(&internal_id).cloned()
        };

        if let Some(map) = tenant_map {
            let mut tenant_lock = map.write();
            if tenant_lock.remove(&key).is_some() {
                *self.dirty.write() = true;
            }
        }
        Ok(())
    }

    pub async fn checkpoint(&self) -> Result<()> {
        info!("Manual checkpoint requested");
        *self.dirty.write() = true;
        // In a real system, we might want to wait for the background task or trigger it immediately.
        Ok(())
    }
}

