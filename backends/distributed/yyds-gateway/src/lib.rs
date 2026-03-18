#![warn(missing_docs)]

use async_trait::async_trait;
#[cfg(feature = "s3")]
use we_trust_s3::PutObjectOutput;
use yykv_operators::OpsGraph;
use yykv_types::{DsError, DsValue};

pub mod rbq;
pub mod sql;
pub mod trust_backend;

use crate::rbq::{RbqParser, RbqStatement};
use crate::sql::{SqlParser, SqlStatement};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use tracing::{error, info};
use uuid::Uuid;
use we_trust_postgres::{BackendMessage, PgServerCodec};

type Result<T> = std::result::Result<T, DsError>;

/// GatewayBackend defines the operations that a gateway can perform on the core.
#[async_trait]
pub trait GatewayBackend: Send + Sync {
    async fn execute_query(&self, tenant_id: Uuid, graph: OpsGraph) -> Result<Vec<DsValue>>;
    async fn execute_insert(
        &self,
        tenant_id: Uuid,
        table: String,
        key: DsValue,
        value: DsValue,
    ) -> Result<()>;
    async fn execute_delete(
        &self,
        tenant_id: Uuid,
        table: String,
        selection: Option<yykv_operators::Expr>,
    ) -> Result<()>;
}

/// YYDS Gateway - Multi-protocol compatibility layer
///
/// Responsibilities:
/// 1. Protocol listening (Postgres/MySQL/S3/Arrow/OTLP)
/// 2. Use we-trust-* for protocol encoding/decoding
/// 3. Convert external requests into kernel-native Operator Graphs (Ops Graph)
/// 4. Distribute generated Ops Graphs to yykv-executor for execution
#[derive(Clone)]
pub struct YydsGateway {
    backend: Arc<dyn GatewayBackend>,
}

impl YydsGateway {
    pub fn new(backend: Arc<dyn GatewayBackend>) -> Self {
        Self { backend }
    }

    /// Start Postgres protocol listening
    pub async fn listen_postgres(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| DsError::io_raw(e, None))?;
        info!("YYDS Postgres Gateway listening on {}", addr);

        loop {
            let (socket, _) = listener
                .accept()
                .await
                .map_err(|e| DsError::io_raw(e, None))?;
            let gateway = self.clone();

            tokio::spawn(async move {
                let mut framed = Framed::new(socket, PgServerCodec::default());

                // Simple handshake / auth placeholder (Tenant ID is fixed for now in PG gateway)
                // In a real scenario, this would come from auth metadata
                let tenant_id = Uuid::nil();

                while let Some(Ok(msg)) = framed.next().await {
                    match msg {
                        we_trust_postgres::Message::Startup { .. } => {
                            let _ = framed.send(BackendMessage::AuthenticationOk).await;
                            let _ = framed
                                .send(BackendMessage::ReadyForQuery { status: b'I' })
                                .await;
                        }
                        we_trust_postgres::Message::Query(query) => {
                            // Convert SQL to OpsGraph and execute
                            let result = gateway.handle_pg_query(tenant_id, &query).await;

                            match result {
                                Ok(rows) => {
                                    if !rows.is_empty() {
                                        // Simple RowDescription for basic types
                                        let fields =
                                            vec![we_trust_postgres::codec::FieldDescription {
                                                name: "value".into(),
                                                table_oid: 0,
                                                column_id: 1,
                                                type_oid: 25, // TEXT
                                                type_size: -1,
                                                type_modifier: -1,
                                                format_code: 0,
                                            }];
                                        let _ = framed
                                            .send(BackendMessage::RowDescription { fields })
                                            .await;

                                        for row in &rows {
                                            let val_bytes = match row {
                                                DsValue::Text(s) => Some(s.clone().into()),
                                                DsValue::Int(i) => Some(i.to_string().into()),
                                                DsValue::Float(f) => Some(f.to_string().into()),
                                                DsValue::Bool(b) => Some(b.to_string().into()),
                                                DsValue::Uuid(u) => Some(u.to_string().into()),
                                                DsValue::Null => None,
                                                _ => Some(format!("{:?}", row).into()),
                                            };
                                            let _ = framed
                                                .send(BackendMessage::DataRow {
                                                    values: vec![val_bytes],
                                                })
                                                .await;
                                        }
                                    }

                                    let tag = if query.to_uppercase().starts_with("INSERT") {
                                        "INSERT 0 1".to_string()
                                    } else if query.to_uppercase().starts_with("DELETE") {
                                        "DELETE 1".to_string()
                                    } else {
                                        let count = rows.len();
                                        format!("SELECT {}", count)
                                    };
                                    let _ =
                                        framed.send(BackendMessage::CommandComplete { tag }).await;
                                }
                                Err(e) => {
                                    error!("Error handling query: {:?}", e);
                                    let _ = framed
                                        .send(BackendMessage::ErrorResponse {
                                            fields: vec![
                                                (b'S', "ERROR".into()),
                                                (b'C', "XX000".into()),
                                                (b'M', format!("{:?}", e)),
                                            ],
                                        })
                                        .await;
                                }
                            }
                            let _ = framed
                                .send(BackendMessage::ReadyForQuery { status: b'I' })
                                .await;
                        }
                        _ => {}
                    }
                }
            });
        }
    }

    /// Handle Postgres protocol requests
    pub async fn handle_pg_query(&self, tenant_id: Uuid, sql: &str) -> Result<Vec<DsValue>> {
        // 1. Parse SQL to SqlStatement (which contains OpsGraph for queries)
        let stmt = SqlParser::parse_sql(sql)
            .map_err(|e| DsError::query_with_sql(sql, format!("Parse error: {:?}", e)))?;

        // 2. Execute based on statement type
        match stmt {
            SqlStatement::Query(graph) => self
                .backend
                .execute_query(tenant_id, graph)
                .await
                .map_err(|e| DsError::internal(format!("Execution error: {:?}", e))),
            SqlStatement::Insert { table, key, value } => {
                self.backend
                    .execute_insert(tenant_id, table, key, value)
                    .await?;
                Ok(Vec::new())
            }
            SqlStatement::Delete { table, selection } => {
                self.backend
                    .execute_delete(tenant_id, table, selection)
                    .await?;
                Ok(Vec::new())
            }
            _ => Err(DsError::query_with_sql(
                sql,
                "Unsupported statement type via gateway for now",
            )),
        }
    }

    /// Handle MySQL protocol requests
    pub async fn handle_mysql_query(&self, tenant_id: Uuid, sql: &str) -> Result<Vec<DsValue>> {
        self.handle_pg_query(tenant_id, sql).await
    }

    /// Handle RBQ protocol requests
    pub async fn handle_rbq_query(&self, tenant_id: Uuid, rbq: &str) -> Result<Vec<DsValue>> {
        // 1. Parse RBQ to RbqStatement
        let stmt = RbqParser::parse_rbq(rbq)
            .map_err(|e| DsError::query_with_sql(rbq, format!("Parse error: {:?}", e)))?;

        // 2. Execute based on statement type
        match stmt {
            RbqStatement::Query(graph) => self
                .backend
                .execute_query(tenant_id, graph)
                .await
                .map_err(|e| DsError::internal(format!("Execution error: {:?}", e))),
        }
    }

    #[cfg(feature = "s3")]
    pub async fn handle_s3_put_object(
        &self,
        _bucket: &str,
        _key: &str,
        _data: bytes::Bytes,
    ) -> Result<PutObjectOutput> {
        let _graph = OpsGraph::new();
        Ok(PutObjectOutput {
            e_tag: "placeholder".into(),
            version_id: None,
        })
    }

    /// Handle Arrow Flight streaming read
    pub async fn handle_arrow_flight(&self, _ticket: Vec<u8>) -> Result<()> {
        // Convert Arrow Flight request to Ops Graph
        // Core advantage: Leverage Arrow zero-copy features to directly return operator execution results
        let _graph = OpsGraph::new();
        // ... mapping logic ...
        Ok(())
    }
}
