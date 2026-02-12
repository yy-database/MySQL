# yykv-event (WAE: Write-Ahead Event)

`yykv-event` 是 YYKV 存算一体化架构的核心，实现了 **WAE (写入即事件)** 机制。它不仅负责数据的持久化日志记录 (WAL)，更将每一次变更抽象为可实时订阅的事件流，支撑起“近数据计算” (Near-Data Computing) 的业务模型。

## 核心原语

### 1. WalEvent (逻辑事件)
代表一次业务层面的变更，包含：
- **租户 ID (Tenant ID)**: 实现物理级的多租户数据隔离。
- **SDR 级别 (Redundancy)**: 软件定义冗余，决定该事件的持久化策略。
- **操作类型 (OpType)**: Insert, Update, Delete。

### 2. WalEntry (物理条目)
经过高效二进制序列化后的磁盘布局：
| 字段 | 大小 | 说明 |
| :--- | :--- | :--- |
| Magic | 2B | 固定为 `0xYY` |
| Version | 1B | 协议版本 |
| SDR | 1B | 软件定义冗余等级 |
| RequestID | 16B | 全球唯一请求 ID (Uuid v7) |
| Checksum | 4B | 基于 CRC32C 的全条目校验 |

## 关键技术点

### ⚡ 写入即事件 (WAE)
传统的 WAL 仅用于崩溃恢复。`yykv-event` 通过 `broadcast` 通道将写入操作实时推送给订阅者（如 `yykv-executor` 中的 WAE Worker），使得计算逻辑可以在数据落盘的同时完成预处理。

### 🛡️ 软件定义冗余 (SDR)
支持在单次写入请求中指定冗余策略：
- `Single`: 追求极致性能，异步刷盘。
- `Quorum`: 确保多数派节点确认。
- `Strong`: 严格同步刷盘，最高可靠性。

### 🧩 高效序列化
利用 `yykv-types` 中的 `DsValueEncoder` 实现零拷贝友好的二进制转换，配合 `bytes` 库管理内存池，减少 GC 压力。

## 使用示例

```rust
let wal = WalManager::new(1024);
let mut rx = wal.subscribe();

// 写入数据并自动触发事件分发
wal.write(
    tenant_id,
    "users".into(),
    OpType::Insert,
    Redundancy::SINGLE,
    DsValue::Text("yykv-data".into()),
).await?;

// 订阅者实时处理
if let Ok(event) = rx.recv().await {
    println!("Received event for table: {}", event.table);
}
```

## 性能指标 (Target)
- **写入吞吐**: > 100k events/sec (Single Thread, NVMe)
- **分发延迟**: < 500μs (from write to subscriber)
