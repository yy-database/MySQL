use crate::OpsGraph;
use uuid::Uuid;
use yykv_types::{DsResult, DsValue, YYSchema as DsSchema};

pub enum SqlStatement {
    Query(OpsGraph),
    Explain(OpsGraph),
    Insert {
        table: String,
        key: DsValue,
        value: DsValue,
    },
    CreateTable {
        name: String,
        schema: Option<DsSchema>,
    },
    CreateIndex {
        name: String,
        table: String,
        columns: Vec<String>,
        unique: bool,
    },
    Delete {
        table: String,
        selection: Option<crate::Expr>,
    },
    Update {
        table: String,
        assignments: Vec<(String, crate::Expr)>,
        selection: Option<crate::Expr>,
    },
    CreateView {
        name: String,
        query: OpsGraph,
    },
    Drop {
        object_type: String,
        name: String,
        if_exists: bool,
    },
}

pub struct SqlParser;

impl SqlParser {
    pub fn parse_sql(_sql: &str) -> DsResult<SqlStatement> {
        Ok(SqlStatement::Query(OpsGraph {
            id: Uuid::new_v4(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }))
    }
}
