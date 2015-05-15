#![warn(missing_docs)]

use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};
use yykv_types::DsError;

pub mod codec;
pub use codec::OtelCodec;

pub type Result<T> = std::result::Result<T, DsError>;

#[derive(Debug, Clone)]
pub struct TraceData {
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
    pub parent_span_id: [u8; 8],
    pub start_time_unix_nano: u64,
    pub end_time_unix_nano: u64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct MetricData {
    pub name: String,
    pub value: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct LogData {
    pub timestamp: u64,
    pub severity: u8,
    pub body: String,
}

#[derive(Debug, Clone)]
pub enum OtelMessage {
    Trace(TraceData),
    Metric(MetricData),
    Log(LogData),
}

/// 使用手动二进制 Codec 处理 OTEL 连接
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut framed = Framed::new(stream, OtelCodec::new());
    info!("New OTEL binary connection established");

    while let Some(result) = framed.next().await {
        match result {
            Ok(msg) => match msg {
                OtelMessage::Trace(t) => {
                    info!("Received Binary Trace: {} (ID: {:x?})", t.name, t.trace_id);
                }
                OtelMessage::Metric(m) => {
                    info!("Received Binary Metric: {} = {}", m.name, m.value);
                }
                OtelMessage::Log(l) => {
                    info!(
                        "Received Binary Log: severity={}, body={}",
                        l.severity, l.body
                    );
                }
            },
            Err(e) => {
                error!("OTEL binary protocol error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
