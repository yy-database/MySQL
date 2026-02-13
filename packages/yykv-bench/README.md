# yykv-bench (Performance Benchmarking Suite)

`yykv-bench` is a stress testing and performance evaluation tool specifically designed for the YYKV storage engine. It simulates real-world multi-modal workloads, helping developers verify performance across different physical media (NVMe, SSD, HDD).

## Core Features

### 🚀 Extreme Stress Testing
- **Throughput Evaluation**: Supports multi-threaded concurrent read/write tests to evaluate the bandwidth ceiling of bare devices in Direct IO mode.
- **Mixed Workload Simulation**: Simulates real-world read/write ratios (e.g., 7:3) to verify the priority scheduling effectiveness of `yykv-scheduler`.

### 📊 Detailed Latency Profiling
- **Percentile Statistics**: Provides reports on average latency, P50, P99, and P99.9 latency.
- **IO Jitter Analysis**: Identifies frontend latency fluctuations during background Compaction or Tiering operations.

### ⚖️ Cross-Engine Comparison
- Provides a standard interface for benchmarking YYKV against RocksDB, TiKV, or native file systems.

## Usage

```bash
# Run basic read/write tests
cargo run -p yykv-bench -- --threads 16 --duration 60s --type mixed

# Test a specific physical drive
cargo run -p yykv-bench -- --device \\.\PhysicalDrive1 --mode direct
```

## Metrics

- **IOPS**: Number of read/write operations per second.
- **Bandwidth**: Throughput (MB/s).
- **Tail Latency**: Distribution of tail latency.
