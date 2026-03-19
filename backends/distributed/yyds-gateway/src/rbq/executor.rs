use crate::rbq::ast::RbqStatement;
use async_trait::async_trait;
use uuid::Uuid;
use yykv_executor::{DistributedExecutor, YykvExecutor};
use yykv_types::{DsError, DsResult, DsValue};

/// RBQ 执行器 trait
#[async_trait]
pub trait RbqExecutor {
    /// 执行 RBQ 语句
    async fn execute(&self, statement: RbqStatement, tenant_id: Uuid) -> DsResult<Vec<DsValue>>;
}

/// 基于 YYKV 执行器的 RBQ 执行器实现
pub struct YykvRbqExecutor {
    executor: YykvExecutor,
}

impl YykvRbqExecutor {
    /// 创建一个新的 RBQ 执行器
    pub fn new(executor: YykvExecutor) -> Self {
        Self {
            executor,
        }
    }
}

#[async_trait]
impl RbqExecutor for YykvRbqExecutor {
    async fn execute(&self, statement: RbqStatement, tenant_id: Uuid) -> DsResult<Vec<DsValue>> {
        match statement {
            RbqStatement::Query(graph) => {
                // 使用 YYKV 执行器执行 OpsGraph
                self.executor.execute_local(graph, tenant_id).await
            }
        }
    }
}
