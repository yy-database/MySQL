use crate::rbq::ast::RbqStatement;
use async_trait::async_trait;
use uuid::Uuid;
use yykv::{Client as YykvClient};
use yykv_types::{DsError, DsResult, DsValue};

/// RBQ 存储引擎集成 trait
#[async_trait]
pub trait RbqStorage {
    /// 执行 RBQ 语句，与存储引擎交互
    async fn execute_rbq(&mut self, statement: RbqStatement, tenant_id: Uuid) -> DsResult<Vec<DsValue>>;
}

/// 基于 YYKV 客户端的 RBQ 存储引擎实现
pub struct YykvRbqStorage {
    client: YykvClient,
}

impl YykvRbqStorage {
    /// 创建一个新的 RBQ 存储引擎实例
    pub async fn new(addr: &str, tenant_id: Uuid, secret_key: &[u8]) -> DsResult<Self> {
        let client = YykvClient::connect(addr, tenant_id, secret_key).await?;
        Ok(Self {
            client,
        })
    }
}

#[async_trait]
impl RbqStorage for YykvRbqStorage {
    async fn execute_rbq(&mut self, statement: RbqStatement, _tenant_id: Uuid) -> DsResult<Vec<DsValue>> {
        match statement {
            RbqStatement::Query(graph) => {
                // 使用 YYKV 客户端执行 OpsGraph
                self.client.query_graph(graph).await
            }
        }
    }
}
