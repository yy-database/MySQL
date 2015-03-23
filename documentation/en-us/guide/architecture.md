# Architecture Design

## 1. Storage Architecture Diagram

The YYKV architecture is divided into the following layers:

- **Execution Pushdown Layer**: `yykv-udf`, `yykv-compute`, `yykv-stream`.
- **Storage Management**: `yykv-scheduler`, `yykv-tiering`, `yykv-compaction`.
- **Hardware Abstraction & Layout**: `yykv-layout`, `yykv-allocator`, `yykv-io`, `yykv-hal`.

## 2. Package Responsibility Description

| Package | Responsibility |
| :--- | :--- |
| `yykv-hal` | **Hardware Abstraction Layer**. Handles low-level I/O interfaces across different operating systems (e.g., Windows `ReadFile` vs. Linux `io_uring`). |
| `yykv-io` | **Async I/O Engine**. Implements Direct I/O, aligned buffer management, and zero-copy reading. |
| `yykv-allocator` | **Raw Space Allocator**. Manages sectors and blocks on raw disks, supporting space reclamation and defragmentation. |
| `yykv-layout` | **Physical Layout**. Defines how data is arranged on disk (e.g., variable-length key-values, index page layout). |
| `yykv-compaction` | **Data Merging & GC**. Responsible for background multi-level data merging and garbage collection. |
| `yykv-scheduler` | **I/O Scheduler**. Manages disk I/O requests based on priority and tenant quotas to ensure low latency. |
| `yykv-tiering` | **Auto-Tiering**. Automatically migrates data between NVMe, SSD, and HDD to optimize storage costs. |
| `yykv-udf` | **User Defined Functions**. Supports execution of sandboxed UDF code directly at the storage layer. |
