# yykv-profile (Performance Profiling & Diagnostics)

`yykv-profile` 是 YYKV 的运行时性能分析与诊断工具包。它通过非侵入式的采样技术，帮助开发者精准定位存储引擎在裸硬件运行时的性能瓶颈。

## 核心功能

### 🔥 CPU 火焰图 (CPU Profiling)
- **采样分析**: 统计各组件（如 `yykv-compute`, `yykv-io`）在 IO 密集型负载下的 CPU 占用。
- **热点定位**: 自动生成火焰图，直观展示函数调用栈的耗时占比。

### 🧠 内存分布画像 (Memory Analysis)
- **堆分配监控**: 追踪 `yykv-allocator` 管理的物理页面缓冲与上层引擎的临时对象分配。
- **内存泄漏检测**: 识别长时间未释放的 `AlignedBuffer` 或异步任务。

### 📊 IO 延迟分布 (Latency Histograms)
- **P99 追踪**: 实时统计从 `yykv-scheduler` 发起请求到硬件 DMA 完成的亚毫秒级延迟分布。

## 使用方法

```bash
# 运行性能剖析并生成火焰图
cargo run -p yykv-profile -- --duration 60 --output flamegraph.svg

# 监控特定租户的资源占用
cargo run -p yykv-profile -- --tenant <UUID> --mode metrics
```

## 技术集成
- **pprof-rs**: 底层基于采样技术，对在线业务性能影响极小。
- **tracing**: 集成 Rust 的 `tracing` 生态，支持跨包的事件关联。
