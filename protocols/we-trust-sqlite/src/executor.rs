use crate::reader::NativeReader;
use crate::writer::NativeWriter;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use yykv_operators::{
    AggExpr, Expr, OperatorNode, OpsGraph,
    sql::{SqlParser, SqlStatement},
};
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

/// Wrapper to make YYValue hashable and comparable
#[derive(Debug, Clone)]
struct HashableYYValue(DsValue);

impl PartialEq for HashableYYValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.partial_cmp(&other.0) == Some(Ordering::Equal)
    }
}

impl Eq for HashableYYValue {}

impl std::hash::Hash for HashableYYValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self.0 {
            DsValue::Null => 0.hash(state),
            DsValue::Bool(b) => b.hash(state),
            DsValue::Int(i) => i.hash(state),
            DsValue::Float(f) => f.to_bits().hash(state),
            DsValue::Text(s) => s.hash(state),
            DsValue::Bytes(b) | DsValue::Binary(b) => b.hash(state),
            DsValue::Uuid(u) => u.hash(state),
            _ => 0.hash(state), // Simplified for other types
        }
    }
}

/// SQLite 专用执行器
pub struct SqliteExecutor {
    reader: Arc<NativeReader>,
    writer: Arc<NativeWriter>,
}

impl SqliteExecutor {
    pub fn new(reader: Arc<NativeReader>, writer: Arc<NativeWriter>) -> Self {
        Self { reader, writer }
    }

    /// 执行 SQL 查询
    pub async fn execute_query(&self, sql: &str) -> Result<Vec<DsValue>> {
        self.writer.ensure_initialized().await?;
        let stmt = SqlParser::parse_sql(sql).map_err(|e| DsError::query(e.to_string()))?;
        match stmt {
            SqlStatement::Query(graph) => self.execute_graph(graph).await,
            SqlStatement::Insert { table, key, value } => {
                let schemas = self.reader.get_schemas().await?;
                let schema = schemas
                    .iter()
                    .find(|s| s.name == table)
                    .ok_or_else(|| DsError::query(format!("Table not found: {}", table)))?;

                let row_id = match key {
                    DsValue::Int(i) => i,
                    _ => {
                        return Err(DsError::query(
                            "Only integer primary keys (rowid) are supported for now",
                        ));
                    }
                };

                let values = match value {
                    DsValue::List(l) => l,
                    _ => vec![value],
                };

                self.writer
                    .insert_into_leaf_page(schema.rootpage, row_id, &values, vec![])
                    .await?;
                self.reader.clear_cache().await;
                Ok(vec![DsValue::Text(format!(
                    "INSERT INTO {} SUCCESS",
                    table
                ))])
            }
            SqlStatement::CreateTable { name, schema } => {
                // 1. 分配新页面
                let new_page_id = self.writer.allocate_page().await?;

                // 2. 初始化为叶子表页面
                self.writer.init_leaf_page(new_page_id).await?;

                // 3. 写入 sqlite_master (Page 1)
                // SQLite master schema: (type, name, tbl_name, rootpage, sql)
                let sql_text = if let Some(s) = schema {
                    let mut cols = Vec::new();
                    for field in s.fields.values() {
                        cols.push(format!("{} {:?}", field.name, field.field_type));
                    }
                    format!("CREATE TABLE {} ({})", name, cols.join(", "))
                } else {
                    format!("CREATE TABLE {} (dummy)", name)
                };

                let master_values = vec![
                    DsValue::Text("table".to_string()),
                    DsValue::Text(name.clone()),
                    DsValue::Text(name.clone()),
                    DsValue::Int(new_page_id as i64),
                    DsValue::Text(sql_text),
                ];

                // 简单的 row_id 生成逻辑 (实际应查找最大 row_id)
                let schemas = self.reader.get_schemas().await?;
                let next_row_id = schemas.len() as i64 + 1;

                self.writer
                    .insert_into_leaf_page(1, next_row_id, &master_values, vec![])
                    .await?;

                self.reader.clear_cache().await;

                Ok(vec![DsValue::Text(format!(
                    "CREATE TABLE {} SUCCESS AT PAGE {}",
                    name, new_page_id
                ))])
            }
            SqlStatement::Delete { table, selection } => {
                let schemas = self.reader.get_schemas().await?;
                let schema = schemas
                    .iter()
                    .find(|s| s.name == table)
                    .ok_or_else(|| DsError::query(format!("Table not found: {}", table)))?;

                let rows = self.reader.scan_table(schema.rootpage).await?;
                let mut deleted_count = 0;

                for row in rows {
                    let mut fields = BTreeMap::new();
                    let columns = schema.get_columns();
                    for (i, val) in row.values.iter().enumerate() {
                        let col_name = columns
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| format!("col_{}", i));
                        fields.insert(col_name, val.clone());
                    }
                    let dict_val = DsValue::Dict(fields);

                    let should_delete = if let Some(expr) = &selection {
                        match expr.evaluate(&dict_val).map_err(DsError::query)? {
                            DsValue::Bool(b) => b,
                            _ => false,
                        }
                    } else {
                        true
                    };

                    if should_delete {
                        self.writer
                            .delete_from_leaf_page(schema.rootpage, row.row_id)
                            .await?;
                        deleted_count += 1;
                    }
                }
                self.reader.clear_cache().await;
                Ok(vec![DsValue::Text(format!(
                    "DELETE {} ROWS FROM {} SUCCESS",
                    deleted_count, table
                ))])
            }
            SqlStatement::Update {
                table,
                assignments,
                selection,
            } => {
                let schemas = self.reader.get_schemas().await?;
                let schema = schemas
                    .iter()
                    .find(|s| s.name == table)
                    .ok_or_else(|| DsError::query(format!("Table not found: {}", table)))?;

                let rows = self.reader.scan_table(schema.rootpage).await?;
                let mut updated_count = 0;

                for row in rows {
                    let mut fields = BTreeMap::new();
                    let columns = schema.get_columns();
                    for (i, val) in row.values.iter().enumerate() {
                        let col_name = columns
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| format!("col_{}", i));
                        fields.insert(col_name, val.clone());
                    }
                    let dict_val = DsValue::Dict(fields.clone());

                    let should_update = if let Some(expr) = &selection {
                        match expr.evaluate(&dict_val).map_err(DsError::query)? {
                            DsValue::Bool(b) => b,
                            _ => false,
                        }
                    } else {
                        true
                    };

                    if should_update {
                        let mut new_values = row.values.clone();
                        for (col_name, expr) in &assignments {
                            if let Some(idx) = columns.iter().position(|c| c == col_name) {
                                new_values[idx] =
                                    expr.evaluate(&dict_val).map_err(DsError::query)?;
                            }
                        }

                        // re-insert with same row_id (will overwrite if implementation supports it)
                        self.writer
                            .insert_into_leaf_page(schema.rootpage, row.row_id, &new_values, vec![])
                            .await?;
                        updated_count += 1;
                    }
                }
                self.reader.clear_cache().await;
                Ok(vec![DsValue::Text(format!(
                    "UPDATE {} ROWS IN {} SUCCESS",
                    updated_count, table
                ))])
            }
            _ => Err(DsError::query("Unsupported SQL statement")),
        }
    }

    /// 执行插入操作 (Placeholder)
    pub async fn execute_insert(
        &self,
        _table: &str,
        _columns: Vec<String>,
        _values: Vec<Vec<DsValue>>,
    ) -> Result<()> {
        // ... (implementation pending adaptation to new SqlStatement)
        Ok(())
    }

    /// 执行算子图
    pub async fn execute_graph(&self, graph: OpsGraph) -> Result<Vec<DsValue>> {
        let mut results = Vec::new();

        for node in &graph.nodes {
            match node {
                OperatorNode::Source(op) => {
                    let rows = self.reader.scan_table_by_name(&op.table).await?;
                    for row in rows {
                        let mut fields = BTreeMap::new();
                        let schemas = self.reader.get_schemas().await?;
                        let schema = schemas.iter().find(|s| s.name == op.table);
                        let columns = if let Some(s) = schema {
                            s.get_columns()
                        } else {
                            op.columns.clone()
                        };

                        for (i, val) in row.values.into_iter().enumerate() {
                            let col_name = columns
                                .get(i)
                                .cloned()
                                .unwrap_or_else(|| format!("col_{}", i));
                            fields.insert(col_name, val);
                        }
                        results.push(DsValue::Dict(fields));
                    }
                }
                OperatorNode::Filter(op) => {
                    let mut filtered_results = Vec::new();
                    for val in results {
                        match op
                            .expression
                            .evaluate(&val)
                            .map_err(DsError::query)?
                        {
                            DsValue::Bool(b) if b => filtered_results.push(val),
                            _ => {}
                        }
                    }
                    results = filtered_results;
                }
                OperatorNode::Project(op) => {
                    let mut projected_results = Vec::new();
                    for val in results {
                        let mut new_fields = BTreeMap::new();
                        for (alias, expr) in &op.projections {
                            new_fields.insert(
                                alias.clone(),
                                expr.evaluate(&val).map_err(DsError::query)?,
                            );
                        }
                        projected_results.push(DsValue::Dict(new_fields));
                    }
                    results = projected_results;
                }
                OperatorNode::Aggregate(op) => {
                    let mut groups: HashMap<Vec<HashableYYValue>, Vec<DsValue>> = HashMap::new();

                    for val in results {
                        let mut group_key = Vec::new();
                        for expr in &op.group_by {
                            group_key.push(HashableYYValue(
                                expr.evaluate(&val).map_err(DsError::query)?,
                            ));
                        }
                        groups.entry(group_key).or_default().push(val);
                    }

                    let mut agg_results = Vec::new();
                    for (key, group_rows) in groups {
                        let mut agg_row = BTreeMap::new();
                        // 放入 group by 的列
                        for (i, expr) in op.group_by.iter().enumerate() {
                            if let Expr::Column(name) = expr {
                                agg_row.insert(name.clone(), key[i].0.clone());
                            }
                        }

                        // 执行聚合
                        for agg_expr in &op.aggs {
                            match agg_expr {
                                AggExpr::Count(_) => {
                                    agg_row.insert(
                                        "count".to_string(),
                                        DsValue::Int(group_rows.len() as i64),
                                    );
                                }
                                AggExpr::Sum(expr) => {
                                    let sum: i64 = group_rows
                                        .iter()
                                        .map(|row| {
                                            if let Ok(DsValue::Int(i)) = expr.evaluate(row) {
                                                i
                                            } else {
                                                0
                                            }
                                        })
                                        .sum();
                                    agg_row.insert("sum".to_string(), DsValue::Int(sum));
                                }
                                _ => {} // 其他聚合函数暂未实现
                            }
                        }
                        agg_results.push(DsValue::Dict(agg_row));
                    }
                    results = agg_results;
                }
                OperatorNode::VectorSearch(op) => {
                    // 简单的暴力余弦相似度搜索
                    let mut scored_results: Vec<(f32, DsValue)> = results
                        .into_iter()
                        .map(|val| {
                            let score = if let Ok(DsValue::Vector(v)) =
                                Expr::Column(op.column.clone()).evaluate(&val)
                            {
                                self.cosine_similarity(&op.vector, &v.0)
                            } else {
                                0.0
                            };
                            (score, val)
                        })
                        .collect();

                    scored_results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                    results = scored_results
                        .into_iter()
                        .take(op.limit)
                        .map(|(_, val)| val)
                        .collect();
                }
                _ => {
                    return Err(DsError::query(format!("Unsupported operator: {:?}", node)));
                }
            }
        }

        Ok(results)
    }

    fn cosine_similarity(&self, v1: &[f32], v2: &[f32]) -> f32 {
        if v1.len() != v2.len() || v1.is_empty() {
            return 0.0;
        }
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm_v1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm_v2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
        if norm_v1 == 0.0 || norm_v2 == 0.0 {
            return 0.0;
        }
        dot_product / (norm_v1 * norm_v2)
    }
}
