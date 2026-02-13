use yykv_operators::OpsGraph;

#[derive(Debug, Clone)]
pub enum RbqStatement {
    Query(OpsGraph),
}
