#![warn(missing_docs)]
#![doc = include_str!("../readme.md")]

pub use we_trust as trust;
#[cfg(feature = "full")]
pub use we_trust_postgres as postgres;
#[cfg(feature = "full")]
pub use we_trust_s3 as s3;
#[cfg(feature = "full")]
pub use we_trust_otel as otel;
#[cfg(feature = "full")]
pub use we_trust_arrow as arrow;
#[cfg(feature = "full")]
pub use we_trust_mysql as mysql;
#[cfg(feature = "full")]
pub use we_trust_vector as vector;

pub use yykv_types as types;
pub use yykv_types::{DsValue, DsError};
pub type Result<T> = std::result::Result<T, DsError>;

#[cfg(feature = "full")]
pub use yyds_consensus as consensus;
#[cfg(feature = "full")]
pub use yyds_compute as compute;
#[cfg(feature = "full")]
pub use yyds_ai as ai;
#[cfg(feature = "full")]
pub use yykv_event as wal;
#[cfg(feature = "full")]
pub use yykv_index as index;
#[cfg(feature = "full")]
pub use yykv_schema as schema;
#[cfg(feature = "full")]
pub use yyds_task as task;
#[cfg(feature = "full")]
pub use yykv_operators as operators;
#[cfg(feature = "full")]
pub use yykv_executor as executor;
#[cfg(feature = "full")]
pub use yyds_gateway as gateway;
#[cfg(feature = "full")]
pub use yyds_observability as observability;
#[cfg(feature = "full")]
pub use yyds_tenant as tenant;
#[cfg(feature = "full")]
pub use yyds_billing as billing;
#[cfg(feature = "full")]
pub use yyds_quota as quota;
#[cfg(feature = "full")]
pub use yyds_pubsub as pubsub;
#[cfg(feature = "full")]
pub use yyds_cache as cache;

/// A lightweight client for the YYDS distributed middleware.
pub struct Client {
    inner: we_trust::WeTrustClient,
}

impl Client {
    /// Connect to a YYDS server.
    pub async fn connect(
        addr: &str,
        tenant_id: uuid::Uuid,
        secret_key: &[u8],
    ) -> Result<Self> {
        let addr = addr.parse().map_err(|e| DsError::internal(format!("Invalid address: {}", e)))?;
        let inner = we_trust::WeTrustClient::connect(addr, tenant_id, secret_key.to_vec())
            .await
            .map_err(|e| DsError::internal(format!("Connection failed: {:?}", e)))?;
        Ok(Self { inner })
    }

    /// Execute a query against the YYDS server.
    pub async fn query(&mut self, query: &str) -> Result<Vec<Vec<DsValue>>> {
        self.inner.send_query(query).await
            .map_err(|e| DsError::internal(format!("Query failed: {:?}", e)))
    }

    /// Put a value into the YYDS storage.
    pub async fn put(&mut self, key: &str, value: DsValue) -> Result<()> {
        self.inner.put(key, value).await
            .map_err(|e| DsError::internal(format!("Put failed: {:?}", e)))
    }

    /// Get a value from the YYDS storage.
    pub async fn get(&mut self, key: &str) -> Result<Option<DsValue>> {
        self.inner.get(key).await
            .map_err(|e| DsError::internal(format!("Get failed: {:?}", e)))
    }

    /// Delete a value from the YYDS storage.
    pub async fn delete(&mut self, key: &str) -> Result<()> {
        self.inner.delete(key).await
            .map_err(|e| DsError::internal(format!("Delete failed: {:?}", e)))
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
