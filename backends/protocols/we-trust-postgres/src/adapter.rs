use crate::connection::PostgresConnection;
use tracing::info;
use yykv_types::DsError;

type Result<T> = std::result::Result<T, DsError>;

pub struct PostgresAdapter;

impl PostgresAdapter {
    pub async fn connect(url: &str) -> Result<PostgresConnection> {
        info!("Postgres connecting to: {}", url);
        Ok(PostgresConnection::new(url.to_string()))
    }
}
