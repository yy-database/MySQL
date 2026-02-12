# we-trust-arrow (Vectorized Data Adapter)

`we-trust-arrow` 实现了 YYKV 存储引擎与 Apache Arrow 内存布局之间的无缝对接。它利用 Arrow 的列式内存结构，为海量数据的 OLAP 分析、AI 特征提取及跨系统数据交换提供极速的向量化传输能力。

## 核心功能

### 🚀 向量化传输 (Vectorized Transfer)
- **批量读取**: 将 YYKV 存储页中的行式或多模态数据高效转化为 Arrow 的 `RecordBatch`。
- **零拷贝分析**: 在可能的情况下，通过内存映射直接在 Arrow 数组上执行 `yykv-compute` 算子，消除序列化开销。

### 📊 列式存储优化
- 针对列式查询（如投影特定列）进行了协议优化，仅传输必要的列数据。
- 深度集成 Arrow 的 `Schema` 定义，确保与大数据生态（如 Pandas, Spark, Polars）的类型兼容。

### ⚡ 近数据分析
允许在存储节点直接生成 Arrow 格式的中间结果，减少网络总线上的冗余数据传输。

## 核心组件

- `ArrowAdapter`: 负责将 YYKV 内部数据流转换为 Arrow `RecordBatch`。
- `ArrowCodec`: 支持基于 Arrow Flight 或自定义二进制流的高性能编解码。

## 使用场景

- **大数据分析**: 将 YYKV 作为高性能数据源，通过 Arrow 协议对接分析引擎。
- **AI 训练**: 高效提取存储在 YYKV 中的特征向量并转化为张量格式。
- **跨语言交互**: 利用 Arrow 的跨语言特性，实现 Python/Go 等语言对 YYKV 数据的高速访问。
