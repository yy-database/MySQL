#![warn(missing_docs)]

use async_trait::async_trait;
use std::path::Path;
use tracing::{debug, trace};
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::*;
use windows::Win32::System::Ioctl::*;
use yykv_hal::{BlockDevice, DeviceInfo, Hal};
use yykv_types::{DsError, DsResult, MediumType};

pub struct WindowsDirectIo {
    handle: HANDLE,
    info: DeviceInfo,
}

impl WindowsDirectIo {
    pub fn open<P: AsRef<Path>>(path: P, medium_type: MediumType) -> DsResult<Self> {
        let path_str = path.as_ref().to_string_lossy();
        debug!(
            "Opening Windows Direct IO device: {} (Medium: {:?})",
            path_str, medium_type
        );
        let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            let handle = windows::Win32::Storage::FileSystem::CreateFileW(
                windows::core::PCWSTR(wide_path.as_ptr()),
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH,
                None,
            )
            .map_err(|e| {
                DsError::io_raw(
                    std::io::Error::from_raw_os_error(e.code().0),
                    Some(path.as_ref().to_path_buf()),
                )
            })?;

            // Query sector size and total size
            let mut info = DeviceInfo {
                name: path_str.to_string(),
                medium_type,
                sector_size: 4096,
                total_sectors: 0,
            };

            let mut alignment = STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR::default();
            let mut bytes_returned = 0;
            let query = STORAGE_PROPERTY_QUERY {
                PropertyId: StorageAccessAlignmentProperty,
                QueryType: PropertyStandardQuery,
                AdditionalParameters: [0; 1],
            };

            if DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                Some(&query as *const _ as *const _),
                std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                Some(&mut alignment as *mut _ as *mut _),
                std::mem::size_of::<STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR>() as u32,
                Some(&mut bytes_returned),
                None,
            )
            .is_ok()
            {
                info.sector_size = alignment.BytesPerLogicalSector;
            }

            let mut file_size: i64 = 0;
            if GetFileSizeEx(handle, &mut file_size).is_ok() {
                info.total_sectors = (file_size as u64) / (info.sector_size as u64);
            }

            debug!(
                "Device opened successfully: {:?} sectors of {} bytes",
                info.total_sectors, info.sector_size
            );
            Ok(Self { handle, info })
        }
    }
}

#[async_trait]
impl BlockDevice for WindowsDirectIo {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    async fn read_at(&self, offset: u64, buf: &mut [u8]) -> DsResult<usize> {
        let handle = self.handle;
        trace!(
            "WindowsDirectIo read_at: offset={} len={}",
            offset,
            buf.len()
        );

        unsafe {
            let mut overlapped = OVERLAPPED::default();
            overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFFFFFF) as u32;
            overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;

            let mut bytes_read = 0;
            ReadFile(
                handle,
                Some(buf),
                Some(&mut bytes_read),
                Some(&mut overlapped),
            )
            .map_err(|e| {
                DsError::storage(format!("ReadFile failed at offset {}: {}", offset, e))
            })?;

            Ok(bytes_read as usize)
        }
    }

    async fn write_at(&self, offset: u64, buf: &[u8]) -> DsResult<usize> {
        let handle = self.handle;
        trace!(
            "WindowsDirectIo write_at: offset={} len={}",
            offset,
            buf.len()
        );

        unsafe {
            let mut overlapped = OVERLAPPED::default();
            overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFFFFFF) as u32;
            overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;

            let mut bytes_written = 0;
            WriteFile(
                handle,
                Some(buf),
                Some(&mut bytes_written),
                Some(&mut overlapped),
            )
            .map_err(|e| {
                DsError::storage(format!("WriteFile failed at offset {}: {}", offset, e))
            })?;

            Ok(bytes_written as usize)
        }
    }

    async fn flush(&self) -> DsResult<()> {
        let handle = self.handle;
        trace!("WindowsDirectIo flush");
        unsafe {
            FlushFileBuffers(handle).map_err(|e| {
                DsError::io_raw(std::io::Error::from_raw_os_error(e.code().0), None)
            })?;
        }
        Ok(())
    }
}

impl Drop for WindowsDirectIo {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

unsafe impl Send for WindowsDirectIo {}
unsafe impl Sync for WindowsDirectIo {}

pub struct WindowsHal;

impl WindowsHal {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsHal {
    fn default() -> Self {
        Self::new()
    }
}

impl Hal for WindowsHal {
    fn list_devices(&self) -> Vec<DeviceInfo> {
        // Mocked for now
        vec![]
    }

    fn open_device(&self, name: &str) -> DsResult<Box<dyn BlockDevice>> {
        let medium = self.detect_medium(name)?;
        let device = WindowsDirectIo::open(name, medium)?;
        Ok(Box::new(device))
    }

    fn detect_medium(&self, _path: &str) -> DsResult<MediumType> {
        // Mocked: assume SSD for prototype
        Ok(MediumType::NvmeSSD)
    }
}
