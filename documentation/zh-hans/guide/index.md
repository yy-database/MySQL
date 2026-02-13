# 介绍

YYKV (独一键值) 是独一数据库 (YY) 生态的底层存储基石，专注于在裸硬件 (Bare Metal) 上实现极致的 I/O 吞吐与数据布局控制。

## 技术愿景 (Technical Vision)

YYKV 的愿景是**抹平硬件差异，释放物理极限**。我们认为，现代数据库的性能瓶颈往往不在于算法复杂度，而在于操作系统的文件系统层和 CPU 的上下文切换开销。YYKV 通过直接驱动硬件，构建一个能够自我感知、自动分层的“智能介质层”。

## 替代目标 (Replacement Targets)

YYKV 并不是一个独立的终端数据库，但它是 **YYDB** 能够成功替代 **SQLite** 和 **Redis** 的核心技术动力：

- **针对 SQLite 的超越**：
  - **绕过文件系统**：SQLite 受限于 OS 文件系统的元数据锁。YYKV 直接管理磁盘块，消除了文件系统层的写入放大和并发限制。
  - **真正的行级并发**：基于 `yykv-allocator` 的精细控制，支持比 SQLite 粗犷的库级/表级锁更细粒度的并发写入。
- **针对 Redis 的挑战**：
  - **内存级磁盘吞吐**：利用 `io_uring` 和零拷贝数据路径，在 NVMe SSD 上实现接近内存访问的极低延迟。
  - **多模态原生存储**：不同于 Redis 简单的 KV 结构，YYKV 物理层原生支持 `YyValue`（List, Dict, Object），在磁盘上即可实现复杂数据结构的 $O(1)$ 随机访问。

## 典型工作场景 (Typical Scenarios)

- **AI 原生存储**：在存储层直接下推向量相似度计算算子，为 RAG（检索增强生成）提供极致的响应速度。
- **Serverless 边缘缓存**：作为 YYDB 的内核，以极小的体积随函数实例冷启动，提供比远程 Redis 快一个数量级的本地状态访问。
- **海量异构数据管理**：在单个集群中自动识别 NVMe (热数据) 与 HDD (冷数据)，实现基于访问热度的自动数据迁移，无需人工干预。

## 核心设计原则

1. **绕过文件系统 (Bypass FS)**：通过 `yykv-hal` 和 `yykv-allocator` 直接管理磁盘块，消除文件系统的元数据锁与日志冗余。
2. **算子下推插槽 (Pushdown Slots)**：在存储层内置 `yykv-compute`，支持在读取数据时直接进行过滤 (Filter) 与投影 (Project)，极大减少总线数据传输。
3. **I/O 调度优先级 (I/O Priority)**：`yykv-scheduler` 支持根据租户优先级和请求类型（如 WAL 写入 vs 背景 Compaction）进行细粒度的 I/O 调度。详见 [调度系统](file:///e:/yydb/yykv/documentation/zh-hans/guide/scheduler/index.md)。
