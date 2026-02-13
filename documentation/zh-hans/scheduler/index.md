# 调度系统总览

YYDS (独一分布式存储) 的调度系统是一个多层级的复杂体系，旨在抹平硬件差异，在分布式环境下提供极致的 I/O 吞吐、任务可靠性以及计算效率。

调度系统从底层的裸硬件抽象到上层的分布式共识，贯穿了整个 YYDS 生态。

## 调度层级

根据职责和执行粒度的不同，调度系统分为以下几个核心维度：

### 1. I/O 调度层 (Low-level I/O)
专注于单机硬件性能的极致释放。
- 相关包：`yykv-scheduler`, `yykv-io`, `yykv-hal`, `yykv-allocator`

### 2. 任务调度层 (Background Tasks)
专注于后台异步任务的可靠执行与持久化。
- 相关包：`yyds-task`, `yykv-event`, `yykv-wal`

### 3. 数据生命周期调度 (Data Lifecycle)
专注于数据在不同存储介质间的流动与形态转换。
- 相关包：`yykv-compaction`, `yykv-tiering`, `yykv-layout`

### 4. 计算与流编排 (Compute & Stream)
专注于计算算子的执行位置调度与数据流驱动。
- 相关包：`yyds-compute`, `yykv-compute`, `yykv-stream`, `yykv-udf`, `yyds-ai`

### 5. 资源与配额调度 (Resource & Quota)
专注于多租户环境下的公平性与资源隔离。
- 相关包：`yyds-quota`, `yyds-tenant`, `yyds-billing`, `yyds-observability`

### 6. 分布式共识与协调 (Global Coordination)
专注于跨节点的任务分配、状态同步与全局索引。
- 相关包：`yyds-consensus`, `yyds-pubsub`, `yykv-index`, `yykv-executor`
