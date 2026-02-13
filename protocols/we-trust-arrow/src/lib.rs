#![warn(missing_docs)]

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use yykv_types::DsError;

pub mod codec;
pub use codec::ArrowCodec;

pub type Result<T> = std::result::Result<T, DsError>;

#[derive(Debug, Clone)]
pub enum ArrowMessage {
    Schema(ArrowSchema),
    Data(ArrowRecordBatch),
}

#[derive(Debug, Clone)]
pub struct ArrowSchema {
    pub fields: Vec<ArrowField>,
}

#[derive(Debug, Clone)]
pub struct ArrowField {
    pub name: String,
    pub data_type: String,
}

#[derive(Debug, Clone)]
pub struct ArrowRecordBatch {
    pub row_count: usize,
    pub columns: Vec<Vec<u8>>,
}

/// 手工实现的 Arrow 协议处理器，完全基于 tokio Framed 重写
pub struct ArrowProtocolHandler;

impl Default for ArrowProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrowProtocolHandler {
    pub fn new() -> Self {
        Self
    }

    /// 处理手工 Arrow 协议连接
    pub async fn handle_connection(&self, stream: TcpStream) -> Result<()> {
        let mut framed = Framed::new(stream, ArrowCodec::new());

        while let Some(result) = framed.next().await {
            match result {
                Ok(msg) => match msg {
                    ArrowMessage::Schema(s) => {
                        println!("Received Arrow Schema with {} fields", s.fields.len());
                    }
                    ArrowMessage::Data(d) => {
                        println!("Received Arrow RecordBatch: {} rows", d.row_count);
                    }
                },
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait ManualFlightService: Send + Sync {
    async fn get_schema(&self, ticket: Vec<u8>) -> Result<ArrowSchema>;
    async fn do_get(
        &self,
        ticket: Vec<u8>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ArrowRecordBatch>> + Send>>>;
}
