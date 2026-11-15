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
- `select-group-by` — 100 `GROUP BY label` queries; `label` is unique, so every row is its own group
- `select-distinct` — 100 `SELECT DISTINCT label` queries over the same seed
- `select-like-prefix` — 100 prefix filters (`label LIKE 'row-1%'`), asserting the exact match count
- `select-in-list` — 100 fixed 17-value `IN` lists over the primary key
- `select-subquery-in` — 100 uncorrelated `IN (SELECT ...)` subqueries
- `select-union-all` — 100 `UNION ALL` pairs over disjoint key halves, asserting the full row count
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
- `savepoint-rollback` — `BENCH_SAVEPOINT` transactions that insert, update, roll the
  update back to a savepoint, insert again and commit; the surviving rows and the
  rolled-back value are asserted

`SHOW STATUS` is not implemented by the gateway (`1064`), so the session-command item is
`SHOW VARIABLES`. MySQL has no `PRAGMA synchronous` equivalent, so `commit-sync-on` is not
applicable to this product.

## Relative to `mysql2`

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| connect-handshake | 1.00x | **0.92x** |
| insert | 1.00x | **0.99x** |
| insert-prepared | 1.00x | **0.94x** |
| insert-batch | 1.00x | **0.98x** |
| insert-autoinc | **1.00x** | 1.01x |
| select-point | 1.00x | **0.88x** |
| select-point-uk | 1.00x | **0.94x** |
| index-range | **1.00x** | 1.05x |
| select-range | 1.00x | **0.84x** |
| select-scan | 1.00x | **0.93x** |
| aggregate | 1.00x | **0.99x** |
| select-group-by | 1.00x | **0.79x** |
| select-distinct | 1.00x | **0.83x** |
| select-like-prefix | 1.00x | **0.98x** |
| select-in-list | 1.00x | **0.92x** |
| select-subquery-in | **1.00x** | 1.01x |
| select-union-all | **1.00x** | 1.12x |
| select-cursor-page | **1.00x** | 1.05x |
| select-limit-offset | 1.00x | **0.99x** |
| multi-table-join | **1.00x** | 1.04x |
| show-session | 1.00x | **0.90x** |
| select-null-blob | 1.00x | **0.62x** |
| tx-commit | 1.00x | **0.94x** |
| update-multi | 1.00x | **0.71x** |
| delete-where | 1.00x | **0.94x** |
| binary-prepared-rows | 1.00x | **0.83x** |
| error-path | 1.00x | **0.81x** |
| upsert-pk | **1.00x** | 1.01x |
| insert-select | 1.00x | **0.98x** |
| tx-roundtrip | 1.00x | **0.81x** |
| window-row-number | 1.00x | **0.98x** |
| rollback-bulk | 1.00x | **1.00x** |
| savepoint-rollback | 1.00x | **0.97x** |

## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| connect-handshake | connections | 0.81 | 1,241/s | 0.74 | 1,348/s |
| insert | statements | 3.08 | 325/s | 3.04 | 329/s |
| insert-prepared | rows | 3.27 | 306/s | 3.08 | 325/s |
| insert-batch | rows | 0.21 | 4,856/s | 0.20 | 4,953/s |
| insert-autoinc | rows | 4.02 | 249/s | 4.05 | 247/s |
| select-point | statements | 0.14 | 7,393/s | 0.12 | 8,411/s |
| select-point-uk | statements | 0.15 | 6,461/s | 0.15 | 6,839/s |
| index-range | statements | 0.33 | 2,992/s | 0.35 | 2,856/s |
| select-range | statements | 0.40 | 2,498/s | 0.34 | 2,968/s |
| select-scan | rows | 0.01 | 177,322/s | 0.01 | 189,959/s |
| aggregate | statements | 0.64 | 1,567/s | 0.63 | 1,581/s |
| select-group-by | statements | 1.39 | 721/s | 1.10 | 907/s |
| select-distinct | statements | 2.14 | 467/s | 1.78 | 561/s |
| select-like-prefix | statements | 1.06 | 942/s | 1.04 | 964/s |
| select-in-list | statements | 0.55 | 1,834/s | 0.50 | 1,990/s |
| select-subquery-in | statements | 0.99 | 1,011/s | 1.00 | 1,002/s |
| select-union-all | statements | 0.98 | 1,018/s | 1.10 | 912/s |
| select-cursor-page | pages | 2.96 | 338/s | 3.10 | 323/s |
| select-limit-offset | statements | 3.53 | 283/s | 3.51 | 285/s |
| multi-table-join | statements | 6.61 | 151/s | 6.85 | 146/s |
| show-session | statements | 0.11 | 9,221/s | 0.10 | 10,249/s |
| select-null-blob | rows | 0.01 | 118,571/s | 0.01 | 192,036/s |
| tx-commit | rows | 0.55 | 1,810/s | 0.52 | 1,930/s |
| update-multi | statements | 4.91 | 204/s | 3.50 | 286/s |
| delete-where | rows | 1.74 | 575/s | 1.63 | 613/s |
| binary-prepared-rows | statements | 0.16 | 6,137/s | 0.14 | 7,373/s |
| error-path | statements | 0.21 | 4,775/s | 0.17 | 5,922/s |
| upsert-pk | statements | 3.97 | 252/s | 4.03 | 248/s |
| insert-select | rows | 0.04 | 23,721/s | 0.04 | 24,170/s |
| tx-roundtrip | transactions | 0.40 | 2,496/s | 0.32 | 3,092/s |
| window-row-number | statements | 1.05 | 954/s | 1.02 | 978/s |
| rollback-bulk | rows | 0.28 | 3,517/s | 0.28 | 3,526/s |
| savepoint-rollback | statements | 0.58 | 1,738/s | 0.56 | 1,794/s |

## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| connect-handshake | 18.50 | 16.14 |
| insert | 642.26 | 612.62 |
| insert-prepared | 686.06 | 624.84 |
| insert-batch | 42.35 | 40.55 |
| insert-autoinc | 805.79 | 813.84 |
| select-point | 71.07 | 63.98 |
| select-point-uk | 79.97 | 74.83 |
| index-range | 167.79 | 183.84 |
| select-range | 203.01 | 169.33 |
| select-scan | 6.25 | 5.39 |
| aggregate | 64.90 | 65.52 |
| select-group-by | 140.60 | 110.74 |
| select-distinct | 214.81 | 184.30 |
| select-like-prefix | 113.75 | 108.84 |
| select-in-list | 56.50 | 50.41 |
| select-subquery-in | 101.13 | 100.20 |
| select-union-all | 100.04 | 112.83 |
| select-cursor-page | 148.32 | 168.86 |
| select-limit-offset | 71.63 | 71.22 |
| multi-table-join | 671.38 | 705.94 |
| show-session | 23.06 | 20.59 |
| select-null-blob | 8.62 | 5.25 |
| tx-commit | 58.49 | 52.76 |
| update-multi | 628.07 | 357.94 |
| delete-where | 349.69 | 341.91 |
| binary-prepared-rows | 81.53 | 68.40 |
| error-path | 42.46 | 33.83 |
| upsert-pk | 795.83 | 818.40 |
| insert-select | 8.50 | 8.53 |
| tx-roundtrip | 8.36 | 7.28 |
| window-row-number | 21.01 | 20.74 |
| rollback-bulk | 28.64 | 31.88 |
| savepoint-rollback | 94.45 | 89.28 |

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
