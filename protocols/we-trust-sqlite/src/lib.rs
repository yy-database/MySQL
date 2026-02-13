#![warn(missing_docs)]

pub mod adapter;
pub mod codec;
pub mod connection;
pub mod executor;
pub mod reader;
pub mod transaction;
pub mod utils;
pub mod writer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageType {
    LeafIndex = 0x0a,
    LeafTable = 0x0d,
    InteriorIndex = 0x02,
    InteriorTable = 0x05,
}

pub use adapter::SqliteAdapter;
pub use codec::{SqliteCodec, SqliteCommand, SqliteResponse};
pub use connection::SqliteConnection;
pub use transaction::SqliteTransaction;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};
use yykv_types::DsError;

pub type Result<T> = std::result::Result<T, DsError>;

/// 使用 tokio Framed 完全重写的 SQLite 连接处理器
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut framed = Framed::new(stream, SqliteCodec);
    info!("New SQLite connection established");

    while let Some(result) = framed.next().await {
        match result {
            Ok(cmd) => {
                match cmd {
                    SqliteCommand::Execute(sql) => info!("SQLite Execute: {}", sql),
                    SqliteCommand::Query(sql) => info!("SQLite Query: {}", sql),
                }
                framed.send(SqliteResponse::Ok).await?;
            }
            Err(e) => {
                error!("SQLite protocol error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
