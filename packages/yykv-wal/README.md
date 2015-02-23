# yykv-wal (Write-Ahead Log)

`yykv-wal` 实现了 YYKV 存储引擎的基础预写日志机制。它是确保系统在发生崩溃或异常重启后，内存中的数据（如 MemTable）能够完整恢复的关键组件。

## 核心功能

### 🛡️ 崩溃恢复保证
在任何变更（Put/Delete）应用到主存储结构之前，先将其持久化到 WAL 文件中。
- **持久性 (Durability)**: 支持 `flush` 操作，确保数据真正落入物理磁盘。
- **自动回放 (Replay)**: 系统启动时自动扫描 WAL 文件，按顺序重构丢失的内存状态。

### ⚡ 顺序 I/O 优化
- **仅追加模式 (Append-Only)**: 利用磁盘的顺序写入特性，最大化写入吞吐量。
- **长度前缀编码**: 采用 `[Length][Data]` 的高效二进制布局。

### 🧩 多租户支持
日志条目（`WalEntry`）内置 `tenant_id`，支持按租户进行细粒度的数据恢复与审计。

## 核心设计

- `WalEntry`: 支持 `Put` 和 `Delete` 操作的枚举定义。
- `WalManager`: 负责日志文件的打开、追加、同步与回放。

## 使用示例

```rust
use yykv_wal::{WalManager, WalEntry};

let mut wal = WalManager::new(PathBuf::from("data.wal")).await?;

// 记录操作
wal.append(WalEntry::Put {
    tenant_id: 1,
    key: "user:123".to_string(),
    value: DsValue::Text("info".into()),
    sdr: Redundancy::Strong,
}).await?;

// 系统崩溃后重启
let recovered_entries = wal.replay().await?;
```

## 与 WAE 的关系
- **WAL**: 侧重于传统的崩溃恢复与数据持久性。
- **WAE (yykv-event)**: 侧重于实时的事件分发与近数据计算下推。
在 YYKV 中，两者可以配合使用，WAL 作为底层物理保障，WAE 作为上层业务驱动。
