use super::*;
use yykv_types::DsError;

type Result<T> = std::result::Result<T, DsError>;

impl NativeWriter {
    /// 执行 WAL Checkpoint，将 WAL 中的页面同步回主数据库
    pub async fn checkpoint(&self) -> Result<()> {
        let wal_path = format!("{}-wal", self.path);
        let mut wal_file = match File::open(&wal_path).await {
            Ok(f) => f,
            Err(_) => return Ok(()), // WAL 不存在，无需 checkpoint
        };

        let page_size = self.get_page_size().await?;
        let mut header = [0u8; 32];
        wal_file.read_exact(&mut header).await?;

        // 验证 Magic
        let magic = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        if magic != 0x377f0682 && magic != 0x377f0683 {
            return Err(DsError::query("Invalid WAL magic"));
        }

        let mut db_file = OpenOptions::new().write(true).open(&self.path).await?;
        let mut frame_header = [0u8; 24];
        let mut page_data = vec![0u8; page_size as usize];

        while wal_file.read_exact(&mut frame_header).await.is_ok() {
            let page_id = u32::from_be_bytes([
                frame_header[0],
                frame_header[1],
                frame_header[2],
                frame_header[3],
            ]);
            wal_file.read_exact(&mut page_data).await?;

            let offset = (page_id as u64 - 1) * page_size as u64;
            db_file.seek(SeekFrom::Start(offset)).await?;
            db_file.write_all(&page_data).await?;
        }

        // Checkpoint 完成后通常需要截断或删除 WAL
        drop(wal_file);
        tokio::fs::remove_file(&wal_path).await?;

        // 同时删除 WAL-index 文件（如果存在）
        let shm_path = format!("{}-shm", self.path);
        let _ = tokio::fs::remove_file(&shm_path).await;

        Ok(())
    }
}

/// WAL 写入器
pub struct WalWriter {
    wal_path: String,
    page_size: u32,
    salt1: u32,
    salt2: u32,
}

impl WalWriter {
    pub fn new(db_path: &str, page_size: u32) -> Self {
        Self {
            wal_path: format!("{}-wal", db_path),
            page_size,
            salt1: 0x12345678, // 实际应用中应随机生成
            salt2: 0x87654321,
        }
    }

    /// 写入 WAL Header
    async fn write_header(&self, file: &mut File) -> Result<()> {
        let mut header = [0u8; 32];
        header[0..4].copy_from_slice(&0x377f0682u32.to_be_bytes()); // Magic
        header[4..8].copy_from_slice(&3007000u32.to_be_bytes()); // Version
        header[8..12].copy_from_slice(&self.page_size.to_be_bytes());
        header[12..16].copy_from_slice(&0u32.to_be_bytes()); // Checkpoint seq
        header[16..20].copy_from_slice(&self.salt1.to_be_bytes());
        header[20..24].copy_from_slice(&self.salt2.to_be_bytes());
        // Checksum (暂时填充 0)
        header[24..28].copy_from_slice(&0u32.to_be_bytes());
        header[28..32].copy_from_slice(&0u32.to_be_bytes());

        file.seek(SeekFrom::Start(0)).await?;
        file.write_all(&header).await?;
        Ok(())
    }

    /// 写入一个 WAL 帧
    pub async fn write_frame(&self, page_id: u32, db_size: u32, data: &[u8]) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&self.wal_path)
            .await?;

        let metadata = file.metadata().await?;
        if metadata.len() == 0 {
            self.write_header(&mut file).await?;
        }

        file.seek(SeekFrom::End(0)).await?;

        let mut frame_header = [0u8; 24];
        frame_header[0..4].copy_from_slice(&page_id.to_be_bytes());
        frame_header[4..8].copy_from_slice(&db_size.to_be_bytes());
        frame_header[8..12].copy_from_slice(&self.salt1.to_be_bytes());
        frame_header[12..16].copy_from_slice(&self.salt2.to_be_bytes());
        // Checksum (暂时填充 0)
        frame_header[16..20].copy_from_slice(&0u32.to_be_bytes());
        frame_header[20..24].copy_from_slice(&0u32.to_be_bytes());

        file.write_all(&frame_header).await?;
        file.write_all(data).await?;
        Ok(())
    }
}
