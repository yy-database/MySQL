use std::sync::Arc;
use tokio::fs;
use we_trust_sqlite::executor::SqliteExecutor;
use we_trust_sqlite::reader::NativeReader;
use we_trust_sqlite::writer::NativeWriter;

#[tokio::test]
async fn test_full_sqlite_flow() {
    let db_path = "test_integration.db";
    // Ensure clean start
    let _ = fs::remove_file(db_path).await;

    let reader = Arc::new(NativeReader::new(db_path));
    let writer = Arc::new(NativeWriter::new(db_path));
    let executor = SqliteExecutor::new(reader, writer);

    // 1. Create Table
    let res = executor
        .execute_query("CREATE TABLE users (id INT, name TEXT)")
        .await
        .unwrap();
    assert!(format!("{:?}", res[0]).contains("SUCCESS"));

    // 2. Insert Data
    executor
        .execute_query("INSERT INTO users VALUES (1, 'Alice')")
        .await
        .unwrap();
    executor
        .execute_query("INSERT INTO users VALUES (2, 'Bob')")
        .await
        .unwrap();

    // 3. Query Data
    let res = executor.execute_query("SELECT * FROM users").await.unwrap();
    assert_eq!(res.len(), 2);

    // 4. Update Data
    executor
        .execute_query("UPDATE users SET name = 'Charlie' WHERE id = 1")
        .await
        .unwrap();
    let res = executor
        .execute_query("SELECT * FROM users WHERE id = 1")
        .await
        .unwrap();
    // In our implementation, SELECT returns Dict for Source nodes
    if let yykv_types::DsValue::Dict(fields) = &res[0] {
        assert_eq!(
            fields.get("name").unwrap(),
            &yykv_types::DsValue::Text("Charlie".to_string())
        );
    }

    // 5. Delete Data
    executor
        .execute_query("DELETE FROM users WHERE id = 2")
        .await
        .unwrap();
    let res = executor.execute_query("SELECT * FROM users").await.unwrap();
    assert_eq!(res.len(), 1);

    // Cleanup
    let _ = fs::remove_file(db_path).await;
}
