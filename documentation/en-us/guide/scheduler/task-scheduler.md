# Task Scheduling and Execution (yyds-task & yykv-executor)

The YYDS task scheduling system is responsible for converting high-level abstract query plans into execution streams on physical nodes and handling complex background asynchronous tasks.

## Task Types

### 1. Real-time Compute Tasks
Directly driven by `yykv-executor`, handling `OpsGraph` from the protocol layer.
- **Near-Data Computing**: Operators act directly on physical pages defined by `yykv-layout`.
- **Wasm Sandboxing**: Computing logic provided by `yykv-udf` executes in a restricted environment to prevent affecting storage kernel stability.

### 2. Background Asynchronous Tasks (yyds-task)
Handles long-running management operations:
- **Distributed Cron**: Periodically triggers billing summaries (`yyds-billing`) and system inspections.
- **State Tracking**: Task states are persisted in the WAL, supporting automatic rescheduling after node failures.

### 3. WAE Event-Driven Scheduling
The **Write-Ahead Event** mechanism implemented by `yykv-event`.
- When data is written to the WAL, it triggers subscribers (`WaeWorker`) in real-time.
- Supports pre-building indexes and updating materialized views before data is flushed to disk.

## Scheduling Policies

- **Collaborative Scheduling**: `yyds-task` is aware of `yykv-scheduler` load and automatically suspends non-urgent background tasks when I/O is busy.
- **Tenant Isolation**: Tasks are bound to a `TenantId` during execution, strictly limiting CPU time slices and memory quotas for each tenant.

## Related Components

- [yykv-operators](file:///e:/yydb/yykv/packages/yykv-operators/readme.md): Defines operator graph IR.
- [yyds-consensus](file:///e:/yydb/yykv/packages/yyds-consensus/README.md): Responsible for task synchronization and consistency across replicas.
- [yyds-observability](file:///e:/yydb/yykv/packages/yyds-observability/README.md): Real-time tracking of task execution latency and resource consumption.
