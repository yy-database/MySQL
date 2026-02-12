use async_trait::async_trait;
use tracing::debug;
use yykv_layout::LayoutManager;
use yykv_types::{DsError, DsResult, DsValue};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VectorMetric {
    L2,
    DotProduct,
    Cosine,
}

pub struct VectorCompute;

impl VectorCompute {
    /// Calculate similarity between two vectors.
    /// This is a simplified version, ideally would use SIMD (e.g., via wide crate or target_feature).
    pub fn similarity(v1: &[f32], v2: &[f32], metric: VectorMetric) -> f32 {
        match metric {
            VectorMetric::L2 => v1
                .iter()
                .zip(v2.iter())
                .map(|(x, y)| (x - y) * (x - y))
                .sum::<f32>()
                .sqrt(),
            VectorMetric::DotProduct => v1.iter().zip(v2.iter()).map(|(x, y)| x * y).sum::<f32>(),
            VectorMetric::Cosine => {
                let dot = Self::similarity(v1, v2, VectorMetric::DotProduct);
                let norm1 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm2 = v2.iter().map(|x| x * x).sum::<f32>().sqrt();
                dot / (norm1 * norm2)
            }
        }
    }
}

#[async_trait]
pub trait ComputeOperator: Send + Sync {
    async fn execute(&self, data: &[u8]) -> DsResult<Vec<u8>>;
}

pub struct VectorSearchOperator {
    pub query_vector: Vec<f32>,
    pub metric: VectorMetric,
    pub top_k: usize,
}

#[async_trait]
impl ComputeOperator for VectorSearchOperator {
    async fn execute(&self, data: &[u8]) -> DsResult<Vec<u8>> {
        let layout = LayoutManager::new();
        if let Some(val) = layout.deserialize(data) {
            match val {
                DsValue::Vector(v) => {
                    let sim = VectorCompute::similarity(&v.0, &self.query_vector, self.metric);
                    // Return similarity as bytes (f32 le)
                    return Ok(sim.to_le_bytes().to_vec());
                }
                _ => {
                    return Err(DsError::serialization_with_format(
                        "binary",
                        "Data is not a vector",
                    ));
                }
            }
        }
        Err(DsError::serialization_with_format(
            "binary",
            "Failed to deserialize data",
        ))
    }
}

pub struct ComputeEngine {}

impl Default for ComputeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// Push down an operator to be executed on raw data.
    pub async fn pushdown<Op: ComputeOperator>(
        &self,
        op: &Op,
        raw_data: &[u8],
    ) -> DsResult<Vec<u8>> {
        debug!(
            "Executing compute pushdown operator on {} bytes",
            raw_data.len()
        );
        op.execute(raw_data).await
    }
}

#[async_trait]
pub trait UdfEngine: Send + Sync {
    async fn call(&self, name: &str, data: Vec<u8>) -> DsResult<DsValue>;
}

pub struct MockWasmRuntime;

#[async_trait]
impl UdfEngine for MockWasmRuntime {
    async fn call(&self, name: &str, data: Vec<u8>) -> DsResult<DsValue> {
        debug!(
            "MockWasmRuntime calling UDF: {} with {} bytes",
            name,
            data.len()
        );
        Ok(DsValue::Binary(data.into()))
    }
}

/// Multimodal Operator Slot: Allows plugging in different indexing/search strategies.
pub trait MultimodalOperator: Send + Sync {
    fn name(&self) -> &str;
    fn process(&self, payload: &[u8]) -> DsResult<Vec<u8>>;
}

pub struct OperatorRegistry {
    operators: std::collections::HashMap<String, Box<dyn MultimodalOperator>>,
}

impl Default for OperatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OperatorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            operators: std::collections::HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.register(Box::new(HnswOperator));
    }

    pub fn register(&mut self, op: Box<dyn MultimodalOperator>) {
        self.operators.insert(op.name().to_string(), op);
    }

    pub fn get(&self, name: &str) -> Option<&dyn MultimodalOperator> {
        self.operators.get(name).map(|b| b.as_ref())
    }
}

/// Example implementation: HNSW Search Slot
pub struct HnswOperator;
impl MultimodalOperator for HnswOperator {
    fn name(&self) -> &str {
        "HNSW"
    }
    fn process(&self, _payload: &[u8]) -> DsResult<Vec<u8>> {
        // HNSW search logic
        Ok(vec![])
    }
}
