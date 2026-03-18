use bytes::Bytes;
use std::collections::BTreeMap;
use std::sync::Arc;
use yykv_types::{DsError, DsValue};

/// Limbo connection wrapper
type LimboConn = limbo::Connection;

/// Convert DsValue to limbo::Value
fn ds_value_to_limbo(value: &DsValue) -> limbo::Value {
    match value {
        DsValue::Null => limbo::Value::Null,
        DsValue::Int(i) => limbo::Value::Integer(*i),
        DsValue::Float(f) => limbo::Value::Real(*f as f64),
        DsValue::Text(s) => limbo::Value::Text(s.clone()),
        DsValue::Bytes(b) => limbo::Value::Blob(b.to_vec()),
        DsValue::Bool(b) => limbo::Value::Integer(if *b { 1 } else { 0 }),
        _ => limbo::Value::Null,
    }
}

/// Convert limbo::Value to DsValue
fn limbo_value_to_ds(value: &limbo::Value) -> DsValue {
    match value {
        limbo::Value::Null => DsValue::Null,
        limbo::Value::Integer(i) => DsValue::Int(*i),
        limbo::Value::Real(f) => DsValue::Float(*f),
        limbo::Value::Text(s) => DsValue::Text(s.clone()),
        limbo::Value::Blob(b) => DsValue::Bytes(Bytes::from(b.clone())),
    }
}

type Result<T> = std::result::Result<T, DsError>;

/// SQLite executor using Limbo (pure Rust implementation)
pub struct SqliteExecutor {
    conn: Arc<LimboConn>,
}

impl SqliteExecutor {
    /// Create a new SQLite executor using Limbo
    pub fn new(conn: Arc<LimboConn>) -> Self {
        Self { conn }
    }

    /// Execute a SQL statement and return the number of affected rows
    pub async fn execute(&self, sql: &str, params: &[DsValue]) -> Result<u64> {
        // Convert DsValue params to limbo::Value
        let limbo_params: Vec<limbo::Value> = params.iter().map(ds_value_to_limbo).collect();
        let result = self.conn.execute(sql, limbo_params).await.map_err(|e| DsError::query(e.to_string()))?;
        Ok(result as u64)
    }

    /// Execute a SQL query and return the results
    pub async fn query(&self, sql: &str, params: &[DsValue]) -> Result<Vec<DsValue>> {
        // Convert DsValue params to limbo::Value
        let limbo_params: Vec<limbo::Value> = params.iter().map(ds_value_to_limbo).collect();
        let mut rows = self.conn.query(sql, limbo_params).await.map_err(|e| DsError::query(e.to_string()))?;
        let mut results = Vec::new();
        
        while let Ok(Some(row)) = rows.next().await {
            let mut fields = BTreeMap::new();
            
            // Get column names by preparing the statement again (simplified approach)
            let stmt = self.conn.prepare(sql).await.map_err(|e| DsError::query(e.to_string()))?;
            let columns = stmt.columns();
            
            for (i, column) in columns.iter().enumerate() {
                let value = self.get_value(&row, i)?;
                fields.insert(column.name().to_string(), value);
            }
            
            results.push(DsValue::Dict(fields));
        }
        
        Ok(results)
    }

    /// Get a value from a row
    fn get_value(&self, row: &limbo::Row, index: usize) -> Result<DsValue> {
        // Try to get the value as different types
        let value = row.get_value(index).map_err(|e| DsError::query(e.to_string()))?;
        Ok(limbo_value_to_ds(&value))
    }
}
