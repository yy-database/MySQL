use crate::connection::MySqlConnection;
use tracing::info;
use yykv_types::DsError;

type Result<T> = std::result::Result<T, DsError>;

pub struct MySqlAdapter;

impl MySqlAdapter {
    pub async fn connect(_url: &str) -> Result<MySqlConnection> {
        info!("MySQL connecting to: {}", _url);
        Ok(MySqlConnection::new())
    }
}
