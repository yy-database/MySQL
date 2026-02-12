# yykv-index (Unified Multi-Modal Index Engine)

`yykv-index` 是 YYKV 存储引擎的统一索引管理器。它支持从传统的全文搜索到现代 AI 驱动的向量检索等多种索引模式，并深度集成了 YYKV 的 **WAE (写入即事件)** 架构，实现了数据的准实时（NRT）索引更新。

## 核心功能

### 🔍 全文搜索 (Full-Text Search)
自研 `YNI (YYKV-Native Index)` 高性能搜索引擎引擎：
- **倒排索引**: 支持基础文本查询与租户级隔离。
- **多租户隔离**: 在索引架构中原生支持 `tenant_id` 过滤，确保物理级隔离。
- **内存索引**: 支持 `In-RAM` 模式，适用于高性能临时检索场景。

### ⚡ 准实时索引更新 (NRT Indexing)
通过订阅 `yykv-event` 发出的 WAL 事件流：
- **自动同步**: 当数据写入存储引擎时，索引模块会自动捕获变更并更新倒排索引。
- **零管理**: 用户无需手动触发索引重建。

### 🏗️ 多模态索引支持 (Roadmap)
- **B-Tree / LSM**: 针对主键与范围查询优化的传统索引。
- **Vector Index (HNSW/IVF)**: 针对 AI 相似度检索优化的向量索引。

## 核心组件

- `SearchIndexManager`: 索引生命周期管理器，负责索引的创建、写入与检索。
- `WalIndexer`: 背景协程，负责将 WAL 事件转化为索引文档。

## 使用示例

```rust
use yykv_index::SearchIndexManager;

// 创建内存索引实例
let index_manager = SearchIndexManager::new_in_memory()?;

// 检索文档
let results = index_manager.search("search query", tenant_uuid).await?;

// 准实时监听 WAL
let index_arc = Arc::new(index_manager);
tokio::spawn(index_arc.run_indexer(wal_manager));
```

## 性能考量
- **批量提交**: 索引写入器支持微批处理，平衡写入吞吐量与搜索可见性。
- **段合并 (Segment Merge)**: 自动管理索引片段，优化搜索延迟。
