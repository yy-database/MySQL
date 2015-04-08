#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use yyds_types::{YYValue, YYError};
use yykv_event::{WalManager, OpType};
use yykv_types::Redundancy;
use uuid::Uuid;
use tokio::sync::broadcast;
use std::sync::Arc;

/// Pub/Sub message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub topic: String,
    pub payload: YYValue,
    pub tenant_id: Uuid,
}

/// Pub/Sub manager
pub struct PubSubManager {
    wal_manager: Arc<WalManager>,
    // Map topic to sender
    topics: tokio::sync::RwLock<std::collections::HashMap<String, broadcast::Sender<Message>>>,
}

impl PubSubManager {
    pub fn new(wal_manager: Arc<WalManager>) -> Self {
        Self {
            wal_manager,
            topics: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Publish a message to a topic
    pub async fn publish(&self, topic: String, tenant_id: Uuid, payload: YYValue) -> Result<Uuid, YYError> {
        let id = Uuid::now_v7();
        let msg = Message {
            id,
            topic: topic.clone(),
            payload: payload.clone(),
            tenant_id,
        };

        // 1. Write to WAL (WAE)
        // We use the topic as the "table" name for Pub/Sub events
        self.wal_manager.write(
            tenant_id,
            format!("pubsub:{}", topic),
            OpType::Insert,
            Redundancy::SINGLE, // Pub/Sub might not need high redundancy by default
            payload
        ).await.map_err(|e| YYError::internal(e.to_string()))?;

        // 2. Broadcast to local subscribers
        let topics = self.topics.read().await;
        if let Some(sender) = topics.get(&topic) {
            let _ = sender.send(msg);
        }

        Ok(id)
    }

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: String) -> broadcast::Receiver<Message> {
        let mut topics = self.topics.write().await;
        let sender = topics.entry(topic).or_insert_with(|| {
            let (tx, _) = broadcast::channel(1024);
            tx
        });
        sender.subscribe()
    }
}
