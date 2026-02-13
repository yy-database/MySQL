# yykv-event (WAE: Write-Ahead Event)

`yykv-event` is the core of YYKV's integrated storage-compute architecture, implementing the **WAE (Write-Ahead Event)** mechanism. It is responsible for not only persistent logging (WAL) but also abstracting every change into a real-time subscribable event stream, supporting the "Near-Data Computing" business model.

## Core Primitives

### 1. WalEvent (Logical Event)
Represents a business-level change, containing:
- **Tenant ID**: Implements physical-level multi-tenant data isolation.
- **SDR Level (Redundancy)**: Software-defined redundancy, determining the persistence strategy for the event.
- **Operation Type**: Insert, Update, Delete.

### 2. WalEntry (Physical Entry)
The binary serialized disk layout:
| Field | Size | Description |
| :--- | :--- | :--- |
| Magic | 2B | Fixed as `0xYY` |
| Version | 1B | Protocol version |
| SDR | 1B | Software-defined redundancy level |
| RequestID | 16B | Globally unique request ID (Uuid v7) |
| Checksum | 4B | CRC32C-based full-entry checksum |

## Key Technical Points

### ⚡ Write-Ahead Event (WAE)
Traditional WAL is only used for crash recovery. `yykv-event` pushes write operations in real-time to subscribers (like WAE Workers in `yykv-executor`) via a `broadcast` channel, enabling compute logic to complete preprocessing while data is being flushed to disk.

### 🛡️ Software-Defined Redundancy (SDR)
Supports specifying redundancy policies per write request:
- `Single`: Pursues extreme performance with asynchronous flushing.
- `Quorum`: Ensures confirmation from a majority of nodes.
- `Strong`: Strict synchronous flushing for maximum reliability.

### 🧩 Efficient Serialization
Utilizes `DsValueEncoder` from `yykv-types` for zero-copy-friendly binary conversion, combined with the `bytes` library for memory pool management to reduce GC pressure.

## Usage Example

```rust
let wal = WalManager::new(1024);
let mut rx = wal.subscribe();

// Write data and automatically trigger event distribution
wal.write(
    tenant_id,
    "users".into(),
    OpType::Insert,
    Redundancy::SINGLE,
    DsValue::Text("yykv-data".into()),
).await?;

// Subscriber handles events in real-time
if let Ok(event) = rx.recv().await {
    println!("Received event for table: {}", event.table);
}
```

## Performance Targets
- **Write Throughput**: > 100k events/sec (Single Thread, NVMe)
- **Distribution Latency**: < 500μs (from write to subscriber)
