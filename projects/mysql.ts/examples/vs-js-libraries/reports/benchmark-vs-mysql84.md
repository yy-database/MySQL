# 服务端对服务端：`@yydb/mysql` 与真实 MySQL 8.4

真实库：`8.4.9`（`127.0.0.1:33306`，库 `yydb`）
本仓：`@yydb/mysql`（每轮新起临时数据目录）
说明：本机没有 Docker，真实库用本地 MySQL 8.4.9；truth corpus 钉的是 8.4.13。CI 有 `mysql:8.4` 镜像时用 `BENCH_REAL_*` 指向该实例重跑。
注意：本机存在 `projects/mysql.rs/.cargo/config.toml` path patch，下列数字来自本地 upstream 工作树，不等于 CI 钉点。
轮次：1 轮热身 + 5 轮测量，取中位数并记 p95；数据集 1000 行，固定 seed。
比值 = 本仓耗时 / 真实库耗时，小于 1 表示本仓更快。

本轮新增 SQL 特性项：400 项 `IN` 列表、深 `OFFSET` 分页、`LIKE` 前缀、宽投影（`SELECT *`）。`ON DUPLICATE KEY` 仍在解析层缺失，暂不作为基准项。

## 按差距排序

| 工作负载 | 单位 | 真实库 ms/op | 本仓 ms/op | 倍数 |
|---------:|-----:|-------------:|-----------:|-----:|
| update-many | rows | 0.040 | 0.531 | 13.19x |
| wide-select | statements | 0.115 | 1.418 | 12.36x |
| index-range | statements | 0.142 | 1.211 | 8.52x |
| range-limit | statements | 0.128 | 0.801 | 6.24x |
| like-prefix | statements | 0.321 | 1.345 | 4.19x |
| offset-limit | statements | 0.192 | 0.793 | 4.12x |
| join | statements | 0.388 | 1.589 | 4.09x |
| point-uk | statements | 0.082 | 0.322 | 3.94x |
| order-limit | statements | 0.291 | 1.076 | 3.69x |
| insert-multi | rows | 0.045 | 0.116 | 2.58x |
| aggregate | statements | 0.316 | 0.796 | 2.52x |
| group-by | statements | 0.454 | 1.082 | 2.38x |
| distinct | statements | 0.283 | 0.653 | 2.31x |
| in-list | statements | 0.537 | 1.147 | 2.14x |
| subquery | statements | 0.654 | 1.219 | 1.86x |
| insert-single | statements | 2.099 | 3.823 | 1.82x |
| tx-commit | rows | 0.419 | 0.541 | 1.29x |
| connect | connections | 0.636 | 0.797 | 1.25x |
| delete-many | rows | 0.040 | 0.042 | 1.05x |
| point-pk | statements | 0.085 | 0.083 | 0.98x |
| baseline-select1 | statements | 0.064 | 0.052 | 0.82x |
| rollback | rows | 0.521 | 0.178 | 0.34x |

## 绝对耗时与 p95

| 工作负载 | 真实库中位总耗时 ms | 本仓中位总耗时 ms | 真实库 p95 ms | 本仓 p95 ms |
|---------:|--------------------:|------------------:|--------------:|------------:|
| insert-single | 629.85 | 1146.86 | 685.82 | 1152.64 |
| insert-multi | 17.94 | 46.29 | 21.23 | 52.11 |
| point-pk | 25.55 | 24.93 | 30.66 | 52.57 |
| point-uk | 24.52 | 96.62 | 53.36 | 132.43 |
| range-limit | 12.84 | 80.15 | 13.26 | 86.54 |
| update-many | 80.56 | 1062.33 | 83.77 | 1087.45 |
| delete-many | 40.01 | 41.84 | 42.00 | 42.13 |
| order-limit | 14.56 | 53.78 | 14.64 | 61.70 |
| group-by | 22.68 | 54.09 | 28.92 | 60.09 |
| tx-commit | 125.80 | 162.42 | 126.44 | 172.96 |
| baseline-select1 | 19.06 | 15.70 | 23.35 | 23.45 |
| distinct | 14.17 | 32.66 | 15.13 | 38.48 |
| aggregate | 6.32 | 15.92 | 8.05 | 17.37 |
| subquery | 13.09 | 24.38 | 17.63 | 27.07 |
| index-range | 14.22 | 121.11 | 20.28 | 148.66 |
| join | 7.76 | 31.78 | 8.50 | 34.11 |
| in-list | 10.73 | 22.94 | 16.19 | 34.22 |
| offset-limit | 5.77 | 23.78 | 13.08 | 28.46 |
| like-prefix | 9.63 | 40.34 | 16.49 | 42.66 |
| wide-select | 3.44 | 42.55 | 5.47 | 51.06 |
| rollback | 5.21 | 1.78 | 7.31 | 2.95 |
| connect | 12.73 | 15.94 | 23.20 | 24.81 |

## 粗粒度阶段差分

线上协议不暴露解析/执行/存储/回包各阶段耗时，这里只做差分估计：以 `baseline-select1` 作为“传输 + 解析 + 回包”地板，其余项减去地板即该路径的额外成本。

| 路径 | 真实库额外 ms/语句 | 本仓额外 ms/语句 | 差额 ms/语句 |
|-----:|-------------------:|-----------------:|-------------:|
| insert-single | 2.036 | 3.771 | 1.735 |
| point-pk | 0.022 | 0.031 | 0.009 |
| point-uk | 0.018 | 0.270 | 0.252 |
| range-limit | 0.065 | 0.749 | 0.684 |
| update-many | 3.964 | 53.064 | 49.100 |
| group-by | 0.390 | 1.029 | 0.639 |
| order-limit | 0.228 | 1.023 | 0.796 |
| distinct | 0.220 | 0.601 | 0.381 |
| aggregate | 0.252 | 0.744 | 0.491 |
| subquery | 0.591 | 1.167 | 0.576 |
| index-range | 0.079 | 1.159 | 1.080 |
| join | 0.325 | 1.536 | 1.212 |
| in-list | 0.473 | 1.095 | 0.622 |
| offset-limit | 0.129 | 0.740 | 0.611 |
| like-prefix | 0.258 | 1.292 | 1.035 |
| wide-select | 0.051 | 1.366 | 1.315 |

## 根因假设与预计收益

- **每语句地板 0.052ms**：`baseline-select1` 本仓 0.052ms/语句（真库 0.064ms）。本轮未观测到历史 5ms 轮询地板，point-pk、range-limit、connect 已回到亚毫秒档。
- **唯一索引点查与主键同档**：`point-uk` 0.322ms/语句，`point-pk` 0.083ms/语句，差 0.239ms，本轮不构成唯一索引缺失的证据。
- **排序与分组全表扫描**：`order-limit` 1.08ms/语句、`group-by` 1.08ms/语句，各比地板多约 1.0-1.0ms，等于 1000 行扫描成本。索引下推与增量聚合后预计降到亚毫秒级。
- **单条写入不慢**：`insert-single` 本仓 3.82ms/语句、真库 2.10ms/语句，差距来自每语句地板与多行插入的逐行落盘（`insert-multi` 0.116 vs 0.045ms/行，2.58x）。批量路径改为一次提交可把每行成本压到真库同档。

## 语义差异（不计入性能比值）

整数列在 1..100 的小值区间上对比 `COUNT(*)`，本轮四项两库计数一致：

| 语句 | 真实库计数 | 本仓计数 |
|-----:|-----------:|--------:|
| `WHERE id BETWEEN 6 AND 46` | 41 | 41 |
| `WHERE id BETWEEN 2 AND 10` | 9 | 9 |
| `WHERE id >= 50` | 51 | 51 |
| `WHERE id > 6` | 94 | 94 |

Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark:mysql84`
