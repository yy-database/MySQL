# MySQL Compatibility Protocol

**we-trust-mysql** implements the MySQL Wire Protocol, allowing YYKV to transparently act as a MySQL server. This significantly reduces the migration cost for existing applications.

## 1. Layered Architecture

The MySQL compatibility layer exists as a **Transformation Adapter** for the Trust protocol:
1. **Listening Layer**: Listens for MySQL binary streams on a port (default `3306`).
2. **Decoder**: Disassembles MySQL `Packet` structures.
3. **Mapping Layer**: Translates MySQL commands (e.g., `COM_QUERY`) into YYKV internal `MessageType::Query` or `MessageType::KQL`.
4. **Encoder**: Packages the result sets into MySQL row data format for return.

## 2. Packet Structure

YYKV strictly follows the MySQL packet layout:

| Length | Field | Description |
| :--- | :--- | :--- |
| 3 Bytes | `Payload Length` | Length of the payload |
| 1 Byte | `Sequence ID` | Packet sequence number for replay protection and ordered acknowledgment |
| N Bytes | `Payload` | Actual command content (e.g., SQL text or auth info) |

## 3. Authentication and Handshake

YYKV implements the standard MySQL handshake flow:
- **Server Greeting**: Sends a Handshake packet containing protocol version, connection ID, and `auth-plugin-data`.
- **Client Response**: Provides username, tenant identifier (typically encoded in the database name), and SHA1/SHA256 hashed password.
- **Privilege Validation**: The adapter forwards authentication requests to YYKV's multi-tenant manager for authorization.

## 4. Core Command Mapping

| MySQL Command | Mapping to YYKV Operation | Description |
| :--- | :--- | :--- |
| `COM_QUERY` | `MessageType::KQL` | Parses SQL and converts it to KQL operators for execution |
| `COM_STMT_EXECUTE` | `MessageType::Query` | Uses prepared statements for improved execution efficiency |
| `COM_QUIT` | Close Connection | Gracefully releases resources |

## 5. Type System Mapping

| MySQL Type | YYKV (YYValue) Type | Description |
| :--- | :--- | :--- |
| `VARCHAR`, `TEXT` | `String` | UTF-8 encoded strings |
| `INT`, `BIGINT` | `Int` | 64-bit signed integers |
| `BLOB`, `BINARY` | `Binary` | Raw byte streams |
| `DECIMAL` | `Decimal` | High-precision numeric values |

## 6. Usage Recommendations

- **Connection String**: `mysql://user:pass@host:3306/tenant_id`
- **Driver Compatibility**: Supports all major drivers including `mysql-connector-java`, `mysql-client`, and `sqlx::mysql`.
