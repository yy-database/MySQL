use crate::SqlServerService;
use crate::connection::SqlServerConnection;
use std::sync::Arc;
use tracing::info;
use yykv_types::DsError;

type Result<T> = std::result::Result<T, DsError>;

pub struct SqlServerAdapter;

impl SqlServerAdapter {
    pub async fn connect(_url: &str) -> Result<SqlServerConnection> {
        info!("SqlServer connecting to: {}", _url);
        let service = Arc::new(SqlServerService::new()?);
        Ok(SqlServerConnection::new(service))
    }
}
