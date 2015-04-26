use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use yyds_types::{DsValue, DsResult, DsSchema, DsError};
use yyds_loader::{Loader, DsvLoader, JsonlLoader};
use indicatif::{ProgressBar, ProgressStyle};
use dialoguer::{Confirm, Select};
use futures::StreamExt;
use std::fs;

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
}

impl Importer {
    pub fn new(
        loader: Box<dyn Loader>,
        schema: Option<DsSchema>,
        state_file: Option<PathBuf>,
        client: yyds::Client,
        table: String,
    ) -> Self {
        Self {
            loader,
            schema,
            state_file,
            client,
            table,
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
                        loop {
                            match schema.validate(&validated_value) {
                                Ok(_) => break,
                                Err(e) => {
                                    let mut action = 0;
                                    pb.suspend(|| {
                                        println!("\nSchema validation error at offset {}: {}", pos, e);
                                        let selections = &["Skip", "Abort", "Fix (Manual - not implemented)"];
                                        action = Select::new()
                                            .with_prompt("What would you like to do?")
                                            .items(selections)
                                            .default(0)
                                            .interact()
                                            .unwrap();
                                    });

                                    match action {
                                        0 => {
                                            // Skip: move to next record
                                            state.offset = pos;
                                            pb.set_position(pos);
                                            continue 'outer;
                                        }
                                        1 => {
                                            println!("Aborting import.");
                                            std::process::exit(1);
                                        }
                                        _ => {
                                            println!("Manual fix not supported yet. Skipping...");
                                            state.offset = pos;
                                            pb.set_position(pos);
                                            continue 'outer;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Process value: insert into DB
                    loop {
                        // Use a key. For now, we could use the offset as key or generate a UUID
                        // Actually, if it's a table, maybe we just append.
                        // YYDS put(key, value) usually requires a key.
                        let key = format!("{}:{}", self.table, state.total_records);
                        
                        match self.client.put(&key, validated_value.clone()).await {
                            Ok(_) => break,
                            Err(e) => {
                                let mut action = 0;
                                pb.suspend(|| {
                                    println!("\nInsertion error at offset {}: {}", pos, e);
                                    let selections = &["Retry", "Skip", "Abort"];
                                    action = Select::new()
                                        .with_prompt("What would you like to do?")
                                        .items(selections)
                                        .default(0)
                                        .interact()
                                        .unwrap();
                                });

                                match action {
                                    0 => continue, // Retry loop
                                    1 => {
                                        state.offset = pos;
                                        pb.set_position(pos);
                                        continue 'outer;
                                    }
                                    _ => {
                                        println!("Aborting import.");
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
                        if !Confirm::new()
                            .with_prompt("Continue with next record?")
                            .interact()
                            .unwrap() {
                            std::process::exit(1);
                        }
                    });
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
