use crate::transaction::SqlServerTransaction;
use crate::{SqlServerService, TdsFrame, TdsPacketType};

use bytes::Bytes;
use std::sync::Arc;
use tracing::info;
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

pub struct SqlServerConnection {
    service: Arc<SqlServerService>,
}

impl SqlServerConnection {
    pub fn new(service: Arc<SqlServerService>) -> Self {
        Self { service }
    }

    pub async fn execute(&self, sql: &str, _params: &[DsValue]) -> Result<u64> {
        info!("SqlServer executing: {}", sql);

        // Wrap SQL in a TdsFrame for processing
        let frame = TdsFrame {
            packet_type: TdsPacketType::SqlBatch,
            status: 1,
            spid: 0,
            packet_id: 1,
            window: 0,
            payload: Bytes::copy_from_slice(sql.as_bytes()),
        };

        match self.service.handle_frame(frame).await? {
            Some(resp) if resp.packet_type == TdsPacketType::TabularResult => Ok(1),
            _ => Ok(0),
        }
    }

    pub async fn query(&self, sql: &str, _params: &[DsValue]) -> Result<Vec<DsValue>> {
        info!("SqlServer querying: {}", sql);

        let frame = TdsFrame {
            packet_type: TdsPacketType::SqlBatch,
            status: 1,
            spid: 0,
            packet_id: 1,
            window: 0,
            payload: Bytes::copy_from_slice(sql.as_bytes()),
        };

        match self.service.handle_frame(frame).await? {
            Some(resp) if resp.packet_type == TdsPacketType::TabularResult => {
                Ok(vec![DsValue::Binary(resp.payload)])
            }
            _ => Ok(vec![]),
        }
    }

    pub async fn begin_transaction(self) -> Result<SqlServerTransaction> {
        Ok(SqlServerTransaction::new(self))
    }
}
