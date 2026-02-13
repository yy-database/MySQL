# Multi-Modal Metadata Management

**yykv-schema** is responsible for the metadata definition and schema management of the YYKV storage engine. It not only supports traditional relational table structures but also uniformly manages document, vector, and time-series data schemas, ensuring structured access and data integrity.

## 1. Core Features

### 📑 Multi-Modal Schema Definition
- **Relational Tables**: Defines field names, data types (`DsValue`), primary keys, and constraints.
- **Vector Spaces**: Defines vector dimensions, distance metrics (L2, Cosine), and indexing parameters.
- **Document Schemas**: Supports flexible JSON-like document structure definitions.

### 🔄 Online Schema Evolution
- **Version Control**: Each schema has its corresponding version number.
- **Smooth Migration**: Supports online field addition and constraint modification, minimizing blockage to existing I/O operations.

### 🛡️ Data Validation and Security
- **Pre-write Validation**: Before data enters the `yykv-event` (WAE) flow, mandatory type checks and non-null constraint validations are performed.
- **Multi-tenant Namespace**: Each tenant has a completely independent metadata namespace, logically invisible to each other.

## 2. Core Components

- `SchemaManager`: Responsible for persistent storage and in-memory caching of metadata.
- `TableSchema`: Detailed structure definition for a single table.
- `ColumnDefinition`: Field attribute definition.

## 3. Usage Example

```rust
use yykv_schema::{SchemaManager, TableSchema, ColumnDefinition};

let mut manager = SchemaManager::new();

// Define a new table
let user_table = TableSchema::new("users")
    .add_column(ColumnDefinition::new("id", DsType::Int64).primary_key())
    .add_column(ColumnDefinition::new("bio", DsType::Text));

manager.create_table(tenant_id, user_table).await?;
```
