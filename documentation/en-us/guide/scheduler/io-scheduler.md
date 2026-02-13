# I/O Scheduler (yykv-scheduler)

`yykv-scheduler` is a core component of the YYKV storage kernel, specifically designed for high-performance NVMe devices. It solves the I/O jitter problem caused by traditional file systems under multi-tenant concurrency.

## Core Architecture

### 1. Priority-Driven Scheduling
The system categorizes all physical I/O requests into three strict priority levels:
- **High**: 
  - Read/write operations involving metadata (`MetaPage`).
  - Sequential WAL log writes.
  - Point queries on the critical path.
- **Medium**: 
  - Standard user SQL queries.
  - Normal KV read/writes.
- **Low**: 
  - LSM-Tree background compaction (`yykv-compaction`).
  - Cold/hot data migration (`yykv-tiering`).
  - Asynchronous index building (`yykv-index`).

### 2. I/O Sharding Technology
To eliminate global lock contention, the scheduler implements hash-based sharding by `PageId`:
- Each shard has an independent task queue.
- Utilizes lock-free queues for enhanced multi-core scalability.
- Combined with `yykv-io`'s aligned buffers to achieve true zero-copy submission.

### 3. Hardware Awareness and Backpressure
- **Queue Depth Awareness**: Real-time monitoring of the NVMe controller's Submission Queue to prevent hardware blocking due to overflow.
- **Streaming Backpressure**: When an underlying shard is busy, it passes wait signals to upper-level operators via `yykv-stream` to dynamically adjust scanning rates.

## Relationship with Other Modules

- [yykv-io](file:///e:/yydb/yykv/packages/yykv-io/readme.md): Provides low-level Direct I/O asynchronous interfaces.
- [yykv-hal](file:///e:/yydb/yykv/packages/yykv-hal/readme.md): Adapts to differentiated physical drive access for Windows/Linux.
- [yykv-allocator](file:///e:/yydb/yykv/packages/yykv-allocator/readme.md): The scheduler performs I/O validation based on the allocator's page alignment requirements.
