# Write-Ahead Log (WAL)

**yykv-wal** implements the fundamental write-ahead logging mechanism for the YYKV storage engine. It is a critical component that ensures data in memory (such as MemTable) can be fully recovered after a system crash or abnormal restart.

## 1. Core Features

### 🛡️ Crash Recovery Guarantee
Before any changes (Put/Delete) are applied to the main storage structure, they are first persisted into the WAL file.
- **Durability**: Supports `flush` operations to ensure data is truly persisted to physical disk.
- **Automatic Replay**: Automatically scans WAL files upon system startup to reconstruct lost in-memory state in sequence.

### ⚡ Sequential I/O Optimization
- **Append-Only Mode**: Leverages the sequential write characteristics of disks to maximize write throughput.
- **Length-Prefix Encoding**: Uses an efficient binary layout of `[Length][Data]`.

### 🧩 Multi-Tenant Support
Log entries (`WalEntry`) have a built-in `tenant_id`, supporting fine-grained data recovery and auditing per tenant.

## 2. Core Design

- `WalEntry`: Enumeration definition supporting `Put` and `Delete` operations.
- `WalManager`: Responsible for opening, appending, synchronizing, and replaying log files.

## 3. Usage Example

```rust
use yykv_wal::{WalManager, WalEntry};

let mut wal = WalManager::new(PathBuf::from("data.wal")).await?;

// Record operation
wal.append(WalEntry::Put {
    tenant_id: 1,
    key: "user:123".to_string(),
    value: DsValue::Text("info".into()),
    sdr: Redundancy::Strong,
}).await?;

// Restart after system crash
let recovered_entries = wal.replay().await?;
```

## 4. Relationship with WAE

- **WAL**: Focuses on traditional crash recovery and data durability.
- **WAE (yykv-event)**: Focuses on real-time event distribution and near-data computing pushdown.
In YYKV, the two can be used together, with WAL as the underlying physical guarantee and WAE as the upper-level business driver.
