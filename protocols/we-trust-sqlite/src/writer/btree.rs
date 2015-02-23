use super::*;
use crate::utils::{encode_record, encode_varint, parse_varint};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt, SeekFrom};
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

impl NativeWriter {
    // --- Table B-Tree Write Logic ---

    /// 初始化一个新的叶子表页面
    pub async fn init_leaf_page(&self, page_id: u32) -> Result<()> {
        self.ensure_initialized().await?;
        let mut file = OpenOptions::new()
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        let page_size = self.get_page_size().await?;
        let offset = (page_id as u64 - 1) * page_size as u64;

        let mut page_data = vec![0u8; page_size as usize];

        // Page Header
        // Byte 0: Page Type (0x0D for Leaf Table)
        page_data[0] = 0x0D;
        // Bytes 1-2: First freeblock (0)
        // Bytes 3-4: Number of cells (0)
        // Bytes 5-6: Start of cell content area (page_size)
        // Bytes 7: Fragmented free bytes (0)

        let header_offset = if page_id == 1 { 100 } else { 0 };
        page_data[header_offset] = 0x0D;

        let content_start = page_size as u16;
        page_data[header_offset + 5..header_offset + 7]
            .copy_from_slice(&content_start.to_be_bytes());

        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        Ok(())
    }

    /// 在叶子表页面插入一行数据
    pub async fn insert_into_leaf_page(
        &self,
        page_id: u32,
        row_id: i64,
        values: &[DsValue],
        path: Vec<u32>,
    ) -> Result<()> {
        self.ensure_initialized().await?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
        let page_size = self.get_page_size().await?;
        let offset = (page_id as u64 - 1) * (page_size as u64);

        let mut page_data = vec![0u8; page_size as usize];
        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.read_exact(&mut page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        let record = encode_record(values);
        let payload_size = record.len() as u64;

        let (stored_len, has_overflow) =
            self.get_stored_payload_info(0x0D, payload_size, page_size);
        let mut cell_data = Vec::new();
        cell_data.extend(encode_varint(payload_size));
        cell_data.extend(encode_varint(row_id as u64));
        cell_data.extend_from_slice(&record[..stored_len]);

        if has_overflow {
            let overflow_page_id = self.write_overflow_pages(&record[stored_len..]).await?;
            cell_data.extend_from_slice(&overflow_page_id.to_be_bytes());
        }

        let header_offset = if page_id == 1 { 100 } else { 0 };
        let cell_count =
            u16::from_be_bytes([page_data[header_offset + 3], page_data[header_offset + 4]]);
        let cell_content_offset =
            u16::from_be_bytes([page_data[header_offset + 5], page_data[header_offset + 6]]);
        let cell_content_offset = if cell_content_offset == 0 {
            page_size as u16
        } else {
            cell_content_offset
        };

        let pointer_array_end = header_offset + 8 + (cell_count as usize * 2);
        let free_space = cell_content_offset as usize - pointer_array_end;

        if cell_data.len() + 2 > free_space {
            return Box::pin(self.split_table_leaf_page(page_id, row_id, values, path)).await;
        }

        let new_cell_offset = cell_content_offset - cell_data.len() as u16;
        page_data[new_cell_offset as usize..new_cell_offset as usize + cell_data.len()]
            .copy_from_slice(&cell_data);

        let mut pointer_array = Vec::new();
        for i in 0..cell_count {
            let off = header_offset + 8 + (i as usize * 2);
            pointer_array.push(u16::from_be_bytes([page_data[off], page_data[off + 1]]));
        }

        let mut insert_pos = cell_count as usize;
        let mut is_update = false;
        for i in 0..cell_count {
            let cell_offset = pointer_array[i as usize] as usize;
            let (_p_size, consumed) = parse_varint(&page_data[cell_offset..]);
            let (existing_row_id, _) = parse_varint(&page_data[cell_offset + consumed..]);
            if (existing_row_id as i64) == row_id {
                insert_pos = i as usize;
                is_update = true;
                break;
            } else if (existing_row_id as i64) > row_id {
                insert_pos = i as usize;
                break;
            }
        }

        if is_update {
            // Simple update: just replace the pointer in the array.
            // Note: This leaves the old cell as "fragmented" or "free" space.
            // SQLite would normally add it to the freeblock list.
            pointer_array[insert_pos] = new_cell_offset;
        } else {
            pointer_array.insert(insert_pos, new_cell_offset);
        }

        let final_cell_count = pointer_array.len() as u16;
        page_data[header_offset + 3..header_offset + 5]
            .copy_from_slice(&final_cell_count.to_be_bytes());
        page_data[header_offset + 5..header_offset + 7]
            .copy_from_slice(&new_cell_offset.to_be_bytes());

        for (i, &ptr) in pointer_array.iter().enumerate() {
            let off = header_offset + 8 + (i * 2);
            page_data[off..off + 2].copy_from_slice(&ptr.to_be_bytes());
        }

        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        Ok(())
    }

    /// 从叶子表页面删除一行数据
    pub async fn delete_from_leaf_page(&self, page_id: u32, row_id: i64) -> Result<bool> {
        self.ensure_initialized().await?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
        let page_size = self.get_page_size().await?;
        let offset = (page_id as u64 - 1) * (page_size as u64);

        let mut page_data = vec![0u8; page_size as usize];
        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.read_exact(&mut page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        let header_offset = if page_id == 1 { 100 } else { 0 };
        let cell_count =
            u16::from_be_bytes([page_data[header_offset + 3], page_data[header_offset + 4]]);

        let mut pointer_array = Vec::new();
        let mut target_idx = None;
        for i in 0..cell_count {
            let off = header_offset + 8 + (i as usize * 2);
            let cell_ptr = u16::from_be_bytes([page_data[off], page_data[off + 1]]);

            let (_p_size, consumed) = parse_varint(&page_data[cell_ptr as usize..]);
            let (existing_row_id, _) = parse_varint(&page_data[cell_ptr as usize + consumed..]);

            if (existing_row_id as i64) == row_id {
                target_idx = Some(i as usize);
            } else {
                pointer_array.push(cell_ptr);
            }
        }

        if target_idx.is_some() {
            let new_cell_count = pointer_array.len() as u16;
            page_data[header_offset + 3..header_offset + 5]
                .copy_from_slice(&new_cell_count.to_be_bytes());

            // Clear old pointer array
            for i in 0..cell_count {
                let off = header_offset + 8 + (i as usize * 2);
                page_data[off..off + 2].copy_from_slice(&[0, 0]);
            }

            // Write new pointer array
            for (i, &ptr) in pointer_array.iter().enumerate() {
                let off = header_offset + 8 + (i * 2);
                page_data[off..off + 2].copy_from_slice(&ptr.to_be_bytes());
            }

            file.seek(SeekFrom::Start(offset))
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
            file.write_all(&page_data)
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 拆分表叶子页面
    async fn split_table_leaf_page(
        &self,
        page_id: u32,
        row_id: i64,
        values: &[DsValue],
        mut path: Vec<u32>,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        let page_size = self.get_page_size().await?;
        let offset = (page_id as u64 - 1) * page_size as u64;

        let mut old_page_data = vec![0u8; page_size as usize];
        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.read_exact(&mut old_page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        let header_offset = if page_id == 1 { 100 } else { 0 };
        let cell_count = u16::from_be_bytes([
            old_page_data[header_offset + 3],
            old_page_data[header_offset + 4],
        ]);

        let mut all_cells = Vec::new();
        for i in 0..cell_count {
            let off = u16::from_be_bytes([
                old_page_data[header_offset + 8 + i as usize * 2],
                old_page_data[header_offset + 9 + i as usize * 2],
            ]) as usize;
            let (payload_size, consumed) = parse_varint(&old_page_data[off..]);
            let (rid, consumed2) = parse_varint(&old_page_data[off + consumed..]);
            let (stored_len, has_overflow) =
                self.get_stored_payload_info(0x0D, payload_size, page_size);
            let total_size = consumed + consumed2 + stored_len + if has_overflow { 4 } else { 0 };
            all_cells.push((rid as i64, old_page_data[off..off + total_size].to_vec()));
        }

        let record = encode_record(values);
        let payload_size = record.len() as u64;
        let (stored_len, has_overflow) =
            self.get_stored_payload_info(0x0D, payload_size, page_size);
        let mut new_cell_data = Vec::new();
        new_cell_data.extend(encode_varint(payload_size));
        new_cell_data.extend(encode_varint(row_id as u64));
        new_cell_data.extend_from_slice(&record[..stored_len]);
        if has_overflow {
            let overflow_page_id = self.write_overflow_pages(&record[stored_len..]).await?;
            new_cell_data.extend_from_slice(&overflow_page_id.to_be_bytes());
        }
        all_cells.push((row_id, new_cell_data));
        all_cells.sort_by_key(|c| c.0);

        let new_page_id = self.allocate_page().await?;
        let mut new_page_data = vec![0u8; page_size as usize];
        new_page_data[0] = 0x0D;

        let mid_idx = all_cells.len() / 2;
        let mut updated_old_page_data = vec![0u8; page_size as usize];
        updated_old_page_data[..header_offset].copy_from_slice(&old_page_data[..header_offset]);
        updated_old_page_data[header_offset] = 0x0D;
        let mut old_cell_content_offset = page_size as u16;
        let mut old_pointer_array = Vec::new();
        for cell in all_cells.iter().take(mid_idx) {
            let cell_data = &cell.1;
            old_cell_content_offset -= cell_data.len() as u16;
            updated_old_page_data[old_cell_content_offset as usize
                ..old_cell_content_offset as usize + cell_data.len()]
                .copy_from_slice(cell_data);
            old_pointer_array.push(old_cell_content_offset);
        }
        updated_old_page_data[header_offset + 3..header_offset + 5]
            .copy_from_slice(&(old_pointer_array.len() as u16).to_be_bytes());
        updated_old_page_data[header_offset + 5..header_offset + 7]
            .copy_from_slice(&old_cell_content_offset.to_be_bytes());
        for (i, &ptr) in old_pointer_array.iter().enumerate() {
            updated_old_page_data[header_offset + 8 + i * 2..header_offset + 10 + i * 2]
                .copy_from_slice(&ptr.to_be_bytes());
        }

        let mut new_cell_content_offset = page_size as u16;
        let mut new_pointer_array = Vec::new();
        for cell in all_cells.iter().skip(mid_idx) {
            let cell_data = &cell.1;
            new_cell_content_offset -= cell_data.len() as u16;
            new_page_data[new_cell_content_offset as usize
                ..new_cell_content_offset as usize + cell_data.len()]
                .copy_from_slice(cell_data);
            new_pointer_array.push(new_cell_content_offset);
        }
        new_page_data[3..5].copy_from_slice(&(new_pointer_array.len() as u16).to_be_bytes());
        new_page_data[5..7].copy_from_slice(&new_cell_content_offset.to_be_bytes());
        for (i, &ptr) in new_pointer_array.iter().enumerate() {
            new_page_data[8 + i * 2..10 + i * 2].copy_from_slice(&ptr.to_be_bytes());
        }

        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&updated_old_page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        let new_page_offset = (new_page_id as u64 - 1) * page_size as u64;
        file.seek(SeekFrom::Start(new_page_offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&new_page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        let separator_key = all_cells[mid_idx].0;
        path.pop();
        if let Some(parent_id) = path.last() {
            Box::pin(self.insert_into_table_interior_page(
                *parent_id,
                page_id,
                separator_key,
                path,
            ))
            .await?;
        } else {
            self.create_new_table_root(page_id, new_page_id, separator_key)
                .await?;
        }
        Ok(())
    }

    /// 插入到表内部页面
    async fn insert_into_table_interior_page(
        &self,
        page_id: u32,
        left_child: u32,
        key: i64,
        path: Vec<u32>,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        let page_size = self.get_page_size().await?;
        let offset = (page_id as u64 - 1) * page_size as u64;

        let mut page_data = vec![0u8; page_size as usize];
        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.read_exact(&mut page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        let header_offset = if page_id == 1 { 100 } else { 0 };
        let cell_count =
            u16::from_be_bytes([page_data[header_offset + 3], page_data[header_offset + 4]]);
        let cell_content_offset =
            u16::from_be_bytes([page_data[header_offset + 5], page_data[header_offset + 6]]);
        let cell_content_offset = if cell_content_offset == 0 {
            page_size as u16
        } else {
            cell_content_offset
        };

        let mut cell_data = Vec::new();
        cell_data.extend_from_slice(&left_child.to_be_bytes());
        cell_data.extend(encode_varint(key as u64));

        let pointer_array_end = header_offset + 12 + (cell_count as usize * 2);
        let free_space = cell_content_offset as usize - pointer_array_end;

        if cell_data.len() + 2 > free_space {
            return Box::pin(self.split_table_interior_page(page_id, left_child, key, path)).await;
        }

        let new_cell_offset = cell_content_offset - cell_data.len() as u16;
        page_data[new_cell_offset as usize..new_cell_offset as usize + cell_data.len()]
            .copy_from_slice(&cell_data);

        let mut pointer_array = Vec::new();
        for i in 0..cell_count {
            let off = header_offset + 12 + (i as usize * 2);
            pointer_array.push(u16::from_be_bytes([page_data[off], page_data[off + 1]]));
        }

        let mut insert_pos = cell_count as usize;
        for i in 0..cell_count {
            let off = pointer_array[i as usize] as usize;
            let (existing_key, _) = parse_varint(&page_data[off + 4..]);
            if existing_key as i64 >= key {
                insert_pos = i as usize;
                break;
            }
        }
        pointer_array.insert(insert_pos, new_cell_offset);

        for (i, &ptr) in pointer_array.iter().enumerate() {
            let off = header_offset + 12 + (i * 2);
            page_data[off..off + 2].copy_from_slice(&ptr.to_be_bytes());
        }

        let new_cell_count = cell_count + 1;
        page_data[header_offset + 3..header_offset + 5]
            .copy_from_slice(&new_cell_count.to_be_bytes());
        page_data[header_offset + 5..header_offset + 7]
            .copy_from_slice(&new_cell_offset.to_be_bytes());

        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        Ok(())
    }

    /// 表内部页面拆分
    async fn split_table_interior_page(
        &self,
        page_id: u32,
        new_left_child: u32,
        new_key: i64,
        mut path: Vec<u32>,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        let page_size = self.get_page_size().await?;
        let offset = (page_id as u64 - 1) * page_size as u64;

        let mut old_page_data = vec![0u8; page_size as usize];
        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.read_exact(&mut old_page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        let header_offset = if page_id == 1 { 100 } else { 0 };
        let cell_count = u16::from_be_bytes([
            old_page_data[header_offset + 3],
            old_page_data[header_offset + 4],
        ]);
        let right_most_ptr = u32::from_be_bytes([
            old_page_data[header_offset + 8],
            old_page_data[header_offset + 9],
            old_page_data[header_offset + 10],
            old_page_data[header_offset + 11],
        ]);

        let mut all_cells = Vec::new();
        for i in 0..cell_count {
            let off = u16::from_be_bytes([
                old_page_data[header_offset + 12 + i as usize * 2],
                old_page_data[header_offset + 13 + i as usize * 2],
            ]) as usize;
            let (k, consumed) = parse_varint(&old_page_data[off + 4..]);
            all_cells.push((k as i64, old_page_data[off..off + 4 + consumed].to_vec()));
        }

        let mut new_cell_data = Vec::new();
        new_cell_data.extend_from_slice(&new_left_child.to_be_bytes());
        new_cell_data.extend(encode_varint(new_key as u64));
        all_cells.push((new_key, new_cell_data));
        all_cells.sort_by_key(|c| c.0);

        let mid_idx = all_cells.len() / 2;
        let separator_key = all_cells[mid_idx].0;
        let new_right_ptr = u32::from_be_bytes([
            all_cells[mid_idx].1[0],
            all_cells[mid_idx].1[1],
            all_cells[mid_idx].1[2],
            all_cells[mid_idx].1[3],
        ]);

        let mut updated_old_page_data = vec![0u8; page_size as usize];
        updated_old_page_data[..header_offset].copy_from_slice(&old_page_data[..header_offset]);
        updated_old_page_data[header_offset] = 0x05;
        updated_old_page_data[header_offset + 8..header_offset + 12]
            .copy_from_slice(&new_right_ptr.to_be_bytes());
        let mut old_cell_content_offset = page_size as u16;
        let mut old_pointer_array = Vec::new();
        for cell in all_cells.iter().take(mid_idx) {
            let cell_data = &cell.1;
            old_cell_content_offset -= cell_data.len() as u16;
            updated_old_page_data[old_cell_content_offset as usize
                ..old_cell_content_offset as usize + cell_data.len()]
                .copy_from_slice(cell_data);
            old_pointer_array.push(old_cell_content_offset);
        }
        updated_old_page_data[header_offset + 3..header_offset + 5]
            .copy_from_slice(&(old_pointer_array.len() as u16).to_be_bytes());
        updated_old_page_data[header_offset + 5..header_offset + 7]
            .copy_from_slice(&old_cell_content_offset.to_be_bytes());
        for (i, &ptr) in old_pointer_array.iter().enumerate() {
            updated_old_page_data[header_offset + 12 + i * 2..header_offset + 14 + i * 2]
                .copy_from_slice(&ptr.to_be_bytes());
        }

        let new_page_id = self.allocate_page().await?;
        let mut new_page_data = vec![0u8; page_size as usize];
        new_page_data[0] = 0x05;
        new_page_data[8..12].copy_from_slice(&right_most_ptr.to_be_bytes());
        let mut new_cell_content_offset = page_size as u16;
        let mut new_pointer_array = Vec::new();
        for cell in all_cells.iter().skip(mid_idx + 1) {
            let cell_data = &cell.1;
            new_cell_content_offset -= cell_data.len() as u16;
            new_page_data[new_cell_content_offset as usize
                ..new_cell_content_offset as usize + cell_data.len()]
                .copy_from_slice(cell_data);
            new_pointer_array.push(new_cell_content_offset);
        }
        new_page_data[3..5].copy_from_slice(&(new_pointer_array.len() as u16).to_be_bytes());
        new_page_data[5..7].copy_from_slice(&new_cell_content_offset.to_be_bytes());
        for (i, &ptr) in new_pointer_array.iter().enumerate() {
            new_page_data[12 + i * 2..14 + i * 2].copy_from_slice(&ptr.to_be_bytes());
        }

        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&updated_old_page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        let new_page_offset = (new_page_id as u64 - 1) * page_size as u64;
        file.seek(SeekFrom::Start(new_page_offset))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&new_page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        path.pop();
        if let Some(parent_id) = path.last() {
            Box::pin(self.insert_into_table_interior_page(
                *parent_id,
                page_id,
                separator_key,
                path,
            ))
            .await?;
        } else {
            self.create_new_table_root(page_id, new_page_id, separator_key)
                .await?;
        }
        Ok(())
    }

    /// 创建新的表根页面
    async fn create_new_table_root(
        &self,
        left_child: u32,
        right_child: u32,
        key: i64,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        let page_size = self.get_page_size().await?;

        let mut root_page_data = vec![0u8; page_size as usize];
        file.seek(SeekFrom::Start(0))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.read_exact(&mut root_page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;

        // Root becomes an interior table page (0x05)
        root_page_data[100] = 0x05;
        root_page_data[101] = 0; // freeblock
        root_page_data[102] = 0;
        root_page_data[103] = 0; // cell count
        root_page_data[104] = 1;

        let mut cell_data = Vec::new();
        cell_data.extend_from_slice(&left_child.to_be_bytes());
        cell_data.extend(encode_varint(key as u64));

        let cell_offset = page_size as u16 - cell_data.len() as u16;
        root_page_data[cell_offset as usize..cell_offset as usize + cell_data.len()]
            .copy_from_slice(&cell_data);
        root_page_data[105..107].copy_from_slice(&cell_offset.to_be_bytes());
        root_page_data[108..112].copy_from_slice(&right_child.to_be_bytes());
        root_page_data[112..114].copy_from_slice(&cell_offset.to_be_bytes());

        file.seek(SeekFrom::Start(0))
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        file.write_all(&root_page_data)
            .await
            .map_err(|e| DsError::io(e.to_string()))?;
        Ok(())
    }
}
