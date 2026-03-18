# We-Trust Protocol

**We-Trust** is YYKV's native high-performance binary communication protocol, specifically designed for ultra-low latency and large-scale multi-tenant environments. It is located at the communication layer of the storage engine and is responsible for efficient data exchange between nodes, clients, and servers.

## 1. Design Goals

- **Microsecond Latency**: Employs a compact binary layout to minimize encoding/decoding overhead and support zero-copy parsing.
- **Native Multi-Tenancy**: Deeply integrates tenant identification in the protocol header, supporting physical-level traffic control and resource isolation.
- **Strong Security**: Every packet includes an encrypted signature to prevent man-in-the-middle attacks and unauthorized access.
- **Streaming Support**: Works with the `tokio-util` `Codec` framework to support streaming scanning and backpressure control for massive data.

## 2. Message Structure (80-Byte Header)

A We-Trust message consists of an **80-byte fixed header** and an **optional Payload**.

### 2.1 Header Layout

| Offset | Length | Field Name | Description |
| :--- | :--- | :--- | :--- |
| 0 | 2 | `Magic` | Protocol magic number, fixed as `0x59 0x59` (ASCII "YY") |
| 2 | 1 | `Version` | Protocol version, currently `0x01` |
| 3 | 1 | `MsgType` | Message type (1: Put, 2: Get, 4: Query, 12: KQL, etc.) |
| 4 | 4 | `Flags` | Control flags (bitmask: 0x01-Compression, 0x02-Encryption, 0x04-High Priority) |
| 8 | 4 | `Length` | Total byte length of the message (including header and payload) |
| 12 | 16 | `RequestID` | Unique request ID (UUID) for asynchronous response matching |
| 28 | 16 | `TenantID` | Tenant ID (UUID), used by the storage engine for physical resource routing |
| 44 | 32 | `Signature` | HMAC-SHA256 based signature of the header and payload |
| 76 | 4 | `Reserved` | Reserved field for future extensions |

### 2.2 Message Types (`MessageType`)

- **`0x01` Put**: Write/Update key-value pairs.
- **`0x02` Get**: Retrieve key-value pairs.
- **`0x04` Query**: Execute complex operator graph queries (OpsGraph).
- **`0x05` Rbq**: Reliable Byte Queue operations as defined in the Architecture White Paper.
- **`0x0C` KQL**: Execute native YYKV Query Language instructions.
- **`0x0B` Heartbeat**: Liveness detection between nodes.
- **`0xFF` Error**: Error response.

## 3. Connection & Authentication Flow

1. **Establish Connection**: Client establishes a TCP connection (default port `8889`).
2. **Handshake Authentication**: Client sends a `MessageType::Auth` message containing `TenantID` and an encrypted `SecretKey`.
3. **Establish Session**: Server validates the signature and returns a `Response`. All subsequent messages must carry a valid `Signature` for real-time verification.

## 4. Performance Optimization Features

- **CRC32C Checksum**: Cyclic redundancy check optimized for physical media.
- **Zero-Copy Parsing**: Strictly aligned header fields support direct reading of structures from the buffer, avoiding memory copies.
- **Pipelining Support**: Supports sending multiple requests over a single connection without waiting for individual responses.
