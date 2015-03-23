# Scheduling System Overview

The YYDS ecosystem builds a multi-level scheduling system, from low-level physical I/O to high-level distributed tasks, ensuring system stability and response speed under high load.

## Scheduling Levels

### 1. I/O Scheduling Layer
Responsible for managing concurrent access and priority preemption for bare-metal devices.
- **Related Components**: `yykv-scheduler`, `yykv-io`, `yykv-hal`
- **Core Goal**: Ensure that frontend queries (High Prio) are not blocked by background maintenance (Low Prio).

### 2. Task Execution Layer
Responsible for task orchestration, distribution, and streaming backpressure control.
- **Related Components**: `yyds-task`, `yykv-executor`, `yykv-stream`
- **Core Goal**: Achieve efficient parallel execution of Near-Data Computing.

### 3. Background Maintenance Layer
Responsible for automated data layout optimization and lifecycle management.
- **Related Components**: `yykv-compaction`, `yykv-tiering`, `yykv-index`
- **Core Goal**: Automatically complete LSM-Tree compaction, cold/hot data migration, and index synchronization.

### 4. Resource Governance Layer
Responsible for quota control and traffic shaping in multi-tenant environments.
- **Related Components**: `yyds-quota`, `yyds-billing`, `yykv-policy`
- **Core Goal**: Achieve physical and logical multi-tenant isolation and fair resource allocation.
