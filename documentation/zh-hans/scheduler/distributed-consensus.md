# 分布式共识与协调

分布式调度层负责跨节点的状态同步、任务分配以及全局索引的维护，是 YYDS 作为分布式数据库的核心大脑。

## 核心组件

### [yyds-consensus](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-consensus)
**分布式共识引擎**。
- **职责**：实现 Raft 或 Gossip 协议。
- **调度逻辑**：负责 Leader 选举、成员变更调度以及强一致性数据的同步。

### [yyds-pubsub](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-pubsub)
**消息发布/订阅调度**。
- **职责**：在节点间分发实时消息和状态变更通知。
- **调度相关**：管理订阅者队列和消息投递策略。

### [yykv-index](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-index)
**全局索引管理器**。
- **职责**：维护跨节点的索引（如向量索引、文本全文索引）。
- **调度相关**：调度索引构建任务，确保数据变更后的索引最终一致性。

### [yykv-executor](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-executor) / [yykv-operators](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-operators)
**执行器与算子库**。
- **职责**：将高级查询计划转换为可执行的算子流。
- **调度相关**：负责算子间的并行执行编排和 pipeline 调度。
