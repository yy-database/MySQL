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

/// 算子图 IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsGraph {
    /// 图 ID
    pub id: Uuid,
    /// 算子节点列表
    pub nodes: Vec<OperatorNode>,
    /// 算子边列表
    pub edges: Vec<Edge>,
}

impl OpsGraph {
    /// 创建新的算子图
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

/// 排序算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOp {
    /// 排序规则
    pub order_by: Vec<OrderBy>,
}

/// 限制算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOp {
    /// 限制条数
    pub limit: usize,
    /// 偏移量
    pub offset: usize,
}

/// 数据源算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceOp {
    /// 表名
    pub table: String,
    /// 列名
    pub columns: Vec<String>,
}

/// 索引扫描算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexScanOp {
    /// 表名
    pub table: String,
    /// 索引名
    pub index_name: String,
    /// 列名
    pub columns: Vec<String>,
    /// 扫描条件
    pub conditions: Vec<IndexCondition>,
}

/// 索引条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCondition {
    /// 列名
    pub column: String,
    /// 运算符
    pub op: BinaryOperator,
    /// 值
    pub value: DsValue,
}

/// 过滤算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOp {
    /// 过滤表达式
    pub expression: Expr,
}

/// 投影算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOp {
    /// 投影列表 (别名, 表达式)
    pub projections: Vec<(String, Expr)>,
}

/// 聚合算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateOp {
    /// 分组列表
    pub group_by: Vec<Expr>,
    /// 聚合列表
    pub aggs: Vec<AggExpr>,
}

/// 向量检索算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchOp {
    /// 向量列
    pub column: String,
    /// 检索向量
    pub vector: Vec<f32>,
    /// 限制条数
    pub limit: usize,
}

/// Join 算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinOp {
    /// 左表列名
    pub left_on: String,
    /// 右表列名
    pub right_on: String,
    /// Join 类型
    pub join_type: JoinType,
}

/// 窗口函数算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowOp {
    /// 分区列表
    pub partition_by: Vec<Expr>,
    /// 排序列表
    pub order_by: Vec<OrderBy>,
    /// 窗口函数
    pub func: WindowFunc,
}

/// 排序规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    /// 排序表达式
    pub expr: Expr,
    /// 是否降序
    pub desc: bool,
}

/// 窗口函数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowFunc {
    /// 行号
    RowNumber,
    /// 排名
    Rank,
    /// 稠密排名
    DenseRank,
    /// 聚合函数
    Aggregate(AggExpr),
}

/// Join 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinType {
    /// 内连接
    Inner,
    /// 左连接
    Left,
    /// 右连接
    Right,
    /// 全连接
    Full,
}

/// UDF 调用算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfOp {
    /// 函数名
    pub function_name: String,
    /// 参数列表
    pub args: Vec<Expr>,
}

/// 汇聚算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkOp {
    /// 目标标识
    pub target: String,
}

/// 表达式 IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    /// 列名
    Column(String),
    /// 字面量
    Literal(DsValue),
    /// 二元运算
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    /// 一元运算
    UnaryOp(Box<UnaryOperator>, Box<Expr>),
    /// 函数调用
    FunctionCall(String, Vec<Expr>),
    /// 参数绑定 ($1, $2, ...)
    Param(usize),
    /// IN 列表
    InList(Box<Expr>, Vec<Expr>, bool),
    /// 标量子查询
    ScalarSubquery(Box<OpsGraph>),
    /// IN 子查询
    InSubquery(Box<Expr>, Box<OpsGraph>, bool),
    /// EXISTS 子查询
    Exists(Box<OpsGraph>, bool),
    /// 类型转换
    Cast(Box<Expr>, String),
}

/// 二元运算符
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BinaryOperator {
    /// 等于
    Eq,
    /// 不等于
    Neq,
    /// 大于
    Gt,
    /// 大于等于
    Gte,
    /// 小于
    Lt,
    /// 小于等于
    Lte,
    /// 加
    Add,
    /// 减
    Sub,
    /// 乘
    Mul,
    /// 除
    Div,
    /// 与
    And,
    /// 或
    Or,
    /// 包含 (LIKE)
    Like,
    /// 不区分大小写包含 (ILIKE)
    ILike,
    /// 正则匹配
    Regex,
    /// 字符串拼接
    Concat,
}

/// 一元运算符
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOperator {
    /// 非
    Not,
    /// 负
    Neg,
}

/// 聚合表达式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggExpr {
    /// 计数
    Count(Expr),
    /// 求和
    Sum(Expr),
    /// 平均值
    Avg(Expr),
    /// 最大值
    Max(Expr),
    /// 最小值
    Min(Expr),
}

/// 子查询评估器接口
pub trait SubqueryEvaluator {
    /// 评估标量子查询
    fn evaluate_scalar(&self, graph: &OpsGraph) -> Result<DsValue, String>;
    /// 评估列表子查询
    fn evaluate_list(&self, graph: &OpsGraph) -> Result<Vec<DsValue>, String>;
    /// 评估是否存在子查询
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
    /// 评估表达式的值
    pub fn evaluate(&self, row: &DsValue) -> Result<DsValue, String> {
        self.evaluate_with_params_and_context(row, &[], None)
    }

    /// 使用参数评估表达式的值
    pub fn evaluate_with_params(
        &self,
        row: &DsValue,
        params: &[DsValue],
    ) -> Result<DsValue, String> {
        self.evaluate_with_params_and_context(row, params, None)
    }

    /// 使用参数和上下文评估表达式的值
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
