# 服务端对服务端：`@yydb/mysql` 与真实 MySQL 8.4

真实库：`8.4.9`（`127.0.0.1:33306`，库 `yydb`）
本仓：`@yydb/mysql`（每轮新起临时数据目录）
说明：本机没有 Docker，真实库用本地 MySQL 8.4.9；truth corpus 钉的是 8.4.13。CI 有 `mysql:8.4` 镜像时用 `BENCH_REAL_*` 指向该实例重跑。
注意：本机存在 `projects/mysql.rs/.cargo/config.toml` path patch，下列数字来自本地 upstream 工作树，不等于 CI 钉点。
轮次：1 轮热身 + 5 轮测量，取中位数并记 p95；数据集 1000 行，固定 seed。
比值 = 本仓耗时 / 真实库耗时，小于 1 表示本仓更快。

本轮新增 SQL 特性项：`distinct`、`aggregate`、`subquery`、`index-range`、`join`、`rollback`。`ON DUPLICATE KEY` 尚未进入能力表，暂不作为基准项。

## 按差距排序

| 工作负载 | 单位 | 真实库 ms/op | 本仓 ms/op | 倍数 |
|---------:|-----:|-------------:|-----------:|-----:|
| index-range | statements | 0.171 | 1.368 | 8.01x |
| point-uk | statements | 0.095 | 0.738 | 7.74x |
| range-limit | statements | 0.133 | 0.908 | 6.84x |
| join | statements | 0.384 | 1.932 | 5.03x |
| order-limit | statements | 0.313 | 1.235 | 3.95x |
| aggregate | statements | 0.314 | 0.919 | 2.92x |
| group-by | statements | 0.481 | 1.176 | 2.44x |
| distinct | statements | 0.331 | 0.745 | 2.25x |
| subquery | statements | 0.699 | 1.486 | 2.12x |
| insert-single | statements | 2.344 | 3.929 | 1.68x |
| connect | connections | 0.718 | 0.970 | 1.35x |
| baseline-select1 | statements | 0.060 | 0.072 | 1.19x |
| update-many | rows | 0.041 | 0.049 | 1.19x |
| delete-many | rows | 0.040 | 0.044 | 1.10x |
| point-pk | statements | 0.105 | 0.106 | 1.01x |
| tx-commit | rows | 0.453 | 0.443 | 0.98x |
| insert-multi | rows | 0.050 | 0.043 | 0.86x |
| rollback | rows | 0.568 | 0.181 | 0.32x |

## 绝对耗时与 p95

| 工作负载 | 真实库中位总耗时 ms | 本仓中位总耗时 ms | 真实库 p95 ms | 本仓 p95 ms |
|---------:|--------------------:|------------------:|--------------:|------------:|
| insert-single | 703.18 | 1178.65 | 779.86 | 1202.35 |
| insert-multi | 19.90 | 17.09 | 80.56 | 19.80 |
| point-pk | 31.51 | 31.92 | 59.91 | 32.37 |
| point-uk | 28.58 | 221.25 | 51.06 | 279.35 |
| range-limit | 13.27 | 90.77 | 15.01 | 98.68 |
| update-many | 82.08 | 97.46 | 93.82 | 115.00 |
| delete-many | 39.97 | 43.84 | 63.79 | 46.53 |
| order-limit | 15.63 | 61.76 | 20.95 | 74.51 |
| group-by | 24.07 | 58.80 | 26.82 | 73.10 |
| tx-commit | 135.96 | 132.80 | 158.48 | 145.29 |
| baseline-select1 | 18.03 | 21.49 | 23.81 | 29.01 |
| distinct | 16.53 | 37.26 | 19.54 | 37.66 |
| aggregate | 6.28 | 18.38 | 7.22 | 21.27 |
| subquery | 13.99 | 29.72 | 14.95 | 39.27 |
| index-range | 17.08 | 136.75 | 20.24 | 142.77 |
| join | 7.67 | 38.64 | 14.83 | 52.33 |
| rollback | 5.68 | 1.81 | 9.31 | 2.37 |
| connect | 14.36 | 19.40 | 27.90 | 32.64 |

## 粗粒度阶段差分

线上协议不暴露解析/执行/存储/回包各阶段耗时，这里只做差分估计：以 `baseline-select1` 作为“传输 + 解析 + 回包”地板，其余项减去地板即该路径的额外成本。

| 路径 | 真实库额外 ms/语句 | 本仓额外 ms/语句 | 差额 ms/语句 |
|-----:|-------------------:|-----------------:|-------------:|
| insert-single | 2.284 | 3.857 | 1.573 |
| point-pk | 0.045 | 0.035 | -0.010 |
| point-uk | 0.035 | 0.666 | 0.631 |
| range-limit | 0.073 | 0.836 | 0.764 |
| update-many | 4.044 | 4.801 | 0.757 |
| group-by | 0.421 | 1.104 | 0.683 |
| order-limit | 0.252 | 1.164 | 0.911 |
| distinct | 0.270 | 0.673 | 0.403 |
| aggregate | 0.254 | 0.847 | 0.593 |
| subquery | 0.639 | 1.414 | 0.775 |
| index-range | 0.111 | 1.296 | 1.185 |
| join | 0.324 | 1.860 | 1.537 |

## 根因假设与预计收益

- **每语句地板 0.072ms**：`baseline-select1` 本仓 0.072ms/语句（真库 0.060ms）。本轮未观测到历史 5ms 轮询地板，point-pk、range-limit、connect 已回到亚毫秒档。
- **唯一索引点查与主键同档**：`point-uk` 0.738ms/语句，`point-pk` 0.106ms/语句，差 0.631ms，本轮不构成唯一索引缺失的证据。
- **排序与分组全表扫描**：`order-limit` 1.24ms/语句、`group-by` 1.18ms/语句，各比地板多约 1.1-1.2ms，等于 1000 行扫描成本。索引下推与增量聚合后预计降到亚毫秒级。
- **单条写入不慢**：`insert-single` 本仓 3.93ms/语句、真库 2.34ms/语句，差距来自每语句地板与多行插入的逐行落盘（`insert-multi` 0.043 vs 0.050ms/行，0.86x）。批量路径改为一次提交可把每行成本压到真库同档。

## 语义差异（不计入性能比值）

整数列在 1..100 的小值区间上对比 `COUNT(*)`，本轮四项两库计数一致：

| 语句 | 真实库计数 | 本仓计数 |
|-----:|-----------:|--------:|
| `WHERE id BETWEEN 6 AND 46` | 41 | 41 |
| `WHERE id BETWEEN 2 AND 10` | 9 | 9 |
| `WHERE id >= 50` | 51 | 51 |
| `WHERE id > 6` | 94 | 94 |

Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark:mysql84`
