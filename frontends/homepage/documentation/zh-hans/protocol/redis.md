# Redis 兼容协议详解

**we-trust-redis** 实现了 Redis 序列化协议 (RESP)，使 YYKV 可以作为高性能、可持久化的 Redis 替代方案。

## 1. 协议核心：RESP

YYKV 完整支持 RESP (REdis Serialization Protocol) 的各种数据类型：
- **Simple Strings**: 以 `+` 开头，如 `+OK\r\n`。
- **Errors**: 以 `-` 开头，包含错误类型。
- **Integers**: 以 `:` 开头。
- **Bulk Strings**: 以 `$` 开头，支持二进制安全。
- **Arrays**: 以 `*` 开头，支持嵌套结构。

## 2. 指令映射逻辑

Redis 兼容层采用**动态映射引擎**，将常见的 KV 操作直接转换为 YYKV 的原生原子指令，绕过 SQL 解析开销。

| Redis 命令 | YYKV 原生操作 | 性能表现 |
| :--- | :--- | :--- |
| `SET` | `MessageType::Put` | $O(1)$，直接写入 WAL 和 MemTable |
| `GET` | `MessageType::Get` | $O(1)$，支持缓存命中优化 |
| `DEL` | `MessageType::Delete` | $O(1)$，写入删除标记 (Tombstone) |
| `KEYS` | `MessageType::Query` | $O(N)$，利用前缀扫描优化 |

## 3. 持久化与一致性

不同于 Redis 默认的 AOF/RDB，YYKV 为 Redis 协议提供了**数据库级别的持久化保证**：
- **WAL 强同步**: 每次 `SET` 操作在返回前确保日志已落盘。
- **多级分层**: 热数据在内存/SSD，冷数据自动沉降至 HDD，突破 Redis 的内存容量限制。

## 4. 多租户扩展

YYKV 在 Redis 协议基础上进行了非破坏性扩展：
- **租户前缀**: 内部通过租户 ID 进行物理隔离，客户端无需感知。
- **AUTH 增强**: 支持通过 `AUTH <tenant_id> <password>` 进行租户级别的细粒度认证。

## 5. 性能优势

- **零拷贝路径**: 利用 `bytes` crate 直接将网络缓冲区的数据挂载到存储引擎，避免了 Redis 内部频繁的 `sds` 字符串拷贝。
- **多核并发**: 借助 Rust 的 `tokio` 调度器，YYKV 能够充分利用多核 CPU 处理 Redis 请求，克服了 Redis 单线程模型的瓶颈。
