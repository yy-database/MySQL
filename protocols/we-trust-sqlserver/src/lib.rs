#![warn(missing_docs)]

pub mod adapter;
pub mod codec;
pub mod connection;
pub mod transaction;

pub use adapter::SqlServerAdapter;
pub use codec::{TdsCodec, TdsPacket};
pub use connection::SqlServerConnection;
pub use transaction::SqlServerTransaction;

use bytes::Bytes;
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};
use yykv_types::DsError;

pub type Result<T> = std::result::Result<T, DsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TdsPacketType {
    SqlBatch = 0x01,
    Rpc = 0x03,
    TabularResult = 0x04,
    Attention = 0x06,
    BulkLoad = 0x07,
    TransactionManager = 0x0E,
}

pub struct TdsFrame {
    pub packet_type: TdsPacketType,
    pub status: u8,
    pub spid: u16,
    pub packet_id: u8,
    pub window: u8,
    pub payload: Bytes,
}

pub struct SqlServerService;

impl SqlServerService {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub async fn handle_frame(&self, _frame: TdsFrame) -> Result<Option<TdsFrame>> {
        // Placeholder for frame handling logic
        Ok(None)
    }
}

/// 使用 tokio Framed 完全重写的 SQL Server (TDS) 连接处理器
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut framed = Framed::new(stream, TdsCodec);
    info!("New SQL Server connection established");

    while let Some(result) = framed.next().await {
        match result {
            Ok(packet) => {
                info!(
                    "Received TDS packet: type={}, len={}",
                    packet.packet_type, packet.length
                );
                // 处理 TDS 登录、SQL 执行等
            }
            Err(e) => {
                error!("TDS protocol error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
