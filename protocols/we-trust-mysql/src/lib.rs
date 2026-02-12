pub mod adapter;
pub mod codec;
pub mod connection;
pub mod transaction;

pub use adapter::MySqlAdapter;
pub use codec::{MySqlCodec, MySqlPacket};
pub use connection::MySqlConnection;
pub use transaction::MySqlTransaction;

use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};
use yykv_types::DsError;

pub type Result<T> = std::result::Result<T, DsError>;

/// 使用 tokio Framed 完全重写的 MySQL 连接处理器
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut framed = Framed::new(stream, MySqlCodec::new());
    info!("New MySQL connection established");

    while let Some(result) = framed.next().await {
        match result {
            Ok(packet) => {
                info!(
                    "Received MySQL packet: seq={}, len={}",
                    packet.sequence_id,
                    packet.payload.len()
                );
                // 这里处理 MySQL 握手、认证和命令执行
            }
            Err(e) => {
                error!("MySQL protocol error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

pub fn init() {
    info!("we-trust-mysql initialized");
}
