# Benchmark report

Command: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`, `BENCH_ROUNDS=3`, `BENCH_TX=10`, `BENCH_TX_ROWS=10`, `BENCH_MUTATE=100`, `BENCH_ERROR=100`
Baseline: `mysql2` (`createConnection` + `query()` / prepared `execute()`) = `1.00x`
Ratio: subject_time / baseline_time (smaller is faster). Fastest cell in each row is bold.
Each measurement starts a fresh `startMysqlServer` with a fresh `mkdtemp` data directory,
creates and seeds the table untimed, then times only the workload.

## Workloads

Text workloads share one SQL and one seed size across clients. `insert-prepared` and
`binary-prepared-rows` prepare the same statement once per client and execute it over the
binary protocol; every other workload uses `COM_QUERY` for both clients.

- `insert` — 200 sequential text inserts into an empty table
- `insert-prepared` — one prepared insert, 200 `COM_STMT_EXECUTE` calls
- `select-point` — 500 primary-key equality selects over 200 seeded rows
- `select-point-uk` — 500 equality selects over 200 rows, predicate on a `UNIQUE` secondary index
- `select-scan` — one `SELECT ... ORDER BY id` over 1,000 rows
- `select-cursor-page` — keyset pagination over the same 1,000 rows, 20 rows per page
- `select-null-blob` — one scan over 1,000 rows, a `NULL` note every third row and a 2 KiB payload every fifth row
- `tx-commit` — 10 transactions of 10 inserts, one `COMMIT` each
- `update-multi` — 100 statements, each updating the same four-row primary-key range of the 200-row seed
- `delete-where` — 100 paired-range `DELETE` statements removing the whole 200-row seed; the unit is rows
- `binary-prepared-rows` — one `SELECT id, label ... WHERE id = ?` prepared per client, then 500 `COM_STMT_EXECUTE` calls over the same 200-row seed as `select-point`
- `error-path` — 100 duplicate-primary-key inserts (errno 1062) and 100 unknown-table selects (errno 1146) on one connection
- `rollback-bulk` — 10 transactions of 10 inserts, one `ROLLBACK` each

## Relative to `mysql2`

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| insert | 1.00x | **0.99x** |
| insert-prepared | 1.00x | **0.96x** |
| select-point | 1.00x | **0.64x** |
| select-point-uk | 1.00x | **0.86x** |
| select-scan | 1.00x | **0.67x** |
| select-cursor-page | 1.00x | **0.80x** |
| select-null-blob | 1.00x | **0.62x** |
| tx-commit | 1.00x | **0.89x** |
| update-multi | **1.00x** | 1.04x |
| delete-where | **1.00x** | 1.14x |
| binary-prepared-rows | 1.00x | **0.71x** |
| error-path | 1.00x | **0.62x** |
| rollback-bulk | 1.00x | **0.81x** |

## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| insert | statements | 5.95 | 168/s | 5.87 | 170/s |
| insert-prepared | rows | 6.03 | 166/s | 5.80 | 172/s |
| select-point | statements | 0.14 | 7,065/s | 0.09 | 11,004/s |
| select-point-uk | statements | 0.35 | 2,880/s | 0.30 | 3,344/s |
| select-scan | rows | 0.01 | 140,044/s | 0.00 | 207,792/s |
| select-cursor-page | pages | 1.80 | 555/s | 1.44 | 694/s |
| select-null-blob | rows | 0.01 | 87,403/s | 0.01 | 141,435/s |
| tx-commit | rows | 0.78 | 1,279/s | 0.70 | 1,437/s |
| update-multi | statements | 6.04 | 166/s | 6.30 | 159/s |
| delete-where | rows | 3.15 | 318/s | 3.59 | 278/s |
| binary-prepared-rows | statements | 0.18 | 5,481/s | 0.13 | 7,749/s |
| error-path | statements | 0.20 | 5,124/s | 0.12 | 8,200/s |
| rollback-bulk | rows | 0.28 | 3,594/s | 0.23 | 4,437/s |

## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| insert | 1248.53 | 1198.75 |
| insert-prepared | 1568.35 | 1171.06 |
| select-point | 78.23 | 50.93 |
| select-point-uk | 188.11 | 183.25 |
| select-scan | 7.38 | 5.78 |
| select-cursor-page | 107.69 | 74.49 |
| select-null-blob | 12.23 | 8.23 |
| tx-commit | 80.26 | 79.06 |
| update-multi | 638.47 | 663.18 |
| delete-where | 662.44 | 754.66 |
| binary-prepared-rows | 95.41 | 70.87 |
| error-path | 46.07 | 28.38 |
| rollback-bulk | 27.91 | 22.98 |

## Alternating legacy / current AB

Measured on one temporary build carrying a legacy-path switch, used only for this AB and
removed before the final build. Three alternating legacy/current process pairs, each with
three measured rounds; the table reports the median of the three run-level medians
(absolute ms, lower is better).

| Workload | Legacy ms | Current ms | Change |
|---------:|----------:|-----------:|-------:|
| insert | 1300.42 | 1214.13 | -6.6% |
| insert-prepared | 1215.90 | 1145.37 | -5.8% |
| select-point | 57.95 | 51.63 | -10.9% |
| select-point-uk | 150.35 | 148.02 | -1.5% |
| binary-prepared-rows | 70.33 | 47.42 | -32.6% |

What the current path changes:

- **SELECT plan cache.** Successful catalog-stable `SELECT` plans are kept per session
  and keyed by SQL text. Repeated point lookups and prepared statements whose rendered
  values cycle skip Oak parsing, catalog binding and planning. The cache is dropped
  whenever the schema revision changes or the connection is reset; writes,
  `@@`-references and unrendered placeholders are never cached.
- **Result buffer reuse.** The result-set assembly buffer lives on the session and grows
  to the largest response, so single-row replies no longer allocate a fresh 8 KiB
  staging vector per statement.
- **Cheap guards before metadata and DDL parsing.** `analyze_meta_query` only runs when
  the statement contains one of its eight trigger words (or starts with `SHOW`), and
  `parse_ddl` only runs when the leading verb is `CREATE` / `ALTER` / `DROP`.

## Known production gaps

- `select-point-uk` still scans every row of the table because the unique constraint is
  only enforced on writes; there is no secondary KV index to push an equality lookup
  into. The plan cache removes bind/plan cost but not the execution-layer scan, hence
  the -1.5% change. Fixing it needs an execution-layer secondary index or pushdown.
- `select-cursor-page` still scans the whole table per page: integer primary keys are
  stored as unpadded decimal ASCII, so `rowid_after` cannot express `id > last` without
  a scan and a row-level filter.
- Prepared statements still re-render their SQL text on every `COM_STMT_EXECUTE`
  (`substitute_sql_parameters`); only the resulting plan is cached. Caching the rendered
  segments and parameter layout is the next in-scope step for prepared writes.
- `update-multi` and `delete-where` are slower than `mysql2` because both clients hit the
  same gateway and the gateway's multi-row UPDATE/DELETE path scans the namespace per
  statement; driver-side parsing is not the differentiator.

## Intentional deltas

- Both clients use one connection over loopback to the same product server and issue the
  same SQL through the text protocol, except the two prepared workloads, which use the
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
  (missing rows, wrong first id, NULL/large-field scan count, wrong errno) fail the run
  before timing is reported.
- `error-path` asserts the exact MySQL errno (`1062` duplicate key, `1146` unknown table)
  on both clients and keeps issuing statements afterwards, so a desynchronised stream
  fails the run instead of being timed.
- `update-multi` and `delete-where` cycle over the same seed ranges so both clients issue
  identical SQL and affect identical rows.
- The real MySQL 8.4 instance used by `benchmark:mysql84` was not reachable
  (`127.0.0.1:33306` closed), so that comparison was not rerun in this round.
- Counts are tuned down from the SQLite example because every insert and point select is a
  full round trip to a separately started product server.

Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
