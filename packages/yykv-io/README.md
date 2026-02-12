# yykv-io (High-Performance Async I/O)

`yykv-io` 是 YYKV 存储内核的物理基石，专门针对高性能 NVMe SSD 和现代存储接口设计。它通过绕过操作系统页缓存 (Bypass Page Cache) 和实现严格的扇区对齐，消除了 IO 路径上的不确定性。

## 核心能力

### ⚡ Direct I/O (O_DIRECT / NO_BUFFERING)
在 Windows 平台上利用 `FILE_FLAG_NO_BUFFERING` 标志，在 Linux 平台上（通过 HAL）使用 `O_DIRECT`。
- **优势**: 消除内核态到用户态的内存拷贝，避免文件系统预读 (Read-ahead) 造成的 IO 抖动。
- **代价**: 必须严格遵循物理设备的扇区对齐要求（通常为 512B 或 4KB）。

### 🛡️ 对齐缓冲区管理 (Aligned Buffer)
提供专门的 `AlignedBuffer` 类型，确保内存起始地址与 IO 长度均符合硬件限制。
- 自动探测物理介质的 `sector_size`。
- 基于 DMA 友好的内存分配策略。

### 🧵 异步 IO 调度
结合 `tokio` 的异步能力与底层的阻塞/非阻塞 IO 接口：
- **Windows**: 使用 `spawn_blocking` 封装同步 `ReadFile`/`WriteFile`（后续计划支持 IOCP/IORing）。
- **并发限制**: 内置信号量控制最大在途 IO 数，防止 NVMe 队列溢出。

## 关键 API

- `BlockDevice`: 统一的异步读写特征。
- `WindowsBlockDevice`: 针对 Windows 物理驱动器或文件的 Direct IO 实现。
- `IoStats`: 实时统计 IO 吞吐、延迟分布与错误率。

## 使用示例

```rust
use yykv_io::{WindowsBlockDevice, BlockDevice};

// 开启裸设备访问
let device = WindowsBlockDevice::new(r"\\.\PhysicalDrive1")?;

// 分配符合硬件对齐要求的缓冲区
let mut buf = device.alloc_buffer(65536); // 64KB

// 异步读取，数据直接从磁盘 DMA 到 buf
device.read_at(offset, &mut buf).await?;
```

## 性能表现
- **吞吐量**: 接近硬件裸带宽（Sequential Read > 7000MB/s on PCIe 4.0 NVMe）。
- **稳定性**: P99 延迟波动极小，适用于对 SLA 要求严苛的数据库场景。
