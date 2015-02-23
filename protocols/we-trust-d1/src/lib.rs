pub mod adapter;
pub mod codec;
pub mod connection;
pub mod transaction;
pub mod utils;

pub use adapter::D1Adapter;
pub use codec::{D1Codec, D1Request, D1Response};
pub use connection::D1Connection;
pub use transaction::D1Transaction;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};
use yykv_types::DsError;

pub type Result<T> = std::result::Result<T, DsError>;

/// 使用 tokio Framed 完全重写的 D1 连接处理器
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut framed = Framed::new(stream, D1Codec);
    info!("New D1 connection established");

    while let Some(result) = framed.next().await {
        match result {
            Ok(req) => {
                info!("Received D1 SQL: {}", req.sql);
                let resp = D1Response {
                    success: true,
                    results: None,
                    error: None,
                };
                framed.send(resp).await?;
            }
            Err(e) => {
                error!("D1 protocol error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
