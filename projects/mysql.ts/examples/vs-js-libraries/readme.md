# vs-js-libraries

Protocol conformance and a side-by-side benchmark of `@yydb/mysql` against
[`mysql2`](https://github.com/sidorares/node-mysql2), with `mysql2` as the
**throughput baseline** (`1.00x`).

Both clients talk to the same loopback server started by `startMysqlServer`:

- `mysql2` — full driver, one connection, `query()` for text workloads and
  `prepare()` / `execute()` for `insert-prepared`
- `@yydb/mysql` — hand-written classic-protocol client over the product's
  native packet codec (handshake v10, `mysql_native_password` with an empty
  password, `COM_QUERY`, `COM_STMT_PREPARE` / `COM_STMT_EXECUTE`, EOF-terminated
  text and binary rows, `COM_QUIT`)

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
workload is measured. Both clients use one connection, no pooling, and no TLS.
Text workloads share the same SQL and seed size; `insert-prepared` prepares the
same statement once on each client and executes it over the binary protocol.
There is one untimed warmup round followed by three measured rounds.

- `insert` — `BENCH_ROWS` sequential `INSERT` statements (text protocol)
- `insert-prepared` — one `INSERT ... VALUES (?, ?, ?)` prepared per client,
  then `BENCH_ROWS` `COM_STMT_EXECUTE` calls over a `BENCH_ROWS`-row table
- `select-point` — `BENCH_POINT` primary-key equality `SELECT`s over `BENCH_ROWS` rows
- `select-point-uk` — the same `BENCH_POINT` count, same seed size, predicate
  moved to a `UNIQUE` secondary index (`uk`)
- `select-scan` — one `SELECT ... ORDER BY id` over `BENCH_SCAN` rows
- `select-cursor-page` — keyset pagination (`WHERE id > last ORDER BY id LIMIT 20`)
  over the same `BENCH_SCAN` rows; the unit is pages
- `select-null-blob` — one scan over `BENCH_SCAN` rows with a `NULL` note every
  third row and a 2 KiB payload every fifth row; the unit is rows
- `tx-commit` — `BENCH_TX` transactions of `BENCH_TX_ROWS` inserts, one `COMMIT` each

```shell
pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark
```

Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`,
`BENCH_ROUNDS=3`, `BENCH_TX=10`, `BENCH_TX_ROWS=10`. Override them through the
environment. The report is in
[`reports/benchmark.md`](./reports/benchmark.md).

## Intentional deltas

- Both clients hit the same product server over loopback; the minimal client is
  hand-written and text/binary only where the workload needs it, while `mysql2`
  is the full driver with JS-side row parsing.
- No pooling or TLS on either side.
- A fresh server plus untimed seed precedes every measurement.
- Only `insert-prepared` uses the binary protocol; every other workload uses
  `COM_QUERY` with the same SQL for both clients.
- `select-scan` and `select-null-blob` throughput is reported in rows, and
  `select-cursor-page` in pages, because each measurement is one statement over
  many rows or several statements over one query shape.
