use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use yyds_types::{DsValue, DsResult, DsSchema, DsError};
use yyds_loader::{Loader, DsvLoader, JsonlLoader};
use indicatif::{ProgressBar, ProgressStyle};
use futures::StreamExt;
use std::fs;
use clap::ValueEnum;

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ErrorStrategy {
    /// Skip the record and continue
    Skip,
    /// Stop the import process
    Abort,
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RetryStrategy {
    /// Retry the operation
    Retry,
    /// Skip the record and continue
    Skip,
    /// Stop the import process
    Abort,
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
            .map_err(|e| DsError::parse("ImportState", e.to_string()))?;
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
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        pb.set_position(state.offset);

        let mut stream = self.loader.load(path, state.offset);
        let mut count = 0;

        'outer: while let Some(res) = stream.next().await {
            match res {
                Ok((pos, value)) => {
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
                                    continue 'outer;
                                }
                                ErrorStrategy::Abort => {
                                    println!("Aborting import due to schema validation error.");
                                    std::process::exit(1);
                                }
                            }
                        }
                    }

                    // Process value: insert into DB
                    loop {
                        let key = format!("{}:{}", self.table, state.total_records);
                        
                        match self.client.put(&key, validated_value.clone()).await {
                            Ok(_) => break,
                            Err(e) => {
                                pb.suspend(|| {
                                    println!("\nInsertion error at offset {}: {}", pos, e);
                                });

                                match self.insertion_error_strategy {
                                    RetryStrategy::Retry => {
                                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                        continue; // Retry loop
                                    }
                                    RetryStrategy::Skip => {
                                        state.offset = pos;
                                        pb.set_position(pos);
                                        continue 'outer;
                                    }
                                    RetryStrategy::Abort => {
                                        println!("Aborting import due to insertion error.");
                                        std::process::exit(1);
                                    }
                                }
                            }
                        }
                    }

                    state.offset = pos;
                    state.total_records += 1;
                    pb.set_position(pos);

                    // Periodically save state
                    count += 1;
                    if count % 1000 == 0 {
                        if let Some(state_file) = &self.state_file {
                            state.save(state_file)?;
                        }
                    }
                }
                Err(e) => {
                    pb.suspend(|| {
                        println!("\nError reading file at offset {}: {}", state.offset, e);
                    });
                    
                    match self.schema_error_strategy {
                        ErrorStrategy::Skip => {
                            // Cannot easily skip without a valid position, but we can try to continue
                            continue 'outer;
                        }
                        ErrorStrategy::Abort => {
                            println!("Aborting import due to read error.");
                            std::process::exit(1);
                        }
                    }
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
