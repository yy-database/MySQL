# Scheduling System Overview

The scheduling system of YYDS (獨一 Distributed Storage) is a multi-layered complex architecture designed to abstract hardware differences and provide extreme I/O throughput, task reliability, and computational efficiency in a distributed environment.

The scheduling system spans the entire YYDS ecosystem, from low-level raw hardware abstraction to high-level distributed consensus.

## Scheduling Layers

Based on different responsibilities and execution granularities, the scheduling system is divided into several core dimensions:

### 1. I/O Scheduling Layer (Low-level I/O)
Focuses on the extreme performance release of single-node hardware.
- Related packages: `yykv-scheduler`, `yykv-io`, `yykv-hal`, `yykv-allocator`

### 2. Task Scheduling Layer (Background Tasks)
Focuses on the reliable execution and persistence of background asynchronous tasks.
- Related packages: `yyds-task`, `yykv-event`, `yykv-wal`

### 3. Data Lifecycle Scheduling (Data Lifecycle)
Focuses on data movement and transformation across different storage media.
- Related packages: `yykv-compaction`, `yykv-tiering`, `yykv-layout`

### 4. Compute & Stream Orchestration (Compute & Stream)
Focuses on the execution location scheduling of compute operators and data stream driving.
- Related packages: `yyds-compute`, `yykv-compute`, `yykv-stream`, `yykv-udf`, `yyds-ai`

### 5. Resource & Quota Scheduling (Resource & Quota)
Focuses on fairness and resource isolation in multi-tenant environments.
- Related packages: `yyds-quota`, `yyds-tenant`, `yyds-billing`, `yyds-observability`

### 6. Distributed Consensus & Coordination (Global Coordination)
Focuses on cross-node task assignment, status synchronization, and global indexing.
- Related packages: `yyds-consensus`, `yyds-pubsub`, `yykv-index`, `yykv-executor`
