pub mod graph;
pub mod text;
pub mod vector;

use async_trait::async_trait;
use std::collections::BTreeSet;
use uuid::Uuid;
use yykv_types::DsResult;

pub use graph::{EdgeDirection, MemoryAdjacencyIndex};
pub use text::{MemoryTextStore, WhitespaceTokenizer};
pub use vector::MemoryVectorIndex;

#[async_trait]
pub trait IndexEngine: Send + Sync {
    // Text Search
    async fn index_text(&mut self, id: Uuid, tenant_id: Uuid, body: &str) -> DsResult<()>;
    async fn index_text_batch(&mut self, items: Vec<(Uuid, Uuid, String)>) -> DsResult<()>;
    async fn search_text(
        &self,
        query: &str,
        tenant_id: Uuid,
        limit: usize,
    ) -> DsResult<Vec<(Uuid, f32)>>;

    // Vector Search
    async fn index_vector(&mut self, id: Uuid, tenant_id: Uuid, vector: Vec<f32>) -> DsResult<()>;
    async fn search_vector(
        &self,
        tenant_id: Uuid,
        query: &[f32],
        limit: usize,
    ) -> DsResult<Vec<(Uuid, f32)>>;

    // Graph Search
    async fn add_edge(
        &mut self,
        tenant_id: Uuid,
        from: Uuid,
        to: Uuid,
        edge_type: &str,
    ) -> DsResult<()>;
    async fn get_neighbors(
        &self,
        tenant_id: Uuid,
        from: Uuid,
        edge_type: &str,
        direction: EdgeDirection,
    ) -> DsResult<Vec<Uuid>>;

    // Lifecycle
    async fn commit(&mut self) -> DsResult<()>;

    // Management
    async fn delete_document(&mut self, id: Uuid, tenant_id: Uuid) -> DsResult<()>;
}

pub struct YniEngine {
    tokenizer: WhitespaceTokenizer,
    text_store: MemoryTextStore,
    vector_index: MemoryVectorIndex,
    graph_index: MemoryAdjacencyIndex,
}

impl YniEngine {
    pub fn new_in_memory() -> Self {
        Self {
            tokenizer: WhitespaceTokenizer,
            text_store: MemoryTextStore::new(),
            vector_index: MemoryVectorIndex::new(),
            graph_index: MemoryAdjacencyIndex::new(),
        }
    }
}

#[async_trait]
impl IndexEngine for YniEngine {
    // Text Search
    async fn index_text(&mut self, id: Uuid, tenant_id: Uuid, body: &str) -> DsResult<()> {
        let tokens = self.tokenizer.tokenize(body);
        for token in tokens {
            self.text_store.add_term(&token, id, tenant_id)?;
        }
        Ok(())
    }

    async fn index_text_batch(&mut self, items: Vec<(Uuid, Uuid, String)>) -> DsResult<()> {
        for (id, tenant_id, body) in items {
            self.index_text(id, tenant_id, &body).await?;
        }
        Ok(())
    }

    async fn search_text(
        &self,
        query: &str,
        tenant_id: Uuid,
        limit: usize,
    ) -> DsResult<Vec<(Uuid, f32)>> {
        let tokens = self.tokenizer.tokenize(query);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let mut results: Option<BTreeSet<Uuid>> = None;
        for token in tokens {
            let docs = self.text_store.get_docs(&token, tenant_id)?;
            match results {
                None => results = Some(docs),
                Some(ref mut set) => {
                    *set = set.intersection(&docs).cloned().collect();
                }
            }
        }

        let ids = results.unwrap_or_default();
        let mut scored: Vec<(Uuid, f32)> = ids.into_iter().map(|id| (id, 1.0)).collect();
        scored.truncate(limit);
        Ok(scored)
    }

    // Vector Search
    async fn index_vector(&mut self, id: Uuid, tenant_id: Uuid, vector: Vec<f32>) -> DsResult<()> {
        self.vector_index.add_vector(id, tenant_id, vector)
    }

    async fn search_vector(
        &self,
        tenant_id: Uuid,
        query: &[f32],
        limit: usize,
    ) -> DsResult<Vec<(Uuid, f32)>> {
        self.vector_index.search_nearest(tenant_id, query, limit)
    }

    // Graph Search
    async fn add_edge(
        &mut self,
        tenant_id: Uuid,
        from: Uuid,
        to: Uuid,
        edge_type: &str,
    ) -> DsResult<()> {
        self.graph_index.add_edge(tenant_id, from, to, edge_type)
    }

    async fn get_neighbors(
        &self,
        tenant_id: Uuid,
        from: Uuid,
        edge_type: &str,
        direction: EdgeDirection,
    ) -> DsResult<Vec<Uuid>> {
        self.graph_index
            .neighbors(tenant_id, from, edge_type, direction)
    }

    async fn commit(&mut self) -> DsResult<()> {
        Ok(())
    }

    async fn delete_document(&mut self, id: Uuid, tenant_id: Uuid) -> DsResult<()> {
        self.text_store.delete_doc(id, tenant_id)?;
        self.vector_index.delete_vector(id, tenant_id)?;
        self.graph_index.delete_node(id, tenant_id)?;
        Ok(())
    }
}
