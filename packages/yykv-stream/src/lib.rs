use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tracing::{debug, info};
use yykv_scheduler::{IoPriority, IoScheduler};
use yykv_types::DsResult;

pub struct RecordStream {
    receiver: mpsc::Receiver<DsResult<Vec<u8>>>,
}

impl Stream for RecordStream {
    type Item = DsResult<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

pub struct StreamingReader {
    scheduler: Arc<IoScheduler>,
}

impl StreamingReader {
    pub fn new(scheduler: Arc<IoScheduler>) -> Self {
        Self { scheduler }
    }

    /// Open a stream for reading a range of records.
    /// This implementation uses a background task to prefetch data into a channel.
    pub fn stream_records(
        &self,
        start_offset: u64,
        record_size: usize,
        count: usize,
        prefetch_buffer: usize,
    ) -> RecordStream {
        debug!(
            "Streaming {} records starting at {} (size: {})",
            count, start_offset, record_size
        );
        let (tx, rx) = mpsc::channel(prefetch_buffer);
        let scheduler = self.scheduler.clone();

        tokio::spawn(async move {
            for i in 0..count {
                let offset = start_offset + (i * record_size) as u64;
                let result = scheduler
                    .read_at(offset, record_size, IoPriority::Medium)
                    .await;
                if tx.send(result).await.is_err() {
                    break;
                }
            }
            debug!("Streaming of records finished");
        });

        RecordStream { receiver: rx }
    }

    /// Parallel streaming optimized for AI long context training.
    /// Uses multiple concurrent tasks to prefetch data with High priority.
    /// Ensures ordered delivery of records.
    pub fn stream_parallel_ordered(
        &self,
        start_offset: u64,
        record_size: usize,
        count: usize,
        parallelism: usize,
        prefetch_buffer: usize,
    ) -> RecordStream {
        info!(
            "Parallel streaming {} records with parallelism {} (buffer: {})",
            count, parallelism, prefetch_buffer
        );
        let (tx, rx) = mpsc::channel(prefetch_buffer);
        let scheduler = self.scheduler.clone();

        tokio::spawn(async move {
            let mut next_to_send = 0;
            let mut pending_results = std::collections::HashMap::new();
            let (result_tx, mut result_rx) = mpsc::channel(prefetch_buffer);

            // Spawn fetchers
            for p in 0..parallelism {
                let scheduler = scheduler.clone();
                let result_tx = result_tx.clone();
                tokio::spawn(async move {
                    for i in (p..count).step_by(parallelism) {
                        let offset = start_offset + (i * record_size) as u64;
                        let result = scheduler
                            .read_at(offset, record_size, IoPriority::High)
                            .await;
                        if result_tx.send((i, result)).await.is_err() {
                            break;
                        }
                    }
                });
            }
            drop(result_tx); // Close original tx so rx ends when all fetchers finish

            // Reorder and send
            while let Some((idx, res)) = result_rx.recv().await {
                pending_results.insert(idx, res);
                while let Some(res) = pending_results.remove(&next_to_send) {
                    if tx.send(res).await.is_err() {
                        return;
                    }
                    next_to_send += 1;
                    if next_to_send == count {
                        debug!("Parallel streaming complete");
                        return;
                    }
                }
            }
        });

        RecordStream { receiver: rx }
    }

    /// High-throughput streaming for AI training with zero-copy focus.
    pub fn stream_ai_training(
        &self,
        start_offset: u64,
        record_size: usize,
        count: usize,
    ) -> RecordStream {
        // AI training usually needs high parallelism and large prefetch
        self.stream_parallel_ordered(start_offset, record_size, count, 16, 128)
    }
}
