# yykv

YYKV High-Performance Key-Value Storage Engine.

## Features

- **Tiered Storage (Tiering)**: Supports automatic migration of hot and cold data across different media like SSD and HDD.
- **Direct IO**: Optimized `FILE_FLAG_NO_BUFFERING` direct IO access for the Windows platform.
- **LSM-Tree Architecture**: Efficient write performance and optimized compaction algorithms.
- **Storage-Level UDF**: Supports running WASM operators directly inside the storage engine for data pre-filtering.
- **Hardware Abstraction Layer (HAL)**: Unified block device abstraction interface, easy to adapt to different operating systems and hardware.

## Sub-projects

- `yykv-io`: Low-level asynchronous Direct IO implementation.
- `yykv-allocator`: Page allocator specifically designed for databases.
- `yykv-tiering`: Logic for hot/cold data detection and automatic migration.
- `yykv-udf`: Storage-level UDF interface based on WIT definitions.
