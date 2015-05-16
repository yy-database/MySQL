use oak_sql::parse;

fn main() {
    let sqls = [
        "SELECT * FROM table",
        "SELECT id, name FROM table",
        "SELECT * FROM table WHERE id = 1",
        "SELECT * FROM table ORDER BY id",
        "SELECT * FROM table LIMIT 10",
        "SELECT * FROM table GROUP BY department",
        "SELECT t1.id, t2.name FROM table1 t1 JOIN table2 t2 ON t1.id = t2.id",
    ];
    
    for sql in &sqls {
        println!("\nTesting SQL: {}", sql);
        match parse(sql) {
            Ok(root) => {
                println!("Parsed successfully!");
                println!("Statements: {:?}", root.statements);
            }
            Err(e) => {
                println!("Parse error: {:?}", e);
            }
        }
    }
}
