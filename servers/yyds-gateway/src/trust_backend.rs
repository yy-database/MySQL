use crate::GatewayBackend;
use async_trait::async_trait;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use uuid::Uuid;
use we_trust::{MessageType, WeTrustClient};
use yykv_operators::OpsGraph;
use yykv_types::{
    layout::{DsValueDecoder, DsValueEncoder},
    DsError, DsValue, ToValue,
};

type Result<T> = std::result::Result<T, DsError>;

#[derive(Clone)]
pub struct TrustBackend {
    addr: SocketAddr,
    secrets: Arc<HashMap<Uuid, Vec<u8>>>,
}

impl TrustBackend {
    pub fn new(addr: SocketAddr, secrets: HashMap<Uuid, Vec<u8>>) -> Self {
        Self {
            addr,
            secrets: Arc::new(secrets),
        }
    }

    async fn connect(&self, tenant_id: Uuid) -> Result<WeTrustClient> {
        let secret_key = self
            .secrets
            .get(&tenant_id)
            .ok_or_else(|| DsError::permission("trust_connect", "unknown_tenant"))?
            .clone();
        WeTrustClient::connect(self.addr, tenant_id, secret_key)
            .await
            .map_err(|e| DsError::network(Some(self.addr.to_string()), e.to_string()))
    }

    fn decode_list(payload: bytes::Bytes) -> Result<Vec<DsValue>> {
        let mut data = payload;
        let v = DsValueDecoder::decode(&mut data).map_err(|e| DsError::protocol(e.to_string()))?;
        match v {
            DsValue::List(items) => Ok(items),
            other => Err(DsError::type_err(
                "DsValue::List",
                format!("{:?}", other),
            )),
        }
    }
}

#[async_trait]
impl GatewayBackend for TrustBackend {
    async fn execute_query(&self, tenant_id: Uuid, graph: OpsGraph) -> Result<Vec<DsValue>> {
        let mut client = self.connect(tenant_id).await?;
        let graph_val = graph.to_value();
        let payload =
            DsValueEncoder::encode(&graph_val).map_err(|e| DsError::protocol(e.to_string()))?;
        let resp = client
            .send_request(MessageType::Query, payload)
            .await
            .map_err(|e| DsError::network(Some(self.addr.to_string()), e.to_string()))?;

        match MessageType::from(resp.header.msg_type) {
            MessageType::QueryResp | MessageType::Response => Self::decode_list(resp.payload),
            MessageType::Error => {
                let mut data = resp.payload;
                if data.is_empty() {
                    return Err(DsError::protocol("Empty error payload"));
                }
                let v = DsValueDecoder::decode(&mut data)
                    .map_err(|e| DsError::protocol(e.to_string()))?;
                Err(DsError::execution_with_op("trust_query", format!("{:?}", v)))
            }
            other => Err(DsError::protocol(format!(
                "Unexpected response type: {:?}",
                other
            ))),
        }
    }

    async fn execute_insert(
        &self,
        tenant_id: Uuid,
        _table: String,
        key: DsValue,
        value: DsValue,
    ) -> Result<()> {
        let mut client = self.connect(tenant_id).await?;
        let key_str = match key {
            DsValue::Text(s) => s,
            DsValue::Int(i) => i.to_string(),
            DsValue::Uuid(u) => u.to_string(),
            _ => {
                return Err(DsError::execution_with_op(
                    "trust_insert",
                    "Unsupported key type",
                ));
            }
        };
        client
            .put(&key_str, value)
            .await
            .map_err(|e| DsError::network(Some(self.addr.to_string()), e.to_string()))
    }

    async fn execute_delete(
        &self,
        _tenant_id: Uuid,
        _table: String,
        _selection: Option<yykv_operators::Expr>,
    ) -> Result<()> {
        Err(DsError::execution_with_op(
            "trust_delete",
            "DELETE via TrustBackend requires key-based delete",
        ))
    }
}

