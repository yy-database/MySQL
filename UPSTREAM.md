# 上游复用审计（MySQL）

本表跟着仓走。任何人要在下游新增命令语义、帧格式或存储行为之前，先按下节判断方法
在上游搜一遍；搜到就必须走上游，不得在本仓写第二份实现。

## 判断方法

1. 先搜 `database.server::yyds::mysql`（`connection.rs` / `service.rs` / `wire.rs`）：
   握手、认证、`COM_*`、`OK` / `ERR` / `EOF`、length-encoded、会话命令、SQL 计划分发。
2. 再搜 `database::cluster` / `storage` / `execution` / `query`：`.yyds` catalog、
   `.yykv` 分片、KV executor、schema registry。
3. 命中即复用。只有在两处都搜不到、且能写出「上游为何不承载」的理由时，才允许在本仓新增。

## 上游能力边界（pin `d302887d`）

| 能力 | 上游位置 | 状态 | 本仓动作 |
|---|---|---|---|
| 帧编解码 | `database-protocol-mysql`（`Packet` / `decode_packet` / `encode_packet` / `PROTOCOL_VERSION`） | 完整 | 绑定直连 |
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
| 数据目录装配 | `instance.rs::MysqlInstance`（`publish_routing` / `acquire_lease` / `open_executor` / `open_schema`） | 完整（批次 4 上移） | 绑定直连 |
| `.yyds` catalog | `database.cluster::catalog::Catalog`（`open` / `publish_routing` / `ensure_schema` / `flush`） | 完整 | 复用 |
| `.yykv` 分片 | `database.storage::cluster_kv::FileShard` | 完整（`.yykv`、原子临时文件） | 复用 |
| KV 执行面 | `database.execution::cluster::LocalExecutor` | 完整（实现 `KeyValueExecutor`） | 复用 |
| schema 持久化 | `database.query::SqlSchemaRegistry::open`（`apply_ddl` 自动落盘、`checkpoint` / `restore`） | 完整 | 复用 |
| 端到端真实客户端 | `database.server/tests/mysql/{main,wire,length_encoded,gateway}.rs` | 只有元数据/帧/长度编码测试 | 本仓 harness 补齐 |

## 监听循环裁决（方案 A）

监听循环与数据目录装配都由上游 `database-server` 承担，本仓 napi / pyo3 只导出 handle，
TypeScript 与 Python 只负责端口与命令组合。

选择 A 的理由：方案 B 需要从 `node:net` 的 fd 重建 `TcpStream`，而 Windows 上
`_handle.fd` 不是 socket handle，为字面合规换一层跨平台不稳定适配不划算。A 是宿主调
handle 起上游服务，属「组合原语」，不违反「不自己实现主循环」的本意。本仓 Rust 仍
不导出 bin、不实现主循环、不做万能 CLI。

## 本仓新增（批次 1）

| 能力 | 位置 | 性质 |
|---|---|---|
| 数据目录 bootstrap（catalog 路由、`.yykv` executor、schema、lease） | 上游 `database.server::yyds::mysql::instance`（批次 4 上移） | 组合上游原语，绑定只做转换 |
| Node-API handle | `mysql-napi::server` | 类型转换与错误映射 |
| TS 服务组合 API | `@yydb/mysql` 的 `startMysqlServer` | 顺序组合原语 |
| 安装态验收 | `@yydb/mysql-harness`（cases / fixtures / corpus / stages） | 真实 `mysql2` 客户端 |

## 本仓新增（批次 2）

| 能力 | 位置 | 性质 |
|---|---|---|
| PyO3 handle（lease / executor / schema / service / 路由发布） | `mysql-pyo3::server` | 类型转换与错误映射，无协议字节 |
| Python 服务组合 API | `yydb-mysql` 的 `start_server` | 顺序组合原语 |
| Python 安装态验收 | `mysql-harness/stages/python.mjs` + `stages/python/run.py` | 真实 `pymysql` 客户端，clean venv |

驱动不可用时不降级：`pymysql` 接不上即报 `driver unavailable: <原因>` 并失败，不改成本仓自比。
`mysql-connector-python` 与我们抢同一个顶层 `mysql` 包，不装进验收 venv，如需验证在独立
venv 单跑。

顶层 `mysql` 归属：`yydb-mysql` 提供它，Python 安装器不校验跨发行版同名顶层包，与
`mysql-connector-python` 一类发行版不可共存。`mysql/__init__.py` 在导入期识别冲突并抛
`ImportError` 指明冲突包名与解法，不静默、不吞异常。

## 本仓新增（批次 3）

| 能力 | 位置 | 性质 |
|---|---|---|
| 真库语料抓取（`mysql:8.4`） | `mysql-harness/stages/truth.mjs capture` | 真值来自真实外部引擎，含镜像 tag/digest、`serverVersion`、捕获时间、驱动版本 |
| 语料对账 | `mysql-harness/stages/truth.mjs verify` | 只读语料，本实现输出不写回 |
| `mysql` CLI 探针 | `mysql-harness/stages/truth.mjs` | 真客户端在组合服务上执行 `SELECT 1` |

冻结规则：`capture` 在语料已存在时拒绝覆盖，除非显式设置
`TRUTH_CAPTURE_REFRESH=1`。`Truth corpus` workflow 只上传产物，不写仓。

已冻结语料：`mysql-harness/corpus/truth.json` 由 CI 产物（`Truth corpus` run `42342968788`）
原样提交，provenance 为 `mysql:8.4`（digest `mysql@sha256:a81d043930e51745522bd5c430961cf8286f0a23f0a9a2a6e0108f7ea71b0902`）、
`8.4.13`、`capturedAt 2026-11-07T05:30:37.514Z`、`mysql2 3.24.6`。

## 已删除的 `mysql-core` 两个测试

`reexport_framing_roundtrips_one_payload` 与 `reexport_framing_rejects_oversized_payload`
断言的是 `database-protocol-mysql` 的**再导出契约**。批次 4 删除 `mysql-core` 时它们
随 crate 一起删除，没有搬进绑定假装还在测帧语义：帧语义测试归上游
`database.server/tests/mysql`，绑定层行为由安装态用例覆盖。

## 批次 4：移除 `mysql-core`

- 绑定直连：`mysql-napi` / `mysql-pyo3` 依赖 `database` 与
  `database-protocol-mysql`，`mysql-wasm` 只依赖 `database-protocol-mysql`。
- 上移落点：`database.server::yyds::mysql::instance`（`MysqlInstance`、三个目录常量、
  `publish_routing` / `acquire_lease` / `open_executor` / `open_schema`），错误文案逐字保留。
- `PROTOCOL_VERSION` 只在 `database-protocol-mysql` 定义一次，握手与三个绑定共用。
- 产物不变：同一组操作前后，`catalog.yyds`（48 字节 `9891c43a…`）、`local-0.yykv`
  （94 字节 `6e167cd2…`）、`sql-schema.v0.json`（820 字节 `8f3d4620…`）的 SHA-256 完全一致。

## 升钉点与扩宽语料验收流程（等待上游 SQL 元数据线）

状态：扩宽抓取已生成（7 scenario），按裁决**保持未提交**。只有上游 `database.rs` 的 `dev`
同时具备以下三项后才执行本流程，在此之前不升 pin、不改语料期望。

1. 日期上报：`mysql_field_type` 按 `field_id` 把 DATE / DATETIME 报成
   `MYSQL_TYPE_DATE` / `MYSQL_TYPE_DATETIME`，驱动侧拿到 JS `Date`。
2. `1364`：NOT NULL 且无默认值的列在 `INSERT` 缺列时报 `1364 / HY000`。
3. `Decimal`：精确聚合（`SUM`）按 `DECIMAL` 文本上报，`mysql2` 返回字符串。

### 一次执行

在 `E:\数据库研究\MySQL` 下用 PowerShell 执行。

```powershell
# 1. 取上游已推送的 dev rev，并逐项确认三项能力在该 rev，而不是只看分支名
git -C E:\数据库研究\database.rs fetch origin dev
$rev = (git -C E:\数据库研究\database.rs rev-parse origin/dev).Trim()
git -C E:\数据库研究\database.rs grep -n "MYSQL_TYPE_DATE\|MYSQL_TYPE_DATETIME" $rev -- projects/database/database.server/src/yyds/mysql/connection.rs
git -C E:\数据库研究\database.rs grep -n "1364" $rev -- projects/database
git -C E:\数据库研究\database.rs grep -n "Decimal" $rev -- projects/database/database.server/src/yyds/mysql/connection.rs

# 2. 两处 pin 必须同 rev（Cargo.toml 第 23、28 行）
$old = 'd302887d5c9d6cefa826a37342ee7a3c55f3ea2f'
$file = 'E:\数据库研究\MySQL\projects\mysql.rs\Cargo.toml'
(Get-Content $file -Raw) -replace $old, $rev | Set-Content $file -NoNewline
(Select-String -Path $file -Pattern $rev -SimpleMatch).Count   # 期望 2

# 3. 测量钉点前先把本地 path patch 移开（未跟踪文件，不提交、不删除）
Move-Item -LiteralPath E:\数据库研究\MySQL\projects\mysql.rs\.cargo\config.toml -Destination E:\数据库研究\MySQL\projects\mysql.rs\.cargo\config.toml.off

# 4. 构建（Rust 会按 pin 重新解析 git 依赖）
pnpm run build:napi
pnpm run build:ts

# 5. 三个运行期门禁
node projects/mysql.ts/packages/mysql-harness/stages/truth.mjs verify
node projects/mysql.ts/packages/mysql-harness/stages/install.mjs
node projects/mysql.ts/packages/mysql-harness/stages/python.mjs
```

期望数值（必须逐条相等，不能只判退出码）：

| 门禁 | 期望输出 |
|---|---|
| truth | `79/79 assertions`、`7 scenarios`、`0 skipped`，无 `FAILED` |
| install | `25/25 assertions`、`5 cases`、`0 skipped` |
| python | `25/25 assertions`、`5 cases`、`0 skipped`（pymysql 在 clean venv） |

`79` = 78 条语料断言 + 1 条 `mysql` CLI 探针。CLI 探针缺客户端时整阶段失败
（`mysql cli: unavailable`），不得当作通过。本机客户端位于
`C:\Users\Administrator\Tools\MySQLClient\PFiles64\MySQL\MySQL Server 8.4\bin`，已加入用户
PATH；CI 侧是 `apt install mysql-client` 的 CI-only 步骤。

### 提交与推送

```powershell
# 6. 恢复本地 patch（保持未跟踪）
Move-Item -LiteralPath E:\数据库研究\MySQL\projects\mysql.rs\.cargo\config.toml.off -Destination E:\数据库研究\MySQL\projects\mysql.rs\.cargo\config.toml

# 7. 只暂存这几条显式路径
git -C E:\数据库研究\MySQL add projects/mysql.ts/packages/mysql-harness/corpus/truth.json projects/mysql.rs/Cargo.toml UPSTREAM.md
git -C E:\数据库研究\MySQL commit -m "✅ Freeze the widened mysql:8.4 truth corpus and bump the upstream pin" -m "Pin `database.rs` to the dev revision that reports DATE and DATETIME column types, raises `1364` for missing NOT NULL defaults and returns exact `DECIMAL` aggregates."

# 8. 拉平后复跑再推
git -C E:\数据库研究\MySQL -c http.proxy=socks5h://127.0.0.1:7897 -c https.proxy=socks5h://127.0.0.1:7897 -c http.version=HTTP/1.1 fetch origin dev
git -C E:\数据库研究\MySQL rebase origin/dev
# 复跑第 5 步三条门禁，数值仍须全等
git -C E:\数据库研究\MySQL -c http.proxy=socks5h://127.0.0.1:7897 -c https.proxy=socks5h://127.0.0.1:7897 -c http.version=HTTP/1.1 push origin dev
git -C E:\数据库研究\MySQL rev-parse HEAD
git -C E:\数据库研究\MySQL rev-parse origin/dev   # 必须与 HEAD 相同
```

### 不得做的事

- 不在上游收口前升 pin，不为凑绿改 `truth.json` 期望或删场景。
- 不提交 `projects/mysql.rs/.cargo/config.toml`（它 patch 到本仓 `database.rs` 工作树）。
- 在 `config.toml` 存在时产生的任何本地数字，一律标注「patch 到脏树，不等于 CI 钉点」，
  不写进验收结论。
- `truth.json` 只接受 `capture` 产物，本实现输出不得写回语料。
