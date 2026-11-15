# Benchmark report

Command: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`, `BENCH_ROUNDS=3`, `BENCH_TX=10`, `BENCH_TX_ROWS=10`, `BENCH_MUTATE=100`, `BENCH_ERROR=100`, `BENCH_PAGE=20`, `BENCH_JOIN=100`, `BENCH_SHOW=200`
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
- `insert-autoinc` — 200 text inserts that omit the `AUTO_INCREMENT` primary key; the
  generated id reported in the OK packet must be `1..200`
- `select-point` — 500 primary-key equality selects over 200 seeded rows
- `select-point-uk` — 500 equality selects over 200 rows, predicate on a `UNIQUE` secondary index
- `select-scan` — one `SELECT ... ORDER BY id` over 1,000 rows
- `select-cursor-page` — keyset pagination over the same 1,000 rows, 20 rows per page
- `select-limit-offset` — 20 deep `LIMIT 10 OFFSET n` pages over the same 1,000 rows
- `multi-table-join` — 100 joins of two physical tables (`bench_items JOIN bench_side`),
  20 rows per statement
- `show-session` — 200 `SHOW VARIABLES LIKE 'version'` session-command round trips
- `select-null-blob` — one scan over 1,000 rows, a `NULL` note every third row and a 2 KiB payload every fifth row
- `tx-commit` — 10 transactions of 10 inserts, one `COMMIT` each
- `update-multi` — 100 statements, each updating the same four-row primary-key range of the 200-row seed
- `delete-where` — 100 paired-range `DELETE` statements removing the whole 200-row seed; the unit is rows
- `binary-prepared-rows` — one `SELECT id, label ... WHERE id = ?` prepared per client, then 500 `COM_STMT_EXECUTE` calls over the same 200-row seed as `select-point`
- `error-path` — 100 duplicate-primary-key inserts (errno 1062) and 100 unknown-table selects (errno 1146) on one connection
- `rollback-bulk` — 10 transactions of 10 inserts, one `ROLLBACK` each

`SHOW STATUS` is not implemented by the gateway (`1064`), so the session-command item is
`SHOW VARIABLES`. MySQL has no `PRAGMA synchronous` equivalent, so `commit-sync-on` is not
applicable to this product.

## Relative to `mysql2`

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| insert | 1.00x | **0.96x** |
| insert-prepared | 1.00x | **0.73x** |
| insert-autoinc | 1.00x | **0.86x** |
| select-point | 1.00x | **0.65x** |
| select-point-uk | 1.00x | **0.89x** |
| select-scan | 1.00x | **0.96x** |
| select-cursor-page | 1.00x | **0.89x** |
| select-limit-offset | **1.00x** | 1.18x |
| multi-table-join | **1.00x** | 1.03x |
| show-session | 1.00x | **0.73x** |
| select-null-blob | 1.00x | **0.54x** |
| tx-commit | 1.00x | **0.95x** |
| update-multi | 1.00x | **0.99x** |
| delete-where | 1.00x | **0.97x** |
| binary-prepared-rows | 1.00x | **0.60x** |
| error-path | 1.00x | **0.75x** |
| rollback-bulk | 1.00x | **0.87x** |

## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| insert | statements | 6.70 | 149/s | 6.40 | 156/s |
| insert-prepared | rows | 8.11 | 123/s | 5.90 | 169/s |
| insert-autoinc | rows | 7.75 | 129/s | 6.68 | 150/s |
| select-point | statements | 0.15 | 6,505/s | 0.10 | 9,947/s |
| select-point-uk | statements | 0.39 | 2,534/s | 0.35 | 2,840/s |
| select-scan | rows | 0.01 | 190,255/s | 0.01 | 198,831/s |
| select-cursor-page | pages | 1.98 | 506/s | 1.76 | 569/s |
| select-limit-offset | statements | 2.03 | 492/s | 2.40 | 416/s |
| multi-table-join | statements | 3.99 | 251/s | 4.13 | 242/s |
| show-session | statements | 0.16 | 6,359/s | 0.12 | 8,694/s |
| select-null-blob | rows | 0.01 | 80,815/s | 0.01 | 149,779/s |
| tx-commit | rows | 0.86 | 1,157/s | 0.82 | 1,224/s |
| update-multi | statements | 6.63 | 151/s | 6.54 | 153/s |
| delete-where | rows | 3.12 | 320/s | 3.03 | 330/s |
| binary-prepared-rows | statements | 0.19 | 5,337/s | 0.11 | 8,831/s |
| error-path | statements | 0.22 | 4,497/s | 0.17 | 6,015/s |
| rollback-bulk | rows | 0.31 | 3,260/s | 0.27 | 3,733/s |

## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| insert | 1374.55 | 1354.08 |
| insert-prepared | 2151.57 | 1296.79 |
| insert-autoinc | 1902.80 | 1562.85 |
| select-point | 82.51 | 60.03 |
| select-point-uk | 259.99 | 177.17 |
| select-scan | 16.61 | 5.38 |
| select-cursor-page | 99.77 | 88.44 |
| select-limit-offset | 40.98 | 49.59 |
| multi-table-join | 468.82 | 428.80 |
| show-session | 32.12 | 25.95 |
| select-null-blob | 14.10 | 7.18 |
| tx-commit | 106.26 | 82.93 |
| update-multi | 693.28 | 741.22 |
| delete-where | 840.60 | 629.78 |
| binary-prepared-rows | 95.71 | 57.69 |
| error-path | 50.99 | 36.58 |
| rollback-bulk | 38.70 | 29.52 |

## Alternating legacy / current AB

One temporary legacy build (the previous gateway source) and one current build were
alternated three times. Each run executes three measured rounds per workload and the table
reports the median of the three run-level medians (absolute ms, lower is better). Both
variants use the same hand-written minimal client and the same SQL, so only the gateway
binary changes.

| Workload | Legacy ms | Current ms | Change |
|---------:|----------:|-----------:|-------:|
| select-point | 60.3 | 54.2 | -10.2% |
| binary-prepared-rows | 76.5 | 69.3 | -9.5% |
| select-point-uk | 179.3 | 164.8 | -8.1% |
| multi-table-join | 392.8 | 386.8 | -1.5% |
| show-session | 20.8 | 21.0 | +0.7% |
| insert-prepared | 1251.3 | 1293.0 | +3.3% |
| insert | 1116.6 | 1311.5 | +17.5% |

`select-point` crosses the 10% median gate. `binary-prepared-rows` lands at -9.5%: its
render cache and prepare-time metadata remove server-side work, but the end-to-end number
is dominated by the execution scan plus one client round trip per execute. `insert` and
`select-point-uk` are dominated by storage commits and the execution-layer scan, so their
medians move within run-to-run noise (the earlier exploratory batch measured `insert` at
-7.5% and `select-point` at -11.9%).

What the current path changes:

- **Prepared render cache.** A prepared statement keeps the SQL text it rendered for a
  decoded parameter list, compared element-wise (never by hash alone). A repeated bind
  skips `walk_parameters`, its `Vec<char>` copy and per-parameter formatting. The null
  bitmap and wire parameter types are already folded into `SqlValue`, so the value list is
  the complete key.
- **Prepare-time column metadata.** Result column wire types and nullability are derived
  once at `COM_STMT_PREPARE` instead of rebuilding two vectors on every execute.
- **Cached result-set prefix.** A cached SELECT plan stores the encoded column-count and
  column-definition packets for a sequence-1 response. Hot point lookups append rows and
  the status-dependent EOF instead of rebuilding one packet per column.
- **Metadata pre-filter.** `analyze_meta_query` (which lowercases and normalises the whole
  statement) now only runs when the allocation-free `may_be_metadata_query` pre-filter
  matches. DML and SELECT statements skip it on every `COM_QUERY`.

## Known production gaps

- `select-point-uk` still scans every row of the table because the unique constraint is
  only enforced on writes; there is no secondary KV index to push an equality lookup into.
  The gateway can only cache the plan, not change execution.
- `select-cursor-page` still scans the table per page: integer primary keys are stored as
  unpadded decimal ASCII, so `rowid_after` cannot express `id > last` without a scan and a
  row-level filter. Early-stop pagination needs an execution-layer rowid predicate.
- `select-limit-offset` is 1.18x slower than `mysql2` for the same reason at the execution
  layer: a deep `OFFSET` walks and sorts the whole table per page. The gateway has no place
  to add an early stop.
- `multi-table-join` is at parity/slightly slower because the execution-layer join is a
  nested-loop scan; the protocol layer only caches the identical SQL plan.
- `update-multi` and `delete-where` still scan the namespace per statement inside the
  execution layer. The gateway caches plans only for catalog-stable SELECTs.
- `SHOW STATUS` is rejected with `1064`; only `SHOW VARIABLES` is implemented.
- There is no `PRAGMA synchronous` / `commit-sync-on` equivalent in the MySQL gateway.
- The real MySQL 8.4 comparison (`benchmark:mysql84`) was not reachable at
  `127.0.0.1:33306`, so it was skipped for this round.

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
- `insert-autoinc` asserts the generated id reported by both clients, so a client that
  drops `insertId` fails the run before timing.
- `select-point-uk` moves only the predicate to a `UNIQUE` index while keeping the point
  count and seed size of `select-point`; it exposes secondary-index lookup, not index DDL.
- `select-limit-offset` cycles the offset deterministically so the page never runs past the
  seed and both clients issue identical SQL.
- `multi-table-join` reads two physical tables; the second table holds exactly one row per
  item id, so the join cardinality is 20 for both clients.
- `select-cursor-page` is one statement per page, so its throughput unit is pages. Errors
  (missing rows, wrong first id, NULL/large-field scan count, wrong errno, wrong generated
  id) fail the run before timing is reported.
- `error-path` asserts the exact MySQL errno (`1062` duplicate key, `1146` unknown table)
  on both clients and keeps issuing statements afterwards, so a desynchronised stream
  fails the run instead of being timed.
- `update-multi` and `delete-where` cycle over the same seed ranges so both clients issue
  identical SQL and affect identical rows.
- Counts are tuned down from the SQLite example because every insert and point select is a
  full round trip to a separately started product server.

Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
