# B-Tree Storage Engine

**yykv-storage** is the core storage engine implementation for the YYKV database, focusing on high-performance B-Tree based data persistence and retrieval.

## 1. Core Features

- **B-Tree Based Storage**: Implements a high-concurrency B-Tree structure optimized for large-scale key-value storage.
- **Page Management & Paging**: Manages physical page allocation, caching, and swap-in/out mechanisms to handle data larger than memory.
- **Data Codec**: Efficient serialization and deserialization of `DsValue` into binary page layouts.
- **Concurrency Control**: Thread-safe page access and B-Tree node locking strategies to ensure data consistency under high load.

## 2. Core Components

- `BTree`: The main B-Tree structure managing the hierarchical data nodes.
- `Pager`: Handles reading and writing of physical pages, integrating with `yykv-io`.
- `BTreePage`: Defines the internal layout of B-Tree nodes (Root, Internal, Leaf).

## 3. Relationship with Other Modules

- **yykv-io**: Uses the asynchronous I/O engine for persistent storage access.
- **yykv-layout**: Follows the physical data layout specifications for page binary structures.
- **yykv-wal**: Logs all storage changes for crash recovery and durability.
