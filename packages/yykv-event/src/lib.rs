#![warn(missing_docs)]

use bytes::{BufMut, Bytes, BytesMut};
use chrono::{DateTime, Utc};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::debug;
use uuid::Uuid;
use yykv_types::layout::DsValueEncoder;
use yykv_types::{DsResult, DsValue, Redundancy};

/// WAE (Write-Ahead Event) - 写入即事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub table: String,
    pub op_type: OpType,
    pub sdr_level: Redundancy,
    pub data: DsValue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpType {
    Insert,
    Update,
    Delete,
}

/// Physical representation of a WAL entry for storage/transmission.
/// | Magic (2B) | Version (1B) | OpType (1B) | SDR (1B) | Flags (3B) |
/// | Timestamp (8B) | RequestID (16B) | TenantID (16B) |
/// | TableLen (4B) | TableName (Var) | DataLen (4B) | Data (Var) |
/// | Checksum (4B) |
#[derive(Debug, Clone)]
pub struct WalEntry {
    pub header: WalHeader,
    pub table: String,
    pub data: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WalHeader {
    pub op_type: OpType,
    pub sdr_level: Redundancy,
    pub timestamp: i64,
    pub request_id: Uuid,
    pub tenant_id: Uuid,
    pub checksum: u32,
}

impl WalEvent {
    pub fn to_entry(&self) -> DsResult<WalEntry> {
        let data_bytes = DsValueEncoder::encode(&self.data)?;

        let mut hasher = Hasher::new();
        hasher.update(self.table.as_bytes());
        hasher.update(&data_bytes);
        let checksum = hasher.finalize();

        Ok(WalEntry {
            header: WalHeader {
                op_type: self.op_type,
                sdr_level: self.sdr_level,
                timestamp: self.timestamp.timestamp_nanos_opt().unwrap_or(0),
                request_id: self.id,
                tenant_id: self.tenant_id,
                checksum,
            },
            table: self.table.clone(),
            data: data_bytes,
        })
    }
}

pub struct WalManager {
    sender: broadcast::Sender<WalEvent>,
}

impl WalManager {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// 写入 WAL 并发布事件
    pub async fn write(
        &self,
        tenant_id: Uuid,
        table: String,
        op_type: OpType,
        sdr_level: Redundancy,
        data: DsValue,
    ) -> DsResult<Uuid> {
        let event = WalEvent {
            id: Uuid::now_v7(),
            tenant_id,
            timestamp: Utc::now(),
            table,
            op_type,
            sdr_level,
            data,
        };

        // 1. 转换为物理 Entry 并持久化
        let entry = event.to_entry()?;
        self.persist(&entry).await?;

        // 2. 发布事件到内存流 (WAE 核心)
        let _ = self.sender.send(event.clone());

        Ok(event.id)
    }

    /// 订阅事件流
    pub fn subscribe(&self) -> broadcast::Receiver<WalEvent> {
        self.sender.subscribe()
    }

    async fn persist(&self, entry: &WalEntry) -> DsResult<()> {
        // TODO: 实际写入到磁盘 Segment 文件
        // 这里模拟编码过程
        let mut buf = BytesMut::new();
        buf.put_u16(0x5741); // Magic "WA"
        buf.put_u8(1); // Version
        buf.put_u8(entry.header.op_type as u8);
        buf.put_u8(entry.header.sdr_level.0);
        buf.put_u8(0); // Flags (3B)
        buf.put_u8(0);
        buf.put_u8(0);
        buf.put_i64(entry.header.timestamp);
        buf.put_slice(entry.header.request_id.as_bytes());
        buf.put_slice(entry.header.tenant_id.as_bytes());
        buf.put_u32(entry.table.len() as u32);
        buf.put_slice(entry.table.as_bytes());
        buf.put_u32(entry.data.len() as u32);
        buf.put_slice(&entry.data);
        buf.put_u32(entry.header.checksum);

        debug!("WAL Persisted: {} bytes", buf.len());
        Ok(())
    }
}
