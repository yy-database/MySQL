# PostgreSQL 兼容协议详解

**we-trust-postgres** 实现了 PostgreSQL 的前端/后端线缆协议 (Frontend/Backend Protocol)，使 YYKV 能够与 PostgreSQL 生态系统完美融合。

## 1. 协议流程

1. **启动阶段 (Startup)**: 客户端发送 `StartupMessage`，包含协议版本和参数（如租户标识 `user=tenant_id`）。
2. **认证阶段 (Authentication)**: 支持 `CleartextPassword` 和 `MD5Password`（推荐使用 Trust 协议原生的签名认证）。
3. **命令循环 (Simple/Extended Query)**:
   - **Simple Query**: 客户端发送文本查询报文。
   - **Extended Query**: 包含 `Parse`, `Bind`, `Describe`, `Execute` 四阶段，支持参数绑定和高性能重复执行。

## 2. 报文格式

每个报文由 **1 字节标识符**、**4 字节长度** 和 **Payload** 组成：

| 标识符 | 含义 |
| :--- | :--- |
| `'Q'` | 简单查询 (Simple Query) |
| `'P'` | 预编译 (Parse) |
| `'B'` | 绑定参数 (Bind) |
| `'E'` | 执行 (Execute) |
| `'C'` | 命令完成 (Command Complete) |

## 3. 核心映射技术

### 3.1 扩展查询映射
Postgres 的扩展查询协议被映射为 YYKV 的 `MessageType::Query`：
- **`Parse`**: 在 YYKV 内部生成执行计划缓存。
- **`Bind`**: 将网络传输的二进制参数映射为 `YYValue` 序列。
- **`Execute`**: 触发算子图 (OpsGraph) 的异步执行流。

### 3.2 错误码兼容
YYKV 将内部存储错误转换为 Postgres 标准的 `SQLSTATE` 错误码（如 `23505` 代表唯一键冲突），确保上层 ORM（如 Hibernate, Diesel）能正确处理异常。

## 4. 类型系统对齐

| Postgres OID | YYKV 类型 | 说明 |
| :--- | :--- | :--- |
| `INT4`, `INT8` | `Int` | 映射为 64 位整数 |
| `TEXT`, `VARCHAR` | `String` | UTF-8 编码 |
| `BYTEA` | `Binary` | 原始字节块 |
| `BOOL` | `Bool` | 布尔值 |

## 5. 特色功能：算子下推

当使用 Postgres 客户端连接时，复杂的过滤条件（WHERE 子句）会被兼容层识别并利用 YYKV 的 `yykv-compute` 直接下推到存储节点执行，仅返回满足条件的行，极大节省网络带宽。
