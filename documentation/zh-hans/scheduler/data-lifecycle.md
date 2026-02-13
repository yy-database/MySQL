# 数据生命周期管理

数据生命周期调度负责根据数据的访问频率、重要程度以及物理硬件的特性，动态调整数据的位置和组织形式。

## 核心组件

### [yykv-compaction](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-compaction)
**数据压缩与合并引擎**。
- **职责**：执行 LSM 树的分层合并。
- **调度逻辑**：根据写入负载和磁盘碎片率，自动调度合并任务，回收过期空间并优化查询效率。

### [yykv-tiering](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-tiering)
**冷热数据分层调度器**。
- **职责**：在 NVMe (热数据)、SSD (温数据) 和 HDD (冷数据) 之间移动数据。
- **调度策略**：基于 LFU/LRU 算法自动识别热点数据，实现存储成本与性能的最佳平衡。

### [yykv-layout](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-layout)
**物理布局定义**。
- **职责**：定义数据在磁盘上的物理结构。
- **调度相关**：通过定义优化的数据排列方式，为 I/O 调度器提供更好的顺序访问可能。
