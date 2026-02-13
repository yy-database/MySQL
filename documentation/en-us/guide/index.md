# Introduction

**YYKV (The Singular Key-Value)** is the underlying storage cornerstone of the YY (YingYing) Database ecosystem, focusing on achieving extreme I/O throughput and data layout control directly on **Bare Metal**.

## 1. Technical Vision

YYKV's vision is to **bridge hardware differences and release physical limits**. We believe that the performance bottleneck of modern databases often lies not in algorithmic complexity, but in the operating system's file system layer and CPU context switching overhead. By directly driving the hardware, YYKV builds a "smart media layer" that is self-aware and automatically tiered.

## 2. Replacement Targets

YYKV is not a standalone terminal database, but it is the core technical engine that enables **YYDB** to successfully replace **SQLite** and **Redis**:

- **Surpassing SQLite**:
  - **Bypassing File System**: SQLite is limited by OS file system metadata locks. YYKV directly manages disk blocks, eliminating write amplification and concurrency limits at the file system layer.
  - **True Row-Level Concurrency**: Based on fine-grained control from `yykv-allocator`, it supports much finer concurrency than SQLite's coarse database-level or table-level locks.
- **Challenging Redis**:
  - **Memory-Level Disk Throughput**: Utilizing `io_uring` and zero-copy data paths, it achieves extremely low latency on NVMe SSDs, approaching memory access speeds.
  - **Multi-Modal Native Storage**: Unlike Redis's simple KV structure, the YYKV physical layer natively supports `DsValue` (List, Dict, Object), enabling $O(1)$ random access for complex data structures directly on disk.

## 3. Typical Scenarios

- **AI Native Storage**: Directly push down vector similarity calculation operators to the storage layer, providing extreme response speed for RAG (Retrieval-Augmented Generation).
- **Serverless Edge Caching**: As the kernel of YYDB, it cold-starts with a very small footprint alongside function instances, providing local state access that is an order of magnitude faster than remote Redis.
- **Massive Heterogeneous Data Management**: Automatically identifies NVMe (Hot data) and HDD (Cold data) within a single cluster, implementing automatic data migration based on access frequency without manual intervention.

## 4. Core Design Principles

1. **Bypass File System (Bypass FS)**: Directly manages disk blocks through `yykv-hal` and `yykv-allocator`, eliminating file system metadata locks and log redundancy.
2. **Pushdown Slots**: Built-in `yykv-compute` in the storage layer supports direct filtering (Filter) and projection (Project) during data reading, greatly reducing bus data transmission.
3. **I/O Priority Scheduling**: `yykv-scheduler` supports fine-grained I/O scheduling based on tenant priority and request type (e.g., WAL writing vs. background Compaction).
