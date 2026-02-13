# Scheduling System

The YYDS ecosystem includes two levels of schedulers, responsible for low-level I/O priority management and high-level background task orchestration.

## 1. I/O Scheduler (yykv-scheduler)

`yykv-scheduler` is the I/O scheduling center for the YYKV storage engine. It ensures that frontend query requests maintain low latency even under the interference of complex background maintenance tasks (such as Compaction/GC) by categorizing I/O requests by priority, implementing multi-sharding concurrency control, and ensuring fair scheduling.

### Core Features

- **Priority-Driven Scheduling**: Categorizes all I/O requests into three levels:
  - **High**: Critical metadata reads/writes, sequential WAL writes. These requests preempt execution resources to ensure database responsiveness.
  - **Medium**: Standard user SQL queries and KV read/write requests.
  - **Low**: Background Compaction, data Tiering, and Garbage Collection (GC). Executed only when system load is low.
- **I/O Sharding Architecture**: Implements multi-threading sharding based on `PageId` or `Offset` to eliminate single-queue lock contention.
- **Concurrency Control & Backpressure**: Automatically senses queue backlog and passes wait signals upward to prevent NVMe drive queue overflow and memory OOM.

## 2. Task Scheduler (yyds-task)

`yyds-task` is the background task scheduling and management system for the YYDS ecosystem, responsible for handling asynchronous operations and scheduled tasks.

### Core Features

- **Asynchronous Task Orchestration**: Handles time-consuming operations like large-scale data migration and index rebuilding.
- **Distributed Cron Scheduling**: Responsible for periodic billing aggregation, expired data cleanup, and system health checks.
- **WAL Persistence (WAE)**: Tasks are written to the Write-Ahead Log (WAL) upon submission, ensuring they are not lost during node failure and can be rescheduled within the cluster.
- **Status Tracking**: Provides full lifecycle management for tasks, supporting retries, suspensions, and progress monitoring.
