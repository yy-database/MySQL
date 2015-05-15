#![warn(missing_docs)]

use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, trace};
use yykv_hal::BlockDevice;
use yykv_types::{DsError, DsResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriority {
    Low = 0,    // Compaction, GC
    Medium = 1, // Standard Queries
    High = 2,   // WAL, Critical Metadata
}

pub enum IoRequest {
    Read {
        offset: u64,
        len: usize,
        priority: IoPriority,
        resp: oneshot::Sender<DsResult<Vec<u8>>>,
    },
    Write {
        offset: u64,
        data: Vec<u8>,
        priority: IoPriority,
        resp: oneshot::Sender<DsResult<usize>>,
    },
}

/// High-performance I/O Scheduler for YYKV.
///
/// This scheduler implements a "Bare-metal" inspired architecture:
/// 1. **Fixed Worker Pool**: Eliminates per-request `tokio::spawn` overhead.
/// 2. **Priority-Aware Dispatcher**: Uses internal queues to ensure high-priority tasks
///    are dispatched immediately even when all workers are busy.
/// 3. **Lock-Free Design**: Leverages MPSC channels for worker push/pull, avoiding
///    mutex contention in the hot path.
/// 4. **Shard-Based Locality**: Partitioning by offset to maintain I/O locality and
///    minimize cache line bouncing.
pub struct IoScheduler {
    shards: Vec<IoShard>,
    shard_mask: u64,
}

struct IoShard {
    high_tx: mpsc::Sender<IoRequest>,
    med_tx: mpsc::Sender<IoRequest>,
    low_tx: mpsc::Sender<IoRequest>,
}

impl IoScheduler {
    pub fn new(
        device: Arc<dyn BlockDevice>,
        concurrency_per_shard: usize,
        shard_count: usize,
    ) -> Self {
        debug!(
            "Initializing IoScheduler with {} shards, {} concurrency per shard",
            shard_count, concurrency_per_shard
        );
        // Ensure shard_count is a power of 2 for easy masking
        let shard_count = shard_count.next_power_of_two();
        let shard_mask = (shard_count - 1) as u64;
        let mut shards = Vec::with_capacity(shard_count);

        for i in 0..shard_count {
            let (high_tx, mut high_rx) = mpsc::channel::<IoRequest>(512);
            let (med_tx, mut med_rx) = mpsc::channel::<IoRequest>(512);
            let (low_tx, mut low_rx) = mpsc::channel::<IoRequest>(512);

            // Queue of available workers
            let (free_worker_tx, mut free_worker_rx) =
                mpsc::channel::<mpsc::Sender<IoRequest>>(concurrency_per_shard);

            let device = device.clone();

            // Pre-spawn worker pool
            for worker_id in 0..concurrency_per_shard {
                let device = device.clone();
                let free_worker_tx = free_worker_tx.clone();
                let (worker_tx, mut worker_rx) = mpsc::channel::<IoRequest>(1);

                // Register initial worker
                free_worker_tx.try_send(worker_tx.clone()).unwrap();

                tokio::spawn(async move {
                    trace!("IoScheduler shard {} worker {} started", i, worker_id);
                    while let Some(req) = worker_rx.recv().await {
                        match req {
                            IoRequest::Read {
                                offset,
                                len,
                                resp,
                                priority,
                                ..
                            } => {
                                trace!(
                                    "Shard {} Worker {} handling Read: offset={} len={} priority={:?}",
                                    i, worker_id, offset, len, priority
                                );
                                let mut buf = vec![0u8; len];
                                let res = device.read_at(offset, &mut buf).await.map(|_| buf);
                                let _ = resp.send(res);
                            }
                            IoRequest::Write {
                                offset,
                                data,
                                resp,
                                priority,
                                ..
                            } => {
                                trace!(
                                    "Shard {} Worker {} handling Write: offset={} len={} priority={:?}",
                                    i,
                                    worker_id,
                                    offset,
                                    data.len(),
                                    priority
                                );
                                let res = device.write_at(offset, &data).await;
                                let _ = resp.send(res);
                            }
                        }
                        // Worker is free again
                        if free_worker_tx.send(worker_tx.clone()).await.is_err() {
                            break;
                        }
                    }
                });
            }

            // Dispatcher task: maintains priority and feeds the worker pool
            tokio::spawn(async move {
                trace!("IoScheduler shard {} dispatcher started", i);
                let mut high_queue = std::collections::VecDeque::new();
                let mut med_queue = std::collections::VecDeque::new();
                let mut low_queue = std::collections::VecDeque::new();

                loop {
                    tokio::select! {
                        biased;
                        // 1. Always try to pull new requests into internal queues
                        Some(req) = high_rx.recv() => high_queue.push_back(req),
                        Some(req) = med_rx.recv() => med_queue.push_back(req),
                        Some(req) = low_rx.recv() => low_queue.push_back(req),

                        // 2. If we have any pending request, try to get a worker and dispatch
                        Some(worker_tx) = free_worker_rx.recv(), if !high_queue.is_empty() || !med_queue.is_empty() || !low_queue.is_empty() => {
                            let req = if let Some(r) = high_queue.pop_front() {
                                r
                            } else if let Some(r) = med_queue.pop_front() {
                                r
                            } else {
                                low_queue.pop_front().unwrap()
                            };

                            if worker_tx.send(req).await.is_err() {
                                break;
                            }
                        }

                        // Shutdown if all channels are closed and queues are empty
                        else => {
                            if high_queue.is_empty() && med_queue.is_empty() && low_queue.is_empty() {
                                break;
                            }
                        }
                    }
                }
            });

            shards.push(IoShard {
                high_tx,
                med_tx,
                low_tx,
            });
        }

        Self { shards, shard_mask }
    }

    pub async fn read_at(
        &self,
        offset: u64,
        len: usize,
        priority: IoPriority,
    ) -> DsResult<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        let req = IoRequest::Read {
            offset,
            len,
            priority,
            resp: tx,
        };
        self.send_request(req, offset, priority).await?;
        rx.await
            .map_err(|e| DsError::internal(format!("IO channel error: {}", e)))?
    }

    pub async fn write_at(
        &self,
        offset: u64,
        data: Vec<u8>,
        priority: IoPriority,
    ) -> DsResult<usize> {
        let (tx, rx) = oneshot::channel();
        let req = IoRequest::Write {
            offset,
            data,
            priority,
            resp: tx,
        };
        let offset_val = req_offset(&req);
        self.send_request(req, offset_val, priority).await?;
        rx.await
            .map_err(|e| DsError::internal(format!("IO channel error: {}", e)))?
    }

    async fn send_request(
        &self,
        req: IoRequest,
        offset: u64,
        priority: IoPriority,
    ) -> DsResult<()> {
        let shard_idx = (offset & self.shard_mask) as usize;
        let shard = &self.shards[shard_idx];

        let tx = match priority {
            IoPriority::High => &shard.high_tx,
            IoPriority::Medium => &shard.med_tx,
            IoPriority::Low => &shard.low_tx,
        };

        tx.send(req)
            .await
            .map_err(|_| DsError::internal("IoScheduler shard channel full/closed"))
    }
}

fn req_offset(req: &IoRequest) -> u64 {
    match req {
        IoRequest::Read { offset, .. } => *offset,
        IoRequest::Write { offset, .. } => *offset,
    }
}
