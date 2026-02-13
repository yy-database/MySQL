#![warn(missing_docs)]

use async_trait::async_trait;
use yykv_wal::WalManager;
use yykv_operators::{OperatorNode, OpsGraph};
use yykv_types::layout::DsValueEncoder;
use yykv_types::{DsError, DsResult, DsValue};
// use yykv_index::SearchIndexManager;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use yykv_compute::UdfEngine;

use std::collections::HashMap;

#[async_trait]
pub trait DistributedExecutor {
    async fn execute(&self, graph: OpsGraph) -> DsResult<Vec<DsValue>>;
    async fn pushdown(&self, node_id: usize, target_node_id: Uuid) -> DsResult<()>;
}

/// Node representing a compute unit in the cluster
pub struct ComputeNode {
    pub id: Uuid,
    pub address: String,
}

/// WAE Worker for Near-Data Computing
pub struct WaeWorker {
    tenant_id: Uuid,
    compute: Arc<dyn UdfEngine>,
    wal: Arc<WalManager>,
}

impl WaeWorker {
    pub fn new(tenant_id: Uuid, compute: Arc<dyn UdfEngine>, wal: Arc<WalManager>) -> Self {
        Self {
            tenant_id,
            compute,
            wal,
        }
    }

    pub async fn start(&self, table_filter: String, udf_name: String) -> mpsc::Receiver<DsValue> {
        let mut rx = self.wal.subscribe();
        let (tx, results_rx) = mpsc::channel(100);
        let compute = self.compute.clone();
        let tenant_id = self.tenant_id;

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                // Multi-tenancy and table filtering
                if event.tenant_id() == tenant_id && event.table() == table_filter {
                    if let Some(data) = event.value() {
                        if let Ok(data_bytes) = DsValueEncoder::encode(data) {
                            if let Ok(processed) = compute.call(&udf_name, data_bytes.to_vec()).await {
                                let _ = tx.send(processed).await;
                            }
                        }
                    }
                }
            }
        });

        results_rx
    }
}

pub struct YykvExecutor {
    pub id: Uuid,
    compute: Arc<dyn UdfEngine>,
    wal: Arc<WalManager>,
    // index: Arc<SearchIndexManager>,
    peers: HashMap<Uuid, ComputeNode>,
}

impl YykvExecutor {
    pub fn new(
        id: Uuid,
        compute: Arc<dyn UdfEngine>,
        wal: Arc<WalManager>, /*, index: Arc<SearchIndexManager> */
    ) -> Self {
        Self {
            id,
            compute,
            wal,
            // index,
            peers: HashMap::new(),
        }
    }

    pub async fn execute_local(&self, graph: OpsGraph, _tenant_id: Uuid) -> DsResult<Vec<DsValue>> {
        let mut results = Vec::new();
        for node in graph.nodes {
            match node {
                OperatorNode::VectorSearch(_op) => {
                    // results = self.index.search(...)
                }
                OperatorNode::Filter(_op) => {
                    // filter results
                }
                _ => {}
            }
        }

        // Mock data for testing
        if results.is_empty() {
            results.push(DsValue::Text("yykv-row-1".into()));
            results.push(DsValue::Text("yykv-row-2".into()));
        }

        Ok(results)
    }

    pub fn add_peer(&mut self, node: ComputeNode) {
        self.peers.insert(node.id, node);
    }

    /// Create a new WAE worker for a specific tenant
    pub fn create_wae_worker(&self, tenant_id: Uuid) -> WaeWorker {
        WaeWorker::new(tenant_id, self.compute.clone(), self.wal.clone())
    }
}

#[async_trait]
impl DistributedExecutor for YykvExecutor {
    async fn execute(&self, graph: OpsGraph) -> DsResult<Vec<DsValue>> {
        self.execute_local(graph, Uuid::nil()).await
    }

    async fn pushdown(&self, _node_id: usize, target_node_id: Uuid) -> DsResult<()> {
        let _peer = self
            .peers
            .get(&target_node_id)
            .ok_or_else(|| DsError::internal(format!("Peer {} not found", target_node_id)))?;

        // Push compute to peer
        Ok(())
    }
}
