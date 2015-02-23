# Redis Compatibility Protocol

**we-trust-redis** implements the REdis Serialization Protocol (RESP), allowing YYKV to serve as a high-performance, persistent alternative to Redis.

## 1. Core Protocol: RESP

YYKV fully supports all data types in the RESP protocol:
- **Simple Strings**: Starts with `+`, e.g., `+OK\r\n`.
- **Errors**: Starts with `-`, containing the error type.
- **Integers**: Starts with `:`.
- **Bulk Strings**: Starts with `$`, supporting binary-safe data.
- **Arrays**: Starts with `*`, supporting nested structures.

## 2. Command Mapping Logic

The Redis compatibility layer uses a **Dynamic Mapping Engine** to translate common KV operations directly into native YYKV atomic instructions, bypassing SQL parsing overhead.

| Redis Command | YYKV Native Operation | Performance |
| :--- | :--- | :--- |
| `SET` | `MessageType::Put` | $O(1)$, writes directly to WAL and MemTable |
| `GET` | `MessageType::Get` | $O(1)$, supports cache hit optimization |
| `DEL` | `MessageType::Delete` | $O(1)$, writes a deletion marker (Tombstone) |
| `KEYS` | `MessageType::Query` | $O(N)$, leverages prefix scan optimizations |

## 3. Persistence and Consistency

Unlike Redis's default AOF/RDB, YYKV provides **Database-Level Persistence Guarantees** for the Redis protocol:
- **Strong WAL Sync**: Every `SET` operation ensures the log is persisted to disk before returning.
- **Multi-Level Tiering**: Hot data stays in memory/SSD, while cold data is automatically moved to HDD, breaking the memory capacity limits of Redis.

## 4. Multi-Tenant Extensions

YYKV extends the Redis protocol non-destructively:
- **Tenant Prefixing**: Physical isolation is handled internally via Tenant IDs, transparent to the client.
- **Enhanced AUTH**: Supports fine-grained tenant-level authentication via `AUTH <tenant_id> <password>`.

## 5. Performance Advantages

- **Zero-Copy Path**: Uses the `bytes` crate to mount data directly from network buffers into the storage engine, avoiding frequent `sds` string copies internal to Redis.
- **Multi-Core Concurrency**: Leverages the Rust `tokio` scheduler to utilize multiple CPU cores for Redis requests, overcoming the single-threaded bottlenecks of Redis.
