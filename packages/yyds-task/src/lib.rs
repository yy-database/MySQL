#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use yyds_types::{DsValue, DsError, DsResult, Redundancy};
use yykv_wal::{WalManager, OpType};

/// Task status in the queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Scheduled(DateTime<Utc>),
}

/// A background task definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub payload: DsValue,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub retry_count: u32,
}

/// Task Manager for handling background operations.
pub struct TaskManager {
    wal_manager: Arc<WalManager>,
    tasks: Arc<RwLock<HashMap<Uuid, Task>>>,
}

impl TaskManager {
    pub fn new(wal_manager: Arc<WalManager>) -> Self {
        Self {
            wal_manager,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Submit a new task to the queue.
    pub async fn submit(&self, tenant_id: Uuid, name: String, payload: DsValue) -> DsResult<Uuid> {
        let id = Uuid::now_v7();
        let task = Task {
            id,
            tenant_id,
            name: name.clone(),
            payload: payload.clone(),
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            retry_count: 0,
        };

        // 1. Write to WAL (WAE) - This ensures task persistence and distribution
        self.wal_manager.write(
            tenant_id,
            "sys:tasks".to_string(),
            OpType::Insert,
            Redundancy::TRUTH, // Tasks are critical, use high redundancy
            payload
        ).await.map_err(|e| DsError::internal(e.to_string()))?;

        // 2. Add to local memory for immediate processing if this is the leader
        let mut tasks = self.tasks.write().await;
        tasks.insert(id, task);

        Ok(id)
    }

    /// Schedule a task for future execution.
    pub async fn schedule(&self, tenant_id: Uuid, name: String, payload: DsValue, when: DateTime<Utc>) -> DsResult<Uuid> {
        let id = Uuid::now_v7();
        let task = Task {
            id,
            tenant_id,
            name: name.clone(),
            payload: payload.clone(),
            status: TaskStatus::Scheduled(when),
            created_at: Utc::now(),
            retry_count: 0,
        };

        // 1. Write to WAL
        self.wal_manager.write(
            tenant_id,
            "sys:tasks".to_string(),
            OpType::Insert,
            Redundancy::TRUTH,
            payload
        ).await.map_err(|e| DsError::internal(e.to_string()))?;

        // 2. Add to local memory
        let mut tasks = self.tasks.write().await;
        tasks.insert(id, task);

        Ok(id)
    }

    pub async fn get_task(&self, id: Uuid) -> Option<Task> {
        let tasks = self.tasks.read().await;
        tasks.get(&id).cloned()
    }

    pub async fn list_tenant_tasks(&self, tenant_id: Uuid) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.values().filter(|t| t.tenant_id == tenant_id).cloned().collect()
    }

    /// Background worker to process tasks.
    /// In a real implementation, this would listen to the WAL stream.
    pub async fn run_worker(self: Arc<Self>) {
        let mut receiver = self.wal_manager.subscribe();
        
        while let Ok(event) = receiver.recv().await {
            if event.table == "sys:tasks" {
                // Handle task event (e.g., execute the task)
                // This is where the actual task logic would go, 
                // possibly involving Wasm-based UDFs as mentioned in the roadmap.
                tracing::debug!("Received task event: {:?}", event.id);
            }
        }
    }
}
