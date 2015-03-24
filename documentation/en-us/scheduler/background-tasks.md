# Task Scheduling Layer

The task scheduling layer manages asynchronous operations, scheduled tasks, and event-based triggers, ensuring reliable execution of complex tasks in a distributed environment.

## Core Components

### [yyds-task](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-task)
**Distributed Task Manager**.
- **Responsibility**: Receives, stores, and distributes various background tasks.
- **Scheduling Features**: Supports Cron-style scheduled scheduling, with task retry, suspension, and full lifecycle tracking capabilities.

### [yykv-event](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-event)
**Event-Driven System**.
- **Responsibility**: Listens for state changes in the storage layer (e.g., Page changes, transaction commits).
- **Scheduling Related**: Triggers subsequent scheduling logic based on events, such as auto-triggering Compaction or notifying upper-layer applications.

### [yykv-wal](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-wal)
**Write-Ahead Log Manager**.
- **Responsibility**: Ensures persistence of tasks and data.
- **Scheduling Assurance**: Through the WAE (Write Ahead Event) mechanism, the task scheduling layer ensures that uncompleted tasks can be rescheduled for execution even after a node crash.
