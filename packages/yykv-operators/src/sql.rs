use crate::{
    AggExpr, AggregateOp, BinaryOperator, Edge, Expr, FilterOp, JoinOp, JoinType, LimitOp, OperatorNode,
    OpsGraph, OrderBy, ProjectOp, SortOp, SourceOp, VectorSearchOp,
};
use oak_core::parser::{ParseSession, Parser as OakParser};
use oak_core::source::SourceText;
use oak_core::tree::{RedNode, RedTree};
use oak_sql::kind::SqlSyntaxKind;
use oak_sql::language::SqlLanguage;
use oak_sql::parser::SqlParser as OakSqlParser;
use uuid::Uuid;
use yykv_types::{DsResult, DsValue, DsSchema, Field, FieldType};

/// SQL 语句类型
#[derive(Debug, Clone)]
pub enum SqlStatement {
    /// 查询语句
    Query(OpsGraph),
    /// 解释执行计划
    Explain(OpsGraph),
    /// 插入数据
    Insert {
        /// 表名
        table: String,
        /// 键
        key: DsValue,
        /// 值
        value: DsValue,
    },
    /// 更新数据
    Update {
        /// 表名
        table: String,
        /// 更新项
        assignments: Vec<(String, Expr)>,
        /// 选择条件
        selection: Option<Expr>,
    },
    /// 删除数据
    Delete {
        /// 表名
        table: String,
        /// 选择条件
        selection: Option<Expr>,
    },
    /// 创建表
    CreateTable {
        /// 表名
        name: String,
        /// Schema
        schema: Option<DsSchema>,
    },
    /// 创建索引
    CreateIndex {
        /// 索引名
        name: String,
        /// 表名
        table: String,
        /// 列名列表
        columns: Vec<String>,
        /// 是否唯一
        unique: bool,
    },
    /// 创建视图
    CreateView {
        /// 视图名
        name: String,
        /// 查询算子图
        query: OpsGraph,
    },
    /// 删除对象
    Drop {
        /// 对象类型 (TABLE, INDEX, VIEW 等)
        object_type: String,
        /// 对象名称
        name: String,
        /// 是否存在才删除
        if_exists: bool,
    },
}

/// SQL 解析器
pub struct SqlParser;

impl SqlParser {
    /// 解析 SQL 字符串为 SqlStatement
    pub fn parse_sql(sql: &str) -> DsResult<SqlStatement> {
        let language = SqlLanguage::default();
        let parser = OakSqlParser::new(&language);
        let mut session = ParseSession::default();
        let source = SourceText::new(sql.to_string());
        let output = parser.parse(&source, &[], &mut session);
        let green = output.green();

        let root = RedNode::new(green, 0);
        let mut children = root.children();

        // 获取第一个非忽略的语句
        let statement_node = children
            .find(|c| {
                if let RedTree::Node(n) = c {
                    let kind: SqlSyntaxKind = n.kind();
                    kind == SqlSyntaxKind::Statement
                        || kind == SqlSyntaxKind::SelectStatement
                        || kind == SqlSyntaxKind::InsertStatement
                        || kind == SqlSyntaxKind::UpdateStatement
                        || kind == SqlSyntaxKind::DeleteStatement
                        || kind == SqlSyntaxKind::CreateStatement
                        || kind == SqlSyntaxKind::DropStatement
                        || kind == SqlSyntaxKind::ExplainStatement
                } else {
                    false
                }
            })
            .and_then(|c| c.as_node())
            .ok_or_else(|| yykv_types::DsError::parse("Empty or invalid SQL"))?;

        let kind: SqlSyntaxKind = statement_node.kind();
        match kind {
            SqlSyntaxKind::SelectStatement => {
                Ok(SqlStatement::Query(Self::parse_query_node(statement_node, &source)?))
            }
            SqlSyntaxKind::ExplainStatement => {
                let mut query_node = None;
                for child in statement_node.children() {
                    if let RedTree::Node(n) = child {
                        if n.kind() == SqlSyntaxKind::SelectStatement {
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
            SqlSyntaxKind::InsertStatement => {
                Self::parse_insert_node(statement_node, &source)
            }
            SqlSyntaxKind::DeleteStatement => {
                Self::parse_delete_node(statement_node, &source)
            }
            SqlSyntaxKind::UpdateStatement => {
                Self::parse_update_node(statement_node, &source)
            }
            SqlSyntaxKind::CreateStatement => {
                Self::parse_create_node(statement_node, &source)
            }
            SqlSyntaxKind::DropStatement => {
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
                    let kind: SqlSyntaxKind = n.kind();
                    match kind {
                        SqlSyntaxKind::TableName | SqlSyntaxKind::Identifier => table_name = n.text(source).to_string(),
                        SqlSyntaxKind::JoinClause => {
                            let mut join_type = JoinType::Inner;
                            let mut joined_table = String::new();
                            let mut left_on = String::new();
                            let mut right_on = String::new();
                            
                            for c in n.children() {
                                if let RedTree::Node(child_node) = c {
                                    if child_node.kind() == SqlSyntaxKind::Expression {
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
                        SqlSyntaxKind::GroupByClause => {
                            for c in n.children() {
                                if let RedTree::Node(child_node) = c {
                                    if child_node.kind() == SqlSyntaxKind::Expression {
                                        group_by.push(Self::translate_expr_node(child_node, source)?);
                                        is_aggregate = true;
                                    }
                                }
                            }
                        }
                        SqlSyntaxKind::HavingClause => {
                            for c in n.children() {
                                if let RedTree::Node(child_node) = c {
                                    if child_node.kind() == SqlSyntaxKind::Expression {
                                        having = Some(Self::translate_expr_node(child_node, source)?);
                                        is_aggregate = true;
                                    }
                                }
                            }
                        }
                        SqlSyntaxKind::OrderByClause => {
                            for c in n.children() {
                                if let RedTree::Node(child_node) = c {
                                    if child_node.kind() == SqlSyntaxKind::Expression {
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
                        SqlSyntaxKind::LimitClause => {
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
                        SqlSyntaxKind::Expression => {
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
            nodes.push(OperatorNode::Join(JoinOp { left_on, right_on, join_type }));
            
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
                    match n.kind() {
                        SqlSyntaxKind::TableName | SqlSyntaxKind::Identifier => table = n.text(source).to_string(),
                        SqlSyntaxKind::Expression => {
                            if current_keyword == Some(SqlSyntaxKind::Values) {
                                let expr = Self::translate_expr_node(n, source)?;
                                if let Expr::Literal(v) = expr {
                                    values.push(v);
                                }
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

    fn parse_delete_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<SqlStatement> {
        let mut table = String::new();
        let mut selection = None;
        let mut current_keyword = None;

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    match n.kind() {
                        SqlSyntaxKind::TableName | SqlSyntaxKind::Identifier => table = n.text(source).to_string(),
                        SqlSyntaxKind::Expression => {
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
        let mut assignments = Vec::new();
        let mut selection = None;
        let mut current_keyword = None;
        let mut last_column = String::new();

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    match n.kind() {
                        SqlSyntaxKind::TableName | SqlSyntaxKind::Identifier => {
                            if current_keyword == Some(SqlSyntaxKind::Update) {
                                table = n.text(source).to_string();
                            } else if current_keyword == Some(SqlSyntaxKind::Set) {
                                last_column = n.text(source).to_string();
                            }
                        }
                        SqlSyntaxKind::Expression => {
                            if current_keyword == Some(SqlSyntaxKind::Set) && !last_column.is_empty() {
                                assignments.push((last_column.clone(), Self::translate_expr_node(n, source)?));
                            } else if current_keyword == Some(SqlSyntaxKind::Where) {
                                selection = Some(Self::translate_expr_node(n, source)?);
                            }
                        }
                        _ => {}
                    }
                }
                RedTree::Leaf(t) => {
                    match t.kind() {
                        SqlSyntaxKind::Update | SqlSyntaxKind::Set | SqlSyntaxKind::Where => current_keyword = Some(t.kind()),
                        _ => {}
                    }
                }
            }
        }
        Ok(SqlStatement::Update { table, assignments, selection })
    }

    fn parse_create_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<SqlStatement> {
        let mut table = String::new();
        let mut is_view = false;
        let mut columns = Vec::new();
        let mut current_keyword = None;

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    match n.kind() {
                        SqlSyntaxKind::TableName => table = n.text(source).to_string(),
                        SqlSyntaxKind::ColumnDefinition => {
                            let mut col_name = String::new();
                            let mut col_type = FieldType::Text;
                            for c in n.children() {
                                if let RedTree::Node(cn) = c {
                                    if cn.kind() == SqlSyntaxKind::ColumnName {
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
                            columns.push(Field { name: col_name, field_type: col_type, nullable: true, default: None });
                        }
                        SqlSyntaxKind::SelectStatement => {
                            if is_view {
                                return Ok(SqlStatement::CreateView { name: table, query: Self::parse_query_node(n, source)? });
                            }
                        }
                        SqlSyntaxKind::Identifier => {
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
            Ok(SqlStatement::CreateTable { name: table, schema: Some(DsSchema { fields: columns }) })
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
                    match n.kind() {
                        SqlSyntaxKind::TableName => table_name = n.text(source).to_string(),
                        SqlSyntaxKind::Identifier => {
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
        let mut object_type = "TABLE".to_string();
        let mut name = String::new();
        let mut if_exists = false;

        for child in node.children() {
            match child {
                RedTree::Node(n) => {
                    if n.kind() == SqlSyntaxKind::TableName || n.kind() == SqlSyntaxKind::Identifier {
                        name = n.text(source).to_string();
                    }
                }
                RedTree::Leaf(t) => {
                    match t.kind() {
                        SqlSyntaxKind::Table => object_type = "TABLE".to_string(),
                        SqlSyntaxKind::Index => object_type = "INDEX".to_string(),
                        SqlSyntaxKind::View => object_type = "VIEW".to_string(),
                        SqlSyntaxKind::Exists => if_exists = true,
                        _ => {}
                    }
                }
            }
        }
        Ok(SqlStatement::Drop { object_type, name, if_exists })
    }

    fn translate_expr_node(node: RedNode<SqlLanguage>, source: &SourceText) -> DsResult<Expr> {
        // Simple expression translation
        let children: Vec<_> = node.children().collect();
        if children.len() == 1 {
            if let RedTree::Leaf(t) = &children[0] {
                let text = t.text(source);
                match t.kind() {
                    SqlSyntaxKind::NumberLiteral => {
                        if text.contains('.') {
                            return Ok(Expr::Literal(DsValue::Float(text.parse().unwrap_or(0.0))));
                        } else {
                            return Ok(Expr::Literal(DsValue::Int(text.parse().unwrap_or(0))));
                        }
                    }
                    SqlSyntaxKind::StringLiteral => {
                        return Ok(Expr::Literal(DsValue::Text(text.trim_matches('\'').to_string())));
                    }
                    SqlSyntaxKind::Identifier | SqlSyntaxKind::Identifier_ => {
                        return Ok(Expr::Column(text.to_string()));
                    }
                    _ => {}
                }
            }
        }

        // Binary op: expr op expr
        if children.len() >= 3 {
            let left = if let RedTree::Node(n) = &children[0] {
                Self::translate_expr_node(n.clone(), source)?
            } else {
                return Err(yykv_types::DsError::parse("Invalid binary expression"));
            };

            let op = if let RedTree::Leaf(t) = &children[1] {
                match t.kind() {
                    SqlSyntaxKind::Equal => BinaryOperator::Eq,
                    SqlSyntaxKind::NotEqual => BinaryOperator::Ne,
                    SqlSyntaxKind::Less => BinaryOperator::Lt,
                    SqlSyntaxKind::LessEqual => BinaryOperator::Le,
                    SqlSyntaxKind::Greater => BinaryOperator::Gt,
                    SqlSyntaxKind::GreaterEqual => BinaryOperator::Ge,
                    SqlSyntaxKind::Plus => BinaryOperator::Add,
                    SqlSyntaxKind::Minus => BinaryOperator::Sub,
                    SqlSyntaxKind::Star => BinaryOperator::Mul,
                    SqlSyntaxKind::Slash => BinaryOperator::Div,
                    _ => return Err(yykv_types::DsError::parse("Unsupported binary operator")),
                }
            } else {
                return Err(yykv_types::DsError::parse("Invalid binary operator"));
            };

            let right = if let RedTree::Node(n) = &children[2] {
                Self::translate_expr_node(n.clone(), source)?
            } else {
                return Err(yykv_types::DsError::parse("Invalid binary expression"));
            };

            return Ok(Expr::BinaryOp(Box::new(left), op, Box::new(right)));
        }

        Ok(Expr::Literal(DsValue::Null))
    }
}
