use crate::connection::SqlServerConnection;
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

pub struct SqlServerTransaction {
    connection: SqlServerConnection,
}

impl SqlServerTransaction {
    pub fn new(connection: SqlServerConnection) -> Self {
        Self { connection }
    }

    pub async fn execute(&self, sql: &str, params: &[DsValue]) -> Result<u64> {
        self.connection.execute(sql, params).await
    }

    pub async fn query(&self, sql: &str, params: &[DsValue]) -> Result<Vec<DsValue>> {
        self.connection.query(sql, params).await
    }

    pub async fn commit(self) -> Result<()> {
        self.connection.execute("COMMIT", &[]).await?;
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        self.connection.execute("ROLLBACK", &[]).await?;
        Ok(())
    }
}
