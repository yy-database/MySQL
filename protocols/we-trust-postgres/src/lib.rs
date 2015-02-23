pub mod adapter;
pub mod codec;
pub mod connection;
pub mod message;
pub mod protocol;
pub mod transaction;

pub use adapter::PostgresAdapter;
pub use codec::{BackendMessage, PgCodec, PgServerCodec};
pub use connection::PostgresConnection;
pub use message::Message;
pub use transaction::PostgresTransaction;

use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};
use yykv_types::DsError;

pub type Result<T> = std::result::Result<T, DsError>;

/// 使用 tokio Framed 完全重写的 PostgreSQL 连接处理器
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut framed = Framed::new(stream, PgServerCodec::new());
    info!("New Postgres connection established");

    while let Some(result) = framed.next().await {
        match result {
            Ok(msg) => {
                info!("Received Postgres message: {:?}", msg);
                // 处理认证、查询等
            }
            Err(e) => {
                error!("Postgres protocol error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
