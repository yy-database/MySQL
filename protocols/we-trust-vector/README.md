# Vector Protocol

**we-trust-vector** is a dedicated protocol extension designed for the AI era, specifically for handling the storage and retrieval of high-dimensional vectors (Vector Search).

## 1. Protocol Features

Unlike traditional KV protocols, the Vector protocol supports the following at the message level:
- **Dimension Definition**: Specify vector dimensions (e.g., 768, 1536) during write operations.
- **Distance Metrics**: Supports specifying `L2` (Euclidean distance), `Cosine` (Cosine similarity), or `IP` (Inner Product) in query messages.
- **Top-K Queries**: A dedicated instruction set for quickly returning the K most similar results.

## 2. Data Encoding

Vector data is transmitted using a compact binary floating-point array format:
- **`f32` Arrays**: Standard precision.
- **`f16` / `Int8` Quantization**: Supports data quantization and compression at the protocol layer, saving 50% - 75% of network bandwidth.

## 3. Search Operator Mapping

Vector instructions map directly to YYKV's underlying vector indices (such as HNSW or IVF-Flat):
- `MessageType::Query` carries the vector payload.
- Storage nodes utilize the vector acceleration module in `yykv-compute` for parallel retrieval.

## 4. Use Cases

- **RAG (Retrieval-Augmented Generation)**: Provides real-time, efficient knowledge base retrieval for Large Language Models.
- **Image/Video Search**: Stores feature vectors and enables fast reverse image/video search.
- **Recommendation Systems**: Real-time personalized recommendations based on user behavior vectors.

## Programming Interface (Pseudo-code)

```rust
// Pseudo-code: Initiating a vector search
let query_vector = vec![0.1, 0.2, 0.3, ...];
let request = VectorQuery::new("face_embeddings")
    .vector(query_vector)
    .metric(MetricType::Cosine)
    .limit(10);

let results = conn.vector_search(request).await?;
```
