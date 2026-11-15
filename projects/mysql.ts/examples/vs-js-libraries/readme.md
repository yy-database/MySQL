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

- `connect-handshake` — `BENCH_CONNECT` extra connections opened and closed
  against the same fresh server while the measurement connection stays open
- `insert` — `BENCH_ROWS` sequential `INSERT` statements (text protocol)
- `insert-prepared` — one `INSERT ... VALUES (?, ?, ?)` prepared per client,
  then `BENCH_ROWS` `COM_STMT_EXECUTE` calls over a `BENCH_ROWS`-row table
- `insert-autoinc` — `BENCH_ROWS` inserts that omit the `AUTO_INCREMENT` primary
  key, so the server allocates the id and reports it as `insertId`; same row
  count and values as `insert`
- `insert-batch` — multi-row `INSERT` statements of `BENCH_BATCH_ROWS` rows each;
  the unit is rows
- `select-point` — `BENCH_POINT` primary-key equality `SELECT`s over `BENCH_ROWS` rows
- `select-point-uk` — the same `BENCH_POINT` count, same seed size, predicate
  moved to a `UNIQUE` secondary index (`uk`)
- `index-range` — `BENCH_POINT` ranged reads of 20 adjacent `uk` values per
  statement on the `UNIQUE` secondary index, ordered by `id`
- `select-range` — `BENCH_POINT` primary-key range reads of 20 adjacent ids per
  statement
- `select-scan` — one `SELECT ... ORDER BY id` over `BENCH_SCAN` rows
- `aggregate` — `BENCH_AGG` `COUNT`/`SUM`/`MIN`/`MAX` aggregates over the
  `BENCH_ROWS` seed
- `select-group-by` — `BENCH_AGG` `GROUP BY label` queries; `label` is unique,
  so every row is its own group
- `select-having` — `BENCH_AGG` `GROUP BY grp HAVING SUM(val) >` queries over an
  eight-slot seed (`grp = id % 8`, `val = id`); the threshold is the median group
  sum, so the surviving group set and per-group counts are asserted
- `select-case` — `BENCH_AGG` select-list `CASE WHEN score > t THEN 'hi' ELSE
  'lo' END` scans over the `BENCH_ROWS` seed; `t` cycles `1..4` and every row's
  band plus the `hi` count is asserted
- `select-distinct` — `BENCH_AGG` `SELECT DISTINCT label` queries over the same seed
- `select-like-prefix` — `BENCH_AGG` prefix filters (`label LIKE 'row-1%'`),
  asserting the exact match count
- `select-in-list` — `BENCH_AGG` fixed 17-value `IN` lists over the primary key
- `select-subquery-in` — `BENCH_AGG` `IN (SELECT ...)` uncorrelated subqueries
- `select-exists` — `BENCH_AGG` correlated `EXISTS` / `NOT EXISTS` pairs against
  a side table that only holds odd item ids, so both counts are exact
- `select-exists-indexed` — the same pair of statements, but the side table carries
  a `UNIQUE(item_id)` index created before the seed, so the executor can fold the
  correlated equality into a point probe
- `select-union-all` — `BENCH_AGG` two `SELECT`s over disjoint key halves joined
  with `UNION ALL`, asserting the full row count
- `select-cursor-page` — keyset pagination (`WHERE id > last ORDER BY id LIMIT 20`)
  over the same `BENCH_SCAN` rows; the unit is pages
- `select-limit-offset` — `BENCH_PAGE` deep `LIMIT 10 OFFSET n` pages over the
  same `BENCH_SCAN` rows, with the offset cycling so both clients issue
  identical SQL
- `order-index-desc` — the same `BENCH_PAGE` cycled offsets walked backwards
  (`ORDER BY id DESC LIMIT 10 OFFSET n`); the first and last id of every page are
  asserted
- `multi-table-join` — `BENCH_JOIN` joins of two physical tables
  (`bench_items JOIN bench_side ON item_id = id`) returning 20 rows, over the
  same `BENCH_ROWS` seed
- `left-join` — the same two-table shape with a sparse side table (odd item ids
  only), so the even ids must come back NULL-padded; row order, weights and the
  exact NULL count are asserted
- `show-session` — `BENCH_SHOW` `SHOW VARIABLES LIKE 'version'` round trips,
  covering the session-command path that returns a synthesized result set
- `select-null-blob` — one scan over `BENCH_SCAN` rows with a `NULL` note every
  third row and a 2 KiB payload every fifth row; the unit is rows
- `tx-commit` — `BENCH_TX` transactions of `BENCH_TX_ROWS` inserts, one `COMMIT` each
- `update-multi` — `BENCH_MUTATE` statements, each updating the same four-row
  primary-key range (`WHERE id BETWEEN lo AND hi`) of the `BENCH_ROWS` seed
- `delete-where` — `BENCH_MUTATE` paired-range `DELETE ... WHERE id BETWEEN lo
  AND hi` statements over the same seed; the unit is rows removed
- `binary-prepared-rows` — one `SELECT id, label FROM bench_items WHERE id = ?`
  prepared per client, then `BENCH_POINT` binary-protocol executes over the
  same `BENCH_ROWS` seed as `select-point`
- `error-path` — `BENCH_ERROR` duplicate-primary-key inserts (errno 1062) and
  `BENCH_ERROR` unknown-table selects (errno 1146) on one connection, verifying
  the gateway keeps the stream synchronised after an error
- `rollback-bulk` — `BENCH_TX` transactions of `BENCH_TX_ROWS` inserts, one
  `ROLLBACK` each; the same row ids are reinserted every transaction
- `savepoint-rollback` — `BENCH_SAVEPOINT` transactions that insert, update, roll
  the update back to a savepoint, insert again and commit; the surviving two rows
  per transaction and the rolled-back value are asserted
- `upsert-pk` — `BENCH_ROWS` `INSERT ... ON DUPLICATE KEY UPDATE` statements
  whose primary key always collides, so every statement rewrites one seeded row
  and `VALUES(label)` must win
- `insert-select` — one `INSERT ... SELECT` copying the `BENCH_ROWS` seed into a
  fresh table, then a `COUNT(*)` assertion; the unit is rows
- `alter-add-column` — `BENCH_ALTER` `ALTER TABLE ... ADD COLUMN cN INT DEFAULT
  d` statements, each followed by a read-back of the seeded row so the default
  must land on rows that already existed
- `tx-roundtrip` — `BENCH_TX` empty `BEGIN`/`COMMIT` plus `BEGIN`/`ROLLBACK`
  pairs, measuring only transaction round trips
- `window-row-number` — `BENCH_WINDOW` `ROW_NUMBER() OVER (ORDER BY id)`
  projections limited to 20 rows over the `BENCH_ROWS` seed
- `window-rank` — `BENCH_WINDOW` `RANK()` / `DENSE_RANK()` projections over the
  unique `score` column, asserting `rank = dense_rank = id` on every row

```shell
pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark
```

Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`,
`BENCH_ROUNDS=3`, `BENCH_TX=10`, `BENCH_TX_ROWS=10`, `BENCH_MUTATE=100`,
`BENCH_ERROR=100`, `BENCH_PAGE=20`, `BENCH_JOIN=100`, `BENCH_SHOW=200`,
`BENCH_WINDOW=20`, `BENCH_ALTER=10`.
Override them through the environment. `BENCH_ONLY=name1,name2` narrows a run to
the named workloads. The report is in
[`reports/benchmark.md`](./reports/benchmark.md).

`SHOW STATUS` is not part of the gateway surface (`1064`), so the session-command
round trip uses `SHOW VARIABLES` instead. MySQL has no `PRAGMA synchronous`
equivalent, so `commit-sync-on` is not applicable to this product and is not
benchmarked.

## Intentional deltas

- Both clients hit the same product server over loopback; the minimal client is
  hand-written and text/binary only where the workload needs it, while `mysql2`
  is the full driver with JS-side row parsing.
- No pooling or TLS on either side.
- A fresh server plus untimed seed precedes every measurement.
- Only `insert-prepared` and `binary-prepared-rows` use the binary protocol;
  every other workload uses `COM_QUERY` with the same SQL for both clients.
- `select-scan` and `select-null-blob` throughput is reported in rows, and
  `select-cursor-page` in pages, because each measurement is one statement over
  many rows or several statements over one query shape.
- `update-multi` and `delete-where` cycle over the same seed ranges so both
  clients issue identical SQL and touch identical rows.
- `error-path` asserts the exact MySQL errno (`1062`, `1146`) on both clients and
  keeps using the connection afterwards; a desynchronised stream fails the run
  instead of being timed.
- `binary-prepared-rows` uses the same 200-row seed and 500 point lookups as
  `select-point`; only the protocol and prepare-time description change.
- `insert-autoinc` asserts the generated id reported by both clients; a client
  that drops `insertId` fails the run before it is timed.
- `select-limit-offset` cycles the offset deterministically so the page never
  runs past the seed and both clients issue the same statements.
- `multi-table-join` reads two physical tables; the second table holds exactly
  one row per item id, so the join cardinality is 20 for both clients.
- `show-session` is a text-protocol round trip on the session-command path. It
  uses `SHOW VARIABLES` because `SHOW STATUS` is rejected with `1064`.
