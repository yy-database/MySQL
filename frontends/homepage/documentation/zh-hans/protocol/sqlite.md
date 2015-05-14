# SQLite 兼容协议详解

**we-trust-sqlite** 是 YYKV 中最复杂且功能最强大的兼容模块之一。它不仅支持 SQLite 的网络访问，更在底层实现了对 SQLite 数据库文件格式 (B-tree, WAL) 的直接读取与写入转换。

## 1. 核心架构：双向映射

SQLite 模块采用了“存储格式模拟”技术：
- **读取层 (Reader)**: 能够直接解析 SQLite 的 B-tree 页面布局、Schema 定义和 WAL (Write-Ahead Log) 日志。
- **写入层 (Writer)**: 将 YYKV 的算子图执行结果重构为符合 SQLite 规范的 Pager 页面。
- **转换层 (Executor)**: 将 SQLite 的 SQL 指令映射为 YYKV 的 `MessageType::KQL`，实现分布式环境下的“虚拟 SQLite”体验。

## 2. 关键组件

| 组件 | 功能描述 |
| :--- | :--- |
| `B-tree Engine` | 模拟 SQLite 的单文件索引结构，支持逻辑上的 Table 和 Index 映射。 |
| `Pager` | 页面管理模块，负责将 YYKV 的分布式 Page 转换为 SQLite 的固定大小页面（默认 4096 字节）。 |
| `WAL Handler` | 实现与 SQLite 兼容的预写日志机制，支持多进程并发读取。 |
| `Schema Translator` | 将 SQLite 的动态类型系统转换为 YYKV 的 `YYValue` 强类型系统。 |

## 3. 性能优化：无缝迁移

- **冷热分离**: 可以直接挂载现有的 `.sqlite` 文件，YYKV 会自动将其热点数据提取到内存中。
- **分布式 SQLite**: 通过 YYKV 的分片机制，使原本单机的 SQLite 具备了横向扩展能力。

## 4. 使用建议

- **边缘计算**: 适用于在边缘节点（如 IoT 设备）使用 SQLite 接口，同时后台与 YYKV 主集群保持同步。
- **驱动**: 完美兼容 `rusqlite`, `sqlite3` 命令行工具以及各种语言的 SQLite 驱动。
