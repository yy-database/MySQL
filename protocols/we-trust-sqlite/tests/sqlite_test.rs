use tokio::fs;
use we_trust_sqlite::adapter::SqliteAdapter;

#[tokio::test]
async fn test_full_sqlite_flow() {
    let db_path = "test_integration.db";
    // Ensure clean start
    let _ = fs::remove_file(db_path).await;

    // Create connection using Limbo
    let connection = SqliteAdapter::connect(db_path).await.unwrap();

    // 1. Create Table
    let res = connection
        .execute("CREATE TABLE users (id INT, name TEXT)", &[])
        .await
        .unwrap();
    assert!(res >= 0);

    // 2. Insert Data
    connection
        .execute("INSERT INTO users VALUES (1, 'Alice')", &[])
        .await
        .unwrap();
    connection
        .execute("INSERT INTO users VALUES (2, 'Bob')", &[])
        .await
        .unwrap();

    // 3. Query Data
    let res = connection.query("SELECT * FROM users", &[]).await.unwrap();
    assert_eq!(res.len(), 2);

    // 4. Update Data
    connection
        .execute("UPDATE users SET name = 'Charlie' WHERE id = 1", &[])
        .await
        .unwrap();
    let res = connection
        .query("SELECT * FROM users WHERE id = 1", &[])
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
    connection
        .execute("DELETE FROM users WHERE id = 2", &[])
        .await
        .unwrap();
    let res = connection.query("SELECT * FROM users", &[]).await.unwrap();
    assert_eq!(res.len(), 1);

    // Cleanup
    let _ = fs::remove_file(db_path).await;
}
