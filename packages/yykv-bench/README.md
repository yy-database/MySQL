# yykv-bench (Performance Benchmarking Suite)

`yykv-bench` 是专门为 YYKV 存储引擎设计的压力测试与性能评估工具。它模拟了真实世界中的多模态负载，帮助开发者验证在不同物理介质（NVMe, SSD, HDD）上的性能表现。

## 核心功能

### 🚀 极致压力测试
- **吞吐量评估**: 支持多线程并发读写测试，评估裸设备在 Direct IO 模式下的带宽上限。
- **混合负载模拟**: 模拟真实的读写比例（如 7:3），验证 `yykv-scheduler` 的优先级调度效果。

### 📊 详细延迟画像
- **百分位统计**: 提供平均延迟、P50、P99 及 P99.9 延迟报告。
- **IO 抖动分析**: 识别在后台 Compaction 或 Tiering 搬运过程中的前端延迟波动。

### ⚖️ 跨引擎对比
- 提供标准接口，支持将 YYKV 与 RocksDB、TiKV 或原生文件系统进行性能基准对比。

## 使用方法

```bash
# 运行基础读写测试
cargo run -p yykv-bench -- --threads 16 --duration 60s --type mixed

# 测试特定物理驱动器
cargo run -p yykv-bench -- --device \\.\PhysicalDrive1 --mode direct
```

## 测试指标

- **IOPS**: 每秒执行的读写操作数。
- **Bandwidth**: 吞吐量 (MB/s)。
- **Tail Latency**: 尾部延迟分布情况。
