# 分布式共识与协调

分布式调度层负责跨节点的状态同步、任务分配以及全局索引的维护，是 YYDS 作为分布式数据库的核心大脑。

## 核心组件

### [yyds-consensus](../../backends/distributed/yyds-consensus)
**分布式共识引擎**。
- **职责**：实现 Raft 或 Gossip 协议，负责任务在多副本间的同步与一致性、Leader 选举及成员变更调度。

### [yyds-pubsub](../../backends/distributed/yyds-pubsub)
**消息发布/订阅调度**。
- **职责**：在节点间分发实时消息和状态变更通知。

### [yykv-index](../../backends/kv-store/yykv-index)
**全局索引管理器**。
- **职责**：维护跨节点的索引（如向量索引、文本全文索引）。捕获 WAL 事件流，异步更新索引并定期调度分段（Segment）合并。

### [yykv-executor](../../backends/kv-store/yykv-executor) / [yykv-operators](../../backends/kv-store/yykv-operators)
**执行器与算子库**。
- **职责**：将高级查询计划（OpsGraph）转换为可执行的算子流，负责算子间的并行执行编排。
