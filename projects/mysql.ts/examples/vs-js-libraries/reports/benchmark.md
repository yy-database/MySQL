# Benchmark report

Command: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
Defaults: `BENCH_ROWS=200`, `BENCH_POINT=500`, `BENCH_SCAN=1000`, `BENCH_ROUNDS=3`
Baseline: `mysql2` (`createConnection` + `query()`) = `1.00x`
Ratio: subject_time / baseline_time (smaller is faster). Fastest cell in each row is bold.
Each measurement starts a fresh `startMysqlServer` with a fresh `mkdtemp` data directory,
creates and seeds the table untimed, then times only the workload.

## Relative to `mysql2`

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| insert | 1.00x | **1.00x** |
| select-point | 1.00x | **1.00x** |
| select-scan | 1.00x | **0.98x** |

## Absolute (median per-round totals normalised per op)

| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |
|---------:|-----:|-------------:|-------------:|------------------:|------------------:|
| insert | statements | 10.52 | 95/s | 10.50 | 95/s |
| select-point | statements | 6.19 | 161/s | 6.18 | 162/s |
| select-scan | rows | 0.03 | 35,188/s | 0.03 | 36,045/s |

## p95 per-round total (ms)

| Workload | mysql2 | @yydb/mysql |
|---------:|-------:|------------:|
| insert | 2176.91 | 2205.55 |
| select-point | 3119.15 | 3093.13 |
| select-scan | 30.73 | 27.76 |

## Intentional deltas

- Both clients use one connection over loopback to the same product server and issue the
  same SQL through the text protocol. No pooling, prepared statements or TLS on either side.
- `@yydb/mysql` is a hand-written classic-protocol client built on the product's native
  packet codec. It reads text rows into arrays and does no type coercion.
- `mysql2` is the full driver and performs JS-side row parsing into typed objects.
- Because both clients traverse the same product server, the measured wall time is dominated
  by server-side work and ratios collapse towards parity. The benchmark still detects driver
  parse differences, which is where `select-scan` separates slightly.
- A fresh server plus untimed seed run precedes every measurement, so start-up and seed cost
  are excluded from the ratio.
- There is no prepared-statement workload because the minimal client does not implement the
  binary protocol.
- `select-scan` is one statement over 1,000 rows. Its throughput unit is rows, not statements.
- Counts are tuned down from the SQLite example because every insert and point select is a
  full round trip to a separately started product server.

Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
