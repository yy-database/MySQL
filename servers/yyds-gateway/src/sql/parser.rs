use crate::sql::ast::SqlStatement;
use oak_core::parser::{ParseSession, Parser as OakParser};
use oak_core::source::SourceText;
use oak_core::tree::{RedNode, RedTree};
use oak_sql::kind::SqlSyntaxKind;
use oak_sql::language::SqlLanguage;
use oak_sql::parser::SqlParser as OakSqlParser;
use oak_sql::SqlElementType;
use yykv_operators::{
    AggExpr, AggregateOp, BinaryOperator, Edge, Expr, FilterOp, JoinType, LimitOp, OperatorNode,
    OpsGraph, OrderBy, ProjectOp, SortOp, SourceOp, UnaryOperator, VectorSearchOp,
};
use uuid::Uuid;
use yykv_types::{DsResult, DsValue, DsSchema, FieldType, Vector};

pub struct SqlParser;

impl SqlParser {
    pub fn parse_sql(sql: &str) -> DsResult<SqlStatement> {
        let language = SqlLanguage::default();
        let parser = OakSqlParser::new(&language);
        let mut session = ParseSession::default();
        let source = SourceText::new(sql.to_string());
        let output = parser.parse(&source, &[], &mut session);
        let green = output.green();

        let root = RedNode::new(green, 0);
        let mut children = root.children();

        // Get the first non-ignored statement
        let statement_node = children
            .find(|c| {
                if let RedTree::Node(n) = c {
                    let kind: SqlElementType = n.kind();
                    matches!(
                        kind,
                        SqlElementType::SelectStatement
                            | SqlElementType::InsertStatement
                            | SqlElementType::UpdateStatement
                            | SqlElementType::DeleteStatement
                            | SqlElementType::CreateStatement
                            | SqlElementType::DropStatement
                            | SqlElementType::AlterStatement
                            | SqlElementType::ExplainStatement
                            | SqlElementType::TransactionStatement
                            | SqlElementType::PragmaStatement
                            | SqlElementType::ShowStatement
                    )
                } else {
                    false
                }
            })
            .and_then(|c| c.as_node())
            .ok_or(yykv_types::DsError::parse("Empty or invalid SQL"))?;

        let kind: SqlElementType = statement_node.kind();
        match kind {
            SqlElementType::SelectStatement => {
                Ok(SqlStatement::Query(Self::parse_query_node(statement_node, &source)?))
            }
            SqlElementType::ExplainStatement => {
                let mut query_node = None;
                for child in statement_node.children() {
                    if let RedTree::Node(n) = child {
                        if n.kind::<SqlElementType>() == SqlElementType::SelectStatement {
                            query_node = Some(n);
                            break;
                        }
                    }
                }
                if let Some(query_node) = query_node {
                    Ok(SqlStatement::Explain(Self::parse_query_node(query_node, &source)?))
                } else {
                    Err(yykv_types::DsError::parse("EXPLAIN must be followed by a SELECT statement"))
                }
            }
            SqlElementType::InsertStatement => {
                Self::parse_insert_node(statement_node, &source)
            }
            SqlElementType::DeleteStatement => {
                Self::parse_delete_node(statement_node, &source)
            }
            SqlElementType::UpdateStatement => {
                Self::parse_update_node(statement_node, &source)
            }
            SqlElementType::CreateStatement => {
                Self::parse_create_node(statement_node, &source)
            }
            SqlElementType::DropStatement => {
                Self::parse_drop_node(statement_node, &source)
            }
            _ => Err(yykv_types::DsError::parse(format!("Unsupported statement type: {:?}", kind))),
        }
    }

    fn parse_query_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<OpsGraph> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let mut table_name = "__dual__".to_string();
        let mut filter_expr = None;
        let mut projections = Vec::new();
        let mut order_bys = Vec::new();
        let mut limit_val = None;
        let mut offset_val = None;
        let mut vector_search_op = None;
        let mut aggs = Vec::new();
        let mut group_by = Vec::new();
        let mut having = None;
        let mut joins = Vec::new();
        let mut is_aggregate = false;

        let mut current_keyword = None;
        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    let kind: SqlElementType = n.kind();
                    match kind {
                        SqlElementType::TableName | SqlElementType::Identifier => table_name = n.text(source).to_string(),
                        SqlElementType::JoinClause => {
                            let mut join_type = JoinType::Inner;
                            let mut joined_table = String::new();
                            let mut left_on = String::new();
                            let mut right_on = String::new();
                            
                            for c in n.children() {
                                if let RedTree::Node(child_node) = c {
                                    if child_node.kind::<SqlElementType>() == SqlElementType::Expression {
                                        let expr = Self::translate_expr_node(child_node, source)?;
                                        if let Expr::BinaryOp(left, op, right) = expr {
                                            if op == BinaryOperator::Eq {
                                                if let Expr::Column(l) = *left { left_on = l; }
                                                if let Expr::Column(r) = *right { right_on = r; }
                                            }
                                        }
                                    }
                                } else if let RedTree::Leaf(t) = c {
                                    match t.kind() {
                                        SqlSyntaxKind::Left => join_type = JoinType::Left,
                                        SqlSyntaxKind::Right => join_type = JoinType::Right,
                                        SqlSyntaxKind::Full => join_type = JoinType::Full,
                                        SqlSyntaxKind::Inner => join_type = JoinType::Inner,
                                        SqlSyntaxKind::Identifier_ | SqlSyntaxKind::Identifier => joined_table = t.text(source).to_string(),
                                        _ => {}
                                    }
                                }
                            }
                            joins.push((joined_table, left_on, right_on, join_type));
                        }
                        SqlElementType::GroupByClause => {
                            for c in n.children() {
                                if let RedTree::Node(child_node) = c {
                                    if child_node.kind::<SqlElementType>() == SqlElementType::Expression {
                                        group_by.push(Self::translate_expr_node(child_node, source)?);
                                        is_aggregate = true;
                                    }
                                }
                            }
                        }
                        SqlElementType::HavingClause => {
                            for c in n.children() {
                                if let RedTree::Node(child_node) = c {
                                    if child_node.kind::<SqlElementType>() == SqlElementType::Expression {
                                        having = Some(Self::translate_expr_node(child_node, source)?);
                                        is_aggregate = true;
                                    }
                                }
                            }
                        }
                        SqlElementType::OrderByClause => {
                            for c in n.children() {
                                if let RedTree::Node(child_node) = c {
                                    if child_node.kind::<SqlElementType>() == SqlElementType::Expression {
                                        order_bys.push(OrderBy { 
                                            expr: Self::translate_expr_node(child_node, source)?,
                                            desc: false 
                                        });
                                    }
                                } else if let RedTree::Leaf(t) = c {
                                    if t.kind() == SqlSyntaxKind::Desc {
                                        if let Some(ob) = order_bys.last_mut() { ob.desc = true; }
                                    }
                                }
                            }
                        }
                        SqlElementType::LimitClause => {
                            for c in n.children() {
                                if let RedTree::Leaf(t) = c {
                                    if t.kind() == SqlSyntaxKind::NumberLiteral {
                                        let val = t.text(source).parse::<usize>().unwrap_or(0);
                                        if limit_val.is_none() { limit_val = Some(val); }
                                        else { offset_val = Some(val); }
                                    }
                                }
                            }
                        }
                        SqlElementType::Expression => {
                            let expr = Self::translate_expr_node(n.clone(), source)?;
                            match current_keyword {
                                Some(SqlSyntaxKind::Select) => {
                                    // Handle Vector Search
                                    let mut is_vector_search = false;
                                    if let Expr::FunctionCall(name, args) = &expr {
                                        if name.to_uppercase() == "VECTOR_SEARCH" && args.len() >= 3 {
                                            let column = if let Expr::Column(c) = &args[0] { c.clone() } else { args[0].to_string() };
                                            let vector_val = if let Expr::Literal(DsValue::Vector(v)) = &args[1] { v.0.clone() } else { vec![] };
                                            let limit = if let Expr::Literal(DsValue::Int(i)) = &args[2] { *i as usize } else { 10 };
                                            vector_search_op = Some(VectorSearchOp { column, vector: vector_val, limit });
                                            is_vector_search = true;
                                        }
                                    }

                                    if !is_vector_search {
                                        let alias = n.text(source).to_string();
                                        
                                        // Handle Aggregates
                                        if let Some(agg_expr) = Self::extract_agg_expr(&expr) {
                                            aggs.push(agg_expr);
                                            is_aggregate = true;
                                            // The projection will be handled later
                                            projections.push((alias, expr));
                                        } else {
                                            projections.push((alias, expr));
                                        }
                                    }
                                }
                                Some(SqlSyntaxKind::Where) => { filter_expr = Some(expr); }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                RedTree::Leaf(t) => {
                    let kind = t.kind();
                    match kind {
                        SqlSyntaxKind::Select | SqlSyntaxKind::From | SqlSyntaxKind::Where => {
                            current_keyword = Some(kind);
                        }
                        _ => {}
                    }
                }
            }
        }

        // 1. Source (FROM)
        let source_node_idx = nodes.len();
        nodes.push(OperatorNode::Source(SourceOp { table: table_name, columns: vec![] }));
        let mut last_node_idx = source_node_idx;

        // 2. Joins
        for (joined_table, left_on, right_on, join_type) in joins {
            let right_source_idx = nodes.len();
            nodes.push(OperatorNode::Source(SourceOp { table: joined_table, columns: vec![] }));
            
            let join_node_idx = nodes.len();
            nodes.push(OperatorNode::Join(yykv_operators::JoinOp { left_on, right_on, join_type }));
            
            edges.push(Edge { from: last_node_idx, to: join_node_idx });
            edges.push(Edge { from: right_source_idx, to: join_node_idx });
            last_node_idx = join_node_idx;
        }

        // 3. Vector Search
        if let Some(op) = vector_search_op {
            let vs_node_idx = nodes.len();
            nodes.push(OperatorNode::VectorSearch(op));
            edges.push(Edge { from: last_node_idx, to: vs_node_idx });
            last_node_idx = vs_node_idx;
        }

        // 4. Filter (WHERE)
        if let Some(expr) = filter_expr {
            let filter_node_idx = nodes.len();
            nodes.push(OperatorNode::Filter(FilterOp { expression: expr }));
            edges.push(Edge { from: last_node_idx, to: filter_node_idx });
            last_node_idx = filter_node_idx;
        }

        // 5. Aggregate (GROUP BY)
        if is_aggregate {
            let agg_node_idx = nodes.len();
            nodes.push(OperatorNode::Aggregate(AggregateOp { group_by, aggs }));
            edges.push(Edge { from: last_node_idx, to: agg_node_idx });
            last_node_idx = agg_node_idx;

            if let Some(having_expr) = having {
                let having_node_idx = nodes.len();
                nodes.push(OperatorNode::Filter(FilterOp { expression: having_expr }));
                edges.push(Edge { from: last_node_idx, to: having_node_idx });
                last_node_idx = having_node_idx;
            }
        }

        // 6. Project (SELECT)
        if !projections.is_empty() {
            let mut final_projections = Vec::new();
            let mut agg_idx = 0;
            for (alias, expr) in projections {
                if Self::extract_agg_expr(&expr).is_some() {
                    let agg_name = match agg_idx {
                        0 => "count",
                        1 => "sum",
                        2 => "avg",
                        3 => "max",
                        4 => "min",
                        _ => "agg",
                    };
                    final_projections.push((alias, Expr::Column(agg_name.into())));
                    agg_idx += 1;
                } else {
                    final_projections.push((alias, expr));
                }
            }
            let project_node_idx = nodes.len();
            nodes.push(OperatorNode::Project(ProjectOp { projections: final_projections }));
            edges.push(Edge { from: last_node_idx, to: project_node_idx });
            last_node_idx = project_node_idx;
        }

        // 7. Sort (ORDER BY)
        if !order_bys.is_empty() {
            let sort_node_idx = nodes.len();
            nodes.push(OperatorNode::Sort(SortOp { order_by: order_bys }));
            edges.push(Edge { from: last_node_idx, to: sort_node_idx });
            last_node_idx = sort_node_idx;
        }

        // 8. Limit (LIMIT)
        if limit_val.is_some() || offset_val.is_some() {
            let limit_node_idx = nodes.len();
            nodes.push(OperatorNode::Limit(LimitOp {
                limit: limit_val.unwrap_or(100),
                offset: offset_val.unwrap_or(0),
            }));
            edges.push(Edge { from: last_node_idx, to: limit_node_idx });
        }

        Ok(OpsGraph { id: Uuid::new_v4(), nodes, edges })
    }

    fn extract_agg_expr(expr: &Expr) -> Option<AggExpr> {
        if let Expr::FunctionCall(name, args) = expr {
            if args.len() == 1 {
                let arg = args[0].clone();
                match name.to_uppercase().as_str() {
                    "COUNT" => return Some(AggExpr::Count(arg)),
                    "SUM" => return Some(AggExpr::Sum(arg)),
                    "AVG" => return Some(AggExpr::Avg(arg)),
                    "MAX" => return Some(AggExpr::Max(arg)),
                    "MIN" => return Some(AggExpr::Min(arg)),
                    _ => {}
                }
            }
        }
        None
    }

    fn parse_insert_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<SqlStatement> {
        let mut table = String::new();
        let mut values = Vec::new();
        let mut current_keyword = None;

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    match n.kind::<SqlElementType>() {
                        SqlElementType::TableName | SqlElementType::Identifier => table = n.text(source).to_string(),
                        SqlElementType::Expression => {
                            if current_keyword == Some(SqlSyntaxKind::Values) {
                                let expr = Self::translate_expr_node(n, source)?;
                                if let Expr::Literal(v) = expr {
                                    values.push(v);
                                }
                            }
                        }
                        SqlElementType::ValueList => {
                            if current_keyword == Some(SqlSyntaxKind::Values) {
                                Self::collect_insert_values(n, source, &mut values)?;
                            }
                        }
                        _ => {}
                    }
                }
                RedTree::Leaf(t) => {
                    match t.kind() {
                        SqlSyntaxKind::Values | SqlSyntaxKind::Into => current_keyword = Some(t.kind()),
                        _ => {}
                    }
                }
            }
        }

        if values.is_empty() {
            return Err(yykv_types::DsError::execution("INSERT must have at least one value"));
        }

        let key = values.get(0).cloned().unwrap();
        let value = if values.len() == 1 { key.clone() } else { DsValue::List(values) };
        Ok(SqlStatement::Insert { table, key, value })
    }

    fn collect_insert_values(
        node: RedNode<SqlLanguage>,
        source: &SourceText,
        out: &mut Vec<DsValue>,
    ) -> DsResult<()> {
        for child in node.children() {
            if let RedTree::Node(n) = child {
                if n.kind::<SqlElementType>() == SqlElementType::Expression {
                    let expr = Self::translate_expr_node(n, source)?;
                    if let Expr::Literal(v) = expr {
                        out.push(v);
                    }
                } else {
                    Self::collect_insert_values(n, source, out)?;
                }
            }
        }
        Ok(())
    }

    fn parse_delete_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<SqlStatement> {
        let mut table = String::new();
        let mut selection = None;
        let mut current_keyword = None;

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    match n.kind::<SqlElementType>() {
                        SqlElementType::TableName | SqlElementType::Identifier => table = n.text(source).to_string(),
                        SqlElementType::Expression => {
                            if current_keyword == Some(SqlSyntaxKind::Where) {
                                selection = Some(Self::translate_expr_node(n, source)?);
                            }
                        }
                        _ => {}
                    }
                }
                RedTree::Leaf(t) => {
                    match t.kind() {
                        SqlSyntaxKind::Where | SqlSyntaxKind::From => current_keyword = Some(t.kind()),
                        _ => {}
                    }
                }
            }
        }
        Ok(SqlStatement::Delete { table, selection })
    }

    fn parse_update_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<SqlStatement> {
        let mut table = String::new();
        let mut updates = Vec::new();
        let mut selection = None;
        let mut current_keyword = None;
        let mut last_column = String::new();

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    match n.kind::<SqlElementType>() {
                        SqlElementType::TableName | SqlElementType::Identifier => {
                            if current_keyword == Some(SqlSyntaxKind::Update) {
                                table = n.text(source).to_string();
                            } else if current_keyword == Some(SqlSyntaxKind::Set) {
                                last_column = n.text(source).to_string();
                            }
                        }
                        SqlElementType::ColumnName => last_column = n.text(source).to_string(),
                        SqlElementType::Expression => {
                            let expr = Self::translate_expr_node(n, source)?;
                            if current_keyword == Some(SqlSyntaxKind::Set) {
                                updates.push((last_column.clone(), expr));
                            } else if current_keyword == Some(SqlSyntaxKind::Where) {
                                selection = Some(expr);
                            }
                        }
                        _ => {}
                    }
                }
                RedTree::Leaf(t) => {
                    match t.kind() {
                        SqlSyntaxKind::Set | SqlSyntaxKind::Where | SqlSyntaxKind::Update => current_keyword = Some(t.kind()),
                        _ => {}
                    }
                }
            }
        }
        Ok(SqlStatement::Update { table, updates, selection })
    }

    fn parse_create_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<SqlStatement> {
        let mut table = String::new();
        let mut is_view = false;
        let mut columns: Vec<(String, FieldType)> = Vec::new();
        let mut current_keyword = None;

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    match n.kind::<SqlElementType>() {
                        SqlElementType::TableName => table = n.text(source).to_string(),
                        SqlElementType::ColumnDefinition => {
                            let mut col_name = String::new();
                            let mut col_type = FieldType::Text;
                            for c in n.children() {
                                if let RedTree::Node(cn) = c {
                                    if cn.kind::<SqlElementType>() == SqlElementType::ColumnName {
                                        col_name = cn.text(source).to_string();
                                    }
                                } else if let RedTree::Leaf(ct) = c {
                                    match ct.text(source).to_uppercase().as_str() {
                                        "INT" | "INTEGER" => col_type = FieldType::Int,
                                        "FLOAT" | "DOUBLE" => col_type = FieldType::Float,
                                        "BOOLEAN" | "BOOL" => col_type = FieldType::Bool,
                                        "VECTOR" => col_type = FieldType::Vector,
                                        _ => {}
                                    }
                                }
                            }
                            columns.push((col_name, col_type));
                        }
                        SqlElementType::SelectStatement => {
                            if is_view {
                                return Ok(SqlStatement::CreateView { name: table, query: Self::parse_query_node(n, source)? });
                            }
                        }
                        SqlElementType::Identifier => {
                            if current_keyword == Some(SqlSyntaxKind::Index) {
                                return Self::parse_create_index_node(node, source);
                            }
                        }
                        _ => {}
                    }
                }
                RedTree::Leaf(t) => {
                    match t.kind() {
                        SqlSyntaxKind::View => is_view = true,
                        SqlSyntaxKind::Table => is_view = false,
                        SqlSyntaxKind::Index => {
                            return Self::parse_create_index_node(node, source);
                        }
                        _ => {}
                    }
                    current_keyword = Some(t.kind());
                }
            }
        }

        if !columns.is_empty() {
            let mut schema = DsSchema::new(0, &table);
            for (tag, (name, field_type)) in columns.into_iter().enumerate() {
                schema.add_field(tag as u32, &name, field_type, true);
            }
            Ok(SqlStatement::CreateTable { name: table, schema: Some(schema) })
        } else {
            Ok(SqlStatement::CreateTable { name: table, schema: None })
        }
    }

    fn parse_create_index_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<SqlStatement> {
        let mut index_name = String::new();
        let mut table_name = String::new();
        let mut columns = Vec::new();
        let mut unique = false;

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    match n.kind::<SqlElementType>() {
                        SqlElementType::TableName => table_name = n.text(source).to_string(),
                        SqlElementType::Identifier => {
                            if index_name.is_empty() {
                                index_name = n.text(source).to_string();
                            } else {
                                columns.push(n.text(source).to_string());
                            }
                        }
                        _ => {}
                    }
                }
                RedTree::Leaf(t) => {
                    if t.kind() == SqlSyntaxKind::Unique {
                        unique = true;
                    }
                }
            }
        }
        Ok(SqlStatement::CreateIndex { name: index_name, table: table_name, columns, unique })
    }

    fn parse_drop_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<SqlStatement> {
        let mut table = String::new();
        let mut object_type = "TABLE".to_string();

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    if n.kind::<SqlElementType>() == SqlElementType::TableName { table = n.text(source).to_string(); }
                }
                RedTree::Leaf(t) => {
                    if t.kind() == SqlSyntaxKind::View { object_type = "VIEW".to_string(); }
                }
            }
        }
        Ok(SqlStatement::Drop { object_type, name: table, if_exists: false })
    }

    fn translate_expr_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<Expr> {
        if node.kind::<SqlElementType>() == SqlElementType::Expression {
            let parts: Vec<_> = node.children().collect();
            if parts.len() >= 2 {
                if let Some(t) = parts[0].as_token() {
                    if t.kind() == SqlSyntaxKind::LeftBracket {
                        let mut vec_data = Vec::new();
                        for part in parts {
                            if let RedTree::Node(n) = part {
                                if let Ok(expr) = Self::translate_expr_node(n, source) {
                                    match expr {
                                        Expr::Literal(DsValue::Int(i)) => vec_data.push(i as f32),
                                        Expr::Literal(DsValue::Float(f)) => vec_data.push(f as f32),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        return Ok(Expr::Literal(DsValue::Vector(Vector(vec_data))));
                    }
                }
            }
        }

        let parts: Vec<_> = node.children().collect();

        if parts.len() == 1 {
            let child = &parts[0];
            match child {
                RedTree::Node(n) => return Self::translate_expr_node(n.clone(), source),
                RedTree::Leaf(t) => {
                    let kind = t.kind();
                    let text = t.text(source);
                    match kind {
                        SqlSyntaxKind::NumberLiteral => {
                            let val = text.parse::<i64>().unwrap_or(0);
                            return Ok(Expr::Literal(DsValue::Int(val)));
                        }
                        SqlSyntaxKind::FloatLiteral => {
                            let val = text.parse::<f64>().unwrap_or(0.0);
                            return Ok(Expr::Literal(DsValue::Float(val)));
                        }
                        SqlSyntaxKind::StringLiteral => {
                            let s = text.trim_matches('\'').to_string();
                            if s.starts_with('[') && s.ends_with(']') {
                                let vec_data = s.trim_matches(|c| c == '[' || c == ']')
                                    .split(',')
                                    .filter_map(|v| v.trim().parse::<f32>().ok())
                                    .collect::<Vec<f32>>();
                                if !vec_data.is_empty() { return Ok(Expr::Literal(DsValue::Vector(Vector(vec_data)))); }
                            }
                            return Ok(Expr::Literal(DsValue::Text(s)));
                        }
                        SqlSyntaxKind::BooleanLiteral | SqlSyntaxKind::True | SqlSyntaxKind::False => {
                            return Ok(Expr::Literal(DsValue::Bool(text.to_uppercase() == "TRUE")));
                        }
                        SqlSyntaxKind::NullLiteral | SqlSyntaxKind::Null => return Ok(Expr::Literal(DsValue::Null)),
                        SqlSyntaxKind::Identifier_ | SqlSyntaxKind::Identifier | SqlSyntaxKind::ColumnName => return Ok(Expr::Column(text.to_string())),
                        _ => return Ok(Expr::Column(text.to_string())),
                    }
                }
            }
        }

        if parts.len() >= 3 {
            if let Some(name_token) = parts[0].as_token() {
                if let (Some(lparen), Some(rparen)) = (parts[1].as_token(), parts.last().and_then(|p| p.as_token())) {
                    if lparen.kind() == SqlSyntaxKind::LeftParen && rparen.kind() == SqlSyntaxKind::RightParen {
                        let name = name_token.text(source).to_uppercase();
                        let mut args = Vec::new();
                        for i in 2..parts.len() - 1 {
                            if let RedTree::Node(n) = &parts[i] {
                                if n.kind::<SqlElementType>() == SqlElementType::Expression { args.push(Self::translate_expr_node(n.clone(), source)?); }
                            }
                        }
                        return Ok(Expr::FunctionCall(name, args));
                    }
                }
            }
        }

        if parts.len() == 2 {
            if let Some(t) = parts[0].as_token() {
                if let Some(Ok(expr)) = parts[1].as_node().map(|n| Self::translate_expr_node(n, source)) {
                    let op = match t.kind() {
                        SqlSyntaxKind::Not => Some(UnaryOperator::Not),
                        SqlSyntaxKind::Minus => Some(UnaryOperator::Neg),
                        _ => None,
                    };
                    if let Some(op) = op {
                        return Ok(Expr::UnaryOp(Box::new(op), Box::new(expr)));
                    }
                }
            }
        }

        if parts.len() >= 3 {
            let left = parts[0].as_node().map(|n| Self::translate_expr_node(n, source));
            let op_token = parts[1].as_token();
            let right = parts[2].as_node().map(|n| Self::translate_expr_node(n, source));

            if let (Some(Ok(l)), Some(t), Some(Ok(r))) = (left, op_token, right) {
                let op = match t.kind() {
                    SqlSyntaxKind::Equal => Some(BinaryOperator::Eq),
                    SqlSyntaxKind::NotEqual => Some(BinaryOperator::Neq),
                    SqlSyntaxKind::Greater => Some(BinaryOperator::Gt),
                    SqlSyntaxKind::GreaterEqual => Some(BinaryOperator::Gte),
                    SqlSyntaxKind::Less => Some(BinaryOperator::Lt),
                    SqlSyntaxKind::LessEqual => Some(BinaryOperator::Lte),
                    SqlSyntaxKind::Plus => Some(BinaryOperator::Add),
                    SqlSyntaxKind::Minus => Some(BinaryOperator::Sub),
                    SqlSyntaxKind::Star => Some(BinaryOperator::Mul),
                    SqlSyntaxKind::Slash => Some(BinaryOperator::Div),
                    SqlSyntaxKind::And => Some(BinaryOperator::And),
                    SqlSyntaxKind::Or => Some(BinaryOperator::Or),
                    SqlSyntaxKind::Concat => Some(BinaryOperator::Concat),
                    _ => None,
                };
                if let Some(op) = op { return Ok(Expr::BinaryOp(Box::new(l), op, Box::new(r))); }
            }
        }

        if parts.len() == 3 {
            if let (Some(l), Some(r)) = (parts[0].as_token(), parts[2].as_token()) {
                if l.kind() == SqlSyntaxKind::LeftParen && r.kind() == SqlSyntaxKind::RightParen {
                    if let RedTree::Node(n) = &parts[1] { return Self::translate_expr_node(n.clone(), source); }
                }
            }
        }

        Ok(Expr::Column(node.text(source).to_string()))
    }
}
