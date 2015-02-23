# SQLite Compatibility Protocol

**we-trust-sqlite** is one of the most complex and powerful compatibility modules in YYKV. It not only supports network access to SQLite but also implements direct read/write conversion of SQLite database file formats (B-tree, WAL) at the storage level.

## 1. Core Architecture: Bi-directional Mapping

The SQLite module uses "Storage Format Simulation" technology:
- **Reader Layer**: Directly parses SQLite B-tree page layouts, schema definitions, and WAL (Write-Ahead Log) entries.
- **Writer Layer**: Reconstructs the results of YYKV operator graph execution into Pager pages compliant with SQLite specifications.
- **Execution Layer**: Maps SQLite SQL instructions to YYKV `MessageType::KQL`, providing a "Virtual SQLite" experience in a distributed environment.

## 2. Key Components

| Component | Description |
| :--- | :--- |
| `B-tree Engine` | Simulates SQLite's single-file index structure, supporting logical Table and Index mapping. |
| `Pager` | Page management module that converts YYKV distributed pages into SQLite fixed-size pages (default 4096 bytes). |
| `WAL Handler` | Implements a Write-Ahead Log mechanism compatible with SQLite, supporting concurrent multi-process reads. |
| `Schema Translator` | Converts SQLite's dynamic type system into YYKV's `YYValue` strong type system. |

## 3. Performance Optimization: Seamless Migration

- **Hot/Cold Tiering**: Existing `.sqlite` files can be mounted directly; YYKV automatically extracts hot data into memory.
- **Distributed SQLite**: Enables horizontal scalability for otherwise single-machine SQLite via YYKV's sharding mechanism.

## 4. Usage Recommendations

- **Edge Computing**: Ideal for using SQLite interfaces on edge nodes (e.g., IoT devices) while maintaining synchronization with the main YYKV cluster.
- **Drivers**: Fully compatible with `rusqlite`, the `sqlite3` CLI, and standard SQLite drivers in various languages.
