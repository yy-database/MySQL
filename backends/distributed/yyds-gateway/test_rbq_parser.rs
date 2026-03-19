use yyds_gateway::rbq::parser::RbqParser;

fn main() {
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

    println!("Testing valid RBQ statement...");
    match RbqParser::parse_rbq(valid_rbq) {
        Ok(statement) => println!("✅ Parsed successfully: {:?}", statement),
        Err(err) => println!("❌ Parse error: {:?}", err),
    }

    // 测试无效的 RBQ 语句
    let invalid_rbq = r#"
    namespace test {
        struct User {
            id: u32
            name: string;
        }
    }
    "#;

    println!("\nTesting invalid RBQ statement...");
    match RbqParser::parse_rbq(invalid_rbq) {
        Ok(statement) => println!("❌ Should have failed, but parsed: {:?}", statement),
        Err(err) => println!("✅ Correctly caught error: {:?}", err),
    }

    println!("\nRBQ parser test completed!");
}
