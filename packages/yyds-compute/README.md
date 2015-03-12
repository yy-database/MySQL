# WASM-based Distributed Compute Engine

`yyds-compute` is a WASM-based distributed compute engine for the YYDS ecosystem.

## Features

- **UDF Runtime**: A lightweight, sandboxed User-Defined Function (UDF) execution environment based on `wasmtime`.
- **High Performance**: Utilizes JIT compilation technology to provide near-native compute performance.
- **WASI Integration**: Supports WASI Preview 2, allowing UDFs to access restricted system resources.
- **Security Isolation**: Strict memory and CPU instruction limits to prevent malicious code from affecting the host.

## Core Interfaces

- `UdfEngine`: Unified definition of the compute engine invocation contract.
- `WasmtimeUdfEngine`: Concrete implementation based on Wasmtime, supporting asynchronous execution.
