use async_trait::async_trait;
use yyds_types::{YYValue, YYError};
use we_trust_s3::{PutObjectOutput};
use yykv_operators::OpsGraph;

pub mod sql;
pub mod rbq;

use crate::sql::{SqlParser, SqlStatement};
use crate::rbq::{RbqParser, RbqStatement};
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use we_trust_postgres::{PgServerCodec, BackendMessage};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tracing::{info, error};
use uuid::Uuid;

type Result<T> = std::result::Result<T, YYError>;

/// GatewayBackend defines the operations that a gateway can perform on the core.
#[async_trait]
pub trait GatewayBackend: Send + Sync {
    async fn execute_query(&self, tenant_id: Uuid, graph: OpsGraph) -> Result<Vec<YYValue>>;
    async fn execute_insert(&self, tenant_id: Uuid, table: String, key: YYValue, value: YYValue) -> Result<()>;
    async fn execute_delete(&self, tenant_id: Uuid, table: String, selection: Option<yykv_operators::Expr>) -> Result<()>;
}

/// YYDS Gateway - 多协议兼容层
/// 
/// 职责：
/// 1. 协议监听 (Postgres/MySQL/S3/Arrow/OTLP)
/// 2. 使用 we-trust-* 处理协议编解码
/// 3. 将外部请求转换为内核原生的算子图 (Ops Graph)
/// 4. 将生成的算子图分发给 yykv-executor 执行
#[derive(Clone)]
pub struct YydsGateway {
    backend: Arc<dyn GatewayBackend>,
}

impl YydsGateway {
    pub fn new(backend: Arc<dyn GatewayBackend>) -> Self {
        Self { backend }
    }

    /// 启动 Postgres 协议监听
    pub async fn listen_postgres(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr).await
            .map_err(|e| YYError::io_raw(e, None))?;
        info!("YYDS Postgres Gateway listening on {}", addr);

        loop {
            let (socket, _) = listener.accept().await
                .map_err(|e| YYError::io_raw(e, None))?;
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
                            let _ = framed.send(BackendMessage::ReadyForQuery { status: b'I' }).await;
                        }
                        we_trust_postgres::Message::Query(query) => {
                            // Convert SQL to OpsGraph and execute
                            let result = gateway.handle_pg_query(tenant_id, &query).await;
                            
                            match result {
                                Ok(rows) => {
                                    if !rows.is_empty() {
                                        // Simple RowDescription for basic types
                                        let fields = vec![
                                            we_trust_postgres::codec::FieldDescription {
                                                name: "value".into(),
                                                table_oid: 0,
                                                column_id: 1,
                                                type_oid: 25, // TEXT
                                                type_size: -1,
                                                type_modifier: -1,
                                                format_code: 0,
                                            }
                                        ];
                                        let _ = framed.send(BackendMessage::RowDescription { fields }).await;

                                        for row in &rows {
                                            let val_bytes = match row {
                                                YYValue::Text(s) => Some(s.clone().into()),
                                                YYValue::Int(i) => Some(i.to_string().into()),
                                                YYValue::Float(f) => Some(f.to_string().into()),
                                                YYValue::Bool(b) => Some(b.to_string().into()),
                                                YYValue::Uuid(u) => Some(u.to_string().into()),
                                                YYValue::Null => None,
                                                _ => Some(format!("{:?}", row).into()),
                                            };
                                            let _ = framed.send(BackendMessage::DataRow { values: vec![val_bytes] }).await;
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
                                    let _ = framed.send(BackendMessage::CommandComplete { tag }).await;
                                }
                                Err(e) => {
                                    error!("Error handling query: {:?}", e);
                                    let _ = framed.send(BackendMessage::ErrorResponse { 
                                        fields: vec![
                                            (b'S', "ERROR".into()),
                                            (b'C', "XX000".into()),
                                            (b'M', format!("{:?}", e)),
                                        ]
                                    }).await;
                                }
                            }
                            let _ = framed.send(BackendMessage::ReadyForQuery { status: b'I' }).await;
                        }
                        _ => {}
                    }
                }
            });
        }
    }

    /// 处理 Postgres 协议请求
    pub async fn handle_pg_query(&self, tenant_id: Uuid, sql: &str) -> Result<Vec<YYValue>> {
        // 1. Parse SQL to SqlStatement (which contains OpsGraph for queries)
        let stmt = SqlParser::parse_sql(sql)
            .map_err(|e| YYError::query(format!("Parse error: {:?}", e)))?;
        
        // 2. Execute based on statement type
        match stmt {
            SqlStatement::Query(graph) => {
                self.backend.execute_query(tenant_id, graph).await
                    .map_err(|e| YYError::internal(format!("Execution error: {:?}", e)))
            }
            SqlStatement::Insert { table, key, value } => {
                self.backend.execute_insert(tenant_id, table, key, value).await?;
                Ok(Vec::new())
            }
            SqlStatement::Delete { table, selection } => {
                self.backend.execute_delete(tenant_id, table, selection).await?;
                Ok(Vec::new())
            }
            _ => {
                Err(YYError::query("Unsupported statement type via gateway for now"))
            }
        }
    }

    /// 处理 MySQL 协议请求
    pub async fn handle_mysql_query(&self, tenant_id: Uuid, sql: &str) -> Result<Vec<YYValue>> {
        self.handle_pg_query(tenant_id, sql).await
    }

    /// 处理 RBQ 协议请求
    pub async fn handle_rbq_query(&self, tenant_id: Uuid, rbq: &str) -> Result<Vec<YYValue>> {
        // 1. Parse RBQ to RbqStatement
        let stmt = RbqParser::parse_rbq(rbq)
            .map_err(|e| YYError::query(format!("Parse error: {:?}", e)))?;
        
        // 2. Execute based on statement type
        match stmt {
            RbqStatement::Query(graph) => {
                self.backend.execute_query(tenant_id, graph).await
                    .map_err(|e| YYError::internal(format!("Execution error: {:?}", e)))
            }
        }
    }

    /// 处理 S3 PutObject 请求
    pub async fn handle_s3_put_object(&self, _bucket: &str, _key: &str, _data: bytes::Bytes) -> Result<PutObjectOutput> {
        // 将 S3 PutObject 转换为 Ops Graph
        let _graph = OpsGraph::new();
        // ... 映射逻辑 ...
        Ok(PutObjectOutput {
            e_tag: "placeholder".into(),
            version_id: None,
        })
    }

    /// 处理 Arrow Flight 流式读取
    pub async fn handle_arrow_flight(&self, _ticket: Vec<u8>) -> Result<()> {
        // 将 Arrow Flight 请求转换为 Ops Graph
        // 核心优势：利用 Arrow 零拷贝特性，直接返回算子执行结果
        let _graph = OpsGraph::new();
        // ... 映射逻辑 ...
        Ok(())
    }
}

pub mod pg {
    // 集成 we-trust-postgres
}

pub mod mysql {
    // 集成 we-trust-mysql
}

pub mod s3 {
    // 集成 we-trust-s3 并实现 S3Service 接口
}

pub mod arrow {
    // 集成 we-trust-arrow 实现 Arrow Flight 接口
}
