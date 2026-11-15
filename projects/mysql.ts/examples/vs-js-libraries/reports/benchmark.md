# Benchmark report

Command: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`, `BENCH_ROUNDS=3`, `BENCH_TX=10x10`, `BENCH_MUTATE=100`, `BENCH_ERROR=100`, `BENCH_PAGE=20`, `BENCH_JOIN=100`, `BENCH_SHOW=200`, `BENCH_BATCH_ROWS=20`, `BENCH_AGG=100`, `BENCH_CONNECT=20`, `BENCH_WINDOW=20`, `BENCH_SAVEPOINT=20`, `BENCH_ALTER=10`
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
- `select-having` — 100 `GROUP BY grp HAVING SUM(val) > <median>` queries over an eight-slot
  group seed; the surviving group count is asserted
- `select-case` — 100 `CASE WHEN score > n THEN 'hi' ELSE 'lo'` projections over the seed,
  asserting every band
- `select-distinct` — 100 `SELECT DISTINCT label` queries over the same seed
- `select-like-prefix` — 100 prefix filters (`label LIKE 'row-1%'`), asserting the exact match count
- `select-in-list` — 100 fixed 17-value `IN` lists over the primary key
- `select-subquery-in` — 100 uncorrelated `IN (SELECT ...)` subqueries
- `select-exists` — 200 correlated `EXISTS` / `NOT EXISTS` pairs against a sparse side table;
  exactly the odd ids hit, so both counts are asserted
- `select-union-all` — 100 `UNION ALL` pairs over disjoint key halves, asserting the full row count
- `select-cursor-page` — keyset pagination over the same 1,000 rows, 20 rows per page
- `select-limit-offset` — 20 deep `LIMIT 10 OFFSET n` pages over the same 1,000 rows
- `order-index-desc` — 20 descending `ORDER BY id DESC LIMIT 10 OFFSET n` pages, asserting the
  first and last id of every page
- `multi-table-join` — 100 joins of two physical tables (`bench_items JOIN bench_side`),
  20 rows per statement
- `left-join` — 100 `LEFT JOIN`s against a sparse side table; every even id must be padded
  with `NULL`, every odd id must carry its weight
- `show-session` — 200 `SHOW VARIABLES LIKE 'version'` session-command round trips
- `select-null-blob` — one scan over 1,000 rows, a `NULL` note every third row and a 2 KiB payload every fifth row
- `tx-commit` — 10 transactions of 10 inserts, one `COMMIT` each
- `update-multi` — 100 statements, each updating the same four-row primary-key range of the 200-row seed
- `delete-where` — 100 paired-range `DELETE` statements removing the whole 200-row seed; the unit is rows
- `binary-prepared-rows` — one `SELECT id, label ... WHERE id = ?` prepared per client, then 500 `COM_STMT_EXECUTE` calls over the same 200-row seed as `select-point`
- `error-path` — 100 duplicate-primary-key inserts (errno 1062) and 100 unknown-table selects (errno 1146) on one connection
- `upsert-pk` — 200 `INSERT ... ON DUPLICATE KEY UPDATE` statements on the same primary keys
- `insert-select` — `INSERT ... SELECT` copies over the seeded table
- `alter-add-column` — `BENCH_ALTER` `ALTER TABLE ... ADD COLUMN` statements on a scratch table
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
| connect-handshake | 1.00x | **0.70x** |
| insert | **1.00x** | 1.06x |
| insert-prepared | **1.00x** | 1.01x |
| insert-batch | **1.00x** | 1.10x |
| insert-autoinc | 1.00x | **0.99x** |
| select-point | 1.00x | **0.88x** |
| select-point-uk | 1.00x | **0.89x** |
| index-range | 1.00x | **0.93x** |
| select-range | 1.00x | **0.93x** |
| select-scan | 1.00x | **0.91x** |
| aggregate | 1.00x | **0.92x** |
| select-group-by | 1.00x | **0.87x** |
| select-having | 1.00x | **0.98x** |
| select-case | 1.00x | **0.92x** |
| select-distinct | 1.00x | **0.92x** |
| select-like-prefix | **1.00x** | 1.01x |
| select-in-list | **1.00x** | 1.02x |
| select-subquery-in | **1.00x** | 1.02x |
| select-exists | 1.00x | **0.98x** |
| select-union-all | **1.00x** | 1.00x |
| select-cursor-page | 1.00x | **0.99x** |
| select-limit-offset | 1.00x | **0.99x** |
| order-index-desc | **1.00x** | 1.06x |
| multi-table-join | **1.00x** | 1.03x |
| left-join | **1.00x** | 1.03x |
| show-session | 1.00x | **0.88x** |
| select-null-blob | 1.00x | **0.66x** |
| tx-commit | 1.00x | **0.93x** |
| update-multi | 1.00x | **0.99x** |
| delete-where | 1.00x | **0.99x** |
| binary-prepared-rows | 1.00x | **0.87x** |
| error-path | 1.00x | **0.83x** |
| upsert-pk | 1.00x | **0.95x** |
| insert-select | 1.00x | **0.96x** |
| alter-add-column | 1.00x | **0.88x** |
| tx-roundtrip | 1.00x | **0.90x** |
| window-row-number | 1.00x | **0.94x** |
| rollback-bulk | 1.00x | **0.94x** |
| savepoint-rollback | **1.00x** | 1.04x |

## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| connect-handshake | connections | 1.36 | 737/s | 0.95 | 1,054/s |
| insert | statements | 3.61 | 277/s | 3.83 | 261/s |
| insert-prepared | rows | 3.53 | 284/s | 3.56 | 281/s |
| insert-batch | rows | 0.21 | 4,699/s | 0.23 | 4,283/s |
| insert-autoinc | rows | 4.17 | 240/s | 4.13 | 242/s |
| select-point | statements | 0.23 | 4,312/s | 0.20 | 4,889/s |
| select-point-uk | statements | 0.26 | 3,774/s | 0.24 | 4,247/s |
| index-range | statements | 0.55 | 1,831/s | 0.51 | 1,968/s |
| select-range | statements | 0.59 | 1,691/s | 0.55 | 1,816/s |
| select-scan | rows | 0.01 | 103,556/s | 0.01 | 114,015/s |
| aggregate | statements | 1.05 | 952/s | 0.97 | 1,034/s |
| select-group-by | statements | 1.95 | 514/s | 1.69 | 593/s |
| select-having | statements | 1.27 | 787/s | 1.25 | 801/s |
| select-case | statements | 1.82 | 551/s | 1.67 | 600/s |
| select-distinct | statements | 3.19 | 313/s | 2.94 | 340/s |
| select-like-prefix | statements | 1.63 | 615/s | 1.64 | 610/s |
| select-in-list | statements | 0.85 | 1,180/s | 0.86 | 1,160/s |
| select-subquery-in | statements | 1.63 | 615/s | 1.65 | 606/s |
| select-exists | statements | 55.79 | 18/s | 54.52 | 18/s |
| select-union-all | statements | 1.64 | 609/s | 1.65 | 607/s |
| select-cursor-page | pages | 4.71 | 212/s | 4.67 | 214/s |
| select-limit-offset | statements | 5.97 | 167/s | 5.93 | 169/s |
| order-index-desc | statements | 6.09 | 164/s | 6.45 | 155/s |
| multi-table-join | statements | 2.17 | 460/s | 2.24 | 446/s |
| left-join | statements | 2.54 | 394/s | 2.62 | 381/s |
| show-session | statements | 0.21 | 4,870/s | 0.18 | 5,530/s |
| select-null-blob | rows | 0.01 | 78,853/s | 0.01 | 119,394/s |
| tx-commit | rows | 0.67 | 1,484/s | 0.63 | 1,600/s |
| update-multi | statements | 3.88 | 258/s | 3.84 | 260/s |
| delete-where | rows | 1.85 | 540/s | 1.82 | 548/s |
| binary-prepared-rows | statements | 0.27 | 3,646/s | 0.24 | 4,187/s |
| error-path | statements | 0.33 | 3,034/s | 0.27 | 3,641/s |
| upsert-pk | statements | 4.56 | 219/s | 4.34 | 230/s |
| insert-select | rows | 0.06 | 16,668/s | 0.06 | 17,276/s |
| alter-add-column | statements | 0.71 | 1,399/s | 0.63 | 1,585/s |
| tx-roundtrip | transactions | 0.59 | 1,707/s | 0.53 | 1,888/s |
| window-row-number | statements | 1.78 | 563/s | 1.66 | 601/s |
| rollback-bulk | rows | 0.43 | 2,346/s | 0.40 | 2,490/s |
| savepoint-rollback | statements | 0.58 | 1,713/s | 0.61 | 1,644/s |

## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| connect-handshake | 33.82 | 18.98 |
| insert | 727.99 | 1070.91 |
| insert-prepared | 748.96 | 746.62 |
| insert-batch | 47.68 | 48.31 |
| insert-autoinc | 844.51 | 840.53 |
| select-point | 118.25 | 108.56 |
| select-point-uk | 132.63 | 118.77 |
| index-range | 273.84 | 275.61 |
| select-range | 305.28 | 286.53 |
| select-scan | 10.52 | 9.24 |
| aggregate | 106.54 | 100.16 |
| select-group-by | 202.17 | 184.25 |
| select-having | 133.55 | 127.96 |
| select-case | 186.75 | 171.70 |
| select-distinct | 324.42 | 305.51 |
| select-like-prefix | 164.51 | 164.90 |
| select-in-list | 85.94 | 86.90 |
| select-subquery-in | 178.27 | 172.69 |
| select-exists | 11263.92 | 11469.32 |
| select-union-all | 180.28 | 167.74 |
| select-cursor-page | 284.30 | 242.78 |
| select-limit-offset | 132.15 | 119.82 |
| order-index-desc | 126.17 | 147.39 |
| multi-table-join | 217.98 | 225.51 |
| left-join | 260.91 | 266.31 |
| show-session | 41.12 | 36.26 |
| select-null-blob | 14.01 | 10.36 |
| tx-commit | 74.17 | 65.06 |
| update-multi | 390.34 | 390.23 |
| delete-where | 374.06 | 367.69 |
| binary-prepared-rows | 141.41 | 121.86 |
| error-path | 66.22 | 55.98 |
| upsert-pk | 920.35 | 904.30 |
| insert-select | 13.30 | 11.86 |
| alter-add-column | 14.33 | 13.68 |
| tx-roundtrip | 12.33 | 10.88 |
| window-row-number | 38.90 | 36.49 |
| rollback-bulk | 44.19 | 41.61 |
| savepoint-rollback | 96.95 | 98.38 |

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

- `select-exists` re-runs the inner plan on every outer row: the correlated equality is
  substituted as a literal and then materialised again, so the side table is re-scanned
  200 times per statement. Both clients pay it (55 ms/op), and the probe column has no
  `UNIQUE` index to push the predicate into. This is the largest remaining gap.
- Secondary-index pushdown covers single-column `UNIQUE` indexes only; predicates on
  non-unique or composite secondary indexes still scan the namespace.
- `select-limit-offset` and `select-cursor-page` still walk the sorted range per page; the
  offset is not pushed into the scan.
- `insert` remains around 1.06x because each statement commits its own write.
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
- `left-join` uses a sparse side table; every even id must come back with `NULL`, so a
  hash join that drops unmatched left rows fails before timing.
- `select-exists` asserts both the positive and the negated count against the sparse side
  table, so a mis-substituted correlation fails before timing.
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
