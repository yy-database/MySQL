# yykv-io (High-Performance Async I/O)

`yykv-io` is the physical foundation of the YYKV storage kernel, specifically designed for high-performance NVMe SSDs and modern storage interfaces. It eliminates uncertainty on the IO path by bypassing the operating system's page cache and implementing strict sector alignment.

## Core Capabilities

### ⚡ Direct I/O (O_DIRECT / NO_BUFFERING)
Utilizes the `FILE_FLAG_NO_BUFFERING` flag on Windows and `O_DIRECT` on Linux (via HAL).
- **Advantages**: Eliminates memory copies from kernel space to user space and avoids IO jitter caused by file system read-ahead.
- **Cost**: Must strictly follow physical device sector alignment requirements (usually 512B or 4KB).

### 🛡️ Aligned Buffer Management
Provides a dedicated `AlignedBuffer` type to ensure that both the starting memory address and the IO length comply with hardware restrictions.
- Automatically detects the physical medium's `sector_size`.
- DMA-friendly memory allocation policy.

### 🧵 Async IO Scheduling
Combines `tokio`'s asynchronous capabilities with underlying blocking/non-blocking IO interfaces:
- **Windows**: Uses `spawn_blocking` to wrap synchronous `ReadFile`/`WriteFile` (support for IOCP/IORing is planned).
- **Concurrency Control**: Built-in semaphore to control the maximum number of in-flight IOs, preventing NVMe queue overflow.

## Key APIs

- `BlockDevice`: A unified asynchronous R/W trait.
- `WindowsBlockDevice`: Direct IO implementation for Windows physical drives or files.
- `IoStats`: Real-time statistics for IO throughput, latency distribution, and error rates.

## Usage Example

```rust
use yykv_io::{WindowsBlockDevice, BlockDevice};

// Open bare-device access
let device = WindowsBlockDevice::new(r"\\.\PhysicalDrive1")?;

// Allocate a buffer that meets hardware alignment requirements
let mut buf = device.alloc_buffer(65536); // 64KB

// Asynchronous read; data is DMA-transferred directly from disk to buf
device.read_at(offset, &mut buf).await?;
```

## Performance
- **Throughput**: Approaches raw hardware bandwidth (Sequential Read > 7000MB/s on PCIe 4.0 NVMe).
- **Stability**: Minimal P99 latency fluctuation, suitable for database scenarios with strict SLA requirements.
