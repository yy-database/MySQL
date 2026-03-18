# yykv-index (Unified Multi-Modal Index Engine)

`yykv-index` is the unified index manager for the YYKV storage engine. It supports various indexing modes, from traditional full-text search to modern AI-driven vector retrieval, and is deeply integrated with YYKV's **WAE (Write-Ahead Event)** architecture for near-real-time (NRT) index updates.

## Core Features

### 🔍 Full-Text Search
In-house developed `YNI (YYKV-Native Index)` high-performance search engine:
- **Inverted Index**: Supports basic text queries and tenant-level isolation.
- **Multi-Tenant Isolation**: Native support for `tenant_id` filtering within the index architecture, ensuring physical-level isolation.
- **In-Memory Index**: Supports `In-RAM` mode, suitable for high-performance temporary retrieval scenarios.

### ⚡ Near-Real-Time Indexing (NRT Indexing)
By subscribing to the WAL event stream emitted by `yykv-event`:
- **Automatic Sync**: When data is written to the storage engine, the index module automatically captures changes and updates the inverted index.
- **Zero Management**: Users do not need to manually trigger index rebuilds.

### 🏗️ Multi-Modal Index Support (Roadmap)
- **B-Tree / LSM**: Traditional indexes optimized for primary keys and range queries.
- **Vector Index (HNSW/IVF)**: Vector indexes optimized for AI similarity retrieval.

## Core Components

- `SearchIndexManager`: Index lifecycle manager, responsible for index creation, writing, and retrieval.
- `WalIndexer`: Background coroutine responsible for converting WAL events into index documents.

## Usage Example

```rust
use yykv_index::SearchIndexManager;

// Create an in-memory index instance
let index_manager = SearchIndexManager::new_in_memory()?;

// Retrieve documents
let results = index_manager.search("search query", tenant_uuid).await?;

// Near-real-time WAL listening
let index_arc = Arc::new(index_manager);
tokio::spawn(index_arc.run_indexer(wal_manager));
```

## Performance Considerations
- **Batch Commits**: The index writer supports micro-batching to balance write throughput and search visibility.
- **Segment Merge**: Automatically manages index fragments to optimize search latency.
