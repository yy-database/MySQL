# yykv-schema (Multi-Modal Metadata Management)

`yykv-schema` 负责 YYKV 存储引擎的元数据定义与模式管理。它不仅支持传统的关系型表结构，还统一管理文档、向量及时序数据模式，确保数据的结构化访问与完整性。

## 核心功能

### 📑 多模态模式定义
- **关系型表**: 定义字段名、数据类型（`DsValue`）、主键及约束。
- **向量空间**: 定义向量维度、距离度量（L2, Cosine）及索引参数。
- **文档模式**: 支持灵活的 JSON-like 文档结构定义。

### 🔄 在线 Schema 演进
- **版本控制**: 每个 Schema 都有其对应的版本号。
- **平滑迁移**: 支持在线添加字段、修改约束，尽量减少对现有 IO 的阻塞。

### 🛡️ 数据校验与安全
- **写入前校验**: 在数据进入 `yykv-event` (WAE) 流程前，强制进行类型检查与非空约束验证。
- **多租户命名空间**: 每个租户拥有完全独立的元数据命名空间，逻辑上彼此不可见。

## 核心组件

- `SchemaManager`: 负责元数据的持久化存储与内存缓存。
- `TableSchema`: 单个表的详细结构定义。
- `ColumnDefinition`: 字段属性定义。

## 使用示例

```rust
use yykv_schema::{SchemaManager, TableSchema, ColumnDefinition};

let mut manager = SchemaManager::new();

// 定义一个新表
let user_table = TableSchema::new("users")
    .add_column(ColumnDefinition::new("id", DsType::Int64).primary_key())
    .add_column(ColumnDefinition::new("bio", DsType::Text));

manager.create_table(tenant_id, user_table).await?;
```
