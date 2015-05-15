# yykv-operators (Logical & Physical Operator IR)

`yykv-operators` defines the operator graph (OpsGraph) Intermediate Representation (IR) and expression system for the YYKV storage engine. It serves as the bridge between upper-level protocols (SQL, Redis, etc.) and the underlying execution engine (`yykv-executor`).

## Core Features

### 🕸️ Operator Graph (OpsGraph)
Defines the physical execution plan for queries:
- **Node Abstraction**: Each operator is a node, supporting multiple inputs and outputs.
- **Serialization**: OpsGraphs can be serialized into binary format, supporting push-down execution across nodes in a distributed environment.

### 🛠️ Multi-Modal Operator Set
- **Base Operators**: `TableScan`, `Filter` (predicate filtering), `Project` (projection), `Limit`.
- **Relational Operators**: `HashJoin`, `Sort`, `Aggregate`.
- **Advanced Operators**:
  - `VectorSearch`: Operators specifically optimized for vector similarity retrieval.
  - `UdfCall`: Operators that trigger the execution of storage-side `yykv-udf` (Wasm).

### 🧮 Expression Engine
Supports complex expression evaluation based on `DsValue`:
- **Scalar Functions**: Arithmetic operations, string processing, time conversion.
- **Predicate Evaluation**: High-performance logical operation judgment.

## Core Components

- `OpsGraph`: The topological structure of the entire execution plan.
- `Operator`: Enumeration of operators, defining all supported execution units.
- `Expression`: Tree-based or flattened expression IR.

## Usage Example

```rust
use yykv_operators::{OpsGraph, Operator, Expression};

let mut graph = OpsGraph::new();

// Build: scan users table -> filter age > 18 -> take top 10
let scan_id = graph.add_node(Operator::TableScan { table: "users".into() });
let filter_id = graph.add_node(Operator::Filter { 
    predicate: Expression::GreaterThan(Box::new(Expression::Column("age")), Box::new(Expression::Literal(18.into())))
});
graph.add_edge(scan_id, filter_id);

// Submit to execution engine
executor.execute(graph).await?;
```

## Technical Advantages
- **Push-down Friendly**: Operator design prioritizes how to push down to storage nodes (Near-Data Computing).
- **Zero-Copy Design**: Data passing between operators uses references or `bytes` views wherever possible.
