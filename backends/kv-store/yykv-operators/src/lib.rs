#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yykv_types::{DsError, DsResult, DsValue, FromValue, ToValue};

impl ToValue for OpsGraph {
    fn to_value(&self) -> DsValue {
        // Serialize via serde_json to a string, then wrap in DsValue::Text
        // This is a pragmatic choice to avoid manual AST serialization while removing bincode.
        // The binary format is effectively JSON-in-DsValue.
        match serde_json::to_string(self) {
            Ok(s) => DsValue::Ext("OpsGraph".to_string(), s.into_bytes()),
            Err(_) => DsValue::Null,
        }
    }
}

impl FromValue for OpsGraph {
    fn from_value(v: DsValue) -> DsResult<Self> {
        match v {
            DsValue::Ext(t, b) if t == "OpsGraph" => {
                serde_json::from_slice(&b).map_err(|e| DsError::serialization(e.to_string()))
            }
            _ => Err(DsError::type_err("OpsGraph (Ext)", format!("{:?}", v))),
        }
    }
}

/// Operator graph IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsGraph {
    /// Graph ID
    pub id: Uuid,
    /// List of operator nodes
    pub nodes: Vec<OperatorNode>,
    /// List of operator edges
    pub edges: Vec<Edge>,
}

impl OpsGraph {
    /// Create a new operator graph
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

/// Operator edge, connecting two operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
}

/// Operator node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperatorNode {
    /// Data source (e.g., TableScan)
    Source(SourceOp),
    /// Filter operator
    Filter(FilterOp),
    /// Project operator
    Project(ProjectOp),
    /// Aggregate operator
    Aggregate(AggregateOp),
    /// Vector search operator
    VectorSearch(VectorSearchOp),
    /// Index scan operator
    IndexScan(IndexScanOp),
    /// Join operator
    Join(JoinOp),
    /// Window function operator
    Window(WindowOp),
    /// UDF call operator
    UdfCall(UdfOp),
    /// Sort operator
    Sort(SortOp),
    /// Limit operator
    Limit(LimitOp),
    /// Sink node (Sink)
    Sink(SinkOp),
}

/// Sort operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOp {
    /// Sorting rules
    pub order_by: Vec<OrderBy>,
}

/// Limit operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOp {
    /// Limit count
    pub limit: usize,
    /// Offset
    pub offset: usize,
}

/// Data source operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceOp {
    /// Table name
    pub table: String,
    /// Column names
    pub columns: Vec<String>,
}

/// Index scan operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexScanOp {
    /// Table name
    pub table: String,
    /// Index name
    pub index_name: String,
    /// Column names
    pub columns: Vec<String>,
    /// Scan conditions
    pub conditions: Vec<IndexCondition>,
}

/// Index condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCondition {
    /// Column name
    pub column: String,
    /// Operator
    pub op: BinaryOperator,
    /// Value
    pub value: DsValue,
}

/// Filter operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOp {
    /// Filter expression
    pub expression: Expr,
}

/// Project operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOp {
    /// Project list (alias, expression)
    pub projections: Vec<(String, Expr)>,
}

/// Aggregate operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateOp {
    /// Group by list
    pub group_by: Vec<Expr>,
    /// Aggregate list
    pub aggs: Vec<AggExpr>,
}

/// Vector search operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchOp {
    /// Vector column
    pub column: String,
    /// Search vector
    pub vector: Vec<f32>,
    /// Limit count
    pub limit: usize,
}

/// Join operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinOp {
    /// Left table column name
    pub left_on: String,
    /// Right table column name
    pub right_on: String,
    /// Join type
    pub join_type: JoinType,
}

/// Window function operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowOp {
    /// Partition by list
    pub partition_by: Vec<Expr>,
    /// Sort by list
    pub order_by: Vec<OrderBy>,
    /// Window function
    pub func: WindowFunc,
}

/// Sorting rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    /// Sorting expression
    pub expr: Expr,
    /// Whether descending
    pub desc: bool,
}

/// Window function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowFunc {
    /// Row number
    RowNumber,
    /// Rank
    Rank,
    /// Dense rank
    DenseRank,
    /// Aggregate function
    Aggregate(AggExpr),
}

/// Join type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinType {
    /// Inner join
    Inner,
    /// Left join
    Left,
    /// Right join
    Right,
    /// Full join
    Full,
}

/// UDF call operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfOp {
    /// Function name
    pub function_name: String,
    /// Parameter list
    pub args: Vec<Expr>,
}

/// Sink operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkOp {
    /// Target identifier
    pub target: String,
}

/// Expression IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    /// Column name
    Column(String),
    /// Literal
    Literal(DsValue),
    /// Binary operation
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    /// Unary operation
    UnaryOp(Box<UnaryOperator>, Box<Expr>),
    /// Function call
    FunctionCall(String, Vec<Expr>),
    /// Parameter binding ($1, $2, ...)
    Param(usize),
    /// IN list
    InList(Box<Expr>, Vec<Expr>, bool),
    /// Scalar subquery
    ScalarSubquery(Box<OpsGraph>),
    /// IN subquery
    InSubquery(Box<Expr>, Box<OpsGraph>, bool),
    /// EXISTS subquery
    Exists(Box<OpsGraph>, bool),
    /// Type cast
    Cast(Box<Expr>, String),
}

/// Binary operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BinaryOperator {
    /// Equal
    Eq,
    /// Not equal
    Neq,
    /// Greater than
    Gt,
    /// Greater than or equal
    Gte,
    /// Less than
    Lt,
    /// Less than or equal
    Lte,
    /// Add
    Add,
    /// Subtract
    Sub,
    /// Multiply
    Mul,
    /// Divide
    Div,
    /// And
    And,
    /// Or
    Or,
    /// Like
    Like,
    /// Case-insensitive like (ILIKE)
    ILike,
    /// Regex match
    Regex,
    /// String concatenation
    Concat,
}

/// Unary operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOperator {
    /// Not
    Not,
    /// Negative
    Neg,
}

/// Aggregate expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggExpr {
    /// Count
    Count(Expr),
    /// Sum
    Sum(Expr),
    /// Average
    Avg(Expr),
    /// Max
    Max(Expr),
    /// Min
    Min(Expr),
}

/// Subquery evaluator interface
pub trait SubqueryEvaluator {
    /// Evaluate scalar subquery
    fn evaluate_scalar(&self, graph: &OpsGraph) -> Result<DsValue, String>;
    /// Evaluate list subquery
    fn evaluate_list(&self, graph: &OpsGraph) -> Result<Vec<DsValue>, String>;
    /// Evaluate if subquery exists
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
            Expr::Cast(e, ty) => write!(f, "CAST({} AS {})", e, ty),
        }
    }
}

impl Expr {
    /// Evaluate the value of the expression
    pub fn evaluate(&self, row: &DsValue) -> Result<DsValue, String> {
        self.evaluate_with_params_and_context(row, &[], None)
    }

    /// Evaluate the value of the expression with parameters
    pub fn evaluate_with_params(
        &self,
        row: &DsValue,
        params: &[DsValue],
    ) -> Result<DsValue, String> {
        self.evaluate_with_params_and_context(row, params, None)
    }

    /// Evaluate the value of the expression with parameters and context
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
                    // Assuming Object fields are tag -> value, simplified here as string match
                    // In reality, Schema info might be needed
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
                    BinaryOperator::Concat => {
                        let s1 = match lv {
                            DsValue::Text(s) => s.clone(),
                            _ => lv.to_string(),
                        };
                        let s2 = match rv {
                            DsValue::Text(s) => s.clone(),
                            _ => rv.to_string(),
                        };
                        Ok(DsValue::Text(format!("{}{}", s1, s2)))
                    }
                    _ => Ok(DsValue::Null),
                }
            }
            Expr::Param(idx) => Ok(params.get(*idx).cloned().unwrap_or(DsValue::Null)),
            Expr::UnaryOp(op, expr) => {
                let v = expr.evaluate_with_params_and_context(row, params, context)?;
                match **op {
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
            Expr::Cast(expr, ty) => {
                let v = expr.evaluate_with_params_and_context(row, params, context)?;
                let ty = ty.to_uppercase();
                match ty.as_str() {
                    "INT" | "INTEGER" => match v {
                        DsValue::Int(i) => Ok(DsValue::Int(i)),
                        DsValue::Float(f) => Ok(DsValue::Int(f as i64)),
                        DsValue::Bool(b) => Ok(DsValue::Int(if b { 1 } else { 0 })),
                        DsValue::Text(s) => Ok(DsValue::Int(s.parse::<i64>().unwrap_or(0))),
                        _ => Ok(DsValue::Null),
                    },
                    "FLOAT" | "DOUBLE" => match v {
                        DsValue::Float(f) => Ok(DsValue::Float(f)),
                        DsValue::Int(i) => Ok(DsValue::Float(i as f64)),
                        DsValue::Bool(b) => Ok(DsValue::Float(if b { 1.0 } else { 0.0 })),
                        DsValue::Text(s) => Ok(DsValue::Float(s.parse::<f64>().unwrap_or(0.0))),
                        _ => Ok(DsValue::Null),
                    },
                    "BOOL" | "BOOLEAN" => match v {
                        DsValue::Bool(b) => Ok(DsValue::Bool(b)),
                        DsValue::Int(i) => Ok(DsValue::Bool(i != 0)),
                        DsValue::Float(f) => Ok(DsValue::Bool(f != 0.0)),
                        DsValue::Text(s) => {
                            let s = s.to_lowercase();
                            Ok(DsValue::Bool(s == "true" || s == "1" || s == "t"))
                        }
                        _ => Ok(DsValue::Null),
                    },
                    "TEXT" | "STRING" => match v {
                        DsValue::Text(s) => Ok(DsValue::Text(s)),
                        other => Ok(DsValue::Text(format!("{:?}", other))),
                    },
                    _ => Ok(v),
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
