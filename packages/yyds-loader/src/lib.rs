use yyds_types::{DsValue, DsResult, DsError};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use std::collections::BTreeMap;
use futures::stream::{BoxStream, StreamExt};

/// Trait for data loaders.
pub trait Loader: Send + Sync {
    /// Load data from the given path as a stream of DsValue.
    fn load(&self, path: &Path) -> BoxStream<'static, DsResult<DsValue>>;
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
    fn load(&self, path: &Path) -> BoxStream<'static, DsResult<DsValue>> {
        let path = path.to_path_buf();
        let delimiter = self.delimiter;
        let has_header = self.has_header;
        let quote = b'"';
        
        let stream = async_stream::stream! {
            let file = File::open(&path).await
                .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
            let mut reader = BufReader::new(file);
            let mut buffer = [0u8; 8192];
            
            let mut current_record = BTreeMap::new();
            let mut current_field = Vec::new();
            let mut field_index = 0u32;
            let mut in_quote = false;
            let mut last_was_quote = false;
            let mut is_first_record = true;
            
            loop {
                let n = reader.read(&mut buffer).await
                    .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
                
                if n == 0 {
                    // EOF: flush remaining field and record
                    if !current_field.is_empty() || field_index > 0 {
                        if !is_first_record || !has_header {
                            let text = String::from_utf8_lossy(&current_field).to_string();
                            current_record.insert(field_index, DsValue::Text(text));
                            yield Ok(DsValue::Object { schema_id: 0, fields: current_record });
                        }
                    }
                    break;
                }
                
                for &b in &buffer[..n] {
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
                                yield Ok(DsValue::Object { schema_id: 0, fields: record });
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
    fn load(&self, path: &Path) -> BoxStream<'static, DsResult<DsValue>> {
        let path = path.to_path_buf();
        
        let stream = async_stream::stream! {
            let file = File::open(&path).await
                .map_err(|e| DsError::io_raw(e, Some(path.clone())))?;
            let reader = BufReader::new(file);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await
                .map_err(|e| DsError::io_raw(e, Some(path.clone())))? {
                if line.trim().is_empty() {
                    continue;
                }
                let value: serde_json::Value = serde_json::from_str(&line)
                    .map_err(|e| DsError::parse("JSONL", e.to_string()))?;
                yield Ok(json_to_ds_value(value));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_dsv_loader_streaming() -> DsResult<()> {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "id,name").unwrap();
        writeln!(file, "1,Alice").unwrap();
        writeln!(file, "2,\"Bob, the builder\"").unwrap();
        writeln!(file, "3,\"Charlie \"\"Chuck\"\"\"").unwrap();

        let loader = DsvLoader::csv(true);
        let mut stream = loader.load(file.path());

        let mut results = Vec::new();
        while let Some(res) = stream.next().await {
            results.push(res?);
        }

        assert_eq!(results.len(), 3);
        
        if let DsValue::Object { fields, .. } = &results[1] {
            assert_eq!(fields.get(&1).unwrap(), &DsValue::Text("Bob, the builder".to_string()));
        } else {
            panic!("Expected Object");
        }

        if let DsValue::Object { fields, .. } = &results[2] {
            assert_eq!(fields.get(&1).unwrap(), &DsValue::Text("Charlie \"Chuck\"".to_string()));
        } else {
            panic!("Expected Object");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_jsonl_loader_streaming() -> DsResult<()> {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{{\"id\": 1, \"name\": \"Alice\"}}").unwrap();
        writeln!(file, "{{\"id\": 2, \"name\": \"Bob\"}}").unwrap();

        let loader = JsonlLoader;
        let mut stream = loader.load(file.path());

        let mut results = Vec::new();
        while let Some(res) = stream.next().await {
            results.push(res?);
        }

        assert_eq!(results.len(), 2);
        Ok(())
    }
}
