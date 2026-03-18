use crate::connection::RedisConnection;
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

pub struct RedisTransaction {
    connection: RedisConnection,
}

impl RedisTransaction {
    pub fn new(connection: RedisConnection) -> Self {
        Self { connection }
    }

    pub async fn execute(&self, sql: &str, params: &[DsValue]) -> Result<u64> {
        self.connection.execute(sql, params).await
    }

    pub async fn query(&self, sql: &str, params: &[DsValue]) -> Result<Vec<DsValue>> {
        self.connection.query(sql, params).await
    }

    pub async fn commit(self) -> Result<()> {
        // Redis MULTI/EXEC placeholder
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        Ok(())
    }
}
