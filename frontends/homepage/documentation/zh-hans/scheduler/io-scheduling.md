# I/O 调度层

I/O 调度层是 YYKV 存储内核的最底层调度，直接面向裸硬件（Bare Metal）。其核心目标是消除操作系统文件系统层的干扰，实现确定性的 I/O 延迟。

## 核心组件

### [yykv-scheduler](../../backends/kv-store/yykv-scheduler)
**I/O 优先级调度中心**。
- **职责**：将所有 I/O 请求按优先级（High/Med/Low）分类，并分配到不同的 Worker Shard。
- **核心架构**：
    - **优先级驱动**：将请求分为 High（元数据、WAL）、Medium（普通读写）、Low（后台维护）。
    - **I/O 分片 (Sharding)**：基于 `PageId` 进行哈希分片，每个分片拥有独立的无锁任务队列，消除全局锁竞争。
    - **硬件感知**：监控 NVMe 控制器的提交队列（Submission Queue）深度，防止硬件阻塞。

### [yykv-io](../../backends/kv-store/yykv-io)
**异步 I/O 引擎**。
- **职责**：实现 Direct I/O 和零拷贝数据路径。
- **调度特性**：管理对齐缓冲区和异步提交队列，优化 NVMe 设备的并发访问。

### [yykv-hal](../../backends/kv-store/yykv-hal)
**硬件抽象层**。
- **职责**：屏蔽不同平台（Windows/Linux）的底层接口差异。
- **调度支持**：提供底层的块设备读写原语，为上层调度器提供物理位置感知的 I/O 操作。

### [yykv-allocator](../../backends/kv-store/yykv-allocator)
**原始空间分配器**。
- **职责**：管理裸磁盘上的扇区与块。
- **调度相关**：通过物理位置连续性的调度建议，减少磁头移动（对于 HDD）或优化闪存页写入（对于 SSD）。
