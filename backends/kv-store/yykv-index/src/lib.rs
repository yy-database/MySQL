#![warn(missing_docs)]

pub mod yni;

use crate::yni::{IndexEngine, YniEngine};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use yyds_types::{DsResult, DsValue};
use yykv_wal::WalManager;

/// Search Index Manager
pub struct SearchIndexManager {
    /// The underlying index engine.
    /// Standard implementation is YniEngine, but you can plug in any implementation
    /// of IndexEngine (e.g., Elasticsearch, Milvus, Tantivy, etc.)
    engine: Arc<Mutex<dyn IndexEngine>>,
}

impl SearchIndexManager {
    pub fn new_in_memory() -> DsResult<Self> {
        let engine = YniEngine::new_in_memory();

        Ok(Self {
            engine: Arc::new(Mutex::new(engine)),
        })
    }

    /// Index a document (Text)
    pub async fn index_document(&self, id: Uuid, tenant_id: Uuid, body: String) -> DsResult<()> {
        let mut engine = self.engine.lock().await;
        engine.index_text(id, tenant_id, &body).await
    }

    /// Search for documents (Text)
    pub async fn search(
        &self,
        query_str: &str,
        tenant_id: Uuid,
        limit: usize,
    ) -> DsResult<Vec<(Uuid, f32)>> {
        let engine = self.engine.lock().await;
        engine.search_text(query_str, tenant_id, limit).await
    }

    /// Listen to WAL for real-time indexing
    pub async fn run_indexer(self: Arc<Self>, wal_manager: Arc<WalManager>) {
        let mut receiver = wal_manager.subscribe();
        while let Ok(event) = receiver.recv().await {
            // Only index tables marked for full-text search or specific system tables
            if (event.table().starts_with("search:") || event.table() == "sys:docs") {
                if let Some(DsValue::Text(content)) = event.value() {
                    let _ = self
                        .index_document(event.id(), event.tenant_id(), content.clone())
                        .await;
                }
            }
        }
    }
}
