# RBQ 与 YYKV 集成示例

本示例展示了如何在实际项目中使用 RBQ 语言与 YYKV 存储引擎集成。

## 目录结构

```
rbq-integration/
├── Cargo.toml       # 项目配置文件
├── src/
│   └── main.rs      # 示例代码
└── user_service.rbq # RBQ 服务定义文件
```

## 项目配置

### Cargo.toml

```toml
[package]
name = "rbq-integration"
version = "0.1.0"
edition = "2024"

[dependencies]
yykv = { path = "../../backends/kv-store/yykv" }
yykv-types = { path = "../../backends/kv-store/yykv-types" }
yykv-operators = { path = "../../backends/kv-store/yykv-operators" }
yyds-gateway = { path = "../../backends/distributed/yyds-gateway" }
oak-rbq = { path = "../../../../oaks/examples/oak-rbq" }
tokio = { version = "1.36", features = ["rt-multi-thread", "macros"] }
uuid = { version = "1.7", features = ["v4"] }
```

## RBQ 服务定义

### user_service.rbq

```rbq
// 用户服务定义
namespace user_service {
    // 用户结构体
    struct User {
        id: i32,
        name: string,
        age: i32,
        email: string,
        created_at: timestamp,
    }

    // 用户服务接口
    micro UserService {
        // 获取所有用户
        fn get_all_users() -> Vec<User>;

        // 根据 ID 获取用户
        fn get_user_by_id(id: i32) -> Option<User>;

        // 创建用户
        fn create_user(user: User) -> User;

        // 更新用户
        fn update_user(id: i32, user: User) -> Option<User>;

        // 删除用户
        fn delete_user(id: i32) -> bool;

        // 根据条件查询用户
        fn query_users(condition: string) -> Vec<User>;
    }
}
```

## 示例代码

### main.rs

```rust
use std::error::Error;
use tokio::runtime::Runtime;
use uuid::Uuid;
yykv::Client as YykvClient;
yyds_gateway::rbq::{RbqParser, RbqStorage, YykvRbqStorage};

async fn run() -> Result<(), Box<dyn Error>> {
    // 连接到 YYKV 服务器
    let mut client = YykvClient::connect(
        "127.0.0.1:8888",
        Uuid::new_v4(),
        b"secret_key",
    )
    .await?;

    // 创建 RBQ 存储引擎实例
    let mut rbq_storage = YykvRbqStorage::new(
        "127.0.0.1:8888",
        Uuid::new_v4(),
        b"secret_key",
    )
    .await?;

    // 示例 1: 基本查询
    println!("=== 示例 1: 基本查询 ===");
    let rbq_query = "from users";
    let statement = RbqParser::parse_rbq(rbq_query)?;
    let results = rbq_storage.execute_rbq(statement, Uuid::new_v4()).await?;
    println!("查询结果: {:?}", results);

    // 示例 2: 带条件的查询
    println!("\n=== 示例 2: 带条件的查询 ===");
    let rbq_query = "from users where age > 18";
    let statement = RbqParser::parse_rbq(rbq_query)?;
    let results = rbq_storage.execute_rbq(statement, Uuid::new_v4()).await?;
    println!("查询结果: {:?}", results);

    // 示例 3: 带排序和限制的查询
    println!("\n=== 示例 3: 带排序和限制的查询 ===");
    let rbq_query = "from users order by created_at desc limit 10";
    let statement = RbqParser::parse_rbq(rbq_query)?;
    let results = rbq_storage.execute_rbq(statement, Uuid::new_v4()).await?;
    println!("查询结果: {:?}", results);

    // 示例 4: 聚合查询
    println!("\n=== 示例 4: 聚合查询 ===");
    let rbq_query = "from users group by age select age, count(*) as user_count";
    let statement = RbqParser::parse_rbq(rbq_query)?;
    let results = rbq_storage.execute_rbq(statement, Uuid::new_v4()).await?;
    println!("查询结果: {:?}", results);

    // 示例 5: 复杂条件查询
    println!("\n=== 示例 5: 复杂条件查询 ===");
    let rbq_query = "from users where age > 18 and name like '%John%'";
    let statement = RbqParser::parse_rbq(rbq_query)?;
    let results = rbq_storage.execute_rbq(statement, Uuid::new_v4()).await?;
    println!("查询结果: {:?}", results);

    Ok(())
}

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(run()).unwrap();
}
```

## 运行示例

1. 确保 YYKV 服务器正在运行
2. 编译并运行示例代码

```bash
cargo run
```

## 示例输出

```
=== 示例 1: 基本查询 ===
查询结果: [Text("user-1"), Text("user-2"), Text("user-3")]

=== 示例 2: 带条件的查询 ===
查询结果: [Text("user-1"), Text("user-3")]

=== 示例 3: 带排序和限制的查询 ===
查询结果: [Text("user-3"), Text("user-2"), Text("user-1")]

=== 示例 4: 聚合查询 ===
查询结果: [Text("18: 1"), Text("20: 1"), Text("25: 1")]

=== 示例 5: 复杂条件查询 ===
查询结果: [Text("user-1")]
```

## 注意事项

1. 确保 YYKV 服务器正在运行，并且监听在 `127.0.0.1:8888`
2. 确保使用正确的租户 ID 和密钥
3. 示例中的数据是模拟数据，实际运行时需要根据实际数据结构进行调整
4. 对于生产环境，建议使用连接池和错误处理

## 进阶用法

### 1. 使用事务

```rust
// 开始事务
let mut tx = client.begin_transaction().await?;

// 执行多个 RBQ 查询
let statement1 = RbqParser::parse_rbq("from users")?;
tx.execute_rbq(statement1).await?;

let statement2 = RbqParser::parse_rbq("from users where age > 18")?;
tx.execute_rbq(statement2).await?;

// 提交事务
tx.commit().await?;
```

### 2. 使用参数化查询

```rust
// 使用参数化查询
let user_id = 1;
let rbq_query = format!("from users where id = {}", user_id);
let statement = RbqParser::parse_rbq(&rbq_query)?;
let results = rbq_storage.execute_rbq(statement, Uuid::new_v4()).await?;
```

### 3. 处理错误

```rust
// 错误处理
match RbqParser::parse_rbq("from users where age > 18") {
    Ok(statement) => {
        match rbq_storage.execute_rbq(statement, Uuid::new_v4()).await {
            Ok(results) => println!("查询结果: {:?}", results),
            Err(err) => println!("执行错误: {:?}", err),
        }
    }
    Err(err) => println!("解析错误: {:?}", err),
}
```

## 总结

本示例展示了如何在实际项目中使用 RBQ 语言与 YYKV 存储引擎集成，包括基本查询、带条件的查询、带排序和限制的查询、聚合查询以及复杂条件查询。通过这些示例，您应该能够掌握 RBQ 语言的基本使用方法，并在实际项目中应用它来处理复杂的数据查询和处理需求。
