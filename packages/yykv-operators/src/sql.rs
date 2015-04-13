use crate::OpsGraph;
use uuid::Uuid;
use yykv_types::{DsResult, DsValue};

pub enum SqlStatement {
    Query(OpsGraph),
    Insert {
        table: String,
        key: DsValue,
        value: DsValue,
    },
    Update {
        table: String,
        updates: Vec<(String, crate::Expr)>,
        selection: Option<crate::Expr>,
    },
    Delete {
        table: String,
        selection: Option<crate::Expr>,
    },
    CreateTable {
        name: String,
        schema: Option<yykv_types::DsSchema>,
    },
}

pub struct SqlParser;

impl SqlParser {
    pub fn parse_sql(sql: &str) -> DsResult<SqlStatement> {
        // TODO: 使用 oak-sql 实现真正的解析
        // 目前返回一个空的 OpsGraph 以保证编译通过
        Ok(SqlStatement::Query(OpsGraph {
            id: Uuid::new_v4(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }))
    }
}
