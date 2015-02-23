# we-trust (High-Performance Binary Protocol)

`we-trust` 是 YYKV 存储引擎的核心二进制通信协议。它被设计用于微秒级的组件间通信（RPC），并在协议头中深度集成了多租户隔离、请求追踪与安全签名机制。

## 核心设计

### ⚡ 极速二进制编解码
- **低开销**: 采用紧凑的二进制布局，相比文本协议（如 JSON）或通用的 Protobuf，在特定场景下具有更低的序列化延迟。
- **异步流式处理**: 基于 `tokio-util` 的 `Codec` 框架，支持大数据的流式传输与背压控制。

### 🏢 多租户原生支持
- **租户上下文**: 协议头内置 16 字节 `TenantID`，确保每一条消息都携带身份信息，支撑物理级的资源隔离与精确计费。
- **请求追踪**: 16 字节 `RequestID` (Uuid v7) 贯穿整个处理链路，方便分布式追踪。

### 🛡️ 安全通信框架
- **消息签名**: 支持 32 字节的 HMAC 或数字签名字段，防止消息篡改，确保集群内部节点间通信的安全性。

## 协议布局 (80 Bytes Header)

| 字段 | 大小 | 说明 |
| :--- | :--- | :--- |
| Magic Number | 2B | 固定为 `0x54 0x52` (TR) |
| Version | 1B | 协议版本号 |
| Message Type | 1B | 请求、响应、通知或心跳 |
| Flags | 4B | 压缩、加密、优先级等标志位 |
| Total Length | 4B | 包含头部的整包字节长度 |
| Request ID | 16B | 全球唯一请求追踪 ID |
| Tenant ID | 16B | 租户空间标识符 |
| Signature | 32B | 消息完整性校验签名 |
| Reserved | 4B | 预留未来扩展 |

## 核心组件

- `TrustCodec`: 实现了 `tokio_util::codec::Encoder` 和 `Decoder`。
- `Header`: 协议头部的序列化与反序列化逻辑。
- `MessageType`: 定义了协议支持的所有操作类型（Query, Mutation, Admin 等）。

## 技术优势
- **微秒级解析**: 头部定长 80 字节，解析开销极低，适合高性能存储场景。
- **存算协同**: 协议直接映射到 `yykv-operators`，减少了中间层的转换成本。
