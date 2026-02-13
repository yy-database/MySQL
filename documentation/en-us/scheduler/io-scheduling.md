# I/O Scheduling Layer

The I/O scheduling layer is the lowest level of scheduling in the YYKV storage kernel, directly facing raw hardware (Bare Metal). Its core goal is to eliminate interference from the OS file system layer and achieve deterministic I/O latency.

## Core Components

### [yykv-scheduler](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-scheduler)
**I/O Priority Scheduling Center**.
- **Responsibility**: Categorizes all I/O requests by priority (High/Med/Low) and assigns them to different Worker Shards.
- **Strategy**: Supports preemptive scheduling, ensuring WAL writes and metadata reads (High) are not blocked by background compaction tasks (Low).

### [yykv-io](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-io)
**Async I/O Engine**.
- **Responsibility**: Implements Direct I/O and zero-copy data paths.
- **Scheduling Features**: Manages aligned buffers and async submission queues, optimizing concurrent access to NVMe devices.

### [yykv-hal](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-hal)
**Hardware Abstraction Layer**.
- **Responsibility**: Shields low-level interface differences between platforms (Windows/Linux).
- **Scheduling Support**: Provides low-level block device read/write primitives, offering physical-location-aware I/O operations for upper-layer schedulers.

### [yykv-allocator](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-allocator)
**Raw Space Allocator**.
- **Responsibility**: Manages sectors and blocks on raw disks.
- **Scheduling Related**: Reduces head movement (for HDD) or optimizes flash page writes (for SSD) through physical location continuity scheduling suggestions.
