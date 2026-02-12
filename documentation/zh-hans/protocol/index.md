# 协议说明

YYKV 支持多种通信协议，包括高性能的原生二进制协议和广泛使用的数据库兼容协议。

## 原生协议 (Native Protocol)

- **[We-Trust](./we-trust.md)**: YYKV 的核心二进制通信协议，专为极致性能和多租户环境设计。

## 数据库兼容协议 (Compatibility Protocols)

YYKV 通过兼容层支持多种主流数据库协议，允许您使用现有的驱动和工具直接连接。

### 关系型数据库
- **[PostgreSQL](./postgres.md)**: 实现了 Postgres 线缆协议，支持标准客户端连接。
- **[MySQL](./mysql.md)**: 提供 MySQL 协议适配，兼容现有驱动。
- **[SQLite](./sqlite.md)**: 支持 SQLite 格式的数据访问与转换。
- **[SQL Server](./sqlserver.md)**: 提供对 TDS 协议的基础支持。

### NoSQL 与对象存储
- **[Redis](./redis.md)**: 兼容 Redis 协议，支持常见的 KV 操作。
- **[S3](./s3.md)**: 提供 S3 接口兼容，支持大对象读写。

## 扩展与专用协议

- **[Arrow](./arrow.md)**: Apache Arrow 向量化数据交换。
- **[Vector](./vector.md)**: 针对向量检索优化的扩展协议。
- **[OTEL](./otel.md)**: OpenTelemetry 全链路观测支持。
- **[D1](./d1.md)**: Cloudflare D1 HTTP 查询接口兼容。
