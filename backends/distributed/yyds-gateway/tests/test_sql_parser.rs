use oak_sql::parse;

fn main() {
    // Test UPDATE with IN condition
    let sql1 = "UPDATE products SET price = price * 1.1 WHERE category IN ('electronics', 'gadgets')";
    println!("Parsing UPDATE with IN condition:");
    match parse(sql1) {
        Ok(root) => {
            println!("Success: {:?}", root);
            if let Some(stmt) = root.statements.first() {
                match stmt {
                    oak_sql::ast::SqlStatement::Update(update) => {
                        println!("Update table: {:?}", update.table_name.name.name);
                        println!("Update assignments: {:?}", update.assignments);
                        println!("Update selection: {:?}", update.selection);
                    }
                    _ => println!("Not an update statement"),
                }
            }
        }
        Err(e) => println!("Error: {}", e),
    }
    
    // Test DELETE with BETWEEN condition
    let sql2 = "DELETE FROM orders WHERE order_date BETWEEN '2024-01-01' AND '2024-03-31'";
    println!("\nParsing DELETE with BETWEEN condition:");
    match parse(sql2) {
        Ok(root) => {
            println!("Success: {:?}", root);
            if let Some(stmt) = root.statements.first() {
                match stmt {
                    oak_sql::ast::SqlStatement::Delete(delete) => {
                        println!("Delete table: {:?}", delete.table_name.name.name);
                        println!("Delete selection: {:?}", delete.selection);
                    }
                    _ => println!("Not a delete statement"),
                }
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}