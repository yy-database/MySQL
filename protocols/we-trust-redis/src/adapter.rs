use crate::RedisService;
use crate::connection::RedisConnection;
use std::sync::Arc;
use yykv_types::DsError;

type Result<T> = std::result::Result<T, DsError>;

pub struct RedisAdapter;

impl RedisAdapter {
    pub async fn connect(_url: &str) -> Result<RedisConnection> {
        let service = Arc::new(RedisService::new()?);
        Ok(RedisConnection::new(service))
    }
}
