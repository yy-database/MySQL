# yykv-compute (Near-Data Computing Engine)

`yykv-compute` is the operator pushdown and compute kernel of the YYKV storage engine. It allows computation logic to run directly on the storage nodes where data resides, minimizing data movement across the bus and serving as the core of the "Integrated Storage & Compute" architecture.

## Core Design

### ⚡ Operator Pushdown
In traditional database architectures, data needs to be loaded from the storage layer to the compute layer for filtering, which incurs massive network and IO overhead. `yykv-compute` implements:
- **Predicate Pushdown**: Executes filters directly during physical page scanning.
- **Projection Pushdown**: Reads only necessary column data from the storage layer.
- **Aggregation Pushdown**: Completes pre-aggregation operations like `Count`, `Sum`, `Min/Max` locally on the storage node.

### 🏹 Multi-Modal Computing Support
- **Vector Operations**: Built-in high-performance vector similarity computation, supporting `L2` (Euclidean distance), `Dot Product`, and `Cosine` similarity.
- **Pluggable Architecture**: Provides the `ComputeOperator` trait, allowing third parties to extend custom operators via `yykv-udf`.

### 🧩 Zero-Copy Execution
- Stream processing engine optimized for binary page layout (`yykv-layout`).
- In most cases, compute operators act directly on `AlignedBuffer`, avoiding unnecessary data deserialization.

## Core Components

- `ComputeEngine`: The scheduling center, responsible for receiving pushdown tasks and executing them in parallel on raw data.
- `VectorCompute`: A mathematical library specifically optimized for high-dimensional vector data.
- `OperatorRegistry`: Manages system-preset and user-defined compute operators.

## Usage Example

```rust
let engine = ComputeEngine::new();
let operator = VectorSearchOperator {
    query_vector: vec![0.1, 0.2, 0.3],
    metric: VectorMetric::Cosine,
    top_k: 10,
};

// Execute vector retrieval directly on raw physical pages, returning only Top-K results
let result = engine.pushdown(&operator, raw_page_data).await?;
```

## Technical Advantages
- **Extreme IO**: Coupled with `yykv-io` Direct IO, data is computed immediately after DMA from disk to memory.
- **High Concurrency**: Supports parallel computing on different storage pages using multi-core CPUs.
