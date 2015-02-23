use std::sync::Arc;
use tracing::{debug, info};
use yykv_layout::LayoutManager;
use yykv_scheduler::{IoPriority, IoScheduler};
use yykv_types::DsResult;

pub struct Compactor {
    scheduler: Arc<IoScheduler>,
    layout: LayoutManager,
}

impl Compactor {
    pub fn new(scheduler: Arc<IoScheduler>) -> Self {
        Self {
            scheduler,
            layout: LayoutManager::new(),
        }
    }

    /// Perform a basic compaction: read data from multiple sources and write back sequentially.
    /// This is a simplified version for demonstration.
    pub async fn compact(&self, sources: Vec<(u64, usize)>, target_offset: u64) -> DsResult<u64> {
        info!("Starting compaction for {} sources", sources.len());
        let mut current_target = target_offset;

        for (offset, len) in sources {
            debug!("Compacting source at offset {} len {}", offset, len);
            // Read with LOW priority
            let data = self.scheduler.read_at(offset, len, IoPriority::Low).await?;

            // Verify and re-serialize if needed, or just write back
            if let Some(value) = self.layout.deserialize(&data) {
                let serialized = self.layout.serialize(&value);
                // Write with LOW priority
                self.scheduler
                    .write_at(current_target, serialized.to_vec(), IoPriority::Low)
                    .await?;
                current_target += serialized.len() as u64;
            }
        }

        info!(
            "Compaction finished, wrote {} bytes",
            current_target - target_offset
        );
        Ok(current_target - target_offset)
    }
}
