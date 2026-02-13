# yyds-server

YYDS 分布式数据库服务器主程序。

## 功能特性

- **集成化核心**: 整合了存储引擎 (`yykv`)、计算引擎 (`yyds-compute`)、共识协议 (`yyds-consensus`) 及执行器 (`yyds-executor`)。
- **多租户支持**: 深度集成 `yyds-tenant`、`yyds-billing` 和 `yyds-quota`，实现租户隔离与资源计费。
- **Trust 协议接入**: 原生支持高性能的 Trust 二进制通信协议。
- **状态管理**: 统一管理全局租户元数据、配额及存储句柄。

## 运行

通过 `cargo run -p yyds-server` 启动服务器。默认监听 127.0.0.1:8888。
