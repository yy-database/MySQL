use yyds_loader::{DsValue, DsResult, DsvLoader, JsonlLoader};
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
    let mut stream = loader.load(file.path(), 0);

    let mut results = Vec::new();
    while let Some(res) = stream.next().await {
        let (_pos, val) = res?;
        results.push(val);
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
    writeln!(file, "{{\"id\": 1, \"name\": \"Alice\"}}")?;
    writeln!(file, "{{\"id\": 2, \"name\": \"Bob\"}}")?;

    let loader = JsonlLoader;
    let mut stream = loader.load(file.path(), 0);

    let mut results = Vec::new();
    while let Some(res) = stream.next().await {
        let (_pos, val) = res?;
        results.push(val);
    }

    assert_eq!(results.len(), 2);
    Ok(())
}
