# yykv-operators (Logical & Physical Operator IR)

`yykv-operators` 定义了 YYKV 存储引擎的算子图（OpsGraph）中间表示（IR）与表达式系统。它是连接上层协议（SQL, Redis 等）与底层执行引擎（`yykv-executor`）的桥梁。

## 核心功能

### 🕸️ 算子图 (OpsGraph)
定义了查询的物理执行计划：
- **节点抽象**: 每个算子是一个节点，支持多输入多输出。
- **序列化**: 算子图可以序列化为二进制格式，支持在分布式环境下跨节点下推执行。

### 🛠️ 多模态算子集
- **基础算子**: `TableScan` (全表扫描), `Filter` (谓词过滤), `Project` (投影), `Limit` (限制)。
- **关系算子**: `HashJoin`, `Sort`, `Aggregate` (聚合)。
- **高级算子**:
  - `VectorSearch`: 专门针对向量相似度检索优化的算子。
  - `UdfCall`: 触发存储侧 `yykv-udf` (Wasm) 执行的算子。

### 🧮 表达式引擎
支持基于 `DsValue` 的复杂表达式计算：
- **标量函数**: 算术运算、字符串处理、时间转换。
- **谓词评估**: 高性能的逻辑运算判断。

## 核心组件

- `OpsGraph`: 整个执行计划的拓扑结构。
- `Operator`: 算子枚举，定义了所有支持的执行单元。
- `Expression`: 树状或扁平化的表达式 IR。

## 使用示例

```rust
use yykv_operators::{OpsGraph, Operator, Expression};

let mut graph = OpsGraph::new();

// 构建一个：扫描 users 表 -> 过滤年龄 > 18 -> 取前 10 条
let scan_id = graph.add_node(Operator::TableScan { table: "users".into() });
let filter_id = graph.add_node(Operator::Filter { 
    predicate: Expression::GreaterThan(Box::new(Expression::Column("age")), Box::new(Expression::Literal(18.into())))
});
graph.add_edge(scan_id, filter_id);

// 提交给执行引擎
executor.execute(graph).await?;
```

## 技术优势
- **下推友好**: 算子设计时优先考虑如何下推到存储节点（Near-Data Computing）。
- **零拷贝设计**: 算子间传递数据尽量采用引用或 `bytes` 视图。
