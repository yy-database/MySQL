# 快速开始：连接到 YYDS

本指南将帮助你快速连接到现有的 YYDS (獨一分布式存储) 实例。

## 1. 获取连接信息

在连接之前，请确保你已拥有以下信息：
- **服务器地址 (Host)**：YYDS 实例的 IP 或域名。
- **端口 (Port)**：默认为 `22122` (We-Trust 原生协议)。
- **租户 ID (Tenant ID)**：你的资源隔离标识。
- **访问凭据 (Access Key)**：用于身份验证。

## 2. 选择连接协议

YYDS 支持多种协议，你可以根据应用场景选择最适合的连接方式：

### 原生协议 (We-Trust)
**推荐用于生产环境**。提供最佳性能和全量功能支持。
- **SDK 支持**：Rust, TypeScript, Go, Python。
- **特点**：支持流式背压、算子下推、强一致性保证。

### 兼容协议
如果你现有的应用使用的是其他数据库，可以通过兼容层直接连接：
- **Redis 协议**：使用标准的 Redis 客户端连接。
- **PostgreSQL 协议**：支持标准的 SQL 查询。
- **S3 协议**：用于大规模对象存储访问。

## 3. 使用 SDK 连接 (以 TypeScript 为例)

```typescript
import { YYClient } from '@yydb/sdk';

const client = new YYClient({
  endpoint: '127.0.0.1:22122',
  tenantId: 'your-tenant-id',
  accessKey: 'your-secret-key'
});

await client.connect();

// 简单的 KV 操作
await client.set('hello', 'yyds');
const val = await client.get('hello');
console.log(val); // 'yyds'
```

## 4. 使用 CLI 工具连接

你可以使用内置的 `yyds-cli` 进行交互式操作：

```bash
yyds-cli connect --host 127.0.0.1 --port 22122 --tenant demo
```

连接成功后，你可以输入 `HELP` 查看所有可用命令。

---

**下一步**：
- 如果你还没有运行中的实例，请查看 [部署指南](./deploy)。
- 了解更多关于 [原生协议](../protocol/we-trust) 的细节。
