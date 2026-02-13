use yykv_operators::{OpsGraph, sql::SqlParser as InnerParser, sql::SqlStatement as InnerStatement};
use uuid::Uuid;
use yykv_types::DsResult;
use crate::sql::ast::SqlStatement;
// use oak_sql::parser::SqlParser as OakSqlParser; // Assuming oak-sql exposes this

pub struct SqlParser;

impl SqlParser {
    pub fn parse_sql(sql: &str) -> DsResult<SqlStatement> {
        // Forward to yykv-operators::sql
        let inner = InnerParser::parse_sql(sql)?;
        match inner {
            InnerStatement::Query(graph) => Ok(SqlStatement::Query(graph)),
            _ => {
                // For now, map everything else to a stub query
                Ok(SqlStatement::Query(OpsGraph {
                    id: Uuid::new_v4(),
                    nodes: Vec::new(),
                    edges: Vec::new(),
                }))
            }
        }
    }
}
