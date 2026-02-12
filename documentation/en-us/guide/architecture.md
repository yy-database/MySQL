# Architecture

## Storage Architecture Diagram

YYKV is structured as follows:

- **Execution Pushdown Layer**: `yykv-udf`, `yykv-compute`, `yykv-stream`.
- **Storage Management**: `yykv-scheduler`, `yykv-tiering`, `yykv-compaction`.
- **Hardware Abstraction & Layout**: `yykv-layout`, `yykv-allocator`, `yykv-io`, `yykv-hal`.

## Package Responsibilities

| Package | Responsibility |
| :--- | :--- |
| `yykv-hal` | Hardware Abstraction Layer. Handles OS-level I/O interfaces. |
| `yykv-io` | Async I/O Engine. Implements Direct I/O and zero-copy. |
| `yykv-allocator` | Raw Space Allocator. Manages sectors and blocks on raw disks. |
| `yykv-layout` | Physical Layout. Defines data arrangement on disk. |
| `yykv-compaction` | Data Merging & GC. Handles multi-level merging. |
| `yykv-udf` | User Defined Functions. Sandbox execution of UDFs. |
