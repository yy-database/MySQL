# Introduction

YYKV (YongYuan Key-Value) is the underlying storage cornerstone of the YongYuan Database ecosystem, focusing on achieving extreme I/O throughput and data layout control on bare metal.

## Core Design Principles

1. **Bypass File System (Bypass FS)**: Directly manages disk blocks through `yykv-hal` and `yykv-allocator`, eliminating file system metadata locks and log redundancy.
2. **Pushdown Slots**: Built-in `yykv-compute` in the storage layer supports direct filtering and projection during data reading, greatly reducing bus data transmission.
3. **I/O Priority Scheduling**: `yykv-scheduler` supports fine-grained I/O scheduling based on tenant priority and request type (e.g., WAL writing vs. background Compaction).
