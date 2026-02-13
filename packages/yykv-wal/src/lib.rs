#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::io::SeekFrom;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use yykv_types::{DsValue, DsResult, Redundancy, DsError, Uuid};
use bytes::{BytesMut, Buf, BufMut, Bytes};
use yykv_types::layout::{DsValueEncoder, YYValueDecoder};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpType {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WalEntry {
    Put {
        tenant_id: Uuid,
        table: String,
        key: String,
        value: DsValue,
        sdr: Redundancy,
        op_type: OpType,
        timestamp: DateTime<Utc>,
    },
    Delete {
        tenant_id: Uuid,
        table: String,
        key: String,
        timestamp: DateTime<Utc>,
    },
}

impl WalEntry {
    pub fn encode(&self) -> DsResult<Bytes> {
        let mut buf = BytesMut::new();
        match self {
            WalEntry::Put { tenant_id, table, key, value, sdr, op_type, timestamp } => {
                buf.put_u8(1); // Type: Put
                buf.put_slice(tenant_id.as_bytes());
                
                let table_bytes = table.as_bytes();
                buf.put_u32(table_bytes.len() as u32);
                buf.put_slice(table_bytes);

                let key_bytes = key.as_bytes();
                buf.put_u32(key_bytes.len() as u32);
                buf.put_slice(key_bytes);

                DsValueEncoder::encode_to_buf(value, &mut buf)?;
                buf.put_u8(sdr.0);
                buf.put_u8(*op_type as u8);
                buf.put_i64(timestamp.timestamp_nanos_opt().unwrap_or(0));
            }
            WalEntry::Delete { tenant_id, table, key, timestamp } => {
                buf.put_u8(2); // Type: Delete
                buf.put_slice(tenant_id.as_bytes());

                let table_bytes = table.as_bytes();
                buf.put_u32(table_bytes.len() as u32);
                buf.put_slice(table_bytes);

                let key_bytes = key.as_bytes();
                buf.put_u32(key_bytes.len() as u32);
                buf.put_slice(key_bytes);
                
                buf.put_i64(timestamp.timestamp_nanos_opt().unwrap_or(0));
            }
        }
        Ok(buf.freeze())
    }

    pub fn decode(src: &[u8]) -> DsResult<Self> {
        let mut buf = Bytes::copy_from_slice(src);
        if buf.is_empty() {
            return Err(DsError::serialization("Empty buffer for WalEntry"));
        }
        let type_id = buf.get_u8();
        match type_id {
            1 => { // Put
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let tenant_id = Uuid::from_bytes(uuid_bytes);

                let table_len = buf.get_u32() as usize;
                let table = String::from_utf8(buf.split_to(table_len).to_vec())
                    .map_err(|e| DsError::serialization(e.to_string()))?;

                let key_len = buf.get_u32() as usize;
                let key = String::from_utf8(buf.split_to(key_len).to_vec())
                    .map_err(|e| DsError::serialization(e.to_string()))?;

                let value = YYValueDecoder::decode(&mut buf)?;
                let sdr = Redundancy::from_u8(buf.get_u8());
                let op_type = match buf.get_u8() {
                    0 => OpType::Insert,
                    1 => OpType::Update,
                    2 => OpType::Delete,
                    _ => OpType::Insert,
                };
                let timestamp = DateTime::<Utc>::from_timestamp_nanos(buf.get_i64());

                Ok(WalEntry::Put { tenant_id, table, key, value, sdr, op_type, timestamp })
            }
            2 => { // Delete
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let tenant_id = Uuid::from_bytes(uuid_bytes);

                let table_len = buf.get_u32() as usize;
                let table = String::from_utf8(buf.split_to(table_len).to_vec())
                    .map_err(|e| DsError::serialization(e.to_string()))?;

                let key_len = buf.get_u32() as usize;
                let key = String::from_utf8(buf.split_to(key_len).to_vec())
                    .map_err(|e| DsError::serialization(e.to_string()))?;
                
                let timestamp = DateTime::<Utc>::from_timestamp_nanos(buf.get_i64());

                Ok(WalEntry::Delete { tenant_id, table, key, timestamp })
            }
            _ => Err(DsError::serialization(format!("Unknown WalEntry type: {}", type_id))),
        }
    }
}

pub struct WalManager {
    path: PathBuf,
    file: Arc<Mutex<File>>,
}

impl WalManager {
    pub async fn new(path: PathBuf) -> DsResult<Self> {
        info!("Opening WAL at {:?}", path);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .await?;

        Ok(Self { path, file: Arc::new(Mutex::new(file)) })
    }

    pub async fn write(
        &self,
        tenant_id: Uuid,
        table: String,
        op_type: OpType,
        sdr_level: Redundancy,
        data: DsValue,
    ) -> DsResult<()> {
        let entry = WalEntry::Put {
            tenant_id,
            table,
            key: "".to_string(), // TODO: key should be passed if needed
            value: data,
            sdr: sdr_level,
            op_type,
            timestamp: Utc::now(),
        };
        self.append(entry).await
    }

    pub async fn append(&self, entry: WalEntry) -> DsResult<()> {
        let data = entry.encode()?;
        let len = data.len() as u32;

        let mut file = self.file.lock().await;
        // Write length prefix then data
        file.write_u32(len).await?;
        file.write_all(&data).await?;
        file.flush().await?;

        Ok(())
    }

    pub async fn replay(&self) -> DsResult<Vec<WalEntry>> {
        info!("Replaying WAL from {:?}", self.path);
        let mut entries = Vec::new();
        let mut file = File::open(&self.path).await?;

        let file_len = file.metadata().await?.len();
        if file_len == 0 {
            return Ok(entries);
        }

        loop {
            let pos = file.seek(SeekFrom::Current(0)).await?;
            if pos >= file_len {
                break;
            }

            let len = match file.read_u32().await {
                Ok(l) => l,
                Err(e) => {
                    warn!("Failed to read WAL entry length at pos {}: {}", pos, e);
                    break;
                }
            };

            let mut buf = vec![0u8; len as usize];
            if let Err(e) = file.read_exact(&mut buf).await {
                warn!("Failed to read WAL entry data at pos {}: {}", pos, e);
                break; // Corrupted
            }

            match WalEntry::decode(&buf) {
                Ok(entry) => {
                    debug!("Recovered WAL entry: {:?}", entry);
                    entries.push(entry);
                }
                Err(e) => {
                    warn!("Failed to deserialize WAL entry at pos {}: {}", pos, e);
                }
            }
        }

        info!("WAL replay complete, recovered {} entries", entries.len());
        Ok(entries)
    }

    pub async fn checkpoint(&self) -> DsResult<()> {
        info!("Checkpointing WAL at {:?}", self.path);
        let mut file = self.file.lock().await;
        // Simple checkpoint: truncate the file
        *file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .await?;
        Ok(())
    }
}
