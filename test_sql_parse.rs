use oak_sql::parse;

fn main() {
    // Test UPDATE with IN condition
    let sql1 = "UPDATE products SET price = price * 1.1 WHERE category IN ('electronics', 'gadgets')";
    println!("Parsing UPDATE with IN condition:");
    match parse(sql1) {
        Ok(root) => println!("Success: {:?}", root),
        Err(e) => println!("Error: {}", e),
    }
    
    // Test DELETE with BETWEEN condition
    let sql2 = "DELETE FROM orders WHERE order_date BETWEEN '2024-01-01' AND '2024-03-31'";
    println!("\nParsing DELETE with BETWEEN condition:");
    match parse(sql2) {
        Ok(root) => println!("Success: {:?}", root),
        Err(e) => println!("Error: {}", e),
    }
}