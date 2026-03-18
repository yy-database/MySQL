use std::sync::Arc;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, SeekFrom},
    sync::RwLock,
};
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

pub mod btree;
pub mod cache;
pub mod schema;
pub mod wal;

pub use btree::{BTreeCell, BTreePage, BTreePageHeader};
pub use cache::PageCache;
pub use schema::TableSchema;
pub use wal::{WalFrameHeader, WalReader};

#[derive(Debug, Clone)]
pub struct Row {
    pub row_id: i64,
    pub values: Vec<DsValue>,
}

/// SQLite 文件头魔数
pub const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// SQLite 文件头信息
#[derive(Debug, Clone, Copy)]
pub struct SqliteHeader {
    pub page_size: u32,
    pub write_version: u8,
    pub read_version: u8,
    pub reserved_space: u8,
}

/// 原生 SQLite 读取器
pub struct NativeReader {
    pub(crate) path: String,
    header: RwLock<Option<SqliteHeader>>,
    cache: Arc<PageCache>,
    wal_reader: RwLock<Option<Arc<WalReader>>>,
}

impl NativeReader {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            header: RwLock::new(None),
            cache: Arc::new(PageCache::new(100)),
            wal_reader: RwLock::new(None),
        }
    }

    /// 获取或初始化 WalReader
    async fn get_wal_reader(&self) -> Result<Arc<WalReader>> {
        {
            let wal = self.wal_reader.read().await;
            if let Some(w) = &*wal {
                return Ok(w.clone());
            }
        }

        let header = self.get_header().await?;
        let wal = Arc::new(WalReader::new(&self.path, header.page_size));
        let mut wal_slot = self.wal_reader.write().await;
        *wal_slot = Some(wal.clone());
        Ok(wal)
    }

    /// 获取数据库中所有表的 Schema
    pub async fn get_schemas(&self) -> Result<Vec<TableSchema>> {
        let root_page = self.get_btree_page(1).await?;
        let rows = root_page.get_rows(self).await?;

        let mut schemas = Vec::new();
        for row in rows {
            if row.values.len() >= 5 {
                let table_type = match &row.values[0] {
                    DsValue::Text(s) => s.clone(),
                    _ => continue,
                };
                let name = match &row.values[1] {
                    DsValue::Text(s) => s.clone(),
                    _ => continue,
                };
                let tbl_name = match &row.values[2] {
                    DsValue::Text(s) => s.clone(),
                    _ => continue,
                };
                let rootpage = match &row.values[3] {
                    DsValue::Int(i) => *i as u32,
                    _ => continue,
                };
                let sql = match &row.values[4] {
                    DsValue::Text(s) => s.clone(),
                    _ => continue,
                };

                schemas.push(TableSchema {
                    table_type,
                    name,
                    tbl_name,
                    rootpage,
                    sql,
                });
            }
        }
        Ok(schemas)
    }

    /// 执行全表扫描
    pub async fn scan_table(&self, root_page_id: u32) -> Result<Vec<Row>> {
        let root_page = self.get_btree_page(root_page_id).await?;
        root_page.get_rows(self).await
    }

    /// 根据表名执行全表扫描
    pub async fn scan_table_by_name(&self, table_name: &str) -> Result<Vec<Row>> {
        let schemas = self.get_schemas().await?;
        let schema = schemas
            .iter()
            .find(|s| s.name == table_name)
            .ok_or_else(|| DsError::query(format!("Table not found: {}", table_name)))?;
        self.scan_table(schema.rootpage).await
    }

    /// 获取 B-Tree 页面
    pub async fn get_btree_page(&self, page_id: u32) -> Result<BTreePage> {
        if let Some(page) = self.cache.get(page_id).await {
            return Ok(page);
        }

        let wal_reader = self.get_wal_reader().await?;
        let data = if let Some(wal_data) = wal_reader.read_page(page_id).await? {
            wal_data
        } else {
            let header = self.get_header().await?;
            let mut file = File::open(&self.path)
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            let offset = (page_id as u64 - 1) * header.page_size as u64;
            let mut buf = vec![0u8; header.page_size as usize];
            file.seek(SeekFrom::Start(offset))
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            file.read_exact(&mut buf)
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            buf
        };

        let page =
            BTreePage::decode(page_id, &data).map_err(|e| DsError::serialization(e.to_string()))?;
        self.cache.insert(page_id, page.clone()).await;
        Ok(page)
    }

    /// 验证文件头
    pub async fn verify_header(&self) -> Result<()> {
        self.get_header().await?;
        Ok(())
    }

    /// 清除页面缓存
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }

    /// 获取 SQLite 文件头信息
    pub async fn get_header(&self) -> Result<SqliteHeader> {
        {
            let header = self.header.read().await;
            if let Some(h) = *header {
                return Ok(h);
            }
        }

        let mut file = File::open(&self.path)
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        let mut magic = [0u8; 16];
        file.read_exact(&mut magic)
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;

        if &magic != SQLITE_MAGIC {
            return Err(DsError::io("Invalid SQLite magic header"));
        }

        let mut page_size_buf = [0u8; 2];
        file.seek(SeekFrom::Start(16))
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        file.read_exact(&mut page_size_buf)
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        let page_size = u16::from_be_bytes(page_size_buf);
        let page_size = if page_size == 1 {
            65536
        } else {
            page_size as u32
        };

        let mut version_buf = [0u8; 2];
        file.seek(SeekFrom::Start(18))
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        file.read_exact(&mut version_buf)
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;

        let mut reserved_buf = [0u8; 1];
        file.seek(SeekFrom::Start(20))
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        file.read_exact(&mut reserved_buf)
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;

        let h = SqliteHeader {
            page_size,
            write_version: version_buf[0],
            read_version: version_buf[1],
            reserved_space: reserved_buf[0],
        };

        let mut header_slot = self.header.write().await;
        *header_slot = Some(h);
        Ok(h)
    }
}
