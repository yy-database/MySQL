#![warn(missing_docs)]

use yyds_types::{DsValue, DsResult, DsError};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use std::collections::BTreeMap;
use futures::stream::{BoxStream, StreamExt};

/// Trait for data loaders.
pub trait Loader: Send + Sync {
    /// Load data from the given path as a stream of DsValue, starting from the given offset.
    fn load(&self, path: &Path, offset: u64) -> BoxStream<'static, DsResult<(u64, DsValue)>>;
}

/// Loader for DSV (Delimiter-Separated Values) format.
pub struct DsvLoader {
    delimiter: u8,
    has_header: bool,
}

impl DsvLoader {
    /// Create a new DSV loader with the given delimiter.
    pub fn new(delimiter: u8, has_header: bool) -> Self {
        Self { delimiter, has_header }
    }

    /// Create a new DSV loader for CSV.
    pub fn csv(has_header: bool) -> Self {
        Self { delimiter: b',', has_header }
    }

    /// Create a new DSV loader for TSV.
    pub fn tsv(has_header: bool) -> Self {
        Self { delimiter: b'\t', has_header }
    }
}

impl Loader for DsvLoader {
    fn load(&self, path: &Path, offset: u64) -> BoxStream<'static, DsResult<(u64, DsValue)>> {
        let path = path.to_path_buf();
        let delimiter = self.delimiter;
        let has_header = self.has_header;
        let quote = b'"';
        
        let stream = async_stream::stream! {
            let mut file = File::open(&path).await
                .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
            
            if offset > 0 {
                use tokio::io::AsyncSeekExt;
                file.seek(std::io::SeekFrom::Start(offset)).await
                    .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
            }

            let mut reader = BufReader::new(file);
            let mut buffer = [0u8; 8192];
            
            let mut current_record = BTreeMap::new();
            let mut current_field = Vec::new();
            let mut field_index = 0u32;
            let mut in_quote = false;
            let mut last_was_quote = false;
            let mut is_first_record = offset == 0;
            let mut current_pos = offset;
            let mut record_start_pos = offset;
            
            loop {
                let n = reader.read(&mut buffer).await
                    .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
                
                if n == 0 {
                    // EOF: flush remaining field and record
                    if !current_field.is_empty() || field_index > 0 {
                        if !is_first_record || !has_header {
                            let text = String::from_utf8_lossy(&current_field).to_string();
                            current_record.insert(field_index, DsValue::Text(text));
                            yield Ok((current_pos, DsValue::Object { schema_id: 0, fields: current_record }));
                        }
                    }
                    break;
                }
                
                for &b in &buffer[..n] {
                    current_pos += 1;
                    if in_quote {
                        if b == quote {
                            if last_was_quote {
                                // Escaped quote: ""
                                current_field.push(quote);
                                last_was_quote = false;
                            } else {
                                last_was_quote = true;
                            }
                        } else {
                            if last_was_quote {
                                // End of quoted field
                                in_quote = false;
                                last_was_quote = false;
                                // Fall through to handle current byte as non-quoted
                            } else {
                                current_field.push(b);
                                continue;
                            }
                        }
                    }
                    
                    if !in_quote {
                        if b == quote {
                            in_quote = true;
                            last_was_quote = false;
                        } else if b == delimiter {
                            let text = String::from_utf8_lossy(&current_field).to_string();
                            current_record.insert(field_index, DsValue::Text(text));
                            current_field.clear();
                            field_index += 1;
                        } else if b == b'\n' || b == b'\r' {
                            if b == b'\r' {
                                continue; 
                            }
                            
                            let text = String::from_utf8_lossy(&current_field).to_string();
                            current_record.insert(field_index, DsValue::Text(text));
                            
                            if !is_first_record || !has_header {
                                let record = std::mem::take(&mut current_record);
                                yield Ok((current_pos, DsValue::Object { schema_id: 0, fields: record }));
                            } else {
                                current_record.clear();
                                is_first_record = false;
                            }
                            
                            current_field.clear();
                            field_index = 0;
                        } else {
                            current_field.push(b);
                        }
                    }
                }
            }
        };
        
        Box::pin(stream)
    }
}

/// Loader for JSONL (JSON Lines) format.
pub struct JsonlLoader;

impl Loader for JsonlLoader {
    fn load(&self, path: &Path, offset: u64) -> BoxStream<'static, DsResult<(u64, DsValue)>> {
        let path = path.to_path_buf();
        
        let stream = async_stream::stream! {
            let mut file = File::open(&path).await
                .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
            
            if offset > 0 {
                use tokio::io::AsyncSeekExt;
                file.seek(std::io::SeekFrom::Start(offset)).await
                    .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
            }

            let mut reader = BufReader::new(file);
            let mut line = String::new();
            let mut current_pos = offset;

            loop {
                line.clear();
                let n = reader.read_line(&mut line).await
                    .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
                
                if n == 0 {
                    break;
                }
                
                current_pos += n as u64;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                
                let value: serde_json::Value = serde_json::from_str(trimmed)
                    .map_err(|e| DsError::parse(format!("JSONL: {}", e)))?;
                yield Ok((current_pos, json_to_ds_value(value)));
            }
        };
        
        Box::pin(stream)
    }
}

fn json_to_ds_value(value: serde_json::Value) -> DsValue {
    match value {
        serde_json::Value::Null => DsValue::Null,
        serde_json::Value::Bool(b) => DsValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                DsValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                DsValue::Float(f)
            } else {
                DsValue::Null
            }
        }
        serde_json::Value::String(s) => DsValue::Text(s),
        serde_json::Value::Array(arr) => DsValue::List(arr.into_iter().map(json_to_ds_value).collect()),
        serde_json::Value::Object(obj) => {
            let mut dict = BTreeMap::new();
            for (k, v) in obj {
                dict.insert(k, json_to_ds_value(v));
            }
            DsValue::Dict(dict)
        }
    }
}


