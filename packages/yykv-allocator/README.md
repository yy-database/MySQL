# yykv-allocator (Media-Aware Page Allocator)

`yykv-allocator` is the physical space manager of the YYKV storage engine, responsible for efficient page allocation and management on bare-metal devices. It perceives the characteristics of the underlying physical media and optimizes the storage layout accordingly.

## Core Features

### 🧠 Media-Aware Allocation
Automatically adjusts the logical page size based on the device's `MediumType` to match the hardware's optimal IO unit:
- **NVM / NVMe SSD**: Default 4KB pages, matching the high-concurrency small IO characteristics of modern controllers.
- **SATA SSD**: Default 8KB pages, balancing write amplification and addressing efficiency.
- **HDD**: Default 256KB pages, significantly reducing mechanical arm seek times.

### 🛡️ Physical Alignment Guarantee
Ensures that the physical offset corresponding to each allocated `PageId` strictly complies with physical sector alignment requirements, supporting lossless transmission in Direct IO mode.

### 🔄 Page Reclamation & Reuse
Maintains a free page bitmap or list, supporting efficient allocation and lazy reclamation of logical pages.

## Core Design

- **PageId**: Globally unique logical page identifier.
- **Physical Offset**: `PageId * PageSize`.
- **Device Alignment**: Mandatory alignment with the physical drive's sector size.

## Usage Example

```rust
use yykv_allocator::PageAllocator;
use yykv_hal::DeviceInfo;
use yykv_types::MediumType;

// Simulate NVMe device info
let info = DeviceInfo {
    medium_type: MediumType::NvmeSSD,
    sector_size: 512,
    total_capacity: 1024 * 1024 * 1024, // 1GB
};

let mut allocator = PageAllocator::new(&info);

// Allocate a new page
let page_id = allocator.allocate();
let size = allocator.page_size(); // 4096

println!("Allocated PageId: {}, Size: {}", page_id, size);
```

## Performance Considerations
- **Zero IO Allocation**: Allocator metadata is cached in memory as much as possible to avoid extra disk IO in the critical path.
- **Defragmentation**: Background support for automatic fragmentation and merging of free pages.
