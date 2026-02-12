# we-trust-vector (Vector Search Extension)

`we-trust-vector` 是 We-Trust 协议的向量搜索（Vector Search）扩展模块。它定义了在客户端与 YYKV 存储引擎之间高效传输向量数据、相似度检索指令及高维特征结果的协议规范。

## 核心功能

### 🏹 向量检索指令下推
- **相似度度量**: 在协议层直接支持 `L2` (欧氏距离), `Cosine` (余弦相似度) 和 `InnerProduct` (内积) 等度量标准的表达。
- **Top-K 查询**: 优化了针对大规模向量集合返回最相似结果集的二进制消息格式。

### 📦 高效向量序列化
- **浮点数组压缩**: 针对 `f32` 和 `f16` 向量数组进行了内存对齐与序列化优化。
- **批量传输**: 支持在一个协议包中传输多个查询向量，减少网络往返开销。

### 🧩 混合检索支持
允许在同一个 We-Trust 请求中结合 SQL 过滤条件与向量相似度搜索，实现真正的“多模态”查询协议。

## 核心组件

- `VectorQuery`: 定义向量搜索请求的结构化表示。
- `VectorResult`: 包含相似度得分与元数据的返回结果集。
- `VectorCodec`: 专为高维向量数据设计的编解码器。

## 使用示例

```rust
// 伪代码：发起向量搜索
let query_vector = vec![0.1, 0.2, 0.3, ...];
let request = VectorQuery::new("face_embeddings")
    .vector(query_vector)
    .metric(MetricType::Cosine)
    .limit(10);

let results = conn.vector_search(request).await?;
```

## 技术优势
- **亚毫秒级协议延迟**: 针对向量搜索的短平快特性优化的二进制协议栈。
- **端到端向量化**: 配合 `yykv-index` 的 HNSW 或 IVF 索引，实现从协议到存储引擎的全链路向量化处理。
