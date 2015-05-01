use yyds_gateway::sql::{SqlParser, SqlStatement};

#[test]
fn sql_select_compiles_to_ops_graph() {
    let stmt = SqlParser::parse_sql("SELECT 1").unwrap();
    match stmt {
        SqlStatement::Query(graph) => {
            assert!(!graph.nodes.is_empty());
        }
        _ => panic!("expected query"),
    }
}

