# PostgreSQL Compatibility Protocol

**we-trust-postgres** implements the PostgreSQL Frontend/Backend Wire Protocol, enabling YYKV to integrate seamlessly with the PostgreSQL ecosystem.

## 1. Protocol Flow

1. **Startup Phase**: Client sends a `StartupMessage` containing protocol version and parameters (e.g., tenant identity as `user=tenant_id`).
2. **Authentication Phase**: Supports `CleartextPassword` and `MD5Password` (though native Trust signatures are recommended).
3. **Command Loop (Simple/Extended Query)**:
   - **Simple Query**: Client sends plain text query messages.
   - **Extended Query**: Consists of `Parse`, `Bind`, `Describe`, and `Execute` phases, supporting parameter binding and high-performance repeated execution.

## 2. Message Format

Each message consists of a **1-byte identifier**, a **4-byte length**, and the **Payload**:

| Identifier | Meaning |
| :--- | :--- |
| `'Q'` | Simple Query |
| `'P'` | Parse (Prepare) |
| `'B'` | Bind (Parameters) |
| `'E'` | Execute |
| `'C'` | Command Complete |

## 3. Core Mapping Technologies

### 3.1 Extended Query Mapping
Postgres extended query protocols are mapped to YYKV `MessageType::Query`:
- **`Parse`**: Generates an execution plan cache within YYKV.
- **`Bind`**: Maps binary parameters transmitted over the network into `YYValue` sequences.
- **`Execute`**: Triggers the asynchronous execution stream of the operator graph (OpsGraph).

### 3.2 Error Code Compatibility
YYKV translates internal storage errors into standard Postgres `SQLSTATE` error codes (e.g., `23505` for unique constraint violations), ensuring ORMs like Hibernate or Diesel can handle exceptions correctly.

## 4. Type System Alignment

| Postgres OID | YYKV Type | Description |
| :--- | :--- | :--- |
| `INT4`, `INT8` | `Int` | Mapped to 64-bit integers |
| `TEXT`, `VARCHAR` | `String` | UTF-8 encoded |
| `BYTEA` | `Binary` | Raw byte blocks |
| `BOOL` | `Bool` | Boolean values |

## 5. Featured Feature: Operator Pushdown

When using a Postgres client, complex filtering conditions (WHERE clauses) are identified by the compatibility layer and pushed down to the storage nodes using `yykv-compute`, returning only matching rows to save network bandwidth.
