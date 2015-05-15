#![warn(missing_docs)]

pub mod adapter;
pub mod codec;
pub mod connection;
pub mod executor;
pub mod transaction;
pub mod utils;

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

/// SQLite connection handler using Limbo (pure Rust implementation)
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
