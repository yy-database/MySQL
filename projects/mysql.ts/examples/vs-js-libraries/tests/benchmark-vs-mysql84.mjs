// 服务端对服务端：本仓 @yydb/mysql 与真实 MySQL 8.4 的同机对比。
//
// 公平口径：
// - 双方都只用 mysql2 的文本协议、单连接、无连接池、无预处理语句、无 TLS，
//   唯一差别是服务端实现；
// - 固定建表语句、固定种子数据、固定语句集合，真实库与本仓各跑一遍；
// - 本仓每轮测量都新起一个临时数据目录的服务端，建表与灌种子不计时；
// - 真实库沿用 truth corpus 口径（root / yydb-truth / 库 yydb），只重建表，不重启实例；
// - 每项先跑一轮热身，再跑 BENCH_ROUNDS 轮，取中位数并记 p95。
//
// 真实库连接：
//   BENCH_REAL_HOST（默认 127.0.0.1）
//   BENCH_REAL_PORT（默认 33306）
//   BENCH_REAL_USER（默认 root）
//   BENCH_REAL_PASSWORD（默认 yydb-truth）
//   BENCH_REAL_DATABASE（默认 yydb）
// 真实库不可达时直接失败，不降级为本仓自比。
//
// 运行：`pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark:mysql84`

import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import mysql from "mysql2/promise";

import { elapsedMs, median, p95 } from "../src/timing.mjs";
import { startMysqlServer } from "../../../packages/mysql/dist/index.js";

const exampleRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

const REAL = {
  host: process.env.BENCH_REAL_HOST ?? "127.0.0.1",
  port: Number(process.env.BENCH_REAL_PORT ?? 33_306),
  user: process.env.BENCH_REAL_USER ?? "root",
  password: process.env.BENCH_REAL_PASSWORD ?? "yydb-truth",
  database: process.env.BENCH_REAL_DATABASE ?? "yydb",
};

const ROUNDS = Number(process.env.BENCH_ROUNDS ?? 5);
const DATASET = Number(process.env.BENCH_DATASET ?? 1_000);
const INSERT_SINGLE = Number(process.env.BENCH_INSERT ?? 300);
const MULTI_BATCH = Number(process.env.BENCH_MULTI_BATCH ?? 100);
const MULTI_STATEMENTS = Number(process.env.BENCH_MULTI_STATEMENTS ?? 4);
const POINT_OPS = Number(process.env.BENCH_POINT ?? 300);
const RANGE_OPS = Number(process.env.BENCH_RANGE ?? 100);
const UPDATE_STATEMENTS = Number(process.env.BENCH_UPDATE ?? 20);
const DELETE_STATEMENTS = Number(process.env.BENCH_DELETE ?? 10);
const ORDER_OPS = Number(process.env.BENCH_ORDER ?? 50);
const GROUP_OPS = Number(process.env.BENCH_GROUP ?? 50);
const TX_COUNT = Number(process.env.BENCH_TX ?? 30);
const TX_ROWS = Number(process.env.BENCH_TX_ROWS ?? 10);
const CONNECT_OPS = Number(process.env.BENCH_CONNECT ?? 20);
const BASELINE_OPS = Number(process.env.BENCH_BASELINE ?? 300);

// 所有引擎共用一张表：主键、唯一索引、分组列、值列、填充列。
const CREATE_TABLE =
  "CREATE TABLE bench (id INT NOT NULL PRIMARY KEY, uk INT UNIQUE, grp INT, val INT, pad VARCHAR(40))";

const GROUP_SIZE = Math.floor(DATASET / 10);
// 主键与唯一键统一用四位整数，避免本仓“按字符串比较”的缺陷干扰吞吐测量。
// 该缺陷单独在语义探针里记录，不混进性能比值。
const ID_BASE = 1_000;

const OURS_OPTIONS = { host: "127.0.0.1", user: "yydb", password: "" };

function seedStatement(startId) {
  // 固定 seed：第 k 行的 uk = k，grp = (k-ID_BASE)/GROUP_SIZE，val = 确定性余数。
  const rows = [];
  for (let id = startId; id < startId + MULTI_BATCH; id += 1) {
    const uk = id;
    const grp = Math.floor((id - ID_BASE) / GROUP_SIZE);
    const val = (id * 137) % 997;
    rows.push(`(${id}, ${uk}, ${grp}, ${val}, 'row-${id}')`);
  }
  return `INSERT INTO bench (id, uk, grp, val, pad) VALUES ${rows.join(", ")}`;
}

async function seed(connection, statements) {
  await connection.query("DROP TABLE IF EXISTS bench");
  await connection.query(CREATE_TABLE);
  for (let statement = 0; statement < statements; statement += 1) {
    await connection.query(seedStatement(ID_BASE + statement * MULTI_BATCH));
  }
  const remainder = DATASET - statements * MULTI_BATCH;
  if (remainder > 0) {
    await connection.query(`INSERT INTO bench (id, uk, grp, val, pad) VALUES ${Array.from(
      { length: remainder },
      (_, index) => {
        const id = ID_BASE + statements * MULTI_BATCH + index;
        const grp = Math.floor((id - ID_BASE) / GROUP_SIZE);
        const val = (id * 137) % 997;
        return `(${id}, ${id}, ${grp}, ${val}, 'row-${id}')`;
      },
    ).join(", ")}`);
  }
}

function datasetStatements() {
  return Math.floor(DATASET / MULTI_BATCH);
}

const WORKLOADS = [
  {
    name: "insert-single",
    unit: "statements",
    ops: INSERT_SINGLE,
    async setup(connection) {
      await connection.query("DROP TABLE IF EXISTS bench");
      await connection.query(CREATE_TABLE);
    },
    async run(connection) {
      for (let offset = 0; offset < INSERT_SINGLE; offset += 1) {
        const id = ID_BASE + offset;
        const grp = Math.floor(offset / GROUP_SIZE);
        const val = (id * 137) % 997;
        await connection.query(
          `INSERT INTO bench (id, uk, grp, val, pad) VALUES (${id}, ${id}, ${grp}, ${val}, 'row-${id}')`,
        );
      }
      return INSERT_SINGLE;
    },
  },
  {
    name: "insert-multi",
    unit: "rows",
    ops: MULTI_BATCH * MULTI_STATEMENTS,
    async setup(connection) {
      await connection.query("DROP TABLE IF EXISTS bench");
      await connection.query(CREATE_TABLE);
    },
    async run(connection) {
      for (let statement = 0; statement < MULTI_STATEMENTS; statement += 1) {
        await connection.query(seedStatement(ID_BASE + statement * MULTI_BATCH));
      }
      return MULTI_BATCH * MULTI_STATEMENTS;
    },
  },
  {
    name: "point-pk",
    unit: "statements",
    ops: POINT_OPS,
    async setup(connection) {
      await seed(connection, datasetStatements());
    },
    async run(connection) {
      for (let index = 0; index < POINT_OPS; index += 1) {
        const id = ID_BASE + (index % DATASET);
        const [rows] = await connection.query(`SELECT id, val FROM bench WHERE id = ${id}`);
        if (rows.length !== 1) throw new Error(`point-pk miss id=${id}`);
      }
      return POINT_OPS;
    },
  },
  {
    name: "point-uk",
    unit: "statements",
    ops: POINT_OPS,
    async setup(connection) {
      await seed(connection, datasetStatements());
    },
    async run(connection) {
      for (let index = 0; index < POINT_OPS; index += 1) {
        const uk = ID_BASE + (index % DATASET);
        const [rows] = await connection.query(`SELECT id, val FROM bench WHERE uk = ${uk}`);
        if (rows.length !== 1) throw new Error(`point-uk miss uk=${uk}`);
      }
      return POINT_OPS;
    },
  },
  {
    name: "range-limit",
    unit: "statements",
    ops: RANGE_OPS,
    async setup(connection) {
      await seed(connection, datasetStatements());
    },
    async run(connection) {
      // 本仓 `BETWEEN` 与下界含等的范围谓词在边界值上会漏行，这里改用
      // `> from AND <= from+20`，两库都能稳定取回 20 行，保证耗时可比。
      const span = Math.floor((DATASET - 20) / RANGE_OPS);
      for (let index = 0; index < RANGE_OPS; index += 1) {
        const from = ID_BASE + index * span;
        const [rows] = await connection.query(
          `SELECT id, val FROM bench WHERE id > ${from} AND id <= ${from + 20}`,
        );
        if (rows.length !== 20) throw new Error(`range-limit rows=${rows.length} from=${from}`);
      }
      return RANGE_OPS;
    },
  },
  {
    name: "update-many",
    unit: "rows",
    ops: UPDATE_STATEMENTS * GROUP_SIZE,
    async setup(connection) {
      await seed(connection, datasetStatements());
    },
    async run(connection) {
      let matched = 0;
      for (let grp = 0; grp < UPDATE_STATEMENTS; grp += 1) {
        const [result] = await connection.query(
          `UPDATE bench SET val = ${1000 + grp} WHERE grp = ${grp % 10}`,
        );
        matched += result.affectedRows ?? 0;
      }
      return matched;
    },
  },
  {
    name: "delete-many",
    unit: "rows",
    ops: DELETE_STATEMENTS * GROUP_SIZE,
    async setup(connection) {
      await seed(connection, datasetStatements());
    },
    async run(connection) {
      let deleted = 0;
      for (let grp = 0; grp < DELETE_STATEMENTS; grp += 1) {
        const [result] = await connection.query(`DELETE FROM bench WHERE grp = ${grp % 10}`);
        deleted += result.affectedRows ?? 0;
      }
      return deleted;
    },
  },
  {
    name: "order-limit",
    unit: "statements",
    ops: ORDER_OPS,
    async setup(connection) {
      await seed(connection, datasetStatements());
    },
    async run(connection) {
      for (let index = 0; index < ORDER_OPS; index += 1) {
        const [rows] = await connection.query(
          "SELECT id, val FROM bench ORDER BY val DESC, id LIMIT 20",
        );
        if (rows.length !== 20) throw new Error(`order-limit rows=${rows.length}`);
      }
      return ORDER_OPS;
    },
  },
  {
    name: "group-by",
    unit: "statements",
    ops: GROUP_OPS,
    async setup(connection) {
      await seed(connection, datasetStatements());
    },
    async run(connection) {
      for (let index = 0; index < GROUP_OPS; index += 1) {
        const [rows] = await connection.query(
          "SELECT grp, COUNT(*) AS n, SUM(val) AS s FROM bench GROUP BY grp ORDER BY grp",
        );
        if (rows.length !== 10) throw new Error(`group-by groups=${rows.length}`);
      }
      return GROUP_OPS;
    },
  },
  {
    name: "tx-commit",
    unit: "rows",
    ops: TX_COUNT * TX_ROWS,
    async setup(connection) {
      await connection.query("DROP TABLE IF EXISTS bench");
      await connection.query(CREATE_TABLE);
    },
    async run(connection) {
      let offset = 0;
      for (let tx = 0; tx < TX_COUNT; tx += 1) {
        await connection.query("BEGIN");
        for (let row = 0; row < TX_ROWS; row += 1) {
          const id = ID_BASE + offset;
          const grp = Math.floor(offset / GROUP_SIZE);
          const val = (id * 137) % 997;
          await connection.query(
            `INSERT INTO bench (id, uk, grp, val, pad) VALUES (${id}, ${id}, ${grp}, ${val}, 'row-${id}')`,
          );
          offset += 1;
        }
        await connection.query("COMMIT");
      }
      return TX_COUNT * TX_ROWS;
    },
  },
  {
    name: "baseline-select1",
    unit: "statements",
    ops: BASELINE_OPS,
    async setup(connection) {
      await connection.query("SELECT 1");
    },
    async run(connection) {
      for (let index = 0; index < BASELINE_OPS; index += 1) {
        await connection.query("SELECT 1");
      }
      return BASELINE_OPS;
    },
  },
  {
    name: "connect",
    unit: "connections",
    ops: CONNECT_OPS,
    async setup() {},
    async run(_connection, open) {
      for (let index = 0; index < CONNECT_OPS; index += 1) {
        const connection = await open();
        await connection.query("SELECT 1");
        await connection.end();
      }
      return CONNECT_OPS;
    },
  },
];

async function openReal() {
  return mysql.createConnection({
    host: REAL.host,
    port: REAL.port,
    user: REAL.user,
    password: REAL.password,
    database: REAL.database,
    disableEval: true,
  });
}

async function openOurs(port) {
  return mysql.createConnection({
    host: OURS_OPTIONS.host,
    port,
    user: OURS_OPTIONS.user,
    password: OURS_OPTIONS.password,
    disableEval: true,
  });
}

// 语义探针：只记录整数范围谓词是否与真库一致，不参与性能比值。
// 小整数（1..100）在真库上按数值比较，本仓在边界上会分叉。
async function probeSemantics() {
  const insert = `INSERT INTO semantics VALUES ${Array.from({ length: 100 }, (_, index) => `(${index + 1})`).join(", ")}`;
  const queries = {
    between: "SELECT COUNT(*) AS n FROM semantics WHERE id BETWEEN 6 AND 46",
    betweenSmall: "SELECT COUNT(*) AS n FROM semantics WHERE id BETWEEN 2 AND 10",
    lowerBound: "SELECT COUNT(*) AS n FROM semantics WHERE id >= 50",
    greater: "SELECT COUNT(*) AS n FROM semantics WHERE id > 6",
  };

  async function run(connection) {
    const result = {};
    for (const [key, sql] of Object.entries(queries)) {
      const [rows] = await connection.query(sql);
      result[key] = rows[0].n;
    }
    return result;
  }

  const realConnection = await openReal();
  let real;
  try {
    await realConnection.query("DROP TABLE IF EXISTS semantics");
    await realConnection.query("CREATE TABLE semantics (id INT NOT NULL PRIMARY KEY)");
    await realConnection.query(insert);
    real = await run(realConnection);
  } finally {
    await realConnection.query("DROP TABLE IF EXISTS semantics").catch(() => {});
    await realConnection.end().catch(() => {});
  }

  const dataDir = await mkdtemp(join(tmpdir(), "mysql-semantics-"));
  const server = startMysqlServer({ dataDir, port: 0 });
  let ours;
  try {
    const connection = await openOurs(server.port);
    try {
      await connection.query("CREATE TABLE semantics (id INT NOT NULL PRIMARY KEY)");
      await connection.query(insert);
      ours = await run(connection);
    } finally {
      await connection.end().catch(() => {});
    }
  } finally {
    server.stop();
    await rm(dataDir, { recursive: true, force: true }).catch(() => {});
  }

  return { real, ours };
}

async function measureOurs(workload) {
  const dataDir = await mkdtemp(join(tmpdir(), "mysql-bench84-"));
  const server = startMysqlServer({ dataDir, port: 0 });
  let connection;
  try {
    connection = await openOurs(server.port);
    await workload.setup(connection);
    const start = process.hrtime.bigint();
    const ops = await workload.run(connection, () => openOurs(server.port));
    return { ms: elapsedMs(start), ops };
  } finally {
    // 先停服再清目录，避免 Windows 上句柄未释放导致删除失败。
    if (connection !== undefined) await connection.end().catch(() => {});
    server.stop();
    await rm(dataDir, { recursive: true, force: true }).catch(() => {});
  }
}

async function measureReal(workload) {
  const connection = await openReal();
  try {
    await workload.setup(connection);
    const start = process.hrtime.bigint();
    const ops = await workload.run(connection, openReal);
    return { ms: elapsedMs(start), ops };
  } finally {
    await connection.end().catch(() => {});
  }
}

function ratio(subject, baseline) {
  return baseline > 0 ? subject / baseline : Number.NaN;
}

function markdownReport(samples, version, semantics) {
  const lines = [];
  lines.push("# 服务端对服务端：`@yydb/mysql` 与真实 MySQL 8.4");
  lines.push("");
  lines.push(`真实库：\`${version}\`（\`${REAL.host}:${REAL.port}\`，库 \`${REAL.database}\`）`);
  lines.push(`本仓：\`@yydb/mysql\`（每轮新起临时数据目录）`);
  lines.push(
    "说明：本机没有 Docker，真实库用本地 MySQL 8.4.9；truth corpus 钉的是 8.4.13。CI 有 `mysql:8.4` 镜像时用 `BENCH_REAL_*` 指向该实例重跑。",
  );
  lines.push(
    `轮次：1 轮热身 + ${ROUNDS} 轮测量，取中位数并记 p95；数据集 ${DATASET} 行，固定 seed。`,
  );
  lines.push("比值 = 本仓耗时 / 真实库耗时，小于 1 表示本仓更快。");
  lines.push("");
  lines.push("## 按差距排序");
  lines.push("");
  lines.push("| 工作负载 | 单位 | 真实库 ms/op | 本仓 ms/op | 倍数 |");
  lines.push("|---------:|-----:|-------------:|-----------:|-----:|");
  const ranked = WORKLOADS.map((workload) => {
    const real = median(samples.get(workload.name).real);
    const ours = median(samples.get(workload.name).ours);
    return { workload, real, ours, factor: ratio(ours, real) };
  }).sort((a, b) => b.factor - a.factor);
  for (const entry of ranked) {
    lines.push(
      `| ${entry.workload.name} | ${entry.workload.unit} | ${(entry.real / entry.workload.ops).toFixed(3)} | ${(entry.ours / entry.workload.ops).toFixed(3)} | ${entry.factor.toFixed(2)}x |`,
    );
  }
  lines.push("");
  lines.push("## 绝对耗时与 p95");
  lines.push("");
  lines.push("| 工作负载 | 真实库中位总耗时 ms | 本仓中位总耗时 ms | 真实库 p95 ms | 本仓 p95 ms |");
  lines.push("|---------:|--------------------:|------------------:|--------------:|------------:|");
  for (const workload of WORKLOADS) {
    const real = median(samples.get(workload.name).real);
    const ours = median(samples.get(workload.name).ours);
    lines.push(
      `| ${workload.name} | ${real.toFixed(2)} | ${ours.toFixed(2)} | ${p95(samples.get(workload.name).real).toFixed(2)} | ${p95(samples.get(workload.name).ours).toFixed(2)} |`,
    );
  }
  lines.push("");
  lines.push("## 粗粒度阶段差分");
  lines.push("");
  lines.push(
    "线上协议不暴露解析/执行/存储/回包各阶段耗时，这里只做差分估计：以 `baseline-select1` 作为“传输 + 解析 + 回包”地板，其余项减去地板即该路径的额外成本。",
  );
  lines.push("");
  lines.push("| 路径 | 真实库额外 ms/语句 | 本仓额外 ms/语句 | 差额 ms/语句 |");
  lines.push("|-----:|-------------------:|-----------------:|-------------:|");
  const floorReal = median(samples.get("baseline-select1").real) / BASELINE_OPS;
  const floorOurs = median(samples.get("baseline-select1").ours) / BASELINE_OPS;
  // 每项按“语句数”折算，写多行的项不让行数稀释掉单语句地板。
  const statements = {
    "insert-single": INSERT_SINGLE,
    "point-pk": POINT_OPS,
    "point-uk": POINT_OPS,
    "range-limit": RANGE_OPS,
    "update-many": UPDATE_STATEMENTS,
    "group-by": GROUP_OPS,
    "order-limit": ORDER_OPS,
  };
  for (const [name, count] of Object.entries(statements)) {
    const real = median(samples.get(name).real) / count - floorReal;
    const ours = median(samples.get(name).ours) / count - floorOurs;
    lines.push(`| ${name} | ${real.toFixed(3)} | ${ours.toFixed(3)} | ${(ours - real).toFixed(3)} |`);
  }
  lines.push("");
  lines.push("## 根因假设与预计收益");
  lines.push("");
  const pkOurs = median(samples.get("point-pk").ours) / POINT_OPS;
  const ukOurs = median(samples.get("point-uk").ours) / POINT_OPS;
  const orderOurs = median(samples.get("order-limit").ours) / ORDER_OPS;
  const groupOurs = median(samples.get("group-by").ours) / GROUP_OPS;
  const singleOurs = median(samples.get("insert-single").ours) / INSERT_SINGLE;
  const singleReal = median(samples.get("insert-single").real) / INSERT_SINGLE;
  const multiOurs = median(samples.get("insert-multi").ours) / (MULTI_BATCH * MULTI_STATEMENTS);
  const multiReal = median(samples.get("insert-multi").real) / (MULTI_BATCH * MULTI_STATEMENTS);
  lines.push(
    `- **每语句约 ${floorOurs.toFixed(1)}ms 固定地板**：上游 \`database.server::yyds::mysql::connection\` 的 \`CancellableSocket::attempt\` 在 \`WouldBlock\` 时固定 \`sleep(5ms)\`，每个 \`COM_QUERY\` 至少命中一次。\`baseline-select1\` 的 ${floorOurs.toFixed(3)}ms/语句就是这个地板（真库 ${floorReal.toFixed(3)}ms）。修掉后 SELECT 1 预计降到 0.3ms 以内，point-pk、range-limit、connect 同步下降约 5ms/语句。`,
  );
  lines.push(
    `- **唯一索引未参与点查**：\`point-uk\` ${ukOurs.toFixed(3)}ms/语句，比 \`point-pk\` 的 ${pkOurs.toFixed(3)}ms 多 ${(ukOurs - pkOurs).toFixed(1)}ms，与全表 ${DATASET} 行扫描同量级，说明 \`WHERE uk = ?\` 走的是扫描而不是唯一索引定位。补上索引定位后预计回到与 \`point-pk\` 同档。`,
  );
  lines.push(
    `- **排序与分组全表扫描**：\`order-limit\` ${orderOurs.toFixed(2)}ms/语句、\`group-by\` ${groupOurs.toFixed(2)}ms/语句，各比地板多约 ${(Math.min(orderOurs, groupOurs) - floorOurs).toFixed(1)}-${(Math.max(orderOurs, groupOurs) - floorOurs).toFixed(1)}ms，等于 ${DATASET} 行扫描成本。索引下推与增量聚合后预计降到亚毫秒级。`,
  );
  lines.push(
    `- **单条写入不慢**：\`insert-single\` 本仓 ${singleOurs.toFixed(2)}ms/语句、真库 ${singleReal.toFixed(2)}ms/语句，差距来自每语句地板与多行插入的逐行落盘（\`insert-multi\` ${multiOurs.toFixed(3)} vs ${multiReal.toFixed(3)}ms/行，${(multiOurs / multiReal).toFixed(2)}x）。批量路径改为一次提交可把每行成本压到真库同档。`,
  );
  lines.push("");
  lines.push("## 语义差异（不计入性能比值）");
  lines.push("");
  lines.push("整数列在 1..100 的小值区间上对比 `COUNT(*)`，真库按数值比较，本仓在边界上分叉：");
  lines.push("");
  lines.push("| 语句 | 真实库计数 | 本仓计数 |");
  lines.push("|-----:|-----------:|--------:|");
  lines.push(`| \`WHERE id BETWEEN 6 AND 46\` | ${semantics.real.between} | ${semantics.ours.between} |`);
  lines.push(`| \`WHERE id BETWEEN 2 AND 10\` | ${semantics.real.betweenSmall} | ${semantics.ours.betweenSmall} |`);
  lines.push(`| \`WHERE id >= 50\` | ${semantics.real.lowerBound} | ${semantics.ours.lowerBound} |`);
  lines.push(`| \`WHERE id > 6\` | ${semantics.real.greater} | ${semantics.ours.greater} |`);
  lines.push("");
  lines.push("Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark:mysql84`");
  lines.push("");
  return lines.join("\n");
}

async function main() {
  // 真实库先探活：不可达就失败，绝不退回本仓自比。
  const probe = await openReal();
  const [[{ version }]] = await probe.query("SELECT VERSION() AS version");
  await probe.end();
  console.log(`真实库 ${version} @ ${REAL.host}:${REAL.port}/${REAL.database}`);
  const semantics = await probeSemantics();

  const samples = new Map();
  for (const workload of WORKLOADS) samples.set(workload.name, { real: [], ours: [] });

  console.log("热身一轮（不计入样本）");
  for (const workload of WORKLOADS) {
    await measureReal(workload);
    await measureOurs(workload);
  }

  for (let round = 1; round <= ROUNDS; round += 1) {
    for (const workload of WORKLOADS) {
      const real = await measureReal(workload);
      const ours = await measureOurs(workload);
      samples.get(workload.name).real.push(real.ms);
      samples.get(workload.name).ours.push(ours.ms);
      console.log(`round ${round}/${ROUNDS} ${workload.name}: real ${real.ms.toFixed(1)}ms, ours ${ours.ms.toFixed(1)}ms`);
    }
  }

  const report = markdownReport(samples, version, semantics);
  await writeFile(join(exampleRoot, "reports", "benchmark-vs-mysql84.md"), report, "utf8");
  console.log("");
  console.log(report);
}

try {
  await main();
} catch (error) {
  console.error("BENCHMARK-VS-MYSQL84 FAIL");
  console.error(error);
  process.exitCode = 1;
}
