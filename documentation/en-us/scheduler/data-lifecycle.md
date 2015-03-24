# Data Lifecycle Management

Data lifecycle scheduling is responsible for dynamically adjusting the location and organization of data based on access frequency, importance, and physical hardware characteristics.

## Core Components

### [yykv-compaction](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-compaction)
**Data Compaction & Merge Engine**.
- **Responsibility**: Performs layered merging for LSM trees.
- **Scheduling Logic**: Automatically schedules compaction tasks based on write load and disk fragmentation rate, reclaiming expired space and optimizing query efficiency.

### [yykv-tiering](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-tiering)
**Hot/Cold Data Tiering Scheduler**.
- **Responsibility**: Moves data between NVMe (Hot), SSD (Warm), and HDD (Cold).
- **Scheduling Strategy**: Automatically identifies hot data based on LFU/LRU algorithms, achieving the best balance between storage cost and performance.

### [yykv-layout](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-layout)
**Physical Layout Definition**.
- **Responsibility**: Defines the physical structure of data on disk.
- **Scheduling Related**: Provides better sequential access possibilities for I/O schedulers by defining optimized data arrangements.
