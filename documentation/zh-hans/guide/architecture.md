# 架构设计

## 存储架构图

YYKV 的架构分为以下几个层次：

- **算子下推层**: `yykv-udf`, `yykv-compute`, `yykv-stream`。
- **存储管理**: `yykv-scheduler`, `yykv-tiering`, `yykv-compaction`。
- **硬件抽象与布局**: `yykv-layout`, `yykv-allocator`, `yykv-io`, `yykv-hal`。

## 子包职责说明

| 子包 | 职责 |
| :--- | :--- |
| `yykv-hal` | **硬件抽象层**。处理不同操作系统的底层 I/O 接口（如 Windows `ReadFile` vs Linux `io_uring`）。 |
| `yykv-io` | **异步 I/O 引擎**。实现 Direct I/O、对齐缓冲区管理与零拷贝读取。 |
| `yykv-allocator` | **原始空间分配器**。管理裸磁盘上的扇区与块，支持空间回收与碎片整理。 |
| `yykv-layout` | **物理布局**。定义数据在磁盘上的排列方式（如变长键值、索引页布局）。 |
| `yykv-compaction` | **数据压缩与合并**。负责后台数据的多级合并与垃圾回收。 |
| `yykv-udf` | **用户定义函数**。支持在存储层执行沙箱化的 UDF 代码。 |
