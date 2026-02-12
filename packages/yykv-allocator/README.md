# yykv-allocator (Media-Aware Page Allocator)

`yykv-allocator` 是 YYKV 存储引擎的物理空间管理器，负责在裸设备（Bare Metal）上进行高效的页面分配与管理。它能够感知底层物理介质的特性，并据此优化存储布局。

## 核心功能

### 🧠 介质感知分配 (Media-Aware Allocation)
根据物理设备的 `MediumType` 自动调整逻辑页面大小，以匹配硬件的最优 IO 单位：
- **NVM / NVMe SSD**: 默认 4KB 页面，匹配现代控制器的高并发小 IO 特性。
- **SATA SSD**: 默认 8KB 页面，平衡写放大与寻址效率。
- **HDD**: 默认 256KB 页面，大幅减少机械臂寻址次数。

### 🛡️ 物理对齐保证
确保分配的每个 `PageId` 对应的物理偏移量严格符合物理扇区对齐要求，支持 Direct IO 模式下的无损传输。

### 🔄 页面回收与复用
维护空闲页面位图（Free Bitmap）或列表，支持逻辑页面的高效分配与延迟回收。

## 核心设计

- **PageId**: 全局唯一的逻辑页面标识符。
- **Physical Offset**: `PageId * PageSize`。
- **Device Alignment**: 强制要求物理驱动器的扇区大小对齐。

## 使用示例

```rust
use yykv_allocator::PageAllocator;
use yykv_hal::DeviceInfo;
use yykv_types::MediumType;

// 模拟 NVMe 设备信息
let info = DeviceInfo {
    medium_type: MediumType::NvmeSSD,
    sector_size: 512,
    total_capacity: 1024 * 1024 * 1024, // 1GB
};

let mut allocator = PageAllocator::new(&info);

// 分配新页面
let page_id = allocator.allocate();
let size = allocator.page_size(); // 4096

println!("Allocated PageId: {}, Size: {}", page_id, size);
```

## 性能考量
- **零 IO 分配**: 分配器元数据尽量缓存在内存中，避免在路径上产生额外的磁盘 IO。
- **碎片整理**: 后台支持自动进行空闲页面的碎片整理与合并。
