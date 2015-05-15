# yykv-hal (Hardware Abstraction Layer)

`yykv-hal` is the hardware abstraction layer for the YYKV storage engine. It hides the differences between different operating systems (Windows/Linux) and different physical storage media (NVM, SSD, HDD), providing a unified bare-device access interface for upper-level components.

## Core Features

### 🏗️ Unified Block Device Interface (BlockDevice Trait)
Defines the core features for asynchronous reading and writing of physical sectors:
- `read_at(offset, buf)`: Asynchronously reads data at a specified offset.
- `write_at(offset, data)`: Asynchronously writes data to the physical medium.
- `sync()`: Enforces a hardware-level disk flush operation.

### 💾 Device Property Detection (Device Info)
- **Sector Size**: Automatically detects the logical/physical sector size (e.g., 512B, 4KB) of the physical device to guide IO alignment for upper layers.
- **Medium Classification (MediumType)**: Distinguishes between `NVM` (persistent memory), `NvmeSSD` (high-performance NVMe), `SataSSD` (standard SSD), and `HDD` (mechanical hard drive).
- **Total Capacity Management**: Provides information about the total available space on the device.

### 🛡️ Physical Alignment Constraints
Enforces that all IO operations comply with physical sector alignment, which is the foundation for achieving **Bypass FS (Bypassing the File System)** and extreme performance.

## Core Components

- `BlockDevice`: Abstract trait definition.
- `DeviceInfo`: Metadata structure for the storage device.
- `MediumType`: Enumeration of medium types.

## Usage Example

```rust
use yykv_hal::{BlockDevice, MediumType};

// Get device info
let info = device.info();
println!("Medium: {:?}, Sector Size: {}", info.medium_type, info.sector_size);

// Execute low-level aligned write
device.write_at(0, &aligned_buffer).await?;
```

## Cross-Platform Support
- **Windows**: Accessed via `CreateFileW` using the `\\.\PhysicalDrive` path.
- **Linux**: (Roadmap) Support for accessing block devices `/dev/sdX` via `io_uring`.
