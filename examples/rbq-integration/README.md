# RBQ 与 YYKV 集成示例

本示例展示了如何在 YYKV 项目中集成和使用 RBQ 语言。

## 项目结构

```
rbq-integration/
├── src/
│   ├── main.rs          # 主示例文件
│   └── rbq_example.rs   # RBQ 示例代码
├── Cargo.toml           # 项目配置文件
└── README.md            # 本说明文件
```

## 环境准备

1. 确保 YYKV 项目已经正确配置了 oak-rbq 依赖
2. 确保 YYKV 服务正在运行

## 配置说明

在 `Cargo.toml` 文件中添加以下依赖：

```toml
[dependencies]
yyds-gateway = { path = "../../backends/distributed/yyds-gateway" }
yykv = { path = "../../backends/kv-store/yykv" }
yykv-types = { path = "../../backends/kv-store/yykv-types" }
tokio = { version = "1.49.0", features = ["rt-multi-thread", "macros"] }
uuid = { version = "1.21.0", features = ["v4"] }
```

## 示例代码

### 基本用法

```rust
use yyds_gateway::rbq::{parser::RbqParser, executor::{YykvRbqExecutor, RbqExecutor}};
use yykv_executor::YykvExecutor;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // 解析 RBQ 语句
    let rbq_statement = r#"
    namespace user {
        struct User {
            id: u32;
            name: string;
            age: u8;
        }
    }
    "#;

    match RbqParser::parse_rbq(rbq_statement) {
        Ok(statement) => {
            println!("✅ RBQ 语句解析成功: {:?}", statement);
            
            // 执行 RBQ 语句
            let executor = YykvExecutor::new(/* 参数 */);
            let rbq_executor = YykvRbqExecutor::new(executor);
            let tenant_id = Uuid::new_v4();
            
            match rbq_executor.execute(statement, tenant_id).await {
                Ok(results) => println!("✅ RBQ 语句执行成功: {:?}", results),
                Err(err) => println!("❌ RBQ 语句执行失败: {:?}", err),
            }
        }
        Err(err) => println!("❌ RBQ 语句解析失败: {:?}", err),
    }
}
```

### 与 YYKV 存储引擎集成

```rust
use yyds_gateway::rbq::{parser::RbqParser, storage::{YykvRbqStorage, RbqStorage}};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // 解析 RBQ 语句
    let rbq_statement = r#"
    namespace user {
        struct User {
            id: u32;
            name: string;
            age: u8;
        }
    }
    "#;

    match RbqParser::parse_rbq(rbq_statement) {
        Ok(statement) => {
            println!("✅ RBQ 语句解析成功: {:?}", statement);
            
            // 连接到 YYKV 存储引擎
            let addr = "127.0.0.1:8080";
            let tenant_id = Uuid::new_v4();
            let secret_key = b"secret_key";
            
            match YykvRbqStorage::new(addr, tenant_id, secret_key).await {
                Ok(mut storage) => {
                    // 执行 RBQ 语句
                    match storage.execute_rbq(statement, tenant_id).await {
                        Ok(results) => println!("✅ RBQ 语句执行成功: {:?}", results),
                        Err(err) => println!("❌ RBQ 语句执行失败: {:?}", err),
                    }
                }
                Err(err) => println!("❌ 连接 YYKV 存储引擎失败: {:?}", err),
            }
        }
        Err(err) => println!("❌ RBQ 语句解析失败: {:?}", err),
    }
}
```

## 运行示例

1. 编译示例项目：

```bash
cargo build
```

2. 运行示例：

```bash
cargo run
```

## 常见问题

### 解析错误

如果遇到 RBQ 语句解析错误，请检查：

- RBQ 语句语法是否正确
- 是否缺少分号或大括号
- 类型定义是否正确

### 执行错误

如果遇到 RBQ 语句执行错误，请检查：

- YYKV 服务是否正在运行
- 连接信息是否正确
- 租户 ID 和密钥是否有效

### 性能问题

为了优化 RBQ 语句的执行性能：

- 只查询需要的字段
- 使用条件过滤减少返回数据
- 合理使用分页避免一次性返回过多数据
- 为频繁查询的字段创建索引

## 更多示例

### 查询示例

```rbq
// 查询所有用户
query {
    User {
        id,
        name,
        age
    }
}

// 查询年龄大于 18 的用户
query {
    User(where: { age > 18 }) {
        id,
        name,
        age
    }
}

// 按年龄降序查询用户
query {
    User(orderBy: { age: desc }) {
        id,
        name,
        age
    }
}

// 分页查询用户
query {
    User(limit: 10, offset: 0) {
        id,
        name,
        age
    }
}
```

### 修改示例

```rbq
// 创建用户
mutation {
    createUser(
        id: 1,
        name: "John Doe",
        age: 30
    )
}

// 更新用户
mutation {
    updateUser(
        id: 1,
        name: "John Smith",
        age: 31
    )
}

// 删除用户
mutation {
    deleteUser(id: 1)
}
```

## 最佳实践

1. **命名规范**：使用驼峰命名法命名结构体、枚举和微服务
2. **类型安全**：始终为字段指定明确的类型
3. **模块化**：使用命名空间组织相关的类型定义
4. **可读性**：保持 RBQ 语句简洁明了
5. **错误处理**：使用 result 类型处理错误情况
6. **性能优化**：合理设计 RBQ 语句，避免不必要的数据查询

## 相关文档

- [RBQ 语言文档](../frontends/homepage/documentation/zh-hans/language/index.md)
- [YYKV 官方文档](https://github.com/yydb/yykv)
- [Oak 框架文档](https://github.com/oak-framework/oak)
