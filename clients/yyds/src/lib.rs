#![warn(missing_docs)]
#![doc = include_str!("../readme.md")]

pub use we_trust as trust;

pub use yykv_types as types;
pub use yykv_types::{DsError, DsValue};
#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, DsError>;

/// A lightweight client for the YYDS distributed middleware.
pub struct Client {
    inner: we_trust::WeTrustClient,
}

impl Client {
    /// Connect to a YYDS server.
    pub async fn connect(addr: &str, tenant_id: uuid::Uuid, secret_key: &[u8]) -> Result<Self> {
        let addr = addr
            .parse()
            .map_err(|e| DsError::internal(format!("Invalid address: {}", e)))?;
        let inner = we_trust::WeTrustClient::connect(addr, tenant_id, secret_key.to_vec())
            .await
            .map_err(|e| DsError::internal(format!("Connection failed: {:?}", e)))?;
        Ok(Self { inner })
    }

    /// Execute a query against the YYDS server.
    pub async fn query(&mut self, query: &str) -> Result<Vec<Vec<DsValue>>> {
        self.inner
            .send_query(query)
            .await
            .map_err(|e| DsError::internal(format!("Query failed: {:?}", e)))
    }

    /// Put a value into the YYDS storage.
    pub async fn put(&mut self, key: &str, value: DsValue) -> Result<()> {
        self.inner
            .put(key, value)
            .await
            .map_err(|e| DsError::internal(format!("Put failed: {:?}", e)))
    }

    /// Get a value from the YYDS storage.
    pub async fn get(&mut self, key: &str) -> Result<Option<DsValue>> {
        self.inner
            .get(key)
            .await
            .map_err(|e| DsError::internal(format!("Get failed: {:?}", e)))
    }

    /// Delete a value from the YYDS storage.
    pub async fn delete(&mut self, key: &str) -> Result<()> {
        self.inner
            .delete(key)
            .await
            .map_err(|e| DsError::internal(format!("Delete failed: {:?}", e)))
    }
}

#[allow(missing_docs)]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
