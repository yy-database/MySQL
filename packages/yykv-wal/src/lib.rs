#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tracing::{debug, info, warn};
use yykv_types::{DsValue, DsResult, Redundancy, DsError};
use bytes::{BytesMut, Buf, BufMut, Bytes};
use yykv_types::layout::{DsValueEncoder, YYValueDecoder};

#[derive(Debug, Serialize, Deserialize)]
pub enum WalEntry {
    Put {
        tenant_id: u64,
        key: String,
        value: DsValue,
        sdr: Redundancy,
    },
    Delete {
        tenant_id: u64,
        key: String,
    },
}

impl WalEntry {
    pub fn encode(&self) -> DsResult<Bytes> {
        let mut buf = BytesMut::new();
        match self {
            WalEntry::Put { tenant_id, key, value, sdr } => {
                buf.put_u8(1); // Type: Put
                buf.put_u64(*tenant_id);
                let key_bytes = key.as_bytes();
                buf.put_u32(key_bytes.len() as u32);
                buf.put_slice(key_bytes);
                DsValueEncoder::encode_to_buf(value, &mut buf)?;
                buf.put_u8(sdr.0);
            }
            WalEntry::Delete { tenant_id, key } => {
                buf.put_u8(2); // Type: Delete
                buf.put_u64(*tenant_id);
                let key_bytes = key.as_bytes();
                buf.put_u32(key_bytes.len() as u32);
                buf.put_slice(key_bytes);
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
                let tenant_id = buf.get_u64();
                let key_len = buf.get_u32() as usize;
                if buf.len() < key_len {
                    return Err(DsError::serialization("Incomplete key"));
                }
                let key = String::from_utf8(buf.split_to(key_len).to_vec())
                    .map_err(|e| DsError::serialization(e.to_string()))?;
                let value = YYValueDecoder::decode_from_stream(&mut buf)?;
                let sdr = Redundancy::from_u8(buf.get_u8());
                Ok(WalEntry::Put { tenant_id, key, value, sdr })
            }
            2 => { // Delete
                let tenant_id = buf.get_u64();
                let key_len = buf.get_u32() as usize;
                if buf.len() < key_len {
                    return Err(DsError::serialization("Incomplete key"));
                }
                let key = String::from_utf8(buf.split_to(key_len).to_vec())
                    .map_err(|e| DsError::serialization(e.to_string()))?;
                Ok(WalEntry::Delete { tenant_id, key })
            }
            _ => Err(DsError::serialization(format!("Unknown WalEntry type: {}", type_id))),
        }
    }
}

pub struct WalManager {
    path: PathBuf,
    file: File,
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

        Ok(Self { path, file })
    }

    pub async fn append(&mut self, entry: WalEntry) -> DsResult<()> {
        debug!("Appending to WAL: {:?}", entry);
        let data = entry.encode()?;
        let len = data.len() as u32;

        // Write length prefix then data
        self.file.write_u32(len).await?;
        self.file.write_all(&data).await?;
        self.file.flush().await?;

        Ok(())
    }

    pub async fn replay(&mut self) -> DsResult<Vec<WalEntry>> {
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

    pub async fn checkpoint(&mut self) -> DsResult<()> {
        info!("Checkpointing WAL at {:?}", self.path);
        // Simple checkpoint: truncate the file
        self.file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .await?;
        Ok(())
    }
}
