use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};
use yykv_allocator::PageAllocator;
use yykv_hal::BlockDevice;
use yykv_layout::{HEADER_SIZE, Header, LayoutManager};
use yykv_types::{DsError, DsResult, MediumType, PageId};

pub struct HotColdDetector {
    access_counts: HashMap<PageId, u32>,
    threshold: u32,
}

impl HotColdDetector {
    pub fn new(threshold: u32) -> Self {
        Self {
            access_counts: HashMap::new(),
            threshold,
        }
    }

    pub fn record_access(&mut self, page_id: PageId) -> bool {
        let count = self.access_counts.entry(page_id).or_insert(0);
        *count += 1;
        *count >= self.threshold
    }

    pub fn is_hot(&self, page_id: PageId) -> bool {
        self.access_counts
            .get(&page_id)
            .is_some_and(|&c| c >= self.threshold)
    }

    pub fn reset_access(&mut self, page_id: PageId) {
        self.access_counts.remove(&page_id);
    }
}

pub struct TieredStorage {
    devices: HashMap<MediumType, Arc<dyn BlockDevice>>,
    allocators: HashMap<MediumType, PageAllocator>,
    layout: LayoutManager,
    detector: HotColdDetector,
    page_map: HashMap<PageId, (MediumType, u64)>, // Maps PageId to physical location
}

impl TieredStorage {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            allocators: HashMap::new(),
            layout: LayoutManager::new(),
            detector: HotColdDetector::new(10), // Threshold of 10 accesses for "hot"
            page_map: HashMap::new(),
        }
    }
}

impl Default for TieredStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TieredStorage {
    pub fn add_device(&mut self, medium: MediumType, device: Arc<dyn BlockDevice>) {
        let info = device.info();
        let allocator = PageAllocator::new(info);
        self.devices.insert(medium, device);
        self.allocators.insert(medium, allocator);
    }

    pub async fn write_value(&mut self, value: &yykv_types::DsValue) -> DsResult<PageId> {
        let data = self.layout.serialize(value);

        // 1. Determine initial storage strategy (SSD for all new writes)
        let medium = if self.devices.contains_key(&MediumType::NvmeSSD) {
            MediumType::NvmeSSD
        } else if self.devices.contains_key(&MediumType::SataSSD) {
            MediumType::SataSSD
        } else {
            MediumType::HDD
        };

        // 2. Allocate pages
        let allocator = self.allocators.get_mut(&medium).ok_or_else(|| {
            DsError::storage_with_op("allocate", format!("Allocator for {:?} not found", medium))
        })?;

        let page_id = allocator.allocate();
        let offset = page_id * allocator.page_size() as u64;

        // 3. Write to device
        let device = self.devices.get(&medium).unwrap();
        device.write_at(offset, &data).await?;

        self.page_map.insert(page_id, (medium, offset));

        Ok(page_id)
    }

    pub async fn read_value(&mut self, page_id: PageId) -> DsResult<yykv_types::DsValue> {
        let data = self.read_raw(page_id).await?;
        self.layout.deserialize(&data).ok_or_else(|| {
            DsError::serialization_with_format("binary", "Failed to deserialize value")
        })
    }

    pub async fn read_raw(&mut self, page_id: PageId) -> DsResult<Vec<u8>> {
        let (medium, offset) = self
            .page_map
            .get(&page_id)
            .map(|(m, o)| (*m, *o))
            .ok_or_else(|| DsError::storage_with_op("read", "Page not found"))?;

        let device = self.devices.get(&medium).unwrap();

        // Record access for hot/cold detection
        let is_hot = self.detector.record_access(page_id);

        // If it's hot and on HDD, we might want to move it to SSD
        let (actual_device, actual_offset) = if is_hot && medium == MediumType::HDD {
            self.move_to_fast_tier(page_id).await?;
            let (new_medium, new_offset) =
                self.page_map.get(&page_id).map(|(m, o)| (*m, *o)).unwrap();
            (self.devices.get(&new_medium).unwrap(), new_offset)
        } else {
            (device, offset)
        };

        let mut header_buf = vec![0u8; HEADER_SIZE];
        actual_device
            .read_at(actual_offset, &mut header_buf)
            .await?;

        let header = Header::from_bytes(&header_buf)
            .ok_or_else(|| DsError::serialization_with_format("header", "Invalid header"))?;

        let mut full_data = vec![0u8; header.length as usize];
        actual_device.read_at(actual_offset, &mut full_data).await?;

        Ok(full_data)
    }

    async fn move_to_fast_tier(&mut self, page_id: PageId) -> DsResult<()> {
        let (old_medium, old_offset) = self.page_map.get(&page_id).cloned().unwrap();

        let target_medium = if self.devices.contains_key(&MediumType::NvmeSSD) {
            MediumType::NvmeSSD
        } else {
            return Ok(()); // Already on fastest or no faster tier available
        };

        if old_medium == target_medium {
            return Ok(());
        }

        info!(
            "Promoting page {} from {:?} to {:?}",
            page_id, old_medium, target_medium
        );

        // 1. Read from old
        let old_device = self.devices.get(&old_medium).unwrap();
        let mut buf = vec![0u8; 4096]; // Assume 4K page for move
        old_device.read_at(old_offset, &mut buf).await?;

        // 2. Allocate on new
        let allocator = self.allocators.get_mut(&target_medium).unwrap();
        let new_page_id = allocator.allocate();
        let new_offset = new_page_id * allocator.page_size() as u64;

        // 3. Write to new
        let new_device = self.devices.get(&target_medium).unwrap();
        new_device.write_at(new_offset, &buf).await?;

        // 4. Update map
        self.page_map.insert(page_id, (target_medium, new_offset));

        // 5. Reset detector for this page
        self.detector.reset_access(page_id);

        Ok(())
    }

    pub async fn demote_cold_data(&mut self) -> DsResult<()> {
        debug!("Scanning for cold data to demote...");
        // Simple background task: move data that hasn't been accessed much to HDD
        // In a real system, this would be more sophisticated (LRU etc.)
        let cold_pages: Vec<PageId> = self
            .page_map
            .iter()
            .filter(|(id, (medium, _))| *medium != MediumType::HDD && !self.detector.is_hot(**id))
            .map(|(id, _)| *id)
            .collect();

        if !cold_pages.is_empty() {
            info!("Demoting {} cold pages to HDD", cold_pages.len());
        }

        for page_id in cold_pages {
            self.move_to_slow_tier(page_id).await?;
        }
        Ok(())
    }

    async fn move_to_slow_tier(&mut self, page_id: PageId) -> DsResult<()> {
        let (old_medium, old_offset) = self.page_map.get(&page_id).cloned().unwrap();
        let target_medium = MediumType::HDD;

        if !self.devices.contains_key(&target_medium) || old_medium == target_medium {
            return Ok(());
        }

        info!(
            "Demoting page {} from {:?} to {:?}",
            page_id, old_medium, target_medium
        );

        // Move logic similar to move_to_fast_tier...
        let old_device = self.devices.get(&old_medium).unwrap();
        let mut buf = vec![0u8; 4096];
        old_device.read_at(old_offset, &mut buf).await?;

        let allocator = self.allocators.get_mut(&target_medium).unwrap();
        let new_page_id = allocator.allocate();
        let new_offset = new_page_id * allocator.page_size() as u64;

        let new_device = self.devices.get(&target_medium).unwrap();
        new_device.write_at(new_offset, &buf).await?;

        self.page_map.insert(page_id, (target_medium, new_offset));
        Ok(())
    }
}
