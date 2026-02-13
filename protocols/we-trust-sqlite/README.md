# SQLite Compatibility Protocol

**we-trust-sqlite** is one of the most complex and powerful compatibility modules in YYKV. It not only supports network access for SQLite but also implements direct read and write conversion for the SQLite database file format (B-tree, WAL) at the storage layer.

## 1. Core Architecture: Bidirectional Mapping

The SQLite module employs "Storage Format Emulation" technology:
- **Reader**: Directly parses SQLite's B-tree page layout, Schema definitions, and WAL (Write-Ahead Log) logs.
- **Writer**: Reconstructs YYKV operator graph execution results into Pager pages that comply with the SQLite specification.
- **Executor**: Maps SQLite's SQL commands to YYKV's `MessageType::KQL`, achieving a "Virtual SQLite" experience in a distributed environment.

## 2. Key Components

| Component | Function Description |
| :--- | :--- |
| `B-tree Engine` | Emulates SQLite's single-file index structure, supporting logical Table and Index mapping. |
| `Pager` | Page management module, responsible for converting YYKV's distributed pages into SQLite's fixed-size pages (default 4096 bytes). |
| `WAL Handler` | Implements a SQLite-compatible write-ahead logging mechanism, supporting multi-process concurrent reads. |
| `Schema Translator` | Converts SQLite's dynamic type system into YYKV's `YYValue` strong type system. |

## 3. Performance Optimization: Seamless Migration

- **Hot/Cold Data Separation**: Existing `.sqlite` files can be directly mounted, and YYKV will automatically extract hot data into memory.
- **Distributed SQLite**: Through YYKV's sharding mechanism, originally single-machine SQLite databases gain horizontal scalability.

## 4. Usage

- **Edge Computing**: Suitable for using the SQLite interface on edge nodes (e.g., IoT devices) while maintaining synchronization with the YYKV main cluster in the background.
- **Driver Compatibility**: Fully compatible with `rusqlite`, `sqlite3` CLI, and SQLite drivers for various programming languages.
