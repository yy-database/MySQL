#![warn(missing_docs)]

pub mod auth;
pub mod codec;
pub mod error;
pub mod models;

pub use codec::S3HttpCodec;
pub use error::{Result, S3Error};
pub use models::*;

use futures::{SinkExt, StreamExt};
use http::{Response, StatusCode};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};
use yykv_types::DsError;

/// 使用 tokio Framed 完全重写的 S3 (HTTP) 连接处理器
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut framed = Framed::new(stream, S3HttpCodec);
    info!("New S3 connection established");

    while let Some(result) = framed.next().await {
        match result {
            Ok(req) => {
                info!("Received S3 request: {:?} {}", req.method(), req.uri());
                let resp = Response::builder()
                    .status(StatusCode::OK)
                    .body(vec![])
                    .map_err(|e| DsError::protocol(e.to_string()))?;
                framed.send(resp).await?;
            }
            Err(e) => {
                error!("S3 protocol error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
