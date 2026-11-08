// Side-by-side throughput with `mysql2` as baseline.
//
// Fairness rules:
// - every (round × workload × client) starts a fresh `startMysqlServer` with a
//   fresh `mkdtemp` data directory;
// - table creation and seeding are untimed;
// - both clients use one connection, text protocol, no pooling, no prepared
//   statements, no TLS;
// - the server and client are stopped after each measurement.
//
// Run: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
// Env: BENCH_ROWS (200), BENCH_POINT (500), BENCH_SCAN (1000), BENCH_ROUNDS (3)

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

const CLIENTS = [
  { name: "mysql2", connect: connectMysql2 },
  { name: "@yydb/mysql", connect: connectMysql },
];

const CREATE_TABLE = "CREATE TABLE bench_items (id INT PRIMARY KEY, label VARCHAR(40), score DOUBLE)";

async function seed(client, rows) {
  await client.query(CREATE_TABLE);
  for (let id = 1; id <= rows; id += 1) {
    await client.query(
      `INSERT INTO bench_items (id, label, score) VALUES (${id}, 'row-${id}', ${id / 4})`,
    );
  }
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
  console.log(`Defaults: BENCH_ROWS=${ROWS}, BENCH_POINT=${POINT_OPS}, BENCH_SCAN=${SCAN_ROWS}, BENCH_ROUNDS=${ROUNDS}`);
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
