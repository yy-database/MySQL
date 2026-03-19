# RBQ 语言文档

## 概述

RBQ (Resource Binding Query) 是一种现代化的查询语言，旨在提供更简洁、强大的数据查询和处理能力。它与 YYKV 存储引擎深度集成，为开发者提供了一种灵活、高效的方式来操作和查询数据。

## 语法基础

### 命名空间

命名空间用于组织和管理类型定义：

```rbq
namespace user {
    // 类型定义
}
```

### 结构体

结构体用于定义数据模型：

```rbq
struct User {
    id: u32;
    name: string;
    age: u8;
    email: string;
}
```

### 枚举

枚举用于定义一组相关的常量值：

```rbq
enum Status {
    Active,
    Inactive,
    Suspended,
}
```

### 联合类型

联合类型用于表示多种可能的类型：

```rbq
union Result {
    Success(string),
    Error(string),
}
```

### 特质

特质用于定义接口：

```rbq
trait Serializable {
    fn serialize() -> string;
}
```

### 微服务

微服务用于定义服务接口：

```rbq
micro UserService {
    fn get_user(id: u32) -> User;
    fn create_user(user: User) -> u32;
}
```

### 导入

导入用于引用其他命名空间的类型：

```rbq
import user::User;
```

### 类型别名

类型别名用于创建类型的别名：

```rbq
type UserId = u32;
```

## 数据类型

RBQ 支持以下基本数据类型：

- `bool` - 布尔值
- `u8`, `u16`, `u32`, `u64` - 无符号整数
- `i8`, `i16`, `i32`, `i64` - 有符号整数
- `f32`, `f64` - 浮点数
- `string` - 字符串
- `bytes` - 字节数组
- `array<T>` - 数组
- `map<K, V>` - 映射
- `option<T>` - 可选值
- `result<T, E>` - 结果类型

## 操作符

RBQ 支持以下操作符：

- 算术操作符：`+`, `-`, `*`, `/`, `%`
- 比较操作符：`==`, `!=`, `<`, `<=`, `>`, `>=`
- 逻辑操作符：`&&`, `||`, `!`
- 赋值操作符：`=`, `+=`, `-=`, `*=`, `/=`

## 与 YYKV 集成

### 查询数据

使用 RBQ 语句查询 YYKV 中的数据：

```rbq
namespace user {
    struct User {
        id: u32;
        name: string;
        age: u8;
    }
}

// 查询所有用户
query {
    User {
        id,
        name,
        age
    }
}
```

### 修改数据

使用 RBQ 语句修改 YYKV 中的数据：

```rbq
namespace user {
    struct User {
        id: u32;
        name: string;
        age: u8;
    }
}

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

## 示例

### 基本查询

```rbq
// 查询所有用户
query {
    User {
        id,
        name,
        age
    }
}
```

### 条件查询

```rbq
// 查询年龄大于 18 的用户
query {
    User(where: { age > 18 }) {
        id,
        name,
        age
    }
}
```

### 排序查询

```rbq
// 按年龄降序查询用户
query {
    User(orderBy: { age: desc }) {
        id,
        name,
        age
    }
}
```

### 分页查询

```rbq
// 分页查询用户
query {
    User(limit: 10, offset: 0) {
        id,
        name,
        age
    }
}
```

## 最佳实践

1. **命名规范**：使用驼峰命名法命名结构体、枚举和微服务
2. **类型安全**：始终为字段指定明确的类型
3. **模块化**：使用命名空间组织相关的类型定义
4. **可读性**：保持 RBQ 语句简洁明了
5. **错误处理**：使用 result 类型处理错误情况

## 常见问题

### 如何处理类型错误？

RBQ 类型系统会在编译时检查类型错误，如果发现类型不匹配，会返回详细的错误信息。

### 如何优化 RBQ 查询性能？

- 只查询需要的字段
- 使用条件过滤减少返回数据
- 合理使用分页避免一次性返回过多数据
- 为频繁查询的字段创建索引

### 如何与 YYKV 存储引擎集成？

RBQ 语句会被转换为 YYKV 的 OpsGraph 执行计划，然后由 YYKV 执行器执行。开发者可以使用 YykvRbqExecutor 和 YykvRbqStorage 来实现与 YYKV 的集成。
