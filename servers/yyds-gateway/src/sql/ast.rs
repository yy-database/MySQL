use yykv_operators::OpsGraph;
use yykv_types::{DsValue, YYSchema as DsSchema};

// Re-defining SqlStatement here as an intermediate representation inside Gateway
// before converting to OpsGraph or executing directly (for DDL).
#[derive(Debug, Clone)]
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
        selection: Option<yykv_operators::Expr>,
    },
    Update {
        table: String,
        updates: Vec<(String, yykv_operators::Expr)>,
        selection: Option<yykv_operators::Expr>,
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
