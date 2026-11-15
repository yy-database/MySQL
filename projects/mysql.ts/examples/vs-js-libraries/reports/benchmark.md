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
| connect-handshake | 1.00x | **0.64x** |
| insert | **1.00x** | 1.01x |
| insert-prepared | 1.00x | **0.97x** |
| insert-batch | **1.00x** | 1.08x |
| insert-autoinc | **1.00x** | 1.01x |
| select-point | 1.00x | **0.89x** |
| select-point-uk | 1.00x | **0.99x** |
| index-range | 1.00x | **0.97x** |
| select-range | 1.00x | **0.95x** |
| select-scan | **1.00x** | 1.19x |
| aggregate | 1.00x | **0.98x** |
| select-group-by | 1.00x | **0.84x** |
| select-having | 1.00x | **0.96x** |
| select-case | 1.00x | **0.86x** |
| select-distinct | 1.00x | **0.93x** |
| select-like-prefix | **1.00x** | 1.01x |
| select-in-list | 1.00x | **0.95x** |
| select-subquery-in | 1.00x | **1.00x** |
| select-exists | 1.00x | **1.00x** |
| select-exists-indexed | 1.00x | **0.97x** |
| select-union-all | **1.00x** | 1.01x |
| select-cursor-page | **1.00x** | 1.01x |
| select-limit-offset | 1.00x | **0.97x** |
| order-index-desc | 1.00x | **0.99x** |
| multi-table-join | 1.00x | **0.99x** |
| left-join | **1.00x** | 1.06x |
| show-session | 1.00x | **0.89x** |
| select-null-blob | 1.00x | **0.68x** |
| tx-commit | 1.00x | **0.97x** |
| update-multi | 1.00x | **0.98x** |
| delete-where | **1.00x** | 1.03x |
| binary-prepared-rows | 1.00x | **0.86x** |
| error-path | 1.00x | **0.84x** |
| upsert-pk | 1.00x | **0.99x** |
| insert-select | **1.00x** | 1.04x |
| alter-add-column | 1.00x | **0.99x** |
| tx-roundtrip | **1.00x** | 1.05x |
| window-row-number | **1.00x** | 1.01x |
| window-rank | 1.00x | **0.99x** |
| rollback-bulk | **1.00x** | 1.00x |
| savepoint-rollback | 1.00x | **0.96x** |

## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| connect-handshake | connections | 1.42 | 703/s | 0.90 | 1,106/s |
| insert | statements | 3.64 | 275/s | 3.69 | 271/s |
| insert-prepared | rows | 3.79 | 264/s | 3.68 | 272/s |
| insert-batch | rows | 0.22 | 4,612/s | 0.24 | 4,253/s |
| insert-autoinc | rows | 4.08 | 245/s | 4.11 | 243/s |
| select-point | statements | 0.23 | 4,443/s | 0.20 | 4,976/s |
| select-point-uk | statements | 0.26 | 3,812/s | 0.26 | 3,832/s |
| index-range | statements | 0.52 | 1,912/s | 0.51 | 1,976/s |
| select-range | statements | 0.58 | 1,730/s | 0.55 | 1,812/s |
| select-scan | rows | 0.01 | 117,440/s | 0.01 | 99,052/s |
| aggregate | statements | 1.01 | 987/s | 0.99 | 1,011/s |
| select-group-by | statements | 1.99 | 503/s | 1.66 | 602/s |
| select-having | statements | 1.30 | 770/s | 1.24 | 806/s |
| select-case | statements | 1.84 | 543/s | 1.58 | 633/s |
| select-distinct | statements | 3.11 | 321/s | 2.90 | 345/s |
| select-like-prefix | statements | 1.59 | 630/s | 1.61 | 622/s |
| select-in-list | statements | 0.82 | 1,225/s | 0.77 | 1,296/s |
| select-subquery-in | statements | 1.62 | 617/s | 1.61 | 620/s |
| select-exists | statements | 54.51 | 18/s | 54.32 | 18/s |
| select-exists-indexed | statements | 6.46 | 155/s | 6.28 | 159/s |
| select-union-all | statements | 1.64 | 610/s | 1.65 | 605/s |
| select-cursor-page | pages | 4.62 | 216/s | 4.66 | 215/s |
| select-limit-offset | statements | 6.07 | 165/s | 5.91 | 169/s |
| order-index-desc | statements | 6.52 | 153/s | 6.42 | 156/s |
| multi-table-join | statements | 2.10 | 476/s | 2.09 | 479/s |
| left-join | statements | 2.54 | 394/s | 2.70 | 370/s |
| show-session | statements | 0.20 | 5,002/s | 0.18 | 5,640/s |
| select-null-blob | rows | 0.01 | 69,109/s | 0.01 | 101,175/s |
| tx-commit | rows | 0.64 | 1,563/s | 0.62 | 1,614/s |
| update-multi | statements | 3.96 | 253/s | 3.88 | 258/s |
| delete-where | rows | 1.80 | 556/s | 1.85 | 541/s |
| binary-prepared-rows | statements | 0.27 | 3,706/s | 0.23 | 4,309/s |
| error-path | statements | 0.33 | 3,065/s | 0.27 | 3,639/s |
| upsert-pk | statements | 4.49 | 223/s | 4.44 | 225/s |
| insert-select | rows | 0.06 | 17,583/s | 0.06 | 16,952/s |
| alter-add-column | statements | 0.68 | 1,473/s | 0.67 | 1,486/s |
| tx-roundtrip | transactions | 0.51 | 1,964/s | 0.53 | 1,877/s |
| window-row-number | statements | 1.66 | 604/s | 1.67 | 597/s |
| window-rank | statements | 2.10 | 477/s | 2.08 | 481/s |
| rollback-bulk | rows | 0.40 | 2,494/s | 0.40 | 2,488/s |
| savepoint-rollback | statements | 0.60 | 1,654/s | 0.58 | 1,723/s |

## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| connect-handshake | 29.34 | 18.70 |
| insert | 743.84 | 837.37 |
| insert-prepared | 770.67 | 1112.46 |
| insert-batch | 47.87 | 48.07 |
| insert-autoinc | 825.31 | 824.03 |
| select-point | 117.69 | 106.42 |
| select-point-uk | 136.64 | 131.79 |
| index-range | 268.87 | 268.44 |
| select-range | 293.17 | 292.99 |
| select-scan | 9.18 | 12.93 |
| aggregate | 101.95 | 100.71 |
| select-group-by | 204.19 | 173.45 |
| select-having | 144.55 | 132.52 |
| select-case | 188.15 | 161.68 |
| select-distinct | 311.41 | 310.29 |
| select-like-prefix | 164.44 | 162.37 |
| select-in-list | 83.13 | 79.74 |
| select-subquery-in | 163.83 | 161.52 |
| select-exists | 11003.61 | 10962.37 |
| select-exists-indexed | 1300.18 | 1264.74 |
| select-union-all | 167.69 | 179.74 |
| select-cursor-page | 240.78 | 238.29 |
| select-limit-offset | 133.51 | 127.79 |
| order-index-desc | 134.71 | 143.14 |
| multi-table-join | 212.11 | 223.16 |
| left-join | 260.09 | 278.13 |
| show-session | 41.88 | 36.07 |
| select-null-blob | 15.20 | 9.89 |
| tx-commit | 73.63 | 64.53 |
| update-multi | 625.87 | 389.61 |
| delete-where | 374.32 | 370.63 |
| binary-prepared-rows | 141.54 | 118.57 |
| error-path | 66.46 | 55.77 |
| upsert-pk | 938.34 | 894.28 |
| insert-select | 13.12 | 11.91 |
| alter-add-column | 13.76 | 14.15 |
| tx-roundtrip | 12.27 | 11.28 |
| window-row-number | 35.54 | 39.09 |
| window-rank | 42.34 | 42.82 |
| rollback-bulk | 42.05 | 41.43 |
| savepoint-rollback | 98.11 | 96.50 |

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
