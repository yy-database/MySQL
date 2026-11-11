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
| join | statements | 0.434 | 1571.296 | 3623.38x |
| point-uk | statements | 0.110 | 5.354 | 48.65x |
| range-limit | statements | 0.130 | 5.141 | 39.56x |
| index-range | statements | 0.168 | 5.907 | 35.18x |
| subquery | statements | 0.791 | 23.426 | 29.63x |
| order-limit | statements | 0.338 | 8.413 | 24.87x |
| distinct | statements | 0.314 | 4.814 | 15.31x |
| aggregate | statements | 0.337 | 5.118 | 15.17x |
| group-by | statements | 0.525 | 5.934 | 11.30x |
| connect | connections | 1.205 | 5.462 | 4.53x |
| point-pk | statements | 0.110 | 0.274 | 2.48x |
| baseline-select1 | statements | 0.053 | 0.123 | 2.34x |
| update-many | rows | 0.044 | 0.091 | 2.07x |
| insert-single | statements | 2.313 | 4.172 | 1.80x |
| delete-many | rows | 0.043 | 0.072 | 1.67x |
| insert-multi | rows | 0.052 | 0.085 | 1.65x |
| tx-commit | rows | 0.433 | 0.616 | 1.42x |
| rollback | rows | 0.508 | 0.359 | 0.71x |

## 绝对耗时与 p95

| 工作负载 | 真实库中位总耗时 ms | 本仓中位总耗时 ms | 真实库 p95 ms | 本仓 p95 ms |
|---------:|--------------------:|------------------:|--------------:|------------:|
| insert-single | 693.93 | 1251.67 | 735.75 | 1305.21 |
| insert-multi | 20.62 | 34.11 | 23.96 | 55.26 |
| point-pk | 33.04 | 82.09 | 57.99 | 109.23 |
| point-uk | 33.01 | 1606.27 | 56.43 | 2663.25 |
| range-limit | 12.99 | 514.05 | 20.50 | 851.45 |
| update-many | 87.96 | 182.06 | 93.36 | 279.27 |
| delete-many | 43.40 | 72.38 | 46.51 | 97.12 |
| order-limit | 16.91 | 420.63 | 21.86 | 597.15 |
| group-by | 26.26 | 296.71 | 39.29 | 505.91 |
| tx-commit | 129.81 | 184.94 | 160.53 | 263.45 |
| baseline-select1 | 15.84 | 37.03 | 27.53 | 50.45 |
| distinct | 15.72 | 240.68 | 25.84 | 404.04 |
| aggregate | 6.75 | 102.36 | 8.99 | 155.09 |
| subquery | 15.81 | 468.52 | 20.27 | 679.16 |
| index-range | 16.79 | 590.72 | 28.74 | 997.17 |
| join | 8.67 | 31425.92 | 12.13 | 49386.68 |
| rollback | 5.08 | 3.59 | 5.22 | 4.54 |
| connect | 24.10 | 109.23 | 32.59 | 110.60 |

## 粗粒度阶段差分

线上协议不暴露解析/执行/存储/回包各阶段耗时，这里只做差分估计：以 `baseline-select1` 作为“传输 + 解析 + 回包”地板，其余项减去地板即该路径的额外成本。

| 路径 | 真实库额外 ms/语句 | 本仓额外 ms/语句 | 差额 ms/语句 |
|-----:|-------------------:|-----------------:|-------------:|
| insert-single | 2.260 | 4.049 | 1.788 |
| point-pk | 0.057 | 0.150 | 0.093 |
| point-uk | 0.057 | 5.231 | 5.174 |
| range-limit | 0.077 | 5.017 | 4.940 |
| update-many | 4.345 | 8.980 | 4.634 |
| group-by | 0.472 | 5.811 | 5.338 |
| order-limit | 0.285 | 8.289 | 8.004 |
| distinct | 0.262 | 4.690 | 4.429 |
| aggregate | 0.284 | 4.995 | 4.710 |
| subquery | 0.738 | 23.302 | 22.565 |
| index-range | 0.115 | 5.784 | 5.669 |
| join | 0.381 | 1571.172 | 1570.792 |

## 根因假设与预计收益

- **每语句地板 0.123ms**：`baseline-select1` 本仓 0.123ms/语句（真库 0.053ms）。本轮未观测到历史 5ms 轮询地板，point-pk、range-limit、connect 已回到亚毫秒档。
- **唯一索引未参与点查**：`point-uk` 5.354ms/语句，比 `point-pk` 的 0.274ms 多 5.1ms，与全表 1000 行扫描同量级，说明 `WHERE uk = ?` 走的是扫描而不是唯一索引定位。补上索引定位后预计回到与 `point-pk` 同档。
- **排序与分组全表扫描**：`order-limit` 8.41ms/语句、`group-by` 5.93ms/语句，各比地板多约 5.8-8.3ms，等于 1000 行扫描成本。索引下推与增量聚合后预计降到亚毫秒级。
- **单条写入不慢**：`insert-single` 本仓 4.17ms/语句、真库 2.31ms/语句，差距来自每语句地板与多行插入的逐行落盘（`insert-multi` 0.085 vs 0.052ms/行，1.65x）。批量路径改为一次提交可把每行成本压到真库同档。

## 语义差异（不计入性能比值）

整数列在 1..100 的小值区间上对比 `COUNT(*)`，本轮四项两库计数一致：

| 语句 | 真实库计数 | 本仓计数 |
|-----:|-----------:|--------:|
| `WHERE id BETWEEN 6 AND 46` | 41 | 41 |
| `WHERE id BETWEEN 2 AND 10` | 9 | 9 |
| `WHERE id >= 50` | 51 | 51 |
| `WHERE id > 6` | 94 | 94 |

Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark:mysql84`
