use crate::connection::D1Connection;
use tracing::info;
use yykv_types::DsError;

type Result<T> = std::result::Result<T, DsError>;

pub struct D1Adapter;

impl D1Adapter {
    pub async fn connect(_url: &str) -> Result<D1Connection> {
        info!("D1 connecting to: {}", _url);
        Ok(D1Connection::new())
    }
}
