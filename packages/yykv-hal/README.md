# yykv-hal (Hardware Abstraction Layer)

`yykv-hal` 是 YYKV 存储引擎的硬件抽象层。它屏蔽了不同操作系统（Windows/Linux）与不同物理存储介质（NVM, SSD, HDD）之间的差异，为上层组件提供统一的裸设备访问接口。

## 核心功能

### 🏗️ 统一块设备接口 (BlockDevice Trait)
定义了异步读写物理扇区的核心特征：
- `read_at(offset, buf)`: 异步读取指定偏移量的数据。
- `write_at(offset, data)`: 异步写入数据到物理介质。
- `sync()`: 强制执行硬件层面的刷盘操作。

### 💾 介质属性探测 (Device Info)
- **扇区大小 (Sector Size)**: 自动探测物理设备的逻辑/物理扇区大小（如 512B, 4KB），指导上层进行 IO 对齐。
- **介质分类 (MediumType)**: 区分 `NVM` (持久内存)、`NvmeSSD` (高性能 NVMe)、`SataSSD` (标准 SSD) 和 `HDD` (机械硬盘)。
- **总容量管理**: 提供设备的总可用空间信息。

### 🛡️ 物理对齐约束
强制要求所有 IO 操作符合物理扇区对齐，这是实现 **Bypass FS (绕过文件系统)** 和极致性能的基础。

## 核心组件

- `BlockDevice`: 抽象特征定义。
- `DeviceInfo`: 存储设备元数据结构。
- `MediumType`: 介质类型枚举。

## 使用示例

```rust
use yykv_hal::{BlockDevice, MediumType};

// 获取设备信息
let info = device.info();
println!("Medium: {:?}, Sector Size: {}", info.medium_type, info.sector_size);

// 执行底层对齐写入
device.write_at(0, &aligned_buffer).await?;
```

## 跨平台支持
- **Windows**: 直接通过 `\\.\PhysicalDrive` 路径使用 `CreateFileW` 访问。
- **Linux**: (Roadmap) 支持通过 `io_uring` 访问块设备 `/dev/sdX`。
