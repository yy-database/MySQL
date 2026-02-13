# 后台维护与自动化 (Maintenance)

YYDS 通过一系列后台调度任务，实现了存储系统的自愈与自优化。

## 核心维护任务

### 1. LSM-Tree 合并 (Compaction)
由 `yykv-compaction` 模块调度：
- **触发机制**: 监控各层级（Level）的文件数量与空间放大率。
- **调度策略**: 采用 `Low Priority` I/O，确保不影响在线业务。
- **垃圾回收**: 清理无效的 Tombstone 记录，释放物理空间。

### 2. 冷热数据分层 (Tiering)
由 `yykv-tiering` 模块管理：
- **热点感知**: 实时统计 `PageId` 的访问频率。
- **自动迁移**: 调度任务将热数据提升（Promote）至 NVMe，冷数据降级（Demote）至 HDD。
- **策略控制**: `yykv-policy` 定义了不同租户的分层规则。

### 3. 近实时索引同步 (NRT Indexing)
由 `yykv-index` 结合 `yykv-event` 完成：
- **流式写入**: 捕获 WAL 事件流，异步更新倒排索引和向量索引。
- **分段合并**: 定期调度索引分段（Segment）的合并任务，优化查询性能。

## 资源控制与配额

- **yyds-quota**: 限制后台任务的总带宽占用，防止“喧宾夺主”。
- **yyds-billing**: 对后台任务产生的 I/O 和存储空间进行审计计费。

## 相关组件

- [yykv-storage](file:///e:/yydb/yykv/packages/yykv-storage/readme.md): 具体的 B-Tree/LSM 实现。
- [yyds-types](file:///e:/yydb/yykv/packages/yyds-types/README.md): 定义租户与介质类型等基础元数据。
