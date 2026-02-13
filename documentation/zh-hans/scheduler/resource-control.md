# 资源控制与多租户

资源控制调度负责在多租户环境下进行准入控制（Admission Control）、配额限制和审计，确保系统的稳定性与公平性。

## 核心组件

### [yyds-quota](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-quota)
**配额控制系统**。
- **职责**：实时监控租户的 I/O、计算和存储消耗。
- **调度逻辑**：根据租户等级自动进行限流（Throttling），防止恶意或异常请求耗尽系统资源。

### [yyds-tenant](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-tenant)
**租户管理中心**。
- **职责**：管理租户身份、权限和元数据。
- **调度相关**：为调度系统提供租户上下文，实现资源请求的路由。

### [yyds-billing](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-billing)
**计费调度系统**。
- **职责**：异步汇总租户资源消耗并生成账单。
- **调度相关**：通过 `yyds-task` 调度周期性的计费聚合任务。

### [yyds-observability](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-observability)
**可观测性系统**。
- **职责**：收集调度系统的各项指标（Latency, Throughput, Queue depth）。
- **调度反馈**：为自动扩缩容和动态负载均衡提供决策依据。

### 类型与策略支持
- **相关包**：`yyds-types`, `yykv-types`, `yykv-policy`, `yykv-schema`
- **职责**：定义统一的数据类型、调度策略模板和元数据模式，确保调度指令在各组件间语义一致。
