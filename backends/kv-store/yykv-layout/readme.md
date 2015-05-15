# yykv-layout (Physical Data Layout)

`yykv-layout` defines the physical organizational structure of the YYKV storage engine on raw disk media. It is responsible for serializing complex in-memory data objects (`DsValue`) into compact, aligned, and self-verifying binary page streams, serving as the cornerstone for the storage engine's high performance and reliability.

## Core Design

### 📑 Unified Page Model
YYKV does not rely on a file system; instead, it manages physical pages directly:
- **Page Specifications**: Supports various sizes like 4KB (SSD optimized) and 16KB (throughput optimized).
- **Page Types**:
  - `DataPage`: Stores actual KV or row records.
  - `IndexPage`: Stores B-Tree or HNSW index nodes.
  - `MetaPage`: Stores tenant metadata and system configurations.

### 📦 Variable-Length Binary Storage
- **Compact Serialization**: Binary layouts optimized for different data types (Varint, Bit-packing).
- **Zero-Copy Reading**: Designed to allow deserialization directly on `AlignedBuffer`, avoiding memory copies.

### 🛡️ Data Integrity Protection
- **CRC32C Checksum**: Each page header includes a cyclic redundancy check to prevent silent data corruption caused by hardware failure.
- **Version Control**: The page header contains version information, supporting smooth online upgrades of storage formats.

## Physical Layout

| Region | Description |
| :--- | :--- |
| **File Header (100B)** | Contains SQLite-compatible magic number, version, page size, and first free page pointer. |
| **Page Header** | Page type (1B), flags (1B), checksum (4B), Lsn (8B). |
| **Page Body** | Actual record slots or B-Tree node entries. |
| **Page Trailer** | Page footer pointer for assisting page scanning and corruption detection. |

## Core Components

- `LayoutManager`: Manages the conversion between memory objects and disk byte streams.
- `Header`: Initial metadata parser for physical files/drives.
- `Page`: Abstract page object supporting parsing of various physical page types.

## Technical Advantages
- **Hardware-Friendly**: All data layouts strictly follow sector alignment, perfectly matching Direct IO.
- **Cross-Modal Consistency**: Both vector data and SQL row data follow a unified page management framework.
