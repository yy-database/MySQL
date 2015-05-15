use crate::codec::Codec;
use crate::page::{PAGE_SIZE, Page, PageId};
use bytes::BytesMut;
use std::sync::Arc;
use tokio::sync::RwLock;
use yykv_types::{DsError, DsResult, DsValue};

pub const BTREE_HEADER_SIZE: usize = 10;
pub const CELL_PTR_SIZE: usize = 2;

pub struct BTreePage {
    pub page: Arc<RwLock<Page>>,
}

impl BTreePage {
    pub fn new(page: Arc<RwLock<Page>>) -> Self {
        Self { page }
    }

    pub async fn is_leaf(&self) -> bool {
        let p = self.page.read().await;
        p.data[1] != 0
    }

    pub async fn set_is_leaf(&self, is_leaf: bool) {
        let mut p = self.page.write().await;
        p.data[1] = if is_leaf { 1 } else { 0 };
        p.dirty = true;
    }

    pub async fn cell_count(&self) -> u16 {
        let p = self.page.read().await;
        u16::from_be_bytes([p.data[2], p.data[3]])
    }

    pub async fn set_cell_count(&self, count: u16) {
        let mut p = self.page.write().await;
        let bytes = count.to_be_bytes();
        p.data[2] = bytes[0];
        p.data[3] = bytes[1];
        p.dirty = true;
    }

    pub async fn free_offset(&self) -> u16 {
        let p = self.page.read().await;
        u16::from_be_bytes([p.data[4], p.data[5]])
    }

    pub async fn set_free_offset(&self, offset: u16) {
        let mut p = self.page.write().await;
        let bytes = offset.to_be_bytes();
        p.data[4] = bytes[0];
        p.data[5] = bytes[1];
        p.dirty = true;
    }

    pub async fn right_child(&self) -> PageId {
        let p = self.page.read().await;
        u32::from_be_bytes([p.data[6], p.data[7], p.data[8], p.data[9]])
    }

    pub async fn set_right_child(&self, child: PageId) {
        let mut p = self.page.write().await;
        let bytes = child.to_be_bytes();
        p.data[6] = bytes[0];
        p.data[7] = bytes[1];
        p.data[8] = bytes[2];
        p.data[9] = bytes[3];
        p.dirty = true;
    }

    pub async fn init(&self, is_leaf: bool) {
        self.set_is_leaf(is_leaf).await;
        self.set_cell_count(0).await;
        self.set_free_offset(PAGE_SIZE as u16).await;
        self.set_right_child(0).await;
    }

    pub async fn get_cell_ptr(&self, index: u16) -> u16 {
        let p = self.page.read().await;
        let offset = BTREE_HEADER_SIZE + (index as usize * CELL_PTR_SIZE);
        u16::from_be_bytes([p.data[offset], p.data[offset + 1]])
    }

    pub async fn set_cell_ptr(&self, index: u16, ptr: u16) {
        let mut p = self.page.write().await;
        let offset = BTREE_HEADER_SIZE + (index as usize * CELL_PTR_SIZE);
        let bytes = ptr.to_be_bytes();
        p.data[offset] = bytes[0];
        p.data[offset + 1] = bytes[1];
        p.dirty = true;
    }

    pub async fn get_cell_key(&self, index: u16) -> DsResult<DsValue> {
        let ptr = self.get_cell_ptr(index).await;
        let is_leaf = self.is_leaf().await;
        let p = self.page.read().await;
        let mut data = &p.data[ptr as usize..];
        if !is_leaf {
            data = &data[4..];
        }
        Codec::decode_value(&mut data)
    }

    pub async fn get_leaf_value(&self, index: u16) -> DsResult<DsValue> {
        let ptr = self.get_cell_ptr(index).await;
        let p = self.page.read().await;
        let mut data = &p.data[ptr as usize..];
        Codec::decode_value(&mut data)?;
        Codec::decode_value(&mut data)
    }

    pub async fn get_internal_child(&self, index: u16) -> DsResult<PageId> {
        if self.is_leaf().await {
            return Err(DsError::storage("Not an internal page"));
        }
        let ptr = self.get_cell_ptr(index).await;
        let p = self.page.read().await;
        let child_bytes = &p.data[ptr as usize..ptr as usize + 4];
        Ok(u32::from_be_bytes([
            child_bytes[0],
            child_bytes[1],
            child_bytes[2],
            child_bytes[3],
        ]))
    }

    pub async fn find_cell(&self, key: &DsValue) -> DsResult<u16> {
        let count = self.cell_count().await;
        let mut left = 0;
        let mut right = count;

        while left < right {
            let mid = left + (right - left) / 2;
            let mid_key = self.get_cell_key(mid).await?;
            if mid_key < *key {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        Ok(left)
    }

    pub async fn insert_leaf_cell(
        &self,
        index: u16,
        key: &DsValue,
        value: &DsValue,
    ) -> DsResult<()> {
        let mut key_buf = BytesMut::new();
        Codec::encode_value(key, &mut key_buf);
        let mut val_buf = BytesMut::new();
        Codec::encode_value(value, &mut val_buf);

        let cell_size = key_buf.len() + val_buf.len();
        let free_offset = self.free_offset().await;
        let new_ptr = free_offset - cell_size as u16;

        {
            let mut p = self.page.write().await;
            let count = u16::from_be_bytes([p.data[2], p.data[3]]);
            let ptr_start = BTREE_HEADER_SIZE + (index as usize * CELL_PTR_SIZE);
            let ptr_end = BTREE_HEADER_SIZE + (count as usize * CELL_PTR_SIZE);
            p.data
                .copy_within(ptr_start..ptr_end, ptr_start + CELL_PTR_SIZE);

            p.data[new_ptr as usize..new_ptr as usize + key_buf.len()].copy_from_slice(&key_buf);
            p.data[new_ptr as usize + key_buf.len()..new_ptr as usize + cell_size]
                .copy_from_slice(&val_buf);
            p.dirty = true;
        }

        self.set_cell_ptr(index, new_ptr).await;
        let count = self.cell_count().await;
        self.set_cell_count(count + 1).await;
        self.set_free_offset(new_ptr).await;
        Ok(())
    }

    pub async fn split(&self, new_page: &BTreePage) -> DsResult<DsValue> {
        let count = self.cell_count().await;
        let mid = count / 2;
        let mid_key = self.get_cell_key(mid).await?;

        let is_leaf = self.is_leaf().await;
        new_page.init(is_leaf).await;

        if is_leaf {
            for i in mid..count {
                let k = self.get_cell_key(i).await?;
                let v = self.get_leaf_value(i).await?;
                new_page.insert_leaf_cell(i - mid, &k, &v).await?;
            }
            self.set_cell_count(mid).await;
        } else {
            // Internal node split logic...
        }

        Ok(mid_key)
    }
}
