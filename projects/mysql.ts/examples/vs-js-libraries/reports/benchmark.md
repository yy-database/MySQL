# Benchmark report

Command: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`, `BENCH_ROUNDS=3`, `BENCH_TX=10`, `BENCH_TX_ROWS=10`
Baseline: `mysql2` (`createConnection` + `query()` / prepared `execute()`) = `1.00x`
Ratio: subject_time / baseline_time (smaller is faster). Fastest cell in each row is bold.
Each measurement starts a fresh `startMysqlServer` with a fresh `mkdtemp` data directory,
creates and seeds the table untimed, then times only the workload.

## Workloads

Text workloads share one SQL and one seed size across clients. `insert-prepared`
prepares the same `INSERT ... VALUES (?, ?, ?)` once per client and executes it
`BENCH_ROWS` times over the binary protocol; every other workload uses
`COM_QUERY` for both clients.

- `insert` — 200 sequential text inserts into an empty table
- `insert-prepared` — one prepared insert, 200 `COM_STMT_EXECUTE` calls
- `select-point` — 500 primary-key equality selects over 200 seeded rows
- `select-point-uk` — 500 equality selects over 200 rows, predicate on a `UNIQUE` secondary index
- `select-scan` — one `SELECT ... ORDER BY id` over 1,000 rows
- `select-cursor-page` — keyset pagination over the same 1,000 rows, 20 rows per page
- `select-null-blob` — one scan over 1,000 rows, a `NULL` note every third row and a 2 KiB payload every fifth row
- `tx-commit` — 10 transactions of 10 inserts, one `COMMIT` each

## Relative to `mysql2`

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| insert | 1.00x | **0.94x** |
| insert-prepared | 1.00x | **0.96x** |
| select-point | 1.00x | **0.74x** |
| select-point-uk | **1.00x** | 1.21x |
| select-scan | 1.00x | **0.92x** |
| select-cursor-page | 1.00x | **0.89x** |
| select-null-blob | 1.00x | **0.56x** |
| tx-commit | 1.00x | **0.76x** |

## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| insert | statements | 4.31 | 232/s | 4.05 | 247/s |
| insert-prepared | rows | 4.28 | 234/s | 4.12 | 243/s |
| select-point | statements | 0.09 | 10,779/s | 0.07 | 14,518/s |
| select-point-uk | statements | 0.24 | 4,225/s | 0.29 | 3,502/s |
| select-scan | rows | 0.00 | 309,195/s | 0.00 | 336,180/s |
| select-cursor-page | pages | 1.32 | 757/s | 1.17 | 851/s |
| select-null-blob | rows | 0.01 | 114,559/s | 0.00 | 202,807/s |
| tx-commit | rows | 0.57 | 1,761/s | 0.43 | 2,323/s |

## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| insert | 877.92 | 874.94 |
| insert-prepared | 858.10 | 839.56 |
| select-point | 69.39 | 44.26 |
| select-point-uk | 170.95 | 162.75 |
| select-scan | 5.47 | 3.71 |
| select-cursor-page | 82.06 | 66.42 |
| select-null-blob | 9.29 | 5.59 |
| tx-commit | 63.95 | 44.72 |

## Intentional deltas

- Both clients use one connection over loopback to the same product server and issue the
  same SQL through the text protocol, except `insert-prepared`, which uses the
  binary protocol on both clients for the same prepared statement. No pooling or TLS.
- `@yydb/mysql` is a hand-written classic-protocol client built on the product's native
  packet codec. Text rows come back as arrays; prepared rows are decoded by column type.
- `mysql2` is the full driver and performs JS-side row parsing into typed objects.
- Because both clients traverse the same product server, the measured wall time is dominated
  by server-side work and ratios collapse towards parity. The benchmark still detects driver
  parse differences, which is where scans separate.
- A fresh server plus untimed seed run precedes every measurement, so start-up and seed cost
  are excluded from the ratio.
- `select-point-uk` moves only the predicate to a `UNIQUE` index while keeping the point
  count and seed size of `select-point`; it exposes secondary-index lookup, not index DDL.
- `select-cursor-page` is one statement per page, so its throughput unit is pages. Errors
  (missing rows, wrong first id, NULL/large-field scan count) fail the run before timing is
  reported.
- Counts are tuned down from the SQLite example because every insert and point select is a
  full round trip to a separately started product server.

Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
