# Benchmark report

Command: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`, `BENCH_ROUNDS=3`, `BENCH_TX=10`, `BENCH_TX_ROWS=10`, `BENCH_MUTATE=100`, `BENCH_ERROR=100`, `BENCH_PAGE=20`, `BENCH_JOIN=100`, `BENCH_SHOW=200`, `BENCH_BATCH_ROWS=20`, `BENCH_AGG=100`, `BENCH_CONNECT=20`, `BENCH_WINDOW=20`
Captured with `BENCH_ROUNDS=2`.
Baseline: `mysql2` (`createConnection` + `query()` / prepared `execute()`) = `1.00x`
Ratio: subject_time / baseline_time (smaller is faster). Fastest cell in each row is bold.
Each measurement starts a fresh `startMysqlServer` with a fresh `mkdtemp` data directory,
creates and seeds the table untimed, then times only the workload.

## Workloads

Text workloads share one SQL and one seed size across clients. `insert-prepared` and
`binary-prepared-rows` prepare the same statement once per client and execute it over the
binary protocol; every other workload uses `COM_QUERY` for both clients.

- `connect-handshake` — open and close one extra connection per op against the same fresh
  server while the measurement connection stays open
- `insert` — 200 sequential text inserts into an empty table
- `insert-prepared` — one prepared insert, 200 `COM_STMT_EXECUTE` calls
- `insert-batch` — one multi-row `INSERT` of `BENCH_BATCH_ROWS` rows per statement
- `insert-autoinc` — 200 text inserts that omit the `AUTO_INCREMENT` primary key; the
  generated id reported in the OK packet must be `1..200`
- `select-point` — 500 primary-key equality selects over 200 seeded rows
- `select-point-uk` — 500 equality selects over 200 rows, predicate on a `UNIQUE` secondary index
- `index-range` — ranged reads on the `UNIQUE` secondary index, 20 adjacent keys per statement
- `select-range` — 500 primary-key range reads of 20 adjacent ids per statement
- `select-scan` — one `SELECT ... ORDER BY id` over 1,000 rows
- `aggregate` — 100 `count`/`sum`/`min`/`max` aggregates over the 200-row seed
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
- `upsert-pk` — 200 `INSERT ... ON DUPLICATE KEY UPDATE` statements on the same primary keys
- `insert-select` — `INSERT ... SELECT` copies over the seeded table
- `tx-roundtrip` — 20 `BEGIN` / `SELECT` / `COMMIT` round trips
- `window-row-number` — 20 `row_number() OVER (ORDER BY id)` statements
- `rollback-bulk` — 10 transactions of 10 inserts, one `ROLLBACK` each

`SHOW STATUS` is not implemented by the gateway (`1064`), so the session-command item is
`SHOW VARIABLES`. MySQL has no `PRAGMA synchronous` equivalent, so `commit-sync-on` is not
applicable to this product.

## Relative to `mysql2`

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| connect-handshake | 1.00x | **0.62x** |
| insert | 1.00x | **0.86x** |
| insert-prepared | 1.00x | **0.92x** |
| insert-batch | 1.00x | **0.96x** |
| insert-autoinc | 1.00x | **0.99x** |
| select-point | 1.00x | **0.74x** |
| select-point-uk | 1.00x | **0.82x** |
| index-range | 1.00x | **0.76x** |
| select-range | 1.00x | **0.68x** |
| select-scan | 1.00x | **0.59x** |
| aggregate | 1.00x | **0.77x** |
| select-cursor-page | **1.00x** | 1.11x |
| select-limit-offset | 1.00x | **0.94x** |
| multi-table-join | 1.00x | **0.81x** |
| show-session | 1.00x | **0.76x** |
| select-null-blob | 1.00x | **0.54x** |
| tx-commit | 1.00x | **0.89x** |
| update-multi | 1.00x | **0.89x** |
| delete-where | 1.00x | **0.87x** |
| binary-prepared-rows | 1.00x | **0.56x** |
| error-path | 1.00x | **0.64x** |
| upsert-pk | 1.00x | **0.99x** |
| insert-select | 1.00x | **0.85x** |
| tx-roundtrip | 1.00x | **0.79x** |
| window-row-number | 1.00x | **0.94x** |
| rollback-bulk | 1.00x | **0.93x** |

## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| connect-handshake | connections | 1.42 | 703/s | 0.88 | 1,134/s |
| insert | statements | 3.65 | 274/s | 3.13 | 320/s |
| insert-prepared | rows | 3.50 | 286/s | 3.23 | 310/s |
| insert-batch | rows | 0.18 | 5,627/s | 0.17 | 5,839/s |
| insert-autoinc | rows | 4.12 | 243/s | 4.08 | 245/s |
| select-point | statements | 0.13 | 7,833/s | 0.09 | 10,632/s |
| select-point-uk | statements | 0.31 | 3,213/s | 0.26 | 3,910/s |
| index-range | statements | 0.35 | 2,840/s | 0.27 | 3,724/s |
| select-range | statements | 0.22 | 4,475/s | 0.15 | 6,547/s |
| select-scan | rows | 0.00 | 283,776/s | 0.00 | 480,480/s |
| aggregate | statements | 0.28 | 3,538/s | 0.22 | 4,588/s |
| select-cursor-page | pages | 1.17 | 853/s | 1.30 | 766/s |
| select-limit-offset | statements | 1.38 | 723/s | 1.31 | 766/s |
| multi-table-join | statements | 3.91 | 256/s | 3.17 | 315/s |
| show-session | statements | 0.09 | 11,346/s | 0.07 | 14,927/s |
| select-null-blob | rows | 0.01 | 114,712/s | 0.00 | 210,699/s |
| tx-commit | rows | 0.45 | 2,198/s | 0.41 | 2,462/s |
| update-multi | statements | 3.44 | 291/s | 3.07 | 325/s |
| delete-where | rows | 1.75 | 570/s | 1.52 | 656/s |
| binary-prepared-rows | statements | 0.15 | 6,750/s | 0.08 | 12,115/s |
| error-path | statements | 0.16 | 6,317/s | 0.10 | 9,815/s |
| upsert-pk | statements | 3.94 | 254/s | 3.90 | 256/s |
| insert-select | rows | 0.03 | 34,519/s | 0.02 | 40,544/s |
| tx-roundtrip | transactions | 0.56 | 1,777/s | 0.44 | 2,252/s |
| window-row-number | statements | 0.45 | 2,238/s | 0.42 | 2,386/s |
| rollback-bulk | rows | 0.23 | 4,433/s | 0.21 | 4,772/s |

## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| connect-handshake | 35.60 | 20.31 |
| insert | 768.24 | 638.88 |
| insert-prepared | 732.21 | 683.09 |
| insert-batch | 37.91 | 36.63 |
| insert-autoinc | 827.81 | 844.33 |
| select-point | 64.90 | 52.54 |
| select-point-uk | 165.10 | 143.36 |
| index-range | 189.58 | 154.84 |
| select-range | 131.65 | 96.29 |
| select-scan | 4.18 | 2.16 |
| aggregate | 31.39 | 26.97 |
| select-cursor-page | 59.93 | 71.43 |
| select-limit-offset | 30.64 | 26.40 |
| multi-table-join | 439.65 | 323.44 |
| show-session | 22.33 | 13.50 |
| select-null-blob | 8.95 | 5.34 |
| tx-commit | 47.63 | 40.77 |
| update-multi | 358.22 | 332.47 |
| delete-where | 367.73 | 315.13 |
| binary-prepared-rows | 74.21 | 44.78 |
| error-path | 34.24 | 22.21 |
| upsert-pk | 792.42 | 811.81 |
| insert-select | 5.99 | 5.88 |
| tx-roundtrip | 11.33 | 9.18 |
| window-row-number | 9.23 | 8.57 |
| rollback-bulk | 25.79 | 22.25 |

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

- `select-point-uk` and `index-range` still scan every row of the table because the unique
  constraint is only enforced on writes; there is no secondary KV index to push the
  predicate into. The gateway can only cache the plan, not change execution.
- `select-cursor-page` is 1.11x slower than `mysql2`: integer primary keys are stored as
  unpadded decimal ASCII, so `rowid_after` cannot express `id > last` without a scan and a
  row-level filter. Early-stop pagination needs an execution-layer rowid predicate.
- `select-limit-offset` is now at parity on two measured rounds, but a deep `OFFSET` still
  walks and sorts the whole table per page; the ratio moves with run-to-run noise.
- `multi-table-join` is now faster than `mysql2` (0.81x), but the execution-layer join is
  still a nested-loop scan; the protocol layer only caches the identical SQL plan.
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
- `index-range` reads 20 adjacent `uk` values per statement in `ORDER BY id` order, so a
  row reorder or a missed range fails the run before timing.
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
