# we-trust-d1 (Cloudflare D1 Edge Adapter)

`we-trust-d1` 是专为 Cloudflare Workers 边缘计算环境设计的驱动适配器。它使部署在边缘节点的服务能够通过标准 We-Trust 接口访问 Cloudflare D1 (基于 SQLite) 的数据，实现“边缘感知”的数据库访问模型。

## 核心功能

### 🌐 Edge 运行时兼容
- **零原生依赖**: 针对 V8 隔离环境优化的纯 Rust 实现，不依赖底层系统库。
- **HTTP API 适配**: 封装了 Cloudflare D1 的 HTTP REST 接口，使其符合 We-Trust 的统一连接协议。

### 🏗️ 统一抽象模型
- **Connection**: 将 D1 的无状态请求封装为长连接风格的 `Connection` 特征。
- **Transaction**: 模拟事务行为，尽管 D1 本质上是基于 HTTP 的，适配层尽量保证了逻辑上的一致性。

### ⚡ 极简实现
- 专注于边缘节点的低内存占用与快速启动需求。

## 核心组件

- `D1Adapter`: 负责与 Cloudflare 环境中的 `D1Database` 绑定进行交互。
- `D1Connection`: 维护请求上下文与结果集转换。

## 使用示例

```rust
// 在 Cloudflare Workers 中使用
let adapter = D1Adapter::new(env.d1("MY_DB")?);
let conn = adapter.connect().await?;

let rows = conn.query("SELECT * FROM users WHERE id = ?", &[1.into()]).await?;
```

## 技术背景
Cloudflare D1 是 Cloudflare 提供的无服务器 SQL 数据库。`we-trust-d1` 的存在使得 YYKV 生态的应用可以无缝切换存储后端，从物理裸设备扩展到全球分布的边缘数据库。
