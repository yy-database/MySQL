# Protocols

YYKV supports multiple communication protocols, including high-performance native binary protocols and widely used database compatibility protocols.

## Native Protocol

- **[We-Trust](./we-trust.md)**: The core binary communication protocol of YYKV, designed for extreme performance and multi-tenant environments.

## Compatibility Protocols

YYKV supports several mainstream database protocols through compatibility layers, allowing you to connect directly using existing drivers and tools.

### Relational Databases
- **[PostgreSQL](./postgres.md)**: Implements the Postgres wire protocol for standard client connections.
- **[MySQL](./mysql.md)**: Provides MySQL protocol adaptation, compatible with existing drivers.
- **[SQLite](./sqlite.md)**: Supports SQLite format data access and conversion.
- **[SQL Server](./sqlserver.md)**: Provides basic support for the TDS protocol.

### NoSQL & Object Storage
- **[Redis](./redis.md)**: Compatible with the Redis protocol, supporting common KV operations.
- **[S3](./s3.md)**: Provides S3 interface compatibility for large object read/write.

## Extensions & Specialized Protocols

- **[Arrow](./arrow.md)**: Apache Arrow vectorized data exchange.
- **[Vector](./vector.md)**: Protocol extensions optimized for vector retrieval.
- **[OTEL](./otel.md)**: Full-link observability support with OpenTelemetry.
- **[D1](./d1.md)**: Compatibility with Cloudflare D1 HTTP query interfaces.
