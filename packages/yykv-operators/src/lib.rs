use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yykv_types::DsValue;

pub mod sql;

/// 算子图 IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsGraph {
    pub id: Uuid,
    pub nodes: Vec<OperatorNode>,
    pub edges: Vec<Edge>,
}

impl OpsGraph {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl Default for OpsGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// 算子边，连接两个算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
}

/// 算子节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperatorNode {
    /// 数据源 (如 TableScan)
    Source(SourceOp),
    /// 过滤算子
    Filter(FilterOp),
    /// 投影算子
    Project(ProjectOp),
    /// 聚合算子
    Aggregate(AggregateOp),
    /// 向量检索算子
    VectorSearch(VectorSearchOp),
    /// 索引扫描算子
    IndexScan(IndexScanOp),
    /// Join 算子
    Join(JoinOp),
    /// 窗口函数算子
    Window(WindowOp),
    /// UDF 调用算子
    UdfCall(UdfOp),
    /// 排序算子
    Sort(SortOp),
    /// 限制算子
    Limit(LimitOp),
    /// 汇聚节点 (Sink)
    Sink(SinkOp),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOp {
    pub order_by: Vec<OrderBy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOp {
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceOp {
    pub table: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexScanOp {
    pub table: String,
    pub index_name: String,
    pub columns: Vec<String>,
    pub conditions: Vec<IndexCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCondition {
    pub column: String,
    pub op: BinaryOperator,
    pub value: DsValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOp {
    pub expression: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOp {
    pub projections: Vec<(String, Expr)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateOp {
    pub group_by: Vec<Expr>,
    pub aggs: Vec<AggExpr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchOp {
    pub column: String,
    pub vector: Vec<f32>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinOp {
    pub left_on: String,
    pub right_on: String,
    pub join_type: JoinType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowOp {
    pub partition_by: Vec<Expr>,
    pub order_by: Vec<OrderBy>,
    pub func: WindowFunc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    pub expr: Expr,
    pub desc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowFunc {
    RowNumber,
    Rank,
    DenseRank,
    Aggregate(AggExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfOp {
    pub function_name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkOp {
    pub target: String,
}

/// 表达式 IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Column(String),
    Literal(DsValue),
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    UnaryOp(UnaryOperator, Box<Expr>),
    FunctionCall(String, Vec<Expr>),
    Param(usize),
    InList(Box<Expr>, Vec<Expr>, bool), // expr, list, negated
    ScalarSubquery(Box<OpsGraph>),
    InSubquery(Box<Expr>, Box<OpsGraph>, bool), // expr, subquery, negated
    Exists(Box<OpsGraph>, bool),                // subquery, negated
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BinaryOperator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Neg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggExpr {
    Count(Expr),
    Sum(Expr),
    Avg(Expr),
    Max(Expr),
    Min(Expr),
}

/// 子查询评估器接口
pub trait SubqueryEvaluator {
    fn evaluate_scalar(&self, graph: &OpsGraph) -> Result<DsValue, String>;
    fn evaluate_list(&self, graph: &OpsGraph) -> Result<Vec<DsValue>, String>;
    fn evaluate_exists(&self, graph: &OpsGraph) -> Result<bool, String>;
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Column(c) => write!(f, "{}", c),
            Expr::Literal(l) => write!(f, "{:?}", l),
            Expr::BinaryOp(l, op, r) => write!(f, "({} {:?} {})", l, op, r),
            Expr::UnaryOp(op, e) => write!(f, "{:?}{}", op, e),
            Expr::FunctionCall(name, args) => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Expr::Param(i) => write!(f, "${}", i),
            Expr::InList(e, list, negated) => {
                write!(f, "{} {} IN (", e, if *negated { "NOT" } else { "" })?;
                for (i, item) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
            Expr::ScalarSubquery(g) => write!(f, "({:?})", g),
            Expr::InSubquery(e, g, negated) => write!(
                f,
                "{} {} IN ({:?})",
                e,
                if *negated { "NOT" } else { "" },
                g
            ),
            Expr::Exists(g, negated) => {
                write!(f, "{}EXISTS({:?})", if *negated { "NOT " } else { "" }, g)
            }
        }
    }
}

impl Expr {
    pub fn evaluate(&self, row: &DsValue) -> Result<DsValue, String> {
        self.evaluate_with_params_and_context(row, &[], None)
    }

    pub fn evaluate_with_params(
        &self,
        row: &DsValue,
        params: &[DsValue],
    ) -> Result<DsValue, String> {
        self.evaluate_with_params_and_context(row, params, None)
    }

    pub fn evaluate_with_params_and_context(
        &self,
        row: &DsValue,
        params: &[DsValue],
        context: Option<&dyn SubqueryEvaluator>,
    ) -> Result<DsValue, String> {
        match self {
            Expr::Literal(v) => Ok(v.clone()),
            Expr::Column(name) => {
                if name == "*" {
                    return Ok(DsValue::Bool(true)); // Special case for count(*)
                }
                if let DsValue::Dict(d) = row {
                    Ok(d.get(name).cloned().unwrap_or(DsValue::Null))
                } else if let DsValue::Object { .. } = row {
                    // 假设 Object 的 fields 是 tag -> value，这里简化为字符串匹配
                    // 实际中可能需要 Schema 信息
                    Ok(DsValue::Null)
                } else {
                    Ok(DsValue::Null)
                }
            }
            Expr::BinaryOp(left, op, right) => {
                let lv = left.evaluate_with_params_and_context(row, params, context)?;
                let rv = right.evaluate_with_params_and_context(row, params, context)?;
                match op {
                    BinaryOperator::Eq => Ok(DsValue::Bool(lv == rv)),
                    BinaryOperator::Neq => Ok(DsValue::Bool(lv != rv)),
                    BinaryOperator::Gt => Ok(DsValue::Bool(lv > rv)),
                    BinaryOperator::Gte => Ok(DsValue::Bool(lv >= rv)),
                    BinaryOperator::Lt => Ok(DsValue::Bool(lv < rv)),
                    BinaryOperator::Lte => Ok(DsValue::Bool(lv <= rv)),
                    BinaryOperator::Add => match (lv, rv) {
                        (DsValue::Int(a), DsValue::Int(b)) => Ok(DsValue::Int(a + b)),
                        (DsValue::Float(a), DsValue::Float(b)) => Ok(DsValue::Float(a + b)),
                        (DsValue::Int(a), DsValue::Float(b)) => Ok(DsValue::Float(a as f64 + b)),
                        (DsValue::Float(a), DsValue::Int(b)) => Ok(DsValue::Float(a + b as f64)),
                        _ => Ok(DsValue::Null),
                    },
                    BinaryOperator::Sub => match (lv, rv) {
                        (DsValue::Int(a), DsValue::Int(b)) => Ok(DsValue::Int(a - b)),
                        (DsValue::Float(a), DsValue::Float(b)) => Ok(DsValue::Float(a - b)),
                        (DsValue::Int(a), DsValue::Float(b)) => Ok(DsValue::Float(a as f64 - b)),
                        (DsValue::Float(a), DsValue::Int(b)) => Ok(DsValue::Float(a - b as f64)),
                        _ => Ok(DsValue::Null),
                    },
                    BinaryOperator::Mul => match (lv, rv) {
                        (DsValue::Int(a), DsValue::Int(b)) => Ok(DsValue::Int(a * b)),
                        (DsValue::Float(a), DsValue::Float(b)) => Ok(DsValue::Float(a * b)),
                        (DsValue::Int(a), DsValue::Float(b)) => Ok(DsValue::Float(a as f64 * b)),
                        (DsValue::Float(a), DsValue::Int(b)) => Ok(DsValue::Float(a * b as f64)),
                        _ => Ok(DsValue::Null),
                    },
                    BinaryOperator::Div => match (lv, rv) {
                        (DsValue::Int(a), DsValue::Int(b)) if b != 0 => Ok(DsValue::Int(a / b)),
                        (DsValue::Float(a), DsValue::Float(b)) if b != 0.0 => {
                            Ok(DsValue::Float(a / b))
                        }
                        (DsValue::Int(a), DsValue::Float(b)) if b != 0.0 => {
                            Ok(DsValue::Float(a as f64 / b))
                        }
                        (DsValue::Float(a), DsValue::Int(b)) if b != 0 => {
                            Ok(DsValue::Float(a / b as f64))
                        }
                        _ => Ok(DsValue::Null),
                    },
                    BinaryOperator::And => {
                        if let (DsValue::Bool(a), DsValue::Bool(b)) = (lv, rv) {
                            Ok(DsValue::Bool(a && b))
                        } else {
                            Ok(DsValue::Null)
                        }
                    }
                    BinaryOperator::Or => {
                        if let (DsValue::Bool(a), DsValue::Bool(b)) = (lv, rv) {
                            Ok(DsValue::Bool(a || b))
                        } else {
                            Ok(DsValue::Null)
                        }
                    }
                }
            }
            Expr::Param(idx) => Ok(params.get(*idx).cloned().unwrap_or(DsValue::Null)),
            Expr::UnaryOp(op, expr) => {
                let v = expr.evaluate_with_params_and_context(row, params, context)?;
                match op {
                    UnaryOperator::Not => {
                        if let DsValue::Bool(b) = v {
                            Ok(DsValue::Bool(!b))
                        } else {
                            Ok(DsValue::Null)
                        }
                    }
                    UnaryOperator::Neg => match v {
                        DsValue::Int(i) => Ok(DsValue::Int(-i)),
                        DsValue::Float(f) => Ok(DsValue::Float(-f)),
                        _ => Ok(DsValue::Null),
                    },
                }
            }
            Expr::InList(expr, list, negated) => {
                if list.is_empty() {
                    return Ok(DsValue::Bool(*negated));
                }
                let v = expr.evaluate_with_params_and_context(row, params, context)?;
                if v == DsValue::Null {
                    return Ok(DsValue::Null);
                }

                let mut found = false;
                let mut has_null = false;
                for item in list {
                    let iv = item.evaluate_with_params_and_context(row, params, context)?;
                    if iv == DsValue::Null {
                        has_null = true;
                    } else if iv == v {
                        found = true;
                        break;
                    }
                }

                if found {
                    Ok(DsValue::Bool(!*negated))
                } else if has_null {
                    Ok(DsValue::Null)
                } else {
                    Ok(DsValue::Bool(*negated))
                }
            }
            Expr::ScalarSubquery(graph) => {
                if let Some(ctx) = context {
                    ctx.evaluate_scalar(graph)
                } else {
                    Ok(DsValue::Null)
                }
            }
            Expr::InSubquery(expr, graph, negated) => {
                if let Some(ctx) = context {
                    let v = expr.evaluate_with_params_and_context(row, params, context)?;
                    if v == DsValue::Null {
                        return Ok(DsValue::Null);
                    }
                    let list = ctx.evaluate_list(graph)?;
                    let mut found = false;
                    let mut has_null = false;
                    for iv in list {
                        if iv == DsValue::Null {
                            has_null = true;
                        } else if iv == v {
                            found = true;
                            break;
                        }
                    }

                    if found {
                        Ok(DsValue::Bool(!*negated))
                    } else if has_null {
                        Ok(DsValue::Null)
                    } else {
                        Ok(DsValue::Bool(*negated))
                    }
                } else {
                    Ok(DsValue::Null)
                }
            }
            Expr::Exists(graph, negated) => {
                if let Some(ctx) = context {
                    let exists = ctx.evaluate_exists(graph)?;
                    Ok(DsValue::Bool(if *negated { !exists } else { exists }))
                } else {
                    Ok(DsValue::Null)
                }
            }
            Expr::FunctionCall(name, args) => {
                // Handle some basic functions if needed
                match name.as_str() {
                    "ABS" => {
                        if args.len() == 1 {
                            let v =
                                args[0].evaluate_with_params_and_context(row, params, context)?;
                            match v {
                                DsValue::Int(i) => Ok(DsValue::Int(i.abs())),
                                DsValue::Float(f) => Ok(DsValue::Float(f.abs())),
                                _ => Ok(DsValue::Null),
                            }
                        } else {
                            Ok(DsValue::Null)
                        }
                    }
                    _ => Ok(DsValue::Null),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_eval() {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("age".into(), DsValue::Int(25));
        let row = DsValue::Dict(fields);

        let expr = Expr::BinaryOp(
            Box::new(Expr::Column("age".into())),
            BinaryOperator::Eq,
            Box::new(Expr::Literal(DsValue::Int(25))),
        );

        assert_eq!(expr.evaluate(&row), Ok(DsValue::Bool(true)));
    }
}
