use serde::{Deserialize, Serialize};
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tracing::{debug, info, warn};
use yykv_types::DsValue;
use yykv_types::{DsResult, Redundancy};

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
        let data = bincode::serialize(&entry)?;
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

            match bincode::deserialize::<WalEntry>(&buf) {
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
