# I/O 调度器 (yykv-scheduler)

`yykv-scheduler` 是 YYKV 存储内核的核心组件，专门针对高性能 NVMe 设备设计，解决了传统文件系统在多租户并发下的 I/O 抖动问题。

## 核心架构

### 1. 优先级驱动调度
系统将所有物理 I/O 请求分为三个严格的优先级：
- **High (高优先级)**: 
  - 涉及元数据（MetaPage）的读写。
  - WAL 顺序日志写入。
  - 关键路径上的点查询。
- **Medium (中优先级)**: 
  - 标准用户 SQL 查询。
  - 普通 KV 读写。
- **Low (低优先级)**: 
  - LSM-Tree 后台合并 (`yykv-compaction`)。
  - 冷热数据迁移 (`yykv-tiering`)。
  - 异步索引构建 (`yykv-index`)。

### 2. I/O 分片 (Sharding) 技术
为了消除全局锁竞争，调度器基于 `PageId` 进行哈希分片：
- 每个分片拥有独立的任务队列。
- 采用无锁队列（Lock-free Queue）提升多核扩展性。
- 结合 `yykv-io` 的对齐缓冲区，实现真正的零拷贝提交。

### 3. 硬件感知与背压
- **队列深度感知**: 实时监控 NVMe 控制器的提交队列（Submission Queue），防止溢出导致硬件阻塞。
- **流式背压**: 当底层分片繁忙时，通过 `yykv-stream` 向上层算子传递等待信号，动态调整扫描速率。

## 与其他模块的联系

- [yykv-io](file:///e:/yydb/yykv/packages/yykv-io/readme.md): 提供底层的 Direct I/O 异步接口。
- [yykv-hal](file:///e:/yydb/yykv/packages/yykv-hal/readme.md): 适配 Windows/Linux 的差异化物理驱动访问。
- [yykv-allocator](file:///e:/yydb/yykv/packages/yykv-allocator/readme.md): 调度器根据分配器的页面对齐要求进行 I/O 校验。
