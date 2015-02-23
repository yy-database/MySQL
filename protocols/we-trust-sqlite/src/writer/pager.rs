use super::*;
use yykv_types::DsError;

impl NativeWriter {
    /// 获取数据库页大小
    pub(super) async fn get_page_size(&self) -> Result<u32> {
        self.ensure_initialized().await?;
        let mut file = File::open(&self.path)
            .await
            .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
        let mut buf = [0u8; 2];
        file.seek(SeekFrom::Start(16))
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        file.read_exact(&mut buf)
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        let ps = u16::from_be_bytes(buf);
        Ok(if ps == 1 { 65536u32 } else { ps as u32 })
    }

    /// 分配一个新页面 (优先从 FreeList 获取)
    pub async fn allocate_page(&self) -> Result<u32> {
        self.ensure_initialized().await?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
        let page_size = self.get_page_size().await? as u64;

        // 读取 FreeList Trunk Page ID (bytes 32-35) 和总计数 (bytes 36-39)
        file.seek(SeekFrom::Start(32))
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        let mut freelist_info = [0u8; 8];
        file.read_exact(&mut freelist_info)
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;

        let trunk_id = u32::from_be_bytes([
            freelist_info[0],
            freelist_info[1],
            freelist_info[2],
            freelist_info[3],
        ]);
        let free_count = u32::from_be_bytes([
            freelist_info[4],
            freelist_info[5],
            freelist_info[6],
            freelist_info[7],
        ]);

        if trunk_id > 0 && free_count > 0 {
            let offset = (trunk_id as u64 - 1) * page_size;
            let mut trunk_header = [0u8; 8];
            file.seek(SeekFrom::Start(offset))
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            file.read_exact(&mut trunk_header)
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;

            let next_trunk = u32::from_be_bytes([
                trunk_header[0],
                trunk_header[1],
                trunk_header[2],
                trunk_header[3],
            ]);
            let leaf_count = u32::from_be_bytes([
                trunk_header[4],
                trunk_header[5],
                trunk_header[6],
                trunk_header[7],
            ]);

            let allocated_page_id;
            if leaf_count > 0 {
                // 从 Trunk 中取出一个叶子节点
                let leaf_idx_offset = 8 + (leaf_count as u64 - 1) * 4;
                file.seek(SeekFrom::Start(offset + leaf_idx_offset))
                    .await
                    .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
                let mut leaf_id_buf = [0u8; 4];
                file.read_exact(&mut leaf_id_buf)
                    .await
                    .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
                allocated_page_id = u32::from_be_bytes(leaf_id_buf);

                // 更新 leaf_count
                file.seek(SeekFrom::Start(offset + 4))
                    .await
                    .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
                file.write_all(&(leaf_count - 1).to_be_bytes())
                    .await
                    .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            } else {
                // 使用 Trunk Page 本身
                allocated_page_id = trunk_id;
                // 更新 Header 指向下一个 Trunk
                file.seek(SeekFrom::Start(32))
                    .await
                    .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
                file.write_all(&next_trunk.to_be_bytes())
                    .await
                    .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            }

            // 更新总 FreeList 计数
            file.seek(SeekFrom::Start(36))
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            file.write_all(&(free_count - 1).to_be_bytes())
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;

            return Ok(allocated_page_id);
        }

        // 没可用页面，追加到文件末尾
        let file_len = file
            .metadata()
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?
            .len();
        let new_page_id = (file_len / page_size) as u32 + 1;

        // 更新数据库大小 (bytes 28-31)
        file.seek(SeekFrom::Start(28))
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        file.write_all(&new_page_id.to_be_bytes())
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;

        // 填充新页面并确保文件增长
        file.seek(SeekFrom::Start(file_len))
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        file.write_all(&vec![0u8; page_size as usize])
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;

        Ok(new_page_id)
    }

    /// 将页面释放回 FreeList
    pub async fn deallocate_page(&self, page_id: u32) -> Result<()> {
        if page_id == 0 {
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
        let page_size = self.get_page_size().await?;

        // 读取 Header 中的 FreeList 信息 (trunk_id: 32-35, total_count: 36-39)
        file.seek(SeekFrom::Start(32))
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        let mut freelist_info = [0u8; 8];
        file.read_exact(&mut freelist_info)
            .await
            .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
        let trunk_id = u32::from_be_bytes([
            freelist_info[0],
            freelist_info[1],
            freelist_info[2],
            freelist_info[3],
        ]);
        let free_count = u32::from_be_bytes([
            freelist_info[4],
            freelist_info[5],
            freelist_info[6],
            freelist_info[7],
        ]);

        let mut added_as_trunk = false;
        if trunk_id > 0 {
            // 尝试放入第一个 Trunk Page
            let trunk_offset = (trunk_id as u64 - 1) * page_size as u64;
            file.seek(SeekFrom::Start(trunk_offset + 4))
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            let mut leaf_count_buf = [0u8; 4];
            file.read_exact(&mut leaf_count_buf)
                .await
                .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
            let leaf_count = u32::from_be_bytes(leaf_count_buf);

            let max_leaves = (page_size - 8) / 4;
            if leaf_count < max_leaves {
                // 有空间，作为叶子节点插入
                let leaf_pos = trunk_offset + 8 + (leaf_count as u64 * 4);
                file.seek(SeekFrom::Start(leaf_pos))
                    .await
                    .map_err(|e| DsError::io_raw(e, Some(self.path.clone().into())))?;
                file.write_all(&page_id.to_be_bytes())
                    .await
                    .map_err(|e| DsError::io(e.to_string()))?;

                // 更新 leaf_count
                file.seek(SeekFrom::Start(trunk_offset + 4))
                    .await
                    .map_err(|e| DsError::io(e.to_string()))?;
                file.write_all(&(leaf_count + 1).to_be_bytes())
                    .await
                    .map_err(|e| DsError::io(e.to_string()))?;
            } else {
                // Trunk Page 已满，将释放的页面作为新的第一个 Trunk Page
                added_as_trunk = true;
            }
        } else {
            // 没有 Trunk Page，将释放的页面作为第一个 Trunk Page
            added_as_trunk = true;
        }

        if added_as_trunk {
            let offset = (page_id as u64 - 1) * page_size as u64;
            let mut trunk_header = [0u8; 8];
            // Next trunk ID = 旧的 trunk_id
            trunk_header[0..4].copy_from_slice(&trunk_id.to_be_bytes());
            // Leaf count = 0
            trunk_header[4..8].copy_from_slice(&0u32.to_be_bytes());

            file.seek(SeekFrom::Start(offset))
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
            file.write_all(&trunk_header)
                .await
                .map_err(|e| DsError::io(e.to_string()))?;

            // 更新 Header 指向新的 Trunk Page
            file.seek(SeekFrom::Start(32))
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
            file.write_all(&page_id.to_be_bytes())
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
        }

        // 更新总 FreeList 计数
        file.seek(SeekFrom::Start(36))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&(free_count + 1).to_be_bytes())
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        Ok(())
    }

    /// 初始化一个新的“伪 SQLite”文件头
    pub async fn init_file(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;

        let mut header = vec![0u8; 100];
        header[0..16].copy_from_slice(crate::reader::SQLITE_MAGIC);

        // Page Size: 4096 (0x1000)
        header[16] = 0x10;
        header[17] = 0x00;

        // Write/Read version
        header[18] = 1;
        header[19] = 1;

        file.write_all(&header)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        Ok(())
    }

    /// 写入一个完整页面内容
    pub async fn write_page(&self, page_id: u32, data: &[u8]) -> Result<()> {
        let page_size = self.get_page_size().await?;
        if data.len() != page_size as usize {
            return Err(DsError::storage(format!(
                "Invalid page data size: expected {}, got {}",
                page_size,
                data.len()
            )));
        }

        let mut file = OpenOptions::new()
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
        let offset = (page_id as u64 - 1) * (page_size as u64);
        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        Ok(())
    }
}
