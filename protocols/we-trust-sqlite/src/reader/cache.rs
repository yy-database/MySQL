use crate::reader::BTreePage;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use tokio::sync::RwLock;

/// 简单的 Page 缓存，支持 LRU 淘汰策略
pub struct PageCache {
    pages: RwLock<HashMap<u32, BTreePage>>,
    lru_order: Mutex<VecDeque<u32>>,
    capacity: usize,
}

impl PageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            pages: RwLock::new(HashMap::new()),
            lru_order: Mutex::new(VecDeque::new()),
            capacity,
        }
    }

    pub async fn get(&self, page_id: u32) -> Option<BTreePage> {
        let pages = self.pages.read().await;
        if let Some(page) = pages.get(&page_id) {
            // 更新 LRU 顺序
            let mut order = self.lru_order.lock();
            if let Some(pos) = order.iter().position(|&id| id == page_id) {
                order.remove(pos);
            }
            order.push_back(page_id);
            return Some(page.clone());
        }
        None
    }

    pub async fn insert(&self, page_id: u32, page: BTreePage) {
        let mut pages = self.pages.write().await;
        let mut order = self.lru_order.lock();

        // 如果已存在，先移除旧的
        if pages.contains_key(&page_id) {
            if let Some(pos) = order.iter().position(|&id| id == page_id) {
                order.remove(pos);
            }
        } else if pages.len() >= self.capacity {
            // 触发淘汰
            if let Some(oldest_id) = order.pop_front() {
                pages.remove(&oldest_id);
            }
        }

        pages.insert(page_id, page);
        order.push_back(page_id);
    }

    pub async fn clear(&self) {
        let mut pages = self.pages.write().await;
        let mut order = self.lru_order.lock();
        pages.clear();
        order.clear();
    }
}
