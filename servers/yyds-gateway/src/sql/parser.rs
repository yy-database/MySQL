use yykv_operators::{sql::SqlParser as InnerParser, sql::SqlStatement as InnerStatement};
use yykv_types::DsResult;
use crate::sql::ast::SqlStatement;
use oak_sql::{SqlParser as OakSqlParser, SqlLanguage};
use oak_core::parser::{Parser, ParseSession};

pub struct SqlParser;

impl SqlParser {
    pub fn parse_sql(sql: &str) -> DsResult<SqlStatement> {
        // Use oak-sql to parse the SQL string
        let language = SqlLanguage::new();
        let parser = OakSqlParser::new(&language);
        let mut session = ParseSession::<SqlLanguage>::new(1024);
        let _oak_ast = parser.parse(sql, &[], &mut session).result;
        
        // Forward to yykv-operators::sql for now until mapping is complete
        let inner = InnerParser::parse_sql(sql)?;
        match inner {
            InnerStatement::Query(graph) => Ok(SqlStatement::Query(graph)),
            InnerStatement::Insert { table, key, value } => Ok(SqlStatement::Insert { table, key, value }),
            InnerStatement::Update { table, updates, selection } => Ok(SqlStatement::Update { table, updates, selection }),
            InnerStatement::Delete { table, selection } => Ok(SqlStatement::Delete { table, selection }),
            InnerStatement::CreateTable { name, schema } => Ok(SqlStatement::CreateTable { name, schema }),
        }
    }
}
