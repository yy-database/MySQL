use std::collections::BTreeMap;
use uuid::Uuid;
use yykv_types::DsResult;

pub struct MemoryVectorIndex {
    // tenant_id -> (doc_id -> vector)
    data: BTreeMap<Uuid, BTreeMap<Uuid, Vec<f32>>>,
}

impl Default for MemoryVectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryVectorIndex {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn add_vector(&mut self, id: Uuid, tenant_id: Uuid, vector: Vec<f32>) -> DsResult<()> {
        self.data.entry(tenant_id).or_default().insert(id, vector);
        Ok(())
    }

    pub fn search_nearest(
        &self,
        tenant_id: Uuid,
        query: &[f32],
        limit: usize,
    ) -> DsResult<Vec<(Uuid, f32)>> {
        let Some(vectors) = self.data.get(&tenant_id) else {
            return Ok(Vec::new());
        };

        let mut results: Vec<(Uuid, f32)> = vectors
            .iter()
            .map(|(id, v)| {
                // Simple Cosine Similarity
                let dot: f32 = v.iter().zip(query.iter()).map(|(a, b)| a * b).sum();
                let norm_a: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm_b: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();
                let score = if norm_a > 0.0 && norm_b > 0.0 {
                    dot / (norm_a * norm_b)
                } else {
                    0.0
                };
                (*id, score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        Ok(results)
    }

    pub fn delete_vector(&mut self, id: Uuid, tenant_id: Uuid) -> DsResult<()> {
        if let Some(vectors) = self.data.get_mut(&tenant_id) {
            vectors.remove(&id);
        }
        Ok(())
    }
}
