# User-Defined Functions (UDF)

**yykv-udf** provides a sandboxed execution environment for user-defined functions within the YYKV storage engine. By pushing computation logic down to the storage layer and executing Wasm bytecode directly during storage page scanning, it significantly reduces the overhead of data transfer across the system bus.

## 1. Core Technologies

### 🧩 Wasmtime Runtime
Integrates the high-performance `wasmtime` engine, utilizing JIT compilation technology to provide near-native execution performance while ensuring strict physical isolation between the computation logic and the storage kernel.

### 🏗️ Component Model
Defines computational contracts executed on the storage side based on the WebAssembly Component Model.
- **WIT Definition**: Standardized input (data pages, filtering parameters) and output (processing results, filtering bitmaps) protocols are defined in `wit/udf.wit`.
- **Integrated Storage & Compute**: Allows UDFs to directly access the binary memory layout of storage pages.

### 🛡️ Resource Management and Security
- **WASI Integration**: Only necessary WASI capabilities are exposed, strictly limiting UDF access to the file system and network.
- **Compute Quotas**: Supports limits on execution time (Gas) and memory usage for each UDF, preventing malicious or infinite loop code from overloading storage nodes.

## 2. Use Cases

- **Predicate Filtering**: Use custom business logic for filtering when scanning massive amounts of KV data.
- **Data Transformation**: Perform data masking, format conversion, or simple aggregation before data is read out of the storage engine.
- **AI Inference Pushdown**: Compile small quantized models into Wasm to perform feature extraction or simple classification calculations directly on storage nodes.

## 3. Core APIs

- `UdfRuntime`: Manages the creation, caching, and scheduling of Wasm instances.
- `UdfState`: Maintains the WASI context and resource table.

## 4. Example Code (Rust Host Side)

```rust
use yykv_udf::UdfRuntime;

let runtime = UdfRuntime::new()?;
// 1. Load Wasm module
// 2. Prepare execution context
// 3. Call UDF within the scanning loop
```
