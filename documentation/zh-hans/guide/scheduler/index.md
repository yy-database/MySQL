# 调度系统概览

YYDS 生态构建了一个多层级的调度体系，从底层的物理 I/O 到上层的分布式任务，确保了系统在高负载下的稳定性和响应速度。

## 调度层级

### 1. I/O 调度层 (I/O Scheduling)
负责管理裸设备（Bare-metal）的并发访问与优先级抢占。
- **相关组件**: `yykv-scheduler`, `yykv-io`, `yykv-hal`
- **核心目标**: 保证前台查询（High Prio）不被后台维护（Low Prio）阻塞。

### 2. 任务执行层 (Task Execution)
负责计算任务的编排、分发与流式背压控制。
- **相关组件**: `yyds-task`, `yykv-executor`, `yykv-stream`
- **核心目标**: 实现近数据计算（Near-Data Computing）的高效并行执行。

### 3. 后台维护层 (Background Maintenance)
负责自动化的数据布局优化与生命周期管理。
- **相关组件**: `yykv-compaction`, `yykv-tiering`, `yykv-index`
- **核心目标**: 自动完成 LSM-Tree 合并、冷热数据迁移及索引同步。

### 4. 资源治理层 (Resource Governance)
负责多租户环境下的配额控制与流量整形。
- **相关组件**: `yyds-quota`, `yyds-billing`, `yykv-policy`
- **核心目标**: 物理与逻辑层面的多租户隔离与资源公平分配。
