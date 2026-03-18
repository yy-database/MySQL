//! YYKV client implementation.
//!
//! This crate provides a lightweight client for connecting to a YYKV server.

#![warn(missing_docs)]

use yykv_types::{DsError, DsResult, DsValue};
use yykv_operators::OpsGraph;
use yykv_types::{ToValue, layout::DsValueDecoder, layout::DsValueEncoder};

/// YYKV Client: A lightweight client for the YYKV storage engine.
pub struct Client {
    inner: we_trust::WeTrustClient,
}

impl Client {
    /// Connect to a YYKV server.
    pub async fn connect(addr: &str, tenant_id: uuid::Uuid, secret_key: &[u8]) -> DsResult<Self> {
        let addr = addr
            .parse()
            .map_err(|e| DsError::internal(format!("Invalid address: {}", e)))?;
        let inner = we_trust::WeTrustClient::connect(addr, tenant_id, secret_key.to_vec())
            .await
            .map_err(|e| DsError::internal(format!("Connection failed: {:?}", e)))?;
        Ok(Self { inner })
    }

    /// Get a value by key.
    pub async fn get(&mut self, key: &str) -> DsResult<Option<DsValue>> {
        self.inner
            .get(key)
            .await
            .map_err(|e| DsError::internal(format!("Get failed: {:?}", e)))
    }

    /// Put a value by key.
    pub async fn put(&mut self, key: &str, value: DsValue) -> DsResult<()> {
        self.inner
            .put(key, value)
            .await
            .map_err(|e| DsError::internal(format!("Put failed: {:?}", e)))
    }

    /// Delete a value by key.
    pub async fn delete(&mut self, key: &str) -> DsResult<()> {
        self.inner
            .delete(key)
            .await
            .map_err(|e| DsError::internal(format!("Delete failed: {:?}", e)))
    }

    /// Execute a query against the YYKV server.
    pub async fn query_graph(&mut self, graph: OpsGraph) -> DsResult<Vec<DsValue>> {
        let graph_val = graph.to_value();
        let payload = DsValueEncoder::encode(&graph_val).map_err(|e| DsError::protocol(e.to_string()))?;
        let resp = self
            .inner
            .send_request(we_trust::MessageType::Query, payload)
            .await
            .map_err(|e| DsError::network(None, e.to_string()))?;

        if resp.header.msg_type == we_trust::MessageType::Error as u8 {
            let err = String::from_utf8_lossy(&resp.payload).to_string();
            return Err(DsError::protocol(err));
        }

        let mut data = resp.payload;
        let v = DsValueDecoder::decode(&mut data).map_err(|e| DsError::protocol(e.to_string()))?;
        match v {
            DsValue::List(items) => Ok(items),
            other => Err(DsError::type_err("DsValue::List", format!("{:?}", other))),
        }
    }
}
