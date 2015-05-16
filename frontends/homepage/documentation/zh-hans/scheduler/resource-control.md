# 资源控制与多租户

资源控制调度负责在多租户环境下进行准入控制（Admission Control）、配额限制和审计，确保系统的稳定性与公平性。

## 核心组件

### [yyds-quota](../../backends/distributed/yyds-quota)
**配额控制系统**。
- **职责**：实时监控租户的 I/O、计算和存储消耗，限制后台任务的总带宽占用。
- **调度逻辑**：根据租户等级自动进行限流（Throttling），防止恶意或异常请求耗尽系统资源。

### [yyds-tenant](../../backends/distributed/yyds-tenant)
**租户管理中心**。
- **职责**：管理租户身份、权限和元数据。
- **租户隔离**：任务执行时绑定 `TenantId`，严格限制每个租户的 CPU 时间片和内存配额。

### [yyds-billing](../../backends/distributed/yyds-billing)
**计费调度系统**。
- **职责**：异步汇总租户资源消耗（I/O、存储空间）并生成账单。

### [yyds-observability](../../backends/distributed/yyds-observability)
**可观测性系统**。
- **职责**：实时追踪任务执行延迟与资源占用，为自动扩缩容提供决策依据。

### 类型与策略支持
- **相关包**：[yyds-types](../../backends/distributed/yyds-types), [yykv-types](../../backends/kv-store/yykv-types), [yykv-policy](../../backends/kv-store/yykv-policy), [yykv-schema](../../backends/kv-store/yykv-schema)
- **职责**：定义统一的数据类型、调度策略模板和元数据模式。
