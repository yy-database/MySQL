use yykv_types::DsError;
type Result<T> = std::result::Result<T, DsError>;
use crate::utils::parse_varint;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};

mod btree;
mod pager;
mod payload;
mod wal;

pub use wal::WalWriter;

/// 原生 SQLite 写入器
pub struct NativeWriter {
    path: String,
}

impl NativeWriter {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// 确保数据库文件已初始化
    pub async fn ensure_initialized(&self) -> Result<()> {
        if tokio::fs::metadata(&self.path).await.is_ok() {
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(format!("Failed to create database file: {}", e)))?;

        let page_size: u32 = 4096;
        let mut header = vec![0u8; page_size as usize];

        // 1. Magic Header String: "SQLite format 3\0"
        header[0..16].copy_from_slice(b"SQLite format 3\0");

        // 2. Page Size (bytes 16-17)
        header[16..18].copy_from_slice(&(page_size as u16).to_be_bytes());

        // 3. File format write version (1: legacy, 2: WAL)
        header[18] = 1;
        // 4. File format read version (1: legacy, 2: WAL)
        header[19] = 1;

        // 5. Reserved space at end of each page
        header[20] = 0;

        // 6. Max embedded payload fraction (64)
        header[21] = 64;
        // 7. Min embedded payload fraction (32)
        header[22] = 32;
        // 8. Leaf payload fraction (32)
        header[23] = 32;

        // 9. File change counter
        header[24..28].copy_from_slice(&1u32.to_be_bytes());

        // 10. Size of database in pages
        header[28..32].copy_from_slice(&1u32.to_be_bytes());

        // 11. First freelist trunk page
        header[32..36].copy_from_slice(&0u32.to_be_bytes());
        // 12. Total number of freelist pages
        header[36..40].copy_from_slice(&0u32.to_be_bytes());

        // 13. Schema cookie
        header[40..44].copy_from_slice(&1u32.to_be_bytes());
        // 14. Schema format number (4 is recommended)
        header[44..48].copy_from_slice(&4u32.to_be_bytes());

        // 15. Default page cache size
        header[48..52].copy_from_slice(&0u32.to_be_bytes());

        // 16. Largest root b-tree page number (0 for vacuum, else 0)
        header[52..56].copy_from_slice(&0u32.to_be_bytes());

        // 17. Text encoding (1: UTF-8)
        header[56..60].copy_from_slice(&1u32.to_be_bytes());

        // 18. User version
        header[60..64].copy_from_slice(&0u32.to_be_bytes());

        // 19. Incremental vacuum mode (0)
        header[64..68].copy_from_slice(&0u32.to_be_bytes());

        // 20. Application ID
        header[68..72].copy_from_slice(&0u32.to_be_bytes());

        // 21. Reserved for expansion
        // ... all zeros ...

        // 22. Version-valid-for number
        header[92..96].copy_from_slice(&1u32.to_be_bytes());

        // 23. SQLITE_VERSION_NUMBER
        header[96..100].copy_from_slice(&3034000u32.to_be_bytes());

        // Page 1 Header (Starts at 100)
        // 0x0D: Leaf Table Page
        header[100] = 0x0D;
        // Cell content start (at end of page)
        header[105] = (page_size >> 8) as u8;
        header[106] = (page_size & 0xFF) as u8;

        file.write_all(&header)
            .await
            .map_err(|e| DsError::io(format!("Failed to write database header: {}", e)))?;

        Ok(())
    }
}
