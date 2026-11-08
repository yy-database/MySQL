# vs-js-libraries

Protocol conformance and a side-by-side benchmark of `@yydb/mysql` against
[`mysql2`](https://github.com/sidorares/node-mysql2), with `mysql2` as the
**throughput baseline** (`1.00x`).

Both clients talk to the same loopback server started by `startMysqlServer`:

- `mysql2` — full driver, one text-protocol connection, `query()` only
- `@yydb/mysql` — hand-written classic-protocol client over the product's
  native packet codec (handshake v10, `mysql_native_password` with an empty
  password, `COM_QUERY`, EOF-terminated text rows, `COM_QUIT`)

## Layout

- `src/mysql-client.mjs` — minimal classic-protocol client
- `src/mysql2-client.mjs` — thin `mysql2` wrapper exposing the same `query` shape
- `src/timing.mjs` — `hrtime` elapsed time, median, p95, throughput
- `tests/conformance.mjs` — cross-client write/read round-trip
- `tests/benchmark.mjs` — measured runs against a fresh server per measurement
- `reports/` — captured results

## Prerequisites

From the repository root:

```shell
pnpm install
```

`mysql2` is a root dev dependency and resolves from the workspace `node_modules`.
Run `pnpm build` only when `packages/mysql/dist` or the platform binding is
missing. Examples are plain ESM and need no build step.

## Correctness

One server, one table, both directions:

1. `mysql2` creates and seeds the table → the minimal client reads exact cells.
2. The minimal client inserts → `mysql2` reads the row.
3. The minimal client updates and deletes → `mysql2` observes the change.
4. An `ORDER BY id` scan is compared on both clients.

```shell
pnpm --dir projects/mysql.ts/examples/vs-js-libraries test
```

The captured output is in [`reports/conformance.md`](./reports/conformance.md).

## Benchmark

Every `(round × workload × client)` starts a fresh `startMysqlServer` with a
fresh `mkdtemp` data directory. Table creation and seeding are untimed. Only the
workload is measured. Both clients use one connection, the text protocol, no
pooling, no prepared statements, and no TLS. There is one untimed warmup round
followed by three measured rounds.

- `insert` — `BENCH_ROWS` sequential `INSERT` statements
- `select-point` — `BENCH_POINT` sequential `SELECT` statements over `BENCH_ROWS` seeded rows
- `select-scan` — one `SELECT ... ORDER BY id` over `BENCH_SCAN` seeded rows, fully materialized

```shell
pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark
```

Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`,
`BENCH_ROUNDS=3`. Override them through the environment. The report is in
[`reports/benchmark.md`](./reports/benchmark.md).

## Intentional deltas

- Both clients hit the same product server over loopback; the minimal client is
  hand-written and text-only, while `mysql2` is the full driver with JS-side row
  parsing.
- No prepared statements, pooling, or TLS on either side.
- A fresh server plus untimed seed precedes every measurement.
- The minimal client does not implement the binary protocol, so there is no
  prepared-statement workload.
- `select-scan` throughput is reported in rows because it is one statement over
  many rows.
