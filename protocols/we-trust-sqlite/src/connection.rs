use crate::executor::SqliteExecutor;
use crate::transaction::SqliteTransaction;
use async_trait::async_trait;
use std::sync::Arc;
use yykv_types::{ColumnInfo, DsError, DsResult, DsValue, EnumInfo, SchemaInspector, TableInfo};

/// Limbo connection wrapper
type LimboConn = limbo::Connection;

#[async_trait]
impl SchemaInspector for SqliteConnection {
    async fn introspect(&self, _schema: Option<&str>) -> DsResult<(Vec<TableInfo>, Vec<EnumInfo>)> {
        // SQLite doesn't have multiple schemas in the same way as Postgres (ignoring ATTACH DATABASE for now)

        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
        let table_rows = self.query(sql, &[]).await?;

        let mut tables = Vec::new();
        for row in table_rows {
            if let DsValue::List(fields) = row {
                let table_name = match fields.first() {
                    Some(DsValue::Text(s)) => s.clone(),
                    _ => continue,
                };

                let mut columns = Vec::new();
                let col_sql = format!("PRAGMA table_info({})", table_name);
                let col_rows = self.query(&col_sql, &[]).await?;

                for col_row in col_rows {
                    if let DsValue::List(cfields) = col_row {
                        let name = match cfields.get(1) {
                            Some(DsValue::Text(s)) => s.clone(),
                            _ => continue,
                        };
                        let data_type = match cfields.get(2) {
                            Some(DsValue::Text(s)) => s.clone(),
                            _ => continue,
                        };
                        let is_nullable = match cfields.get(3) {
                            Some(DsValue::Int(v)) => *v == 0,
                            _ => true,
                        };
                        let is_primary_key = match cfields.get(5) {
                            Some(DsValue::Int(v)) => *v > 0,
                            _ => false,
                        };
                        let default = match cfields.get(4) {
                            Some(DsValue::Text(s)) => Some(s.clone()),
                            _ => None,
                        };

                        columns.push(ColumnInfo {
                            name,
                            data_type,
                            is_nullable,
                            is_primary_key,
                            is_enum: false,
                            foreign_key: None,
                            default,
                            description: None,
                        });
                    }
                }

                tables.push(TableInfo {
                    name: table_name,
                    columns,
                    description: None,
                });
            }
        }

        Ok((tables, Vec::new()))
    }
}

type Result<T> = std::result::Result<T, DsError>;

/// SQLite connection using Limbo (pure Rust implementation)
pub struct SqliteConnection {
    conn: Arc<LimboConn>,
    executor: Arc<SqliteExecutor>,
}

impl SqliteConnection {
    /// Create a new SQLite connection using Limbo
    pub fn new(
        conn: Arc<LimboConn>,
        executor: Arc<SqliteExecutor>,
    ) -> Self {
        Self {
            conn,
            executor,
        }
    }

    /// Execute a SQL statement and return the number of affected rows
    pub async fn execute(&self, sql: &str, params: &[DsValue]) -> Result<u64> {
        self.executor.execute(sql, params).await
    }

    /// Execute a SQL query and return the results
    pub async fn query(&self, sql: &str, params: &[DsValue]) -> Result<Vec<DsValue>> {
        self.executor.query(sql, params).await
    }

    /// Begin a new transaction
    pub async fn begin_transaction(self) -> Result<SqliteTransaction> {
        Ok(SqliteTransaction::new(self))
    }
}
