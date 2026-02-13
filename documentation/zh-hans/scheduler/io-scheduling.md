# I/O 调度层

I/O 调度层是 YYKV 存储内核的最底层调度，直接面向裸硬件（Bare Metal）。其核心目标是消除操作系统文件系统层的干扰，实现确定性的 I/O 延迟。

## 核心组件

### [yykv-scheduler](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-scheduler)
**I/O 优先级调度中心**。
- **职责**：将所有 I/O 请求按优先级（High/Med/Low）分类，并分配到不同的 Worker Shard。
- **调度策略**：支持抢占式调度，确保 WAL 写入和元数据读取（High）不会被背景合并任务（Low）阻塞。

### [yykv-io](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-io)
**异步 I/O 引擎**。
- **职责**：实现 Direct I/O 和零拷贝数据路径。
- **调度特性**：管理对齐缓冲区和异步提交队列，优化 NVMe 设备的并发访问。

### [yykv-hal](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-hal)
**硬件抽象层**。
- **职责**：屏蔽不同平台（Windows/Linux）的底层接口差异。
- **调度支持**：提供底层的块设备读写原语，为上层调度器提供物理位置感知的 I/O 操作。

### [yykv-allocator](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-allocator)
**原始空间分配器**。
- **职责**：管理裸磁盘上的扇区与块。
- **调度相关**：通过物理位置连续性的调度建议，减少磁头移动（对于 HDD）或优化闪存页写入（对于 SSD）。
