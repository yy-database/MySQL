# 上游复用审计（MySQL）

本表跟着仓走。任何人要在下游新增命令语义、帧格式或存储行为之前，先按下节判断方法
在上游搜一遍；搜到就必须走上游，不得在本仓写第二份实现。

## 判断方法

1. 先搜 `database.server::yyds::mysql`（`connection.rs` / `service.rs` / `wire.rs`）：
   握手、认证、`COM_*`、`OK` / `ERR` / `EOF`、length-encoded、会话命令、SQL 计划分发。
2. 再搜 `database::cluster` / `storage` / `execution` / `query`：`.yyds` catalog、
   `.yykv` 分片、KV executor、schema registry。
3. 命中即复用。只有在两处都搜不到、且能写出「上游为何不承载」的理由时，才允许在本仓新增。

## 上游能力边界（pin `9180088a`）

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

## 升钉与扩宽语料（已收口）

上游 `database.rs` 的三项 SQL 元数据能力落地后，扩宽语料（7 scenario）由 `mysql:8.4`
真库抓取并随 `81b1a83` 冻结提交。`truth` 门禁稳定 `79/79 assertions`、`7 scenarios`、
`0 skipped`，包含 DATE / DATETIME 列类型、NOT NULL 缺列 `1364 / HY000`、`SUM` 精确
`DECIMAL` 文本三项。`79` = 78 条语料断言 + 1 条 `mysql` CLI 探针，CLI 缺客户端时整阶段
失败，不得当作通过。

2026-11-13 的查询性能收口把 pin 抬到 `9180088a`（哈希连接、`IN` 列表集合化、阻塞
`accept`、扫描去复制），`truth` 门禁保持 `79/79`。

当前三档运行期门禁的期望输出（必须逐条相等，不能只判退出码）：

| 门禁 | 期望输出 |
|---|---|
| truth | `79/79 assertions`、`7 scenarios`、`0 skipped`，无 `FAILED` |
| install | `25/25 assertions`、`5 cases`、`0 skipped` |
| python | `25/25 assertions`、`5 cases`、`0 skipped`（pymysql 在 clean venv） |

### 不得做的事

- 不为凑绿改 `truth.json` 期望或删场景。
- 不提交 `projects/mysql.rs/.cargo/config.toml`（它 patch 到本仓 `database.rs` 工作树）。
- 在 `config.toml` 存在时产生的任何本地数字，一律标注「patch 到脏树，不等于 CI 钉点」，
  不写进验收结论。
- `truth.json` 只接受 `capture` 产物，本实现输出不得写回语料。

## 已知缺失 SQL 能力清单

下表登记 SQL 能力缺口及收口状态。1-3 是尚未收口的上游 SQL 能力缺口，本仓没有解析器或
引擎可补。4-6 已由上游 SQL 元数据线补齐，扩宽语料冻结后 `truth` 门禁 `79/79` 通过，
保留在表中作为历史证据。

| # | 缺失能力 | 现象 | 真实 MySQL 行为 | 证据 | 归属 | 当前状态 |
|---|---|---|---|---|---|---|
| 1 | `SELECT ... FOR UPDATE` | 行锁子句不被解析。示例改为在 `BEGIN` 之前读库存、再写计算后的字面量，事务没有锁住读到的行 | 事务内对命中行加排他锁，第二个事务的 `FOR UPDATE` 读阻塞到第一个事务提交或回滚。`FOR SHARE` 加共享锁 | 提交 `73d3e16` 的 `projects/mysql.ts/examples/with-express/readme.md` §Product limitations observed 第 91-92 行。该示例的 `tests/api.mjs` 在绕过该限制后通过 201/400/409/404/204 与库存回滚断言 | 上游 `database.rs`（`database.query` 解析与计划、`database.execution` 锁与事务） | 上游缺失，未排期 |
| 2 | `UPDATE ... SET col = col - 1` 一类表达式自减 | 被拒绝，驱动报 `ER_NOT_SUPPORTED_YET`，文案 `write values must be simple literals`。示例改成先读当前库存、再用字面量写回 | 服务端按行做算术更新，`affectedRows` 按匹配行数计，并发下依赖行锁 | 同上 readme 第 93-95 行。钉点 `d302887d` 的 `database.query/src/sql_planner.rs` 第 1017-1044 行 `literal_from_expression` 只接受 `Literal` 与二进制 `CAST`，`Binary` / `Unary` 一律报 `SQL gateway write values must be simple literals` | 上游 `database.rs`（`database.query::sql_planner`） | 上游缺失，未排期 |
| 3 | `CREATE DATABASE` / `USE` | 两者都不支持。连接参数里的库名会被接受，但服务端只有一个隐式 catalog | `CREATE DATABASE` 建库，`USE` 切换当前库，`COM_INIT_DB` 切换默认库，跨库按库名限定访问 | 同上 readme 第 96-97 行与 `src/server.mjs` 第 32 行（`MYSQL_DATABASE` 传给 pool）。钉点 `connection.rs` 第 410-411 行注释写明 gateway keeps a single catalog，`COM_INIT_DB` 只记录默认 schema。钉点源码无 `CREATE DATABASE` 解析 | 上游 `database.rs`（`database.server::yyds::mysql` 与 `database.cluster` catalog） | 上游缺失，未排期 |
| 4 | DATE / DATETIME 列类型 | 曾把列值以字符串返回，wire 列类型是 `VAR_STRING` | 列类型为 `MYSQL_TYPE_DATE` / `MYSQL_TYPE_DATETIME`，`mysql2` 把值转成 JS `Date` | 已冻结语料 `corpus/truth.json` `types` 场景；上游 `4810b98` 上报列类型，`truth` 门禁 79/79 | 上游 `database.rs` | 已收口 |
| 5 | NOT NULL 无默认值 | 曾把缺列 `INSERT` 静默插入 | 缺列且 NOT NULL 无默认值时拒绝，返回 `errno 1364` / `SQLSTATE HY000`，该行不落库 | 已冻结语料 `errors` 场景的最后一条 `SELECT COUNT(*)`；上游 `plan_insert` 的 `MissingDefault` 守卫，`truth` 门禁 79/79 | 上游 `database.rs` | 已收口 |
| 6 | DECIMAL 类型 | 曾没有 DECIMAL 类型，`SUM(score)` 列类型不是 `NEWDECIMAL` | `SUM` 返回 DECIMAL，`mysql2` 按文本返回 `"80"`，列类型码 `NEWDECIMAL` | 已冻结语料 `aggregates` 场景期望 `"80"`；上游 `SqlFieldType::Decimal` 与 `mysql_field_type` 映射，`truth` 门禁 79/79 | 上游 `database.rs` | 已收口 |

1-3 的证据来自 `73d3e16` 的示例实测，按要求不重跑。六条都不在本仓补实现：本仓只有绑定层与
服务组合，SQL 解析、行锁、约束检查、列类型上报全在上游 `database.rs`。

## benchmark 发现的上游性能与语义缺口

证据来自 `projects/mysql.ts/examples/vs-js-libraries/tests/benchmark-vs-mysql84.mjs`
（真实库 8.4.9，同机、同数据、同语句、5 轮中位数 + p95）。报告
`reports/benchmark-vs-mysql84.md` 由该脚本生成，不手填数字。数字取自 2026-11-13
pin `9180088a` 的 release 构建；历史数字来自同脚本在 pin `00c9556` 的 debug 构建。

| # | 缺口 | 历史现象 | 当前结果 | 证据 | 归属 | 当前状态 |
|---|---|---|---|---|---|---|
| 7 | 每语句 5ms 轮询地板 | `SELECT 1` 6.210ms/语句，真库 0.134ms | `baseline-select1` 0.072ms/语句，真库 0.060ms（1.19x） | `4748841` 把 `CancellableSocket` 改为阻塞读写加短超时 | 上游 `database.rs` | 已收口 |
| 8 | UNIQUE 索引未参与点查 | `WHERE uk = ?` 18.329ms/语句，与全表扫描同量级 | `point-uk` 0.738ms/语句（7.74x），`point-pk` 0.106ms（1.01x），差额 0.63ms 仍是 1000 行扫描 | 报告 `point-uk` 与 `point-pk` 两行与阶段差分表 | 上游 `database.rs`（计划与读取路径） | 未排期，需二级索引或索引下推 |
| 9 | `ORDER BY` / `GROUP BY` 全表扫描 | `order-limit` 23.119ms/语句、`group-by` 19.805ms/语句 | `order-limit` 1.235ms（3.95x）、`group-by` 1.176ms（2.44x），仍是扫描 + 内存排序/分桶 | 报告两行与阶段差分表 | 上游 `database.rs`（计划与聚合） | 未排期，需索引排序或增量聚合 |
| 10 | 整数范围谓词边界错误 | `id >= 50` 返回 55、`id > 6` 返回 43 等 | 语义探针四项与真库一致（41 / 9 / 51 / 94） | `e63b469` 按数值序过滤；报告“语义差异”表 | 上游 `database.rs` | 已收口 |
| 11 | 多行 `INSERT` 每行成本 | `insert-multi` 0.174ms/行，真库 0.094ms/行 | `insert-multi` 0.043ms/行，真库 0.050（0.86x） | 报告 `insert-multi` 行 | 上游 `database.rs` | 已收口 |
| 12 | 连接建立开销 | 16.728ms/连接，真库 1.777ms | `connect` 0.970ms/连接，真库 0.718ms（1.35x） | `9180088a` 的阻塞 `accept` 与自身空连接唤醒；报告 `connect` 行 | 上游 `database.rs` | 已收口 |
| 13 | 等值连接嵌套循环 | `join` 1571.296ms/语句（3623x） | `join` 1.932ms/语句（5.03x） | `9180088a` 的哈希连接与基表谓词前置剪枝；报告 `join` 行 | 上游 `database.rs` | 已收口（剩余为扫描成本） |
| 14 | `IN` 列表逐值比较与栈溢出 | `WHERE id IN (...)` 在 400 项左右耗尽线程栈，subquery 23.426ms/语句 | 900 项列表正常返回，`subquery` 1.486ms/语句（2.12x） | `9180088a` 把等值 `OR` 链折叠回 `InValues`，整数列表归一化后二分查找；报告 `subquery` 行 | 上游 `database.rs` | 已收口 |
| 15 | 扫描逐行复制与无谓解码 | 1000 行扫描 5-8ms | 扫描类 workload 1-2ms，`COUNT(*)` 不再解码行 | `9180088a` 的单分片结果接管、免解码聚合、按字段线性匹配 | 上游 `database.rs` | 已收口 |

剩余差距集中在三处：二级索引点查与范围（8）、排序/分组下推（9）、以及单条写入路径
（`insert-single` 1.68x，差距来自每语句地板与逐行落盘）。这三处都需要上游新增索引结构
或写入批量语义，本仓不越界实现。
