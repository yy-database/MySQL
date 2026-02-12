# We-Trust Protocol Details

**We-Trust** is the native high-performance binary communication protocol of YYKV, specifically designed for ultra-low latency and large-scale multi-tenant environments. It sits at the communication layer of the storage engine, facilitating efficient data exchange between nodes and between clients and servers.

## 1. Design Goals

- **Microsecond Latency**: Uses a compact binary layout to minimize encoding/decoding overhead and support zero-copy parsing.
- **Native Multi-tenancy**: Deeply integrates tenant identifiers in the protocol header, supporting physical-level traffic control and resource isolation.
- **Strong Security**: Every packet includes a cryptographic signature to prevent man-in-the-middle attacks and unauthorized access.
- **Streaming Support**: Works with the `tokio-util` `Codec` framework to support streaming scans and backpressure control for massive data sets.

## 2. Packet Structure (80-Byte Header)

A We-Trust message consists of an **80-byte fixed header** followed by an **optional payload**.

### 2.1 Header Layout

| Offset | Length | Field | Description |
| :--- | :--- | :--- | :--- |
| 0 | 2 | `Magic` | Protocol magic bytes, fixed at `0x59 0x59` (ASCII "YY") |
| 2 | 1 | `Version` | Protocol version, currently `0x01` |
| 3 | 1 | `MsgType` | Message type (1: Put, 2: Get, 4: Query, 12: KQL, etc.) |
| 4 | 4 | `Flags` | Control flags (bitmask: 0x01-Compression, 0x02-Encryption, 0x04-High Priority) |
| 8 | 4 | `Length` | Total length of the message (Header + Payload) in bytes |
| 12 | 16 | `RequestID` | Unique Request ID (UUID) for asynchronous response matching |
| 28 | 16 | `TenantID` | Tenant ID (UUID) for physical resource routing in the storage engine |
| 44 | 32 | `Signature` | HMAC-SHA256 signature of the header and payload |
| 76 | 4 | `Reserved` | Reserved for future extensions |

### 2.2 Message Types (`MessageType`)

- **`0x01` Put**: Write or update key-value pairs.
- **`0x02` Get**: Retrieve key-value pairs.
- **`0x04` Query**: Execute complex operator graphs (OpsGraph).
- **`0x05` Rbq**: Reliable Byte Queue operations as defined in the Architecture Whitepaper.
- **`0x0C` KQL**: Execute native YYKV Query Language instructions.
- **`0x0B` Heartbeat**: Liveness detection between nodes.
- **`0xFF` Error**: Error response.

## 3. Connection and Authentication Flow

1. **Establish Connection**: Client establishes a TCP connection (default port `8889`).
2. **Handshake/Auth**: Client sends an `MessageType::Auth` message containing the `TenantID` and an encrypted `SecretKey`.
3. **Establish Session**: Server verifies the signature and returns a `Response`. All subsequent packets must carry a valid `Signature` for real-time validation.

## 4. Performance Optimization Features

- **CRC32C Checksum**: Optimized cyclic redundancy check for physical storage media.
- **Zero-Copy Parsing**: Strictly aligned header fields allow reading structures directly from the buffer, avoiding memory copies.
- **Pipelining**: Supports sending multiple requests over a single connection without waiting for individual responses.
