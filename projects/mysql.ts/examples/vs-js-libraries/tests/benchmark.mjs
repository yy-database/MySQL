// Side-by-side throughput with `mysql2` as baseline.
//
// Fairness rules:
// - every (round × workload × client) starts a fresh `startMysqlServer` with a
//   fresh `mkdtemp` data directory;
// - table creation and seeding are untimed;
// - both clients use one connection, no pooling, no TLS;
// - text-protocol workloads share one SQL and one seed size across clients;
// - `insert-prepared` is the exception: both clients prepare once and execute
//   the same binary-protocol statement `BENCH_ROWS` times;
// - the server and client are stopped after each measurement.
//
// Run: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
// Env: BENCH_ROWS (200), BENCH_POINT (500), BENCH_SCAN (1000), BENCH_ROUNDS (3),
//      BENCH_TX (10), BENCH_TX_ROWS (10)

import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { startMysqlServer } from "../../../packages/mysql/dist/index.js";
import { connectMysql } from "../src/mysql-client.mjs";
import { connectMysql2 } from "../src/mysql2-client.mjs";
import { elapsedMs, median, p95, throughput } from "../src/timing.mjs";

const ROWS = Number(process.env.BENCH_ROWS ?? 200);
const POINT_OPS = Number(process.env.BENCH_POINT ?? 500);
const SCAN_ROWS = Number(process.env.BENCH_SCAN ?? 1_000);
const ROUNDS = Number(process.env.BENCH_ROUNDS ?? 3);
const TX_COUNT = Number(process.env.BENCH_TX ?? 10);
const TX_ROWS = Number(process.env.BENCH_TX_ROWS ?? 10);
const CURSOR_PAGE_SIZE = 20;

const CLIENTS = [
  { name: "mysql2", connect: connectMysql2 },
  { name: "@yydb/mysql", connect: connectMysql },
];

const CREATE_TABLE = "CREATE TABLE bench_items (id INT PRIMARY KEY, label VARCHAR(40), score DOUBLE)";
const CREATE_TABLE_UK =
  "CREATE TABLE bench_items_uk (id INT PRIMARY KEY, uk INT UNIQUE, label VARCHAR(40), score DOUBLE)";
const CREATE_TABLE_LARGE = "CREATE TABLE bench_large (id INT PRIMARY KEY, note VARCHAR(40) NULL, payload TEXT)";

async function seed(client, rows) {
  await client.query(CREATE_TABLE);
  for (let id = 1; id <= rows; id += 1) {
    await client.query(
      `INSERT INTO bench_items (id, label, score) VALUES (${id}, 'row-${id}', ${id / 4})`,
    );
  }
}

async function seedUnique(client, rows) {
  await client.query(CREATE_TABLE_UK);
  for (let id = 1; id <= rows; id += 1) {
    await client.query(
      `INSERT INTO bench_items_uk (id, uk, label, score) VALUES (${id}, ${id}, 'row-${id}', ${id / 4})`,
    );
  }
}

// Large/NULL mix: every third row has a NULL note, every fifth row carries a
// 2 KiB payload. Batched seed keeps untimed setup bounded at 1,000 rows.
async function seedLarge(client, rows) {
  await client.query(CREATE_TABLE_LARGE);
  const batch = 100;
  for (let start = 1; start <= rows; start += batch) {
    const values = [];
    for (let id = start; id < Math.min(start + batch, rows + 1); id += 1) {
      const note = id % 3 === 0 ? "NULL" : `'note-${id}'`;
      const payload = id % 5 === 0 ? "x".repeat(2048) : "small";
      values.push(`(${id}, ${note}, '${payload}')`);
    }
    await client.query(`INSERT INTO bench_large (id, note, payload) VALUES ${values.join(", ")}`);
  }
}

function cellAt(result, row, index) {
  return Array.isArray(row) ? row[index] : row[result.columns[index]];
}

const WORKLOADS = [
  {
    name: "insert",
    unit: "statements",
    ops: ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE);
    },
    async run(client) {
      for (let id = 1; id <= ROWS; id += 1) {
        await client.query(
          `INSERT INTO bench_items (id, label, score) VALUES (${id}, 'row-${id}', ${id / 4})`,
        );
      }
      return ROWS;
    },
  },
  {
    name: "insert-prepared",
    unit: "rows",
    ops: ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE);
    },
    async run(client) {
      // Same SQL and row count as `insert`, but one prepare followed by
      // `ROWS` binary-protocol executes.
      const statement = await client.prepare(
        "INSERT INTO bench_items (id, label, score) VALUES (?, ?, ?)",
      );
      try {
        for (let id = 1; id <= ROWS; id += 1) {
          await statement.execute([id, `row-${id}`, id / 4]);
        }
      } finally {
        await statement.close();
      }
      return ROWS;
    },
  },
  {
    name: "select-point",
    unit: "statements",
    ops: POINT_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      for (let index = 0; index < POINT_OPS; index += 1) {
        const id = (index % ROWS) + 1;
        const result = await client.query(`SELECT label FROM bench_items WHERE id = ${id}`);
        if (result.rows.length !== 1) throw new Error(`point miss id=${id}`);
      }
      return POINT_OPS;
    },
  },
  {
    name: "select-point-uk",
    unit: "statements",
    ops: POINT_OPS,
    async setup(client) {
      await seedUnique(client, ROWS);
    },
    async run(client) {
      // Same cardinality as `select-point`; only the predicate moves from the
      // primary key to a UNIQUE secondary index.
      for (let index = 0; index < POINT_OPS; index += 1) {
        const uk = (index % ROWS) + 1;
        const result = await client.query(
          `SELECT label FROM bench_items_uk WHERE uk = ${uk}`,
        );
        if (result.rows.length !== 1) throw new Error(`point-uk miss uk=${uk}`);
      }
      return POINT_OPS;
    },
  },
  {
    name: "select-scan",
    unit: "rows",
    ops: SCAN_ROWS,
    async setup(client) {
      await seed(client, SCAN_ROWS);
    },
    async run(client) {
      const result = await client.query("SELECT id, label, score FROM bench_items ORDER BY id");
      if (result.rows.length !== SCAN_ROWS) {
        throw new Error(`${result.rows.length} rows, expected ${SCAN_ROWS}`);
      }
      return SCAN_ROWS;
    },
  },
  {
    name: "select-cursor-page",
    unit: "pages",
    ops: Math.ceil(SCAN_ROWS / CURSOR_PAGE_SIZE),
    async setup(client) {
      await seed(client, SCAN_ROWS);
    },
    async run(client) {
      // Keyset pagination over the same 1,000 seeded rows as `select-scan`.
      // Each page is one statement; the unit is pages, not rows.
      const pages = Math.ceil(SCAN_ROWS / CURSOR_PAGE_SIZE);
      let last = 0;
      for (let page = 0; page < pages; page += 1) {
        const result = await client.query(
          `SELECT id, label FROM bench_items WHERE id > ${last} ORDER BY id LIMIT ${CURSOR_PAGE_SIZE}`,
        );
        const expected = Math.min(CURSOR_PAGE_SIZE, SCAN_ROWS - page * CURSOR_PAGE_SIZE);
        if (result.rows.length !== expected) {
          throw new Error(`cursor page ${page} rows=${result.rows.length}, expected=${expected}`);
        }
        last = Number(cellAt(result, result.rows[result.rows.length - 1], 0));
      }
      return pages;
    },
  },
  {
    name: "select-null-blob",
    unit: "rows",
    ops: SCAN_ROWS,
    async setup(client) {
      await seedLarge(client, SCAN_ROWS);
    },
    async run(client) {
      // One statement over the same 1,000 row count as `select-scan`, with a
      // NULL every third row and a 2 KiB payload every fifth row.
      const result = await client.query("SELECT id, note, payload FROM bench_large ORDER BY id");
      if (result.rows.length !== SCAN_ROWS) {
        throw new Error(`${result.rows.length} rows, expected ${SCAN_ROWS}`);
      }
      return SCAN_ROWS;
    },
  },
  {
    name: "tx-commit",
    unit: "rows",
    ops: TX_COUNT * TX_ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE);
    },
    async run(client) {
      // Same shape as the MySQL 8.4 comparison: N transactions of 10 inserts,
      // one COMMIT each.
      for (let tx = 0; tx < TX_COUNT; tx += 1) {
        await client.query("BEGIN");
        for (let row = 0; row < TX_ROWS; row += 1) {
          const id = tx * TX_ROWS + row + 1;
          await client.query(
            `INSERT INTO bench_items (id, label, score) VALUES (${id}, 'row-${id}', ${id / 4})`,
          );
        }
        await client.query("COMMIT");
      }
      return TX_COUNT * TX_ROWS;
    },
  },
];

async function measure(workload, client) {
  const dataDir = await mkdtemp(join(tmpdir(), "mysql-bench-"));
  const server = startMysqlServer({ dataDir, port: 0 });
  let connection;
  try {
    connection = await client.connect(server.port);
    await workload.setup(connection);
    const start = process.hrtime.bigint();
    const ops = await workload.run(connection);
    return { ms: elapsedMs(start), ops };
  } finally {
    if (connection !== undefined) await connection.close().catch(() => {});
    server.stop();
    await rm(dataDir, { recursive: true, force: true }).catch(() => {});
  }
}

function formatMs(value) {
  return value.toFixed(2);
}

function cell(value, fastest, formatted) {
  return fastest ? `**${formatted}**` : formatted;
}

async function main() {
  const samples = new Map();
  for (const workload of WORKLOADS) {
    const perClient = {};
    for (const client of CLIENTS) perClient[client.name] = [];
    samples.set(workload.name, perClient);
  }

  console.log("mysql vs-js-libraries benchmark");
  console.log(`Loopback 127.0.0.1, one text-protocol connection per client, fresh server + seed per measurement`);
  console.log(
    `Defaults: BENCH_ROWS=${ROWS}, BENCH_POINT=${POINT_OPS}, BENCH_SCAN=${SCAN_ROWS}, BENCH_ROUNDS=${ROUNDS}, BENCH_TX=${TX_COUNT}x${TX_ROWS}`,
  );
  console.log("Warmup: 1 untimed round");
  for (const workload of WORKLOADS) {
    for (const client of CLIENTS) await measure(workload, client);
  }

  for (let round = 1; round <= ROUNDS; round += 1) {
    for (const workload of WORKLOADS) {
      for (const client of CLIENTS) {
        const { ms } = await measure(workload, client);
        samples.get(workload.name)[client.name].push(ms);
      }
    }
  }

  console.log("\nAbsolute (median per-round totals normalised per op)");
  console.log("| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |");
  console.log("|---------:|-----:|-------------:|-------------:|------------------:|------------------:|");
  for (const workload of WORKLOADS) {
    const baseline = median(samples.get(workload.name)["mysql2"]);
    const subject = median(samples.get(workload.name)["@yydb/mysql"]);
    console.log(
      `| ${workload.name} | ${workload.unit} | ${formatMs(baseline / workload.ops)} | ${throughput(workload.ops, baseline)} | ${formatMs(subject / workload.ops)} | ${throughput(workload.ops, subject)} |`,
    );
  }

  console.log("\nRelative to `mysql2` (ratio = @yydb/mysql_time / mysql2_time, 1.00x = parity)");
  console.log("| Workload | mysql2 | @yydb/mysql |");
  console.log("|---------:|-------:|------------:|");
  for (const workload of WORKLOADS) {
    const baseline = median(samples.get(workload.name)["mysql2"]);
    const subject = median(samples.get(workload.name)["@yydb/mysql"]);
    const ratio = subject / baseline;
    const subjectFastest = ratio < 1;
    const formattedRatio = `${ratio.toFixed(2)}x`;
    console.log(
      `| ${workload.name} | ${cell(1, !subjectFastest, "1.00x")} | ${cell(ratio, subjectFastest, formattedRatio)} |`,
    );
  }

  console.log("\np95 per-round total (ms)");
  console.log("| Workload | mysql2 | @yydb/mysql |");
  console.log("|---------:|-------:|------------:|");
  for (const workload of WORKLOADS) {
    const baseline = p95(samples.get(workload.name)["mysql2"]);
    const subject = p95(samples.get(workload.name)["@yydb/mysql"]);
    console.log(`| ${workload.name} | ${formatMs(baseline)} | ${formatMs(subject)} |`);
  }
}

try {
  await main();
} catch (error) {
  console.error("BENCHMARK FAIL");
  console.error(error);
  process.exitCode = 1;
}
