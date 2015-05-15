use yykv_types::DsError;
type Result<T> = std::result::Result<T, DsError>;
use parking_lot::Mutex;
use std::collections::HashMap;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, SeekFrom},
};

/// WAL 帧头部
#[derive(Debug)]
pub struct WalFrameHeader {
    pub page_id: u32,
    pub db_size: u32,
    pub salt1: u32,
    pub salt2: u32,
    pub checksum1: u32,
    pub checksum2: u32,
}

/// WAL 文件管理器
pub struct WalReader {
    wal_path: String,
    page_size: u32,
    index_cache: Mutex<HashMap<u32, u64>>, // page_id -> frame_offset
    last_indexed_offset: Mutex<u64>,       // 上次建立索引的文件偏移量
}

impl WalReader {
    pub fn new(db_path: &str, page_size: u32) -> Self {
        Self {
            wal_path: format!("{}-wal", db_path),
            page_size,
            index_cache: Mutex::new(HashMap::new()),
            last_indexed_offset: Mutex::new(32), // WAL Header 是 32 字节
        }
    }

    /// 增量更新 WAL 索引
    async fn update_index(&self, file: &mut File) -> Result<()> {
        let metadata = file
            .metadata()
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        let file_len = metadata.len();

        let start_offset = {
            let last_off = self.last_indexed_offset.lock();
            if file_len <= *last_off {
                return Ok(());
            }
            *last_off
        };

        let frame_size = 24 + self.page_size as u64;
        let mut current_offset = start_offset;

        while current_offset + frame_size <= file_len {
            file.seek(SeekFrom::Start(current_offset))
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
            let mut frame_header_buf = [0u8; 4]; // 只需要前 4 字节 page_id
            file.read_exact(&mut frame_header_buf)
                .await
                .map_err(|e| DsError::io(e.to_string()))?;

            let frame_page_id = u32::from_be_bytes(frame_header_buf);

            {
                let mut cache = self.index_cache.lock();
                cache.insert(frame_page_id, current_offset);
            }

            current_offset += frame_size;
        }

        {
            let mut last_off = self.last_indexed_offset.lock();
            if current_offset > *last_off {
                *last_off = current_offset;
            }
        }
        Ok(())
    }

    /// 尝试从 WAL 文件中查找最新的页面内容
    pub async fn read_page(&self, page_id: u32) -> Result<Option<Vec<u8>>> {
        let mut file = match File::open(&self.wal_path).await {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };

        // 增量扫描
        self.update_index(&mut file).await?;

        // 查缓存
        let offset = {
            let cache = self.index_cache.lock();
            cache.get(&page_id).cloned()
        };

        if let Some(off) = offset {
            file.seek(SeekFrom::Start(off + 24))
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
            let mut page_data = vec![0u8; self.page_size as usize];
            file.read_exact(&mut page_data)
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
            return Ok(Some(page_data));
        }

        Ok(None)
    }
}
