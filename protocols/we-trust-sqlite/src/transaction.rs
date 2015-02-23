use crate::connection::SqliteConnection;
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

pub struct SqliteTransaction {
    connection: SqliteConnection,
}

impl SqliteTransaction {
    pub fn new(connection: SqliteConnection) -> Self {
        Self { connection }
    }

    pub async fn execute(&self, sql: &str, params: &[DsValue]) -> Result<u64> {
        self.connection.execute(sql, params).await
    }

    pub async fn query(&self, sql: &str, params: &[DsValue]) -> Result<Vec<DsValue>> {
        self.connection.query(sql, params).await
    }

    pub async fn commit(self) -> Result<()> {
        // SQLite native driver currently doesn't have explicit transaction support in writer
        // but it performs atomic writes.
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        Ok(())
    }
}
