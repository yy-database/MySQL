#![warn(missing_docs)]

use async_trait::async_trait;
use tracing::trace;
use yykv_types::{DsError, DsResult, MediumType};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub medium_type: MediumType,
    pub sector_size: u32,
    pub total_sectors: u64,
}

#[async_trait]
pub trait BlockDevice: Send + Sync {
    fn info(&self) -> &DeviceInfo;

    async fn read_at(&self, offset: u64, buf: &mut [u8]) -> DsResult<usize>;
    async fn write_at(&self, offset: u64, buf: &[u8]) -> DsResult<usize>;
    async fn flush(&self) -> DsResult<()>;
}

pub trait Hal: Send + Sync {
    fn list_devices(&self) -> Vec<DeviceInfo>;
    fn open_device(&self, name: &str) -> DsResult<Box<dyn BlockDevice>>;
    fn detect_medium(&self, path: &str) -> DsResult<MediumType>;
}

pub struct MemDevice {
    info: DeviceInfo,
    data: std::sync::Arc<std::sync::RwLock<Vec<u8>>>,
}

impl MemDevice {
    pub fn new(name: String, medium: MediumType, size: usize) -> Self {
        Self {
            info: DeviceInfo {
                name,
                medium_type: medium,
                sector_size: 4096,
                total_sectors: (size / 4096) as u64,
            },
            data: std::sync::Arc::new(std::sync::RwLock::new(vec![0u8; size])),
        }
    }
}

#[async_trait]
impl BlockDevice for MemDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    async fn read_at(&self, offset: u64, buf: &mut [u8]) -> DsResult<usize> {
        trace!(
            "MemDevice {} read_at offset={} len={}",
            self.info.name,
            offset,
            buf.len()
        );
        let data = self
            .data
            .read()
            .map_err(|e| DsError::internal(e.to_string()))?;
        let start = offset as usize;
        let end = start + buf.len();
        if end > data.len() {
            return Err(DsError::storage("Read out of bounds"));
        }
        buf.copy_from_slice(&data[start..end]);
        Ok(buf.len())
    }

    async fn write_at(&self, offset: u64, buf: &[u8]) -> DsResult<usize> {
        trace!(
            "MemDevice {} write_at offset={} len={}",
            self.info.name,
            offset,
            buf.len()
        );
        let mut data = self
            .data
            .write()
            .map_err(|e| DsError::internal(e.to_string()))?;
        let start = offset as usize;
        let end = start + buf.len();
        if end > data.len() {
            return Err(DsError::storage("Write out of bounds"));
        }
        data[start..end].copy_from_slice(buf);
        Ok(buf.len())
    }

    async fn flush(&self) -> DsResult<()> {
        trace!("MemDevice {} flush", self.info.name);
        Ok(())
    }
}
