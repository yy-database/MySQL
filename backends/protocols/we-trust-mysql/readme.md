# MySQL Compatibility Protocol

**we-trust-mysql** implements the MySQL Wire Protocol, allowing YYKV to act transparently as a MySQL server. This significantly reduces the migration cost for existing applications.

## 1. Protocol Layer Architecture

The MySQL compatibility layer acts as a **conversion adapter** for the Trust protocol:
1. **Listening Layer**: Listens for MySQL binary streams on a port (default `3306`).
2. **Decoder**: Disassembles MySQL `Packet` structures.
3. **Mapping Layer**: Converts MySQL commands (e.g., `COM_QUERY`) into YYKV internal `MessageType::Query` or `MessageType::KQL`.
4. **Encoder**: Encapsulates ResultSets into MySQL row data format for the response.

## 2. Packet Structure

YYKV strictly follows the MySQL packet layout:

| Length | Field | Description |
| :--- | :--- | :--- |
| 3 Bytes | `Payload Length` | Length of the payload |
| 1 Byte | `Sequence ID` | Packet sequence number, used for anti-replay and ordered confirmation |
| N Bytes | `Payload` | Specific command content (e.g., SQL text or authentication info) |

## 3. Handshake & Authentication

YYKV implements the standard MySQL handshake process:
- **Server Challenge**: Sends a Handshake packet containing the protocol version, connection ID, and `auth-plugin-data`.
- **Client Response**: Provides the username, tenant identifier (usually encoded in the database name), and a SHA1/SHA256 hashed password.
- **Permission Verification**: The compatibility layer forwards the authentication request to YYKV's multi-tenant manager for authorization.

## 4. Execution Flow

```mermaid
sequenceDiagram
    Client->>we-trust-mysql: SQL Query (COM_QUERY)
    we-trust-mysql->>yykv-operators: Parse & Translate to OpsGraph
    yykv-operators->>yykv-executor: Execute Distributed Task
    yykv-executor->>yykv-event: WAE Persistence
    yykv-executor-->>we-trust-mysql: ResultSet (YYValue)
    we-trust-mysql-->>Client: MySQL Protocol Row Packets
```

## 5. Core Command Mapping

| MySQL Command | Mapped to YYKV Operation | Description |
| :--- | :--- | :--- |
| `COM_QUERY` | `MessageType::KQL` | Parses SQL statements and converts them to KQL operators |
| `COM_STMT_EXECUTE` | `MessageType::Query` | Uses prepared statements to improve execution efficiency |
| `COM_QUIT` | Close Connection | Gracefully releases resources |

## 6. Type System Mapping

| MySQL Type | YYKV (YYValue) Type | Description |
| :--- | :--- | :--- |
| `VARCHAR`, `TEXT` | `String` | UTF-8 encoded string |
| `INT`, `BIGINT` | `Int` | 64-bit signed integer |
| `BLOB`, `BINARY` | `Binary` | Raw byte stream |
| `DECIMAL` | `Decimal` | High-precision numeric |

## 7. Usage

- **Connection String**: `mysql://user:pass@host:3306/tenant_id`
- **Driver Compatibility**: Supports all major drivers including `mysql-connector-java`, `mysql-client`, `sqlx::mysql`, etc.
