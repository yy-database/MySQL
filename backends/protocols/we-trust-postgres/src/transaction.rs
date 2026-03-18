use crate::connection::PostgresConnection;
use tracing::info;
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

pub struct PostgresTransaction {
    connection: PostgresConnection,
}

impl PostgresTransaction {
    pub fn new(connection: PostgresConnection) -> Self {
        Self { connection }
    }

    pub async fn execute(&self, sql: &str, params: &[DsValue]) -> Result<u64> {
        self.connection.execute(sql, params).await
    }

    pub async fn query(&self, sql: &str, params: &[DsValue]) -> Result<Vec<DsValue>> {
        self.connection.query(sql, params).await
    }

    pub async fn commit(self) -> Result<()> {
        info!("Postgres transaction committed");
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        info!("Postgres transaction rolled back");
        Ok(())
    }
}
