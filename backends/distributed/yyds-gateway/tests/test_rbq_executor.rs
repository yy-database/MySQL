use yyds_gateway::rbq::{parser::RbqParser, executor::{YykvRbqExecutor, RbqExecutor}};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // 测试有效的 RBQ 语句
    let valid_rbq = r#"
    namespace test {
        struct User {
            id: u32;
            name: string;
            age: u8;
        }
    }
    "#;

    println!("Testing RBQ statement parsing...");
    match RbqParser::parse_rbq(valid_rbq) {
        Ok(statement) => {
            println!("✅ Parsed successfully: {:?}", statement);
        }
        Err(err) => println!("❌ Parse error: {:?}", err),
    }

    println!("\nRBQ executor test completed!");
}
