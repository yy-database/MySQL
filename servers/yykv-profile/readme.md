# yykv-profile (Performance Profiling & Diagnostics)

`yykv-profile` is a runtime performance analysis and diagnostics toolkit for YYKV. It uses non-intrusive sampling techniques to help developers precisely locate performance bottlenecks in the storage engine during bare-metal operation.

## Core Features

### 🔥 CPU Profiling
- **Sampling Analysis**: Statistics on CPU usage of various components (e.g., `yykv-compute`, `yykv-io`) under IO-intensive loads.
- **Hotspot Location**: Automatically generates flame graphs to visually show time consumption percentages of function call stacks.

### 🧠 Memory Analysis
- **Heap Allocation Monitoring**: Tracks physical page buffers managed by `yykv-allocator` and temporary object allocations from the upper-level engine.
- **Memory Leak Detection**: Identifies `AlignedBuffer` or asynchronous tasks that have not been released for a long time.

### 📊 IO Latency Distribution
- **P99 Tracking**: Real-time statistics of sub-millisecond latency distributions from `yykv-scheduler` request initiation to hardware DMA completion.

## Usage

```bash
# Run performance profiling and generate a flame graph
cargo run -p yykv-profile -- --duration 60 --output flamegraph.svg

# Monitor resource usage for a specific tenant
cargo run -p yykv-profile -- --tenant <UUID> --mode metrics
```

## Technical Integration
- **pprof-rs**: Based on sampling technology with minimal impact on online business performance.
- **tracing**: Integrates with the Rust `tracing` ecosystem, supporting event correlation across packages.
