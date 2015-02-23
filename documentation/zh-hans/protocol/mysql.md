# MySQL 兼容协议详解

**we-trust-mysql** 实现了 MySQL 线缆协议 (Wire Protocol)，使 YYKV 能够透明地作为 MySQL 服务器接收连接。这极大地降低了现有应用的迁移成本。

## 1. 协议层架构

MySQL 兼容层作为 Trust 协议的一个**转换代理 (Adapter)** 存在：
1. **监听层**: 在端口（默认 `3306`）监听 MySQL 二进制流。
2. **解码器**: 将 MySQL 的 `Packet` 结构拆解。
3. **映射层**: 将 MySQL 指令（如 `COM_QUERY`）转换为 YYKV 内部的 `MessageType::Query` 或 `MessageType::KQL`。
4. **编码器**: 将结果集 (ResultSet) 封装为 MySQL 的行数据格式返回。

## 2. 报文结构

YYKV 严格遵循 MySQL 的报文布局：

| 长度 | 字段 | 说明 |
| :--- | :--- | :--- |
| 3 字节 | `Payload Length` | 负载长度 |
| 1 字节 | `Sequence ID` | 报文序列号，用于防止重放攻击和有序确认 |
| N 字节 | `Payload` | 具体的命令内容（如 SQL 文本或认证信息） |

## 3. 认证与握手

YYKV 实现了标准的 MySQL 握手流程：
- **服务端挑战**: 发送包含协议版本、连接 ID 和 `auth-plugin-data` 的 Handshake 报文。
- **客户端响应**: 提供用户名、租户标识（通常编码在数据库名中）和经过 SHA1/SHA256 混淆的密码。
- **权限校验**: 兼容层将认证请求转发给 YYKV 的多租户管理器进行鉴权。

## 4. 核心指令映射

| MySQL 命令 | 映射至 YYKV 操作 | 说明 |
| :--- | :--- | :--- |
| `COM_QUERY` | `MessageType::KQL` | 将 SQL 语句解析并转换为 KQL 算子执行 |
| `COM_STMT_EXECUTE` | `MessageType::Query` | 使用预编译语句提高执行效率 |
| `COM_QUIT` | 关闭连接 | 优雅释放资源 |

## 5. 类型系统映射

| MySQL 类型 | YYKV (YYValue) 类型 | 说明 |
| :--- | :--- | :--- |
| `VARCHAR`, `TEXT` | `String` | UTF-8 编码字符串 |
| `INT`, `BIGINT` | `Int` | 64 位有符号整数 |
| `BLOB`, `BINARY` | `Binary` | 原始字节流 |
| `DECIMAL` | `Decimal` | 高精度数值 |

## 6. 使用建议

- **连接字符串**: `mysql://user:pass@host:3306/tenant_id`
- **驱动兼容性**: 支持 `mysql-connector-java`, `mysql-client`, `sqlx::mysql` 等所有主流驱动。
