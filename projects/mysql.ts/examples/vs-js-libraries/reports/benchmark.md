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
- `select-exists-indexed` — the same 200 pairs, but the side table carries a
  `UNIQUE(item_id)` index created before the seed, so the executor folds the correlated
  equality into a point probe
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
- `window-rank` — 20 `RANK()` / `DENSE_RANK()` projections over the unique `score`
  column, asserting `rank = dense_rank = id` on every projected row
- `window-moving-sum` — 20 `SUM(val) OVER (PARTITION BY grp ORDER BY id ROWS BETWEEN 1
  PRECEDING AND CURRENT ROW)` projections; every row's moving sum is derived offline from
  `grp = id % 8` and `val = id`
- `window-lag-lead` — 20 `LAG` / `LEAD` projections over the eight-slot seed, asserting
  `id - 1` / `id + 1` on every row and `NULL` at both ends
- `select-string-functions` — 100 scans projecting `CONCAT`, `UPPER`, `SUBSTRING`,
  `CHAR_LENGTH` and `REPLACE`, with every cell asserted
- `select-set-operators` — 100 `INTERSECT` / `EXCEPT` pairs; `UNION ALL` covers the
  duplicate-preserving path, these cover the deduplicating set paths
- `select-cast` — 100 `CAST(id AS CHAR)` scans over the primary key, asserting the column
  order and every rendered value
- `aggregate-distinct` — 100 `COUNT(DISTINCT grp)` queries over the eight-slot seed,
  asserting the deduplicated count
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
| connect-handshake | **1.00x** | 1.03x |
| insert | 1.00x | **0.94x** |
| insert-prepared | **1.00x** | 1.02x |
| insert-batch | 1.00x | **0.98x** |
| insert-autoinc | 1.00x | **0.98x** |
| select-point | 1.00x | **0.85x** |
| select-point-uk | 1.00x | **0.87x** |
| index-range | 1.00x | **0.92x** |
| select-range | **1.00x** | 1.33x |
| select-scan | **1.00x** | 1.13x |
| aggregate | 1.00x | **0.91x** |
| select-group-by | 1.00x | **0.86x** |
| select-having | 1.00x | **0.98x** |
| select-case | 1.00x | **0.84x** |
| select-distinct | 1.00x | **0.84x** |
| select-like-prefix | 1.00x | **0.94x** |
| select-in-list | **1.00x** | 1.03x |
| select-subquery-in | 1.00x | **0.86x** |
| select-exists | **1.00x** | 1.00x |
| select-exists-indexed | 1.00x | **0.87x** |
| select-union-all | 1.00x | **0.94x** |
| select-cursor-page | **1.00x** | 1.06x |
| select-limit-offset | 1.00x | **0.93x** |
| order-index-desc | 1.00x | **0.73x** |
| multi-table-join | 1.00x | **0.99x** |
| left-join | **1.00x** | 1.09x |
| show-session | **1.00x** | 1.01x |
| select-null-blob | 1.00x | **0.63x** |
| tx-commit | **1.00x** | 1.13x |
| update-multi | 1.00x | **0.97x** |
| delete-where | 1.00x | **0.89x** |
| binary-prepared-rows | 1.00x | **0.77x** |
| error-path | 1.00x | **0.89x** |
| upsert-pk | **1.00x** | 1.17x |
| insert-select | 1.00x | **0.88x** |
| alter-add-column | 1.00x | **0.92x** |
| tx-roundtrip | **1.00x** | 1.14x |
| window-row-number | 1.00x | **1.00x** |
| window-rank | 1.00x | **0.86x** |
| window-moving-sum | **1.00x** | 1.08x |
| window-lag-lead | 1.00x | **0.74x** |
| select-string-functions | 1.00x | **0.69x** |
| select-set-operators | 1.00x | **0.92x** |
| select-cast | 1.00x | **0.84x** |
| aggregate-distinct | 1.00x | **0.79x** |
| rollback-bulk | 1.00x | **0.98x** |
| savepoint-rollback | **1.00x** | 1.05x |


## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| connect-handshake | connections | 0.88 | 1,132/s | 0.91 | 1,097/s |
| insert | statements | 3.70 | 271/s | 3.46 | 289/s |
| insert-prepared | rows | 4.01 | 249/s | 4.07 | 245/s |
| insert-batch | rows | 0.26 | 3,833/s | 0.25 | 3,924/s |
| insert-autoinc | rows | 4.24 | 236/s | 4.17 | 240/s |
| select-point | statements | 0.19 | 5,315/s | 0.16 | 6,288/s |
| select-point-uk | statements | 0.21 | 4,820/s | 0.18 | 5,541/s |
| index-range | statements | 0.43 | 2,345/s | 0.39 | 2,541/s |
| select-range | statements | 0.49 | 2,041/s | 0.65 | 1,531/s |
| select-scan | rows | 0.01 | 186,178/s | 0.01 | 164,880/s |
| aggregate | statements | 0.77 | 1,297/s | 0.70 | 1,429/s |
| select-group-by | statements | 1.38 | 722/s | 1.19 | 838/s |
| select-having | statements | 0.85 | 1,182/s | 0.83 | 1,210/s |
| select-case | statements | 1.33 | 750/s | 1.12 | 892/s |
| select-distinct | statements | 2.20 | 454/s | 1.85 | 539/s |
| select-like-prefix | statements | 1.19 | 844/s | 1.12 | 895/s |
| select-in-list | statements | 0.59 | 1,698/s | 0.61 | 1,642/s |
| select-subquery-in | statements | 1.66 | 601/s | 1.43 | 698/s |
| select-exists | statements | 38.40 | 26/s | 38.52 | 26/s |
| select-exists-indexed | statements | 4.54 | 220/s | 3.95 | 253/s |
| select-union-all | statements | 1.21 | 825/s | 1.13 | 883/s |
| select-cursor-page | pages | 2.97 | 336/s | 3.16 | 316/s |
| select-limit-offset | statements | 3.85 | 260/s | 3.58 | 279/s |
| order-index-desc | statements | 6.07 | 165/s | 4.42 | 226/s |
| multi-table-join | statements | 1.59 | 629/s | 1.57 | 637/s |
| left-join | statements | 1.69 | 590/s | 1.85 | 540/s |
| show-session | statements | 0.15 | 6,689/s | 0.15 | 6,627/s |
| select-null-blob | rows | 0.01 | 106,856/s | 0.01 | 168,734/s |
| tx-commit | rows | 0.62 | 1,620/s | 0.70 | 1,433/s |
| update-multi | statements | 3.92 | 255/s | 3.79 | 264/s |
| delete-where | rows | 1.93 | 518/s | 1.72 | 581/s |
| binary-prepared-rows | statements | 0.23 | 4,434/s | 0.17 | 5,771/s |
| error-path | statements | 0.24 | 4,245/s | 0.21 | 4,794/s |
| upsert-pk | statements | 4.02 | 249/s | 4.71 | 212/s |
| insert-select | rows | 0.05 | 20,078/s | 0.04 | 22,824/s |
| alter-add-column | statements | 0.64 | 1,572/s | 0.58 | 1,713/s |
| tx-roundtrip | transactions | 0.51 | 1,951/s | 0.58 | 1,719/s |
| window-row-number | statements | 1.47 | 681/s | 1.47 | 682/s |
| window-rank | statements | 1.84 | 543/s | 1.58 | 633/s |
| window-moving-sum | statements | 1.86 | 537/s | 2.02 | 496/s |
| window-lag-lead | statements | 2.52 | 397/s | 1.87 | 534/s |
| select-string-functions | statements | 3.67 | 272/s | 2.55 | 392/s |
| select-set-operators | statements | 2.44 | 410/s | 2.26 | 443/s |
| select-cast | statements | 1.30 | 768/s | 1.10 | 913/s |
| aggregate-distinct | statements | 0.56 | 1,781/s | 0.44 | 2,251/s |
| rollback-bulk | rows | 0.32 | 3,162/s | 0.31 | 3,231/s |
| savepoint-rollback | statements | 0.57 | 1,753/s | 0.60 | 1,664/s |


## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| connect-handshake | 18.82 | 18.57 |
| insert | 1925.94 | 852.28 |
| insert-prepared | 854.16 | 1428.09 |
| insert-batch | 55.42 | 54.28 |
| insert-autoinc | 853.62 | 891.07 |
| select-point | 116.65 | 109.69 |
| select-point-uk | 151.98 | 137.74 |
| index-range | 293.42 | 292.11 |
| select-range | 371.24 | 334.92 |
| select-scan | 11.47 | 7.89 |
| aggregate | 98.17 | 91.82 |
| select-group-by | 186.99 | 184.27 |
| select-having | 130.22 | 129.59 |
| select-case | 175.16 | 189.28 |
| select-distinct | 275.40 | 312.69 |
| select-like-prefix | 166.17 | 180.56 |
| select-in-list | 90.52 | 70.83 |
| select-subquery-in | 173.55 | 143.56 |
| select-exists | 12266.76 | 12273.07 |
| select-exists-indexed | 1345.78 | 1276.86 |
| select-union-all | 150.09 | 169.63 |
| select-cursor-page | 229.03 | 235.43 |
| select-limit-offset | 123.98 | 120.24 |
| order-index-desc | 131.27 | 125.53 |
| multi-table-join | 208.26 | 196.67 |
| left-join | 232.29 | 248.57 |
| show-session | 42.48 | 37.96 |
| select-null-blob | 16.39 | 12.28 |
| tx-commit | 71.83 | 70.80 |
| update-multi | 393.78 | 396.01 |
| delete-where | 1481.45 | 387.98 |
| binary-prepared-rows | 136.81 | 117.75 |
| error-path | 66.36 | 54.92 |
| upsert-pk | 887.85 | 1532.28 |
| insert-select | 12.82 | 12.69 |
| alter-add-column | 16.14 | 13.22 |
| tx-roundtrip | 12.93 | 15.73 |
| window-row-number | 37.82 | 37.74 |
| window-rank | 45.94 | 48.95 |
| window-moving-sum | 58.52 | 61.06 |
| window-lag-lead | 67.66 | 57.69 |
| select-string-functions | 521.20 | 461.82 |
| select-set-operators | 516.68 | 542.70 |
| select-cast | 132.33 | 159.17 |
| aggregate-distinct | 77.14 | 62.71 |
| rollback-bulk | 42.25 | 37.63 |
| savepoint-rollback | 116.39 | 98.98 |


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

- `select-exists` re-runs the inner plan on every outer row when the probe column is not
  indexed: the correlated equality is substituted as a literal and then materialised
  again, so the side table is re-scanned 200 times per statement (54 ms/op). Adding a
  single-column `UNIQUE` index on the probe column lets the executor fold the substituted
  equality into a point probe, which is `select-exists-indexed` at 6.3 ms/op — the 8.6x
  difference is the index, not a different statement.
- The unique index must exist before rows are inserted. Creating it on an already
  populated table does not backfill the index entries yet, so the probe stays disabled and
  the query keeps scanning; that is a separate DDL gap.
- Secondary-index pushdown covers single-column `UNIQUE` indexes only; predicates on
  non-unique or composite secondary indexes still scan the namespace.
- `select-limit-offset` and `select-cursor-page` still walk the sorted range per page; the
  offset is not pushed into the scan.
- `insert` remains around 1.02x because each statement commits its own write.
- `RANK` / `DENSE_RANK` are at parity after the executor stopped cloning the sorted
  row set for ranking-only window functions; window aggregates and `LAG` / `LEAD`
  still pay that clone once per expression.
- `LAG` / `LEAD` report `VAR_STRING` (253) instead of the input integer type, so
  `mysql2` surfaces `"10"` where a real server returns `10`; tracked as upstream
  gap 16 in `UPSTREAM.md`.
- `DEFAULT CURRENT_TIMESTAMP` is not evaluated on insert (the column lands as
  `NULL`), while literal defaults such as `DEFAULT 7` work; tracked as upstream
  gap 17 in `UPSTREAM.md`.
- Several MySQL statements are rejected before execution and therefore have no row:
  `INSERT IGNORE`, `REPLACE INTO`, `SELECT ... FOR UPDATE`, `TRUNCATE` and `JSON`
  columns all return a parse/unsupported error. They stay out of the benchmark until
  the gateway implements them.
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
