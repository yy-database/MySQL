# yykv-compute (Near-Data Computing Engine)

`yykv-compute` 是 YYKV 存储引擎的算子下推（Pushdown）与计算内核。它允许计算逻辑直接在数据所在的存储节点上运行，最大限度地减少了跨总线的数据搬运，是实现“存算一体化”架构的核心。

## 核心设计

### ⚡ 算子下推 (Pushdown)
在传统的数据库架构中，数据需要从存储层加载到计算层进行过滤，这会产生巨大的网络与 IO 开销。`yykv-compute` 实现了：
- **谓词下推**: 在扫描物理页面的过程中直接执行过滤（Filter）。
- **投影下推**: 仅从存储层读取必要的列数据（Project）。
- **聚合下推**: 在存储节点本地完成 `Count`, `Sum`, `Min/Max` 等预聚合操作。

### 🏹 多模态计算支持
- **向量运算**: 内置高性能向量相似度计算，支持 `L2` (欧式距离)、`Dot Product` (点积) 和 `Cosine` (余弦相似度)。
- **插槽式架构**: 提供 `ComputeOperator` 特征，允许第三方通过 `yykv-udf` 扩展自定义算子。

### 🧩 零拷贝执行
- 针对二进制页面布局（`yykv-layout`）优化的流式处理引擎。
- 在大多数情况下，计算算子直接作用于 `AlignedBuffer`，避免了不必要的数据反序列化。

## 核心组件

- `ComputeEngine`: 调度中心，负责接收下推任务并在原始数据上并行执行。
- `VectorCompute`: 专门针对高维向量数据优化的数学计算库。
- `OperatorRegistry`: 算子注册中心，管理系统预置与用户自定义的计算算子。

## 使用示例

```rust
let engine = ComputeEngine::new();
let operator = VectorSearchOperator {
    query_vector: vec![0.1, 0.2, 0.3],
    metric: VectorMetric::Cosine,
    top_k: 10,
};

// 直接在原始物理页面上执行向量检索，仅返回满足 Top-K 的结果
let result = engine.pushdown(&operator, raw_page_data).await?;
```

## 技术优势
- **极速 IO**: 配合 `yykv-io` 的 Direct IO，数据从磁盘 DMA 到内存后立即参与计算。
- **高并发**: 支持利用多核 CPU 对不同存储页面进行并行计算处理。
