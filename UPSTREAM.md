# 上游复用审计（MySQL）

本表跟着仓走。任何人要在下游新增命令语义、帧格式或存储行为之前，先按下节判断方法
在上游搜一遍；搜到就必须走上游，不得在本仓写第二份实现。

## 判断方法

1. 先搜 `database.server::yyds::mysql`（`connection.rs` / `service.rs` / `wire.rs`）：
   握手、认证、`COM_*`、`OK` / `ERR` / `EOF`、length-encoded、会话命令、SQL 计划分发。
2. 再搜 `database::cluster` / `storage` / `execution` / `query`：`.yyds` catalog、
   `.yykv` 分片、KV executor、schema registry。
3. 命中即复用。只有在两处都搜不到、且能写出「上游为何不承载」的理由时，才允许在本仓新增。

## 上游能力边界（pin `647d97a1`）

| 能力 | 上游位置 | 状态 | 本仓动作 |
|---|---|---|---|
| 帧编解码 | `database-protocol-mysql`（`Packet` / `decode_packet` / `encode_packet`） | 完整 | 仅再导出 |
| classic wire 辅助 | `database.server::yyds::mysql::wire` | 完整 | 不接触 |
| 握手 v10 | `connection.rs::serve_session` | 完整（capability / status flags / `mysql_native_password`） | 复用 |
| 认证 | 同上 | loopback + 空密码；空用户或非空密码 → `1045` / `28000` | 复用，本批不扩展 |
| `COM_QUERY` | `connection.rs` | 完整：DDL→`schema.apply_ddl`，SELECT→ResultSet，DML→OK(`AffectedRows=1`) | 复用 |
| `COM_INIT_DB` / `PING` / `QUIT` / `RESET_CONNECTION` | 同上 | 完整 | 复用 |
| `COM_STMT_PREPARE` / `EXECUTE` / `CLOSE` / `RESET` | 同上 | 完整 | 复用 |
| 会话命令 | `SqlSessionCommand`（`SET NAMES`、`autocommit`、事务、savepoint、`SHOW VARIABLES`、`SET @@`、`PRAGMA`、reset/discard） | 完整 | 复用 |
| `OK` / `ERR` / `EOF` | `connection.rs` | 完整（含 SQLSTATE、错误码 1045/1047/1193） | 复用 |
| length-encoded | `connection.rs` + `tests/mysql/length_encoded.rs` | 完整（uint/string、251 边界） | 复用 |
| 监听循环 | `service.rs::MysqlService::{start, start_with_registry, start_with_executor, start_with_executor_lane, address, is_finished, stop}` | 完整（loopback、worker 线程、可停） | 复用（见下节裁决） |
| `.yyds` catalog | `database.cluster::catalog::Catalog`（`open` / `publish_routing` / `ensure_schema` / `flush`） | 完整 | 复用 |
| `.yykv` 分片 | `database.storage::cluster_kv::FileShard` | 完整（`.yykv`、原子临时文件） | 复用 |
| KV 执行面 | `database.execution::cluster::LocalExecutor` | 完整（实现 `KeyValueExecutor`） | 复用 |
| schema 持久化 | `database.query::SqlSchemaRegistry::open`（`apply_ddl` 自动落盘、`checkpoint` / `restore`） | 完整 | 复用 |
| 端到端真实客户端 | `database.server/tests/mysql/{main,wire,length_encoded,gateway}.rs` | 只有元数据/帧/长度编码测试 | 本仓 harness 补齐 |

## 监听循环裁决（方案 A）

监听循环由上游 `database-server` 承担，本仓 napi / pyo3 只导出 handle，TypeScript 与
Python 负责数据目录 bootstrap、端口和命令组合。

选择 A 的理由：方案 B 需要从 `node:net` 的 fd 重建 `TcpStream`，而 Windows 上
`_handle.fd` 不是 socket handle，为字面合规换一层跨平台不稳定适配不划算。A 是宿主调
handle 起上游服务，属「组合原语」，不违反「不自己实现主循环」的本意。本仓 Rust 仍
不导出 bin、不实现主循环、不做万能 CLI。

## 本仓新增（批次 1）

| 能力 | 位置 | 性质 |
|---|---|---|
| 数据目录 bootstrap（catalog 路由、`.yykv` executor、schema、lease） | `mysql-core::server` | 组合上游原语，无协议字节 |
| Node-API handle | `mysql-napi::server` | 类型转换与错误映射 |
| TS 服务组合 API | `@yydb/mysql` 的 `startMysqlServer` | 顺序组合原语 |
| 安装态验收 | `@yydb/mysql-harness`（cases / fixtures / corpus / stages） | 真实 `mysql2` 客户端 |

## 本仓新增（批次 2）

| 能力 | 位置 | 性质 |
|---|---|---|
| PyO3 handle（lease / executor / schema / service / 路由发布） | `mysql-pyo3::server` | 类型转换与错误映射，无协议字节 |
| Python 服务组合 API | `yydb-mysql` 的 `start_server` | 顺序组合原语 |
| Python 安装态验收 | `mysql-harness/stages/python.mjs` + `stages/python/run.py` | 真实 `pymysql` 客户端，clean venv |

驱动不可用时不降级：`pymysql` 接不上先试 `mysql-connector-python`，两者都失败即报
`driver unavailable: <原因>` 并失败，不改成本仓自比。

## 本仓新增（批次 3）

| 能力 | 位置 | 性质 |
|---|---|---|
| 真库语料抓取（`mysql:8.4`） | `mysql-harness/stages/truth.mjs capture` | 真值来自真实外部引擎，含镜像 tag/digest、`serverVersion`、捕获时间、驱动版本 |
| 语料对账 | `mysql-harness/stages/truth.mjs verify` | 只读语料，本实现输出不写回 |
| `mysql` CLI 探针 | `mysql-harness/stages/truth.mjs` | 真客户端在组合服务上执行 `SELECT 1` |

冻结规则：`capture` 在语料已存在时拒绝覆盖，除非显式设置
`TRUTH_CAPTURE_REFRESH=1`。`Truth corpus` workflow 只上传产物，不写仓。

## `mysql-core` 两个既有测试的定位

`reexport_framing_roundtrips_one_payload` 与 `reexport_framing_rejects_oversized_payload`
断言的是 `database-protocol-mysql` 的**再导出契约**：pin 之后绑定面没有静默改变。
它们不构成第二份帧实现，帧语义测试归上游 `database.server/tests/mysql`。两条保留。
