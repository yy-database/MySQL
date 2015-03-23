# 任务调度与执行 (yyds-task & yykv-executor)

YYDS 的任务调度体系负责将高层抽象的查询计划转化为底层物理节点上的执行流，并处理复杂的后台异步任务。

## 任务类型

### 1. 实时计算任务
由 `yykv-executor` 直接驱动，处理来自协议层的 `OpsGraph`。
- **近数据计算**: 算子直接作用于 `yykv-layout` 定义的物理页。
- **Wasm 沙箱**: `yykv-udf` 提供的计算逻辑在受限的环境中执行，防止影响存储内核稳定性。

### 2. 后台异步任务 (yyds-task)
处理耗时较长的管理操作：
- **分布式 Cron**: 定时触发计费汇总（`yyds-billing`）和系统巡检。
- **状态追踪**: 任务状态持久化于 WAL，支持节点宕机后的自动重调度。

### 3. WAE 事件驱动调度
`yykv-event` 实现的 **Write-Ahead Event** 机制。
- 当数据写入 WAL 时，实时触发订阅者（WaeWorker）。
- 支持在数据落盘前完成索引预构建和物化视图更新。

## 调度策略

- **协同调度**: `yyds-task` 会感知 `yykv-scheduler` 的负载，在 I/O 繁忙时自动挂起非紧急后台任务。
- **租户隔离**: 任务执行时绑定 `TenantId`，严格限制每个租户的 CPU 时间片和内存配额。

## 相关组件

- [yykv-operators](file:///e:/yydb/yykv/packages/yykv-operators/readme.md): 定义算子图 IR。
- [yyds-consensus](file:///e:/yydb/yykv/packages/yyds-consensus/README.md): 负责任务在多副本间的同步与一致性。
- [yyds-observability](file:///e:/yydb/yykv/packages/yyds-observability/README.md): 实时追踪任务执行延迟与资源占用。
