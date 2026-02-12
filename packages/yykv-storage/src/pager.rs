use crate::page::{PAGE_SIZE, Page, PageId, PageType};
use lru::LruCache;
use parking_lot::RwLock as SyncRwLock;
use tokio::sync::RwLock;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use yykv_hal::BlockDevice;
use yykv_types::{DsError, DsResult};

pub struct Pager {
    device: Arc<dyn BlockDevice>,
    cache: SyncRwLock<LruCache<PageId, Arc<RwLock<Page>>>>,
    max_pages: AtomicU32,
}

impl Pager {
    pub async fn new(device: Arc<dyn BlockDevice>, cache_size: usize) -> DsResult<Self> {
        let info = device.info();
        let total_size = info.total_sectors * info.sector_size as u64;

        if !total_size.is_multiple_of(PAGE_SIZE as u64) {
            return Err(DsError::storage(format!(
                "Device size {} is not a multiple of page size {}",
                total_size, PAGE_SIZE
            )));
        }

        let max_pages = (total_size / PAGE_SIZE as u64) as PageId;

        let pager = Self {
            device,
            cache: SyncRwLock::new(LruCache::new(NonZeroUsize::new(cache_size).unwrap())),
            max_pages: AtomicU32::new(max_pages),
        };

        if max_pages == 0 {
            pager.init_meta_page().await?;
        }

        Ok(pager)
    }

    pub fn page_count(&self) -> PageId {
        self.max_pages.load(Ordering::SeqCst)
    }

    pub async fn get_page(&self, id: PageId) -> DsResult<Arc<RwLock<Page>>> {
        {
            let mut cache = self.cache.write();
            if let Some(page) = cache.get(&id) {
                return Ok(page.clone());
            }
        }

        if id >= self.max_pages.load(Ordering::SeqCst) {
            return Err(DsError::storage(format!("Page {} not found", id)));
        }

        let mut page_data = Box::new([0u8; PAGE_SIZE]);
        self.device
            .read_at(id as u64 * PAGE_SIZE as u64, page_data.as_mut_slice())
            .await?;

        let page = Arc::new(RwLock::new(Page {
            id,
            data: page_data,
            dirty: false,
        }));

        self.cache.write().put(id, page.clone());
        Ok(page)
    }

    async fn init_meta_page(&self) -> DsResult<()> {
        let page_arc = self.allocate_page_internal().await?;
        let mut page = page_arc.write().await;
        page.set_type(PageType::Meta);

        // Root page ID (0 initially)
        page.data[8..12].copy_from_slice(&0u32.to_le_bytes());
        // Free list head (0 means empty)
        page.data[12..16].copy_from_slice(&0u32.to_le_bytes());

        page.dirty = true;
        drop(page);
        self.flush_page(0).await?;
        Ok(())
    }

    async fn allocate_page_internal(&self) -> DsResult<Arc<RwLock<Page>>> {
        let id = self.max_pages.fetch_add(1, Ordering::SeqCst);

        // Extend device by writing zeros if necessary (some devices might handle this)
        let zeros = [0u8; PAGE_SIZE];
        self.device
            .write_at(id as u64 * PAGE_SIZE as u64, &zeros)
            .await?;

        let page = Arc::new(RwLock::new(Page::new(id)));
        self.cache.write().put(id, page.clone());
        Ok(page)
    }

    pub async fn allocate_page(&self) -> DsResult<Arc<RwLock<Page>>> {
        let meta_arc = self.get_page(0).await?;
        let mut meta = meta_arc.write().await;

        let free_head = u32::from_le_bytes(meta.data[12..16].try_into().unwrap());

        if free_head != 0 {
            let free_page_arc = self.get_page(free_head).await?;
            let mut free_page = free_page_arc.write().await;

            let next_free = u32::from_le_bytes(free_page.data[1..5].try_into().unwrap());

            meta.data[12..16].copy_from_slice(&next_free.to_le_bytes());
            meta.dirty = true;

            free_page.data.fill(0);
            free_page.dirty = true;

            Ok(free_page_arc.clone())
        } else {
            drop(meta);
            self.allocate_page_internal().await
        }
    }

    pub async fn flush_page(&self, id: PageId) -> DsResult<()> {
        let page_arc = {
            let mut cache = self.cache.write();
            cache.get(&id).cloned()
        };

        if let Some(page_arc) = page_arc {
            let mut page = page_arc.write().await;
            if page.dirty {
                let data = page.data.as_slice();
                self.device
                    .write_at(id as u64 * PAGE_SIZE as u64, data)
                    .await?;
                page.dirty = false;
            }
        }
        Ok(())
    }

    pub async fn set_root_page_id(&self, id: PageId) -> DsResult<()> {
        let meta_arc = self.get_page(0).await?;
        let mut meta = meta_arc.write().await;
        meta.data[8..12].copy_from_slice(&id.to_le_bytes());
        meta.dirty = true;
        Ok(())
    }

    pub async fn get_root_page_id(&self) -> DsResult<PageId> {
        let meta_arc = self.get_page(0).await?;
        let meta = meta_arc.read().await;
        Ok(u32::from_le_bytes(meta.data[8..12].try_into().unwrap()))
    }
}
