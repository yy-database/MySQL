# yyds-compute

基于 WASM 的分布式计算引擎。

## 功能特性

- **UDF 运行时**: 基于 `wasmtime` 实现的轻量级、沙箱化用户自定义函数 (UDF) 执行环境。
- **高性能**: 利用 JIT 编译技术，提供接近原生的计算性能。
- **WASI 集成**: 支持 WASI 预览 2，允许 UDF 访问受限的系统资源。
- **安全隔离**: 严格的内存与 CPU 指令限制，防止恶意代码影响主机。

## 核心接口

- `UdfEngine`: 统一定义计算引擎的调用契约。
- `WasmtimeUdfEngine`: 基于 Wasmtime 的具体实现，支持异步执行。
