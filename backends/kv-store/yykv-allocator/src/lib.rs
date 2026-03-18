#![warn(missing_docs)]

use yykv_hal::DeviceInfo;
use yykv_types::{MediumType, PageId};

pub struct PageAllocator {
    medium_type: MediumType,
    page_size: u32,
    next_page_id: PageId,
}

impl PageAllocator {
    pub fn new(device_info: &DeviceInfo) -> Self {
        let page_size = match device_info.medium_type {
            MediumType::NVM => 4096,     // 4K
            MediumType::NvmeSSD => 4096, // 4K
            MediumType::SataSSD => 8192, // 8K
            MediumType::HDD => 262144,   // 256K
        };

        Self {
            medium_type: device_info.medium_type,
            page_size,
            next_page_id: 0,
        }
    }

    pub fn allocate(&mut self) -> PageId {
        let id = self.next_page_id;
        self.next_page_id += 1;
        id
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    pub fn medium_type(&self) -> MediumType {
        self.medium_type
    }
}
