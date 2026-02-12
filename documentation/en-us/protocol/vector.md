# Vector Protocol Details

**we-trust-vector** is a specialized protocol extension designed for the AI era, specifically for the storage and retrieval of high-dimensional vectors (Vector Search).

## 1. Protocol Features

Unlike traditional KV protocols, the Vector protocol supports the following at the packet level:
- **Dimension Definition**: Specify vector dimensions (e.g., 768, 1536) during writes.
- **Distance Metrics**: Supports specifying `L2` (Euclidean distance), `Cosine` (Cosine similarity), or `IP` (Inner Product) in query messages.
- **Top-K Queries**: Dedicated instruction set for rapidly returning the K most similar results.

## 2. Data Encoding

Vector data is transmitted in a compact binary floating-point array format:
- **`f32` Arrays**: Standard precision.
- **`f16` / `Int8` Quantization**: Supports data quantization compression at the protocol layer, saving 50% - 75% of network bandwidth.

## 3. Search Operator Mapping

Vector instructions map directly to YYKV's underlying vector indices (such as HNSW or IVF-Flat):
- `MessageType::Query` carries the vector payload.
- Storage nodes utilize the vector acceleration module of `yykv-compute` for parallel retrieval.

## 4. Application Scenarios

- **RAG (Retrieval-Augmented Generation)**: Provides real-time, efficient knowledge base retrieval for Large Language Models.
- **Image/Video Search**: Stores feature vectors and enables rapid image-based search.
- **Recommendation Systems**: Implements real-time personalized recommendations based on user behavior vectors.
