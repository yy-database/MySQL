use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use yyds_types::{DsValue, DsResult, DsSchema, DsError};
use yyds_loader::{Loader, DsvLoader, JsonlLoader};
use indicatif::{ProgressBar, ProgressStyle};
use futures::StreamExt;
use std::fs;
use clap::ValueEnum;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ErrorStrategy {
    /// Skip the record and continue
    Skip,
    /// Stop the import process
    Abort,
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RetryStrategy {
    /// Retry the operation with exponential backoff
    Retry,
    /// Skip the record and continue
    Skip,
    /// Stop the import process
    Abort,
}

#[derive(Debug)]
struct PendingRecord {
    key: String,
    value: DsValue,
    pos: u64,
    retry_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportState {
    pub path: PathBuf,
    pub offset: u64,
    pub total_records: u64,
}

impl ImportState {
    pub fn load(state_file: &Path) -> Option<Self> {
        if state_file.exists() {
            let content = fs::read_to_string(state_file).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    pub fn save(&self, state_file: &Path) -> DsResult<()> {
        let content = serde_json::to_string(self)
            .map_err(|e| DsError::parse(format!("ImportState: {}", e)))?;
        fs::write(state_file, content)
            .map_err(|e| DsError::io_raw(e, Some(state_file.to_path_buf())))?;
        Ok(())
    }
}

pub struct Importer {
    loader: Box<dyn Loader>,
    schema: Option<DsSchema>,
    state_file: Option<PathBuf>,
    client: yyds::Client,
    table: String,
    schema_error_strategy: ErrorStrategy,
    insertion_error_strategy: RetryStrategy,
}

impl Importer {
    pub fn new(
        loader: Box<dyn Loader>,
        schema: Option<DsSchema>,
        state_file: Option<PathBuf>,
        client: yyds::Client,
        table: String,
        schema_error_strategy: ErrorStrategy,
        insertion_error_strategy: RetryStrategy,
    ) -> Self {
        Self {
            loader,
            schema,
            state_file,
            client,
            table,
            schema_error_strategy,
            insertion_error_strategy,
        }
    }

    pub async fn run(&mut self, path: &Path) -> DsResult<()> {
        let mut state = if let Some(state_file) = &self.state_file {
            ImportState::load(state_file).unwrap_or(ImportState {
                path: path.to_path_buf(),
                offset: 0,
                total_records: 0,
            })
        } else {
            ImportState {
                path: path.to_path_buf(),
                offset: 0,
                total_records: 0,
            }
        };

        if state.path != path {
            println!("Warning: State file path mismatch. Starting from offset 0.");
            state.offset = 0;
            state.total_records = 0;
            state.path = path.to_path_buf();
        }

        let file_size = fs::metadata(path)
            .map_err(|e| DsError::io_raw(e, Some(path.to_path_buf())))?
            .len();

        let pb = ProgressBar::new(file_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"));
        pb.set_position(state.offset);

        let mut stream = self.loader.load(path, state.offset);
        let mut count = 0;
        let mut pending_records: VecDeque<PendingRecord> = VecDeque::new();
        const MAX_RETRIES: u32 = 10;
        const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
        const MAX_BACKOFF: Duration = Duration::from_secs(60);
        
        // Circuit breaker logic
        let mut consecutive_failures = 0;
        const FAILURE_THRESHOLD: u32 = 5; // Pause if 5 consecutive records fail
        let mut circuit_open_until: Option<Instant> = None;
        let mut circuit_break_count = 0;
        const INITIAL_CIRCUIT_PAUSE: Duration = Duration::from_secs(5);
        const MAX_CIRCUIT_PAUSE: Duration = Duration::from_secs(300); // Max 5 minutes

        loop {
            // Check if circuit is open
            if let Some(until) = circuit_open_until {
                let now = Instant::now();
                if now < until {
                    let remaining = until.duration_since(now);
                    pb.set_message(format!("(Network Error! Circuit open, backoff: {}s...)", remaining.as_secs()));
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                } else {
                    circuit_open_until = None;
                    // We don't reset consecutive_failures or circuit_break_count yet.
                    // We need a successful insertion to reset them.
                    pb.set_message("(Testing connection...)");
                }
            }

            // 1. Process pending records first
            if !pending_records.is_empty() {
                let record = pending_records.pop_front().unwrap();
                match self.client.put(&record.key, record.value.clone()).await {
                    Ok(_) => {
                        // Success! Reset circuit breaker state
                        consecutive_failures = 0;
                        circuit_break_count = 0;
                        pb.set_message("");
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        if consecutive_failures >= FAILURE_THRESHOLD {
                            circuit_break_count += 1;
                            let pause = INITIAL_CIRCUIT_PAUSE.mul_f32(2.0f32.powi(circuit_break_count as i32 - 1));
                            let pause = pause.min(MAX_CIRCUIT_PAUSE);
                            circuit_open_until = Some(Instant::now() + pause);
                            pb.suspend(|| {
                                println!("\nConsecutive failures threshold reached. Circuit opened for {}s.", pause.as_secs());
                            });
                        }

                        if record.retry_count >= MAX_RETRIES {
                            pb.suspend(|| {
                                println!("\nMax retries reached for record at offset {}: {}", record.pos, e);
                            });
                            match self.insertion_error_strategy {
                                RetryStrategy::Retry | RetryStrategy::Abort => {
                                    println!("Aborting due to max retries.");
                                    std::process::exit(1);
                                }
                                RetryStrategy::Skip => {
                                    // Just drop it
                                }
                            }
                        } else {
                            // Put back to pending for next cycle
                            pending_records.push_back(PendingRecord {
                                retry_count: record.retry_count + 1,
                                ..record
                            });
                        }
                        
                        // If we failed and the circuit is now open, skip to next loop iteration
                        if circuit_open_until.is_some() {
                            continue;
                        }
                    }
                }
            }

            // Update progress bar message with pending count
            if !pending_records.is_empty() {
                pb.set_message(format!("(Pending: {})", pending_records.len()));
            } else {
                pb.set_message("");
            }

            // 2. Read next record from stream
            tokio::select! {
                res = stream.next() => {
                    match res {
                        Some(Ok((pos, value))) => {
                            let mut validated_value = value;
                            
                            // Validate schema if present
                            if let Some(schema) = &self.schema {
                                if let Err(e) = schema.validate(&validated_value) {
                                    pb.suspend(|| {
                                        println!("\nSchema validation error at offset {}: {}", pos, e);
                                    });

                                    match self.schema_error_strategy {
                                        ErrorStrategy::Skip => {
                                            state.offset = pos;
                                            pb.set_position(pos);
                                            continue;
                                        }
                                        ErrorStrategy::Abort => {
                                            println!("Aborting import due to schema validation error.");
                                            std::process::exit(1);
                                        }
                                    }
                                }
                            }

                            // Process value: insert into DB
                            let key = format!("{}:{}", self.table, state.total_records);
                            match self.client.put(&key, validated_value.clone()).await {
                                Ok(_) => {
                                    consecutive_failures = 0;
                                    circuit_break_count = 0;
                                    state.offset = pos;
                                    state.total_records += 1;
                                    pb.set_position(pos);
                                }
                                Err(e) => {
                                    consecutive_failures += 1;
                                    if consecutive_failures >= FAILURE_THRESHOLD {
                                        circuit_break_count += 1;
                                        let pause = INITIAL_CIRCUIT_PAUSE.mul_f32(2.0f32.powi(circuit_break_count as i32 - 1));
                                        let pause = pause.min(MAX_CIRCUIT_PAUSE);
                                        circuit_open_until = Some(Instant::now() + pause);
                                        pb.suspend(|| {
                                            println!("\nConsecutive failures threshold reached. Circuit opened for {}s.", pause.as_secs());
                                        });
                                    }

                                    pb.suspend(|| {
                                        println!("\nInsertion error at offset {}: {}. Adding to pending queue.", pos, e);
                                    });

                                    match self.insertion_error_strategy {
                                        RetryStrategy::Retry => {
                                            pending_records.push_back(PendingRecord {
                                                key,
                                                value: validated_value,
                                                pos,
                                                retry_count: 1,
                                            });
                                            // We count it as attempted, and move offset forward to continue reading
                                            state.offset = pos;
                                            state.total_records += 1;
                                            pb.set_position(pos);
                                        }
                                        RetryStrategy::Skip => {
                                            state.offset = pos;
                                            pb.set_position(pos);
                                        }
                                        RetryStrategy::Abort => {
                                            println!("Aborting import due to insertion error.");
                                            std::process::exit(1);
                                        }
                                    }
                                }
                            }

                            // Periodically save state
                            count += 1;
                            if count % 1000 == 0 {
                                if let Some(state_file) = &self.state_file {
                                    state.save(state_file)?;
                                }
                            }
                        }
                        Some(Err(e)) => {
                            pb.suspend(|| {
                                println!("\nError reading file at offset {}: {}", state.offset, e);
                            });
                            
                            match self.schema_error_strategy {
                                ErrorStrategy::Skip => {
                                    continue;
                                }
                                ErrorStrategy::Abort => {
                                    println!("Aborting import due to read error.");
                                    std::process::exit(1);
                                }
                            }
                        }
                        None => {
                            // Stream ended, but we might still have pending records
                            if pending_records.is_empty() {
                                break;
                            } else {
                                // Wait a bit for pending records to be ready
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)), if !pending_records.is_empty() => {
                    // Just a wake up to check pending records
                }
            }
        }

        pb.finish_with_message("Import completed.");
        if let Some(state_file) = &self.state_file {
            state.save(state_file)?;
        }

        println!("Successfully imported {} records.", state.total_records);
        Ok(())
    }
}
