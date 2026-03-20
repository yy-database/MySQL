# yyds-loader

数据加载器模块，用于从各种数据源加载数据到YYDS系统。

## 功能

- 支持多种数据源的数据加载
- 提供异步加载能力
- 与YYDS系统集成

## 依赖

- oak-dsv
- async-stream
- 其他YYDS组件

## 使用方式

```rust
use yyds_loader::Loader;

// 创建加载器
let loader = Loader::new();

// 加载数据
loader.load_from_source().await;
```