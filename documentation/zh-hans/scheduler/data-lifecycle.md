# 数据生命周期管理

数据生命周期调度负责根据数据的访问频率、重要程度以及物理硬件的特性，动态调整数据的位置 and 组织形式。

## 核心组件

### [yykv-compaction](../../packages/yykv-compaction)
**数据压缩与合并引擎**。
- **职责**：执行 LSM 树的分层合并。
- **调度逻辑**：监控各层级的文件数量与空间放大率，自动调度合并任务。采用 `Low Priority` I/O，确保不影响在线业务。

### [yykv-tiering](../../packages/yykv-tiering)
**冷热数据分层调度器**。
- **职责**：在 NVMe (热数据)、SSD (温数据) 和 HDD (冷数据) 之间移动数据。
- **调度策略**：基于 LFU/LRU 算法实时统计 `PageId` 的访问频率，自动执行提升（Promote）或降级（Demote）任务。

### [yykv-layout](../../packages/yykv-layout)
**物理布局定义**。
- **职责**：定义数据在磁盘上的物理结构，为算子提供近数据计算的物理基础。
