use super::*;
use yykv_types::DsError;

impl NativeWriter {
    /// 计算在页面上存储的 payload 长度
    pub(super) fn get_stored_payload_info(
        &self,
        page_type: u8,
        payload_size: u64,
        page_size: u32,
    ) -> (usize, bool) {
        let u = page_size;
        let (x, m) = match page_type {
            0x0D => (u - 35, ((u - 12) * 32) / 255 + 23), // Table Leaf
            0x0A | 0x02 => {
                // Index Leaf or Interior
                let x = ((u - 12) * 64) / 255 - 23;
                let m = ((u - 12) * 32) / 255 + 23;
                (x, m)
            }
            _ => return (payload_size as usize, false),
        };

        if payload_size <= x as u64 {
            (payload_size as usize, false)
        } else {
            let k = m + ((payload_size - m as u64) % (u as u64 - 4)) as u32;
            let stored_len = if k <= x { k } else { m };
            (stored_len as usize, true)
        }
    }

    /// 处理溢出页写入，返回第一个溢出页 ID
    pub(super) async fn write_overflow_pages(&self, data: &[u8]) -> Result<u32> {
        let page_size = self.get_page_size().await?;
        let usable_size = page_size;
        let overflow_payload_limit = usable_size - 4;

        let mut first_page_id = 0;
        let mut last_page_id = 0;
        let mut remaining_data = data;

        while !remaining_data.is_empty() {
            let new_page_id = self.allocate_page().await?;
            if first_page_id == 0 {
                first_page_id = new_page_id;
            }

            if last_page_id != 0 {
                // 更新上一个溢出页的 next 指针
                let mut file = OpenOptions::new()
                    .write(true)
                    .open(&self.path)
                    .await
                    .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
                let prev_offset = (last_page_id as u64 - 1) * page_size as u64;
                file.seek(SeekFrom::Start(prev_offset))
                    .await
                    .map_err(|e| DsError::io(e.to_string()))?;
                file.write_all(&new_page_id.to_be_bytes())
                    .await
                    .map_err(|e| DsError::io(e.to_string()))?;
            }

            let chunk_size = std::cmp::min(remaining_data.len(), overflow_payload_limit as usize);
            let mut page_data = vec![0u8; page_size as usize];
            // next page pointer (initially 0)
            page_data[0..4].copy_from_slice(&0u32.to_be_bytes());
            page_data[4..4 + chunk_size].copy_from_slice(&remaining_data[..chunk_size]);

            let mut file = OpenOptions::new()
                .write(true)
                .open(&self.path)
                .await
                .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
            let offset = (new_page_id as u64 - 1) * page_size as u64;
            file.seek(SeekFrom::Start(offset))
                .await
                .map_err(|e| DsError::io(e.to_string()))?;
            file.write_all(&page_data)
                .await
                .map_err(|e| DsError::io(e.to_string()))?;

            remaining_data = &remaining_data[chunk_size..];
            last_page_id = new_page_id;
        }

        Ok(first_page_id)
    }

    /// 读取完整的 payload (处理溢出页)
    pub async fn read_full_payload(
        &self,
        page_data: &[u8],
        cell_offset: usize,
        page_type: u8,
        page_size: u32,
    ) -> Result<Vec<u8>> {
        let (payload_size, consumed) = parse_varint(&page_data[cell_offset..]);
        let (stored_len, has_overflow) =
            self.get_stored_payload_info(page_type, payload_size, page_size);

        let mut payload =
            page_data[cell_offset + consumed..cell_offset + consumed + stored_len].to_vec();

        if has_overflow {
            let mut next_page_id = u32::from_be_bytes([
                page_data[cell_offset + consumed + stored_len],
                page_data[cell_offset + consumed + stored_len + 1],
                page_data[cell_offset + consumed + stored_len + 2],
                page_data[cell_offset + consumed + stored_len + 3],
            ]);

            let mut file = File::open(&self.path)
                .await
                .map_err(|e| DsError::io(format!("Failed to open database file: {}", e)))?;
            while next_page_id > 0 {
                let offset = (next_page_id as u64 - 1) * page_size as u64;
                file.seek(SeekFrom::Start(offset))
                    .await
                    .map_err(|e| DsError::io(e.to_string()))?;
                let mut ovfl_header = [0u8; 4];
                file.read_exact(&mut ovfl_header)
                    .await
                    .map_err(|e| DsError::io(e.to_string()))?;

                let next_ovfl_id = u32::from_be_bytes(ovfl_header);
                let remaining = payload_size as usize - payload.len();
                let chunk_size = std::cmp::min(remaining, (page_size - 4) as usize);

                let mut chunk = vec![0u8; chunk_size];
                file.read_exact(&mut chunk)
                    .await
                    .map_err(|e| DsError::io(e.to_string()))?;
                payload.extend(chunk);

                next_page_id = next_ovfl_id;
            }
        }

        Ok(payload)
    }
}
