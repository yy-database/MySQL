use crate::connection::SqliteConnection;
use crate::executor::SqliteExecutor;
use std::sync::Arc;
use yykv_types::DsError;

/// Limbo builder
type LimboBuilder = limbo::Builder;

/// Limbo database wrapper
type LimboDb = limbo::Database;

/// Limbo connection wrapper
type LimboConn = limbo::Connection;

type Result<T> = std::result::Result<T, DsError>;

pub struct SqliteAdapter;

impl SqliteAdapter {
    /// Connect to a SQLite database using Limbo (pure Rust implementation)
    pub async fn connect(path: &str) -> Result<SqliteConnection> {
        // Open database using Limbo
        let db = LimboBuilder::new_local(path).build().await.map_err(|e| DsError::network(None, e.to_string()))?;
        
        // Create connection using Limbo
        let conn = db.connect().map_err(|e| DsError::network(None, e.to_string()))?;
        let conn = Arc::new(conn);
        
        // Create executor using Limbo
        let executor = Arc::new(SqliteExecutor::new(conn.clone()));

        Ok(SqliteConnection::new(conn, executor))
    }
}
