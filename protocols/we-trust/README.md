# we-trust (High-Performance Binary Protocol)

`we-trust` is the core binary communication protocol of the YYKV storage engine. It is designed for microsecond-level inter-component communication (RPC) and deeply integrates multi-tenant isolation, request tracking, and security signature mechanisms into the protocol header.

## Core Design

### ⚡ Ultra-Fast Binary Codec
- **Low Overhead**: Adopts a compact binary layout, featuring lower serialization latency in specific scenarios compared to text protocols (like JSON) or general Protobuf.
- **Async Streaming**: Based on the `tokio-util` `Codec` framework, supporting streaming of large data and backpressure control.

### 🏢 Native Multi-Tenant Support
- **Tenant Context**: The protocol header includes a 16-byte `TenantID`, ensuring every message carries identity information, supporting physical-level resource isolation and precise billing.
- **Request Tracking**: A 16-byte `RequestID` (Uuid v7) runs through the entire processing chain, facilitating distributed tracing.

### 🛡️ Secure Communication Framework
- **Message Signature**: Supports 32-byte HMAC or digital signature fields to prevent message tampering and ensure the security of communication between nodes within the cluster.

## Protocol Layout (80 Bytes Header)

| Field | Size | Description |
| :--- | :--- | :--- |
| Magic Number | 2B | Fixed as `0x54 0x52` (TR) |
| Version | 1B | Protocol version number |
| Message Type | 1B | Request, Response, Notification, or Heartbeat |
| Flags | 4B | Compression, encryption, priority, etc. |
| Total Length | 4B | Byte length of the entire packet including the header |
| Request ID | 16B | Globally unique request tracking ID |
| Tenant ID | 16B | Tenant space identifier |
| Signature | 32B | Message integrity verification signature |
| Reserved | 4B | Reserved for future expansion |

## Core Components

- `TrustCodec`: Implements `tokio_util::codec::Encoder` and `Decoder`.
- `Header`: Serialization and deserialization logic for the protocol header.
- `MessageType`: Defines all operation types supported by the protocol (Query, Mutation, Admin, etc.).

## Technical Advantages
- **Microsecond Parsing**: The header is a fixed 80 bytes, with extremely low parsing overhead, suitable for high-performance storage scenarios.
- **Storage-Compute Synergy**: The protocol maps directly to `yykv-operators`, reducing conversion costs in intermediate layers.
