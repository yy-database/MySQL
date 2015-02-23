pub mod codec;
pub use codec::{VectorCodec, VectorSearchRequest, VectorSearchResponse};

use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};
use yykv_types::DsError;

pub type Result<T> = std::result::Result<T, DsError>;

/// 使用 tokio Framed 完全重写的 Vector 搜索连接处理器
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut framed = Framed::new(stream, VectorCodec);
    info!("New Vector connection established");

    while let Some(result) = framed.next().await {
        match result {
            Ok(req) => {
                info!(
                    "Received Vector search request for collection: {}, dim: {}",
                    req.collection,
                    req.vector.len()
                );
                let resp = VectorSearchResponse {
                    ids: vec![],
                    scores: vec![],
                };
                framed.send(resp).await?;
            }
            Err(e) => {
                error!("Vector protocol error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
