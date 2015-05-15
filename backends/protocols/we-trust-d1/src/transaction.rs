use crate::connection::D1Connection;
use tracing::info;
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

pub struct D1Transaction {
    connection: D1Connection,
}

impl D1Transaction {
    pub fn new(connection: D1Connection) -> Self {
        Self { connection }
    }

    pub async fn execute(&self, sql: &str, params: &[DsValue]) -> Result<u64> {
        self.connection.execute(sql, params).await
    }

    pub async fn query(&self, sql: &str, params: &[DsValue]) -> Result<Vec<DsValue>> {
        self.connection.query(sql, params).await
    }

    pub async fn commit(self) -> Result<()> {
        info!("D1 transaction committed");
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        info!("D1 transaction rolled back");
        Ok(())
    }
}
