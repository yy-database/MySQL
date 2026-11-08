# 服务端对服务端：`@yydb/mysql` 与真实 MySQL 8.4

真实库：`8.4.9`（`127.0.0.1:33306`，库 `yydb`）
本仓：`@yydb/mysql`（每轮新起临时数据目录）
说明：本机没有 Docker，真实库用本地 MySQL 8.4.9；truth corpus 钉的是 8.4.13。CI 有 `mysql:8.4` 镜像时用 `BENCH_REAL_*` 指向该实例重跑。
轮次：1 轮热身 + 5 轮测量，取中位数并记 p95；数据集 1000 行，固定 seed。
比值 = 本仓耗时 / 真实库耗时，小于 1 表示本仓更快。

## 按差距排序

| 工作负载 | 单位 | 真实库 ms/op | 本仓 ms/op | 倍数 |
|---------:|-----:|-------------:|-----------:|-----:|
| point-uk | statements | 0.200 | 18.329 | 91.83x |
| baseline-select1 | statements | 0.134 | 6.210 | 46.33x |
| point-pk | statements | 0.178 | 6.282 | 35.33x |
| order-limit | statements | 0.659 | 23.119 | 35.07x |
| range-limit | statements | 0.265 | 7.319 | 27.59x |
| group-by | statements | 1.108 | 19.805 | 17.87x |
| tx-commit | rows | 0.759 | 7.629 | 10.06x |
| connect | connections | 1.777 | 16.728 | 9.41x |
| update-many | rows | 0.071 | 0.206 | 2.90x |
| delete-many | rows | 0.071 | 0.158 | 2.22x |
| insert-single | statements | 5.613 | 10.529 | 1.88x |
| insert-multi | rows | 0.094 | 0.174 | 1.86x |

## 绝对耗时与 p95

| 工作负载 | 真实库中位总耗时 ms | 本仓中位总耗时 ms | 真实库 p95 ms | 本仓 p95 ms |
|---------:|--------------------:|------------------:|--------------:|------------:|
| insert-single | 1683.92 | 3158.79 | 1811.15 | 3510.79 |
| insert-multi | 37.44 | 69.46 | 38.40 | 92.98 |
| point-pk | 53.34 | 1884.74 | 90.26 | 2325.69 |
| point-uk | 59.88 | 5498.79 | 89.07 | 6872.48 |
| range-limit | 26.53 | 731.88 | 34.86 | 1054.06 |
| update-many | 141.79 | 411.08 | 145.98 | 433.25 |
| delete-many | 71.07 | 157.91 | 86.52 | 161.61 |
| order-limit | 32.96 | 1155.93 | 38.70 | 1697.28 |
| group-by | 55.42 | 990.27 | 64.03 | 1805.06 |
| tx-commit | 227.57 | 2288.75 | 249.68 | 2629.04 |
| baseline-select1 | 40.21 | 1862.94 | 52.79 | 6001.26 |
| connect | 35.54 | 334.56 | 58.14 | 533.96 |

## 粗粒度阶段差分

线上协议不暴露解析/执行/存储/回包各阶段耗时，这里只做差分估计：以 `baseline-select1` 作为“传输 + 解析 + 回包”地板，其余项减去地板即该路径的额外成本。

| 路径 | 真实库额外 ms/语句 | 本仓额外 ms/语句 | 差额 ms/语句 |
|-----:|-------------------:|-----------------:|-------------:|
| insert-single | 5.479 | 4.319 | -1.160 |
| point-pk | 0.044 | 0.073 | 0.029 |
| point-uk | 0.066 | 12.120 | 12.054 |
| range-limit | 0.131 | 1.109 | 0.978 |
| update-many | 6.955 | 14.344 | 7.389 |
| group-by | 0.974 | 13.596 | 12.621 |
| order-limit | 0.525 | 16.909 | 16.384 |

## 根因假设与预计收益

- **每语句约 6.2ms 固定地板**：上游 `database.server::yyds::mysql::connection` 的 `CancellableSocket::attempt` 在 `WouldBlock` 时固定 `sleep(5ms)`，每个 `COM_QUERY` 至少命中一次。`baseline-select1` 的 6.210ms/语句就是这个地板（真库 0.134ms）。修掉后 SELECT 1 预计降到 0.3ms 以内，point-pk、range-limit、connect 同步下降约 5ms/语句。
- **唯一索引未参与点查**：`point-uk` 18.329ms/语句，比 `point-pk` 的 6.282ms 多 12.0ms，与全表 1000 行扫描同量级，说明 `WHERE uk = ?` 走的是扫描而不是唯一索引定位。补上索引定位后预计回到与 `point-pk` 同档。
- **排序与分组全表扫描**：`order-limit` 23.12ms/语句、`group-by` 19.81ms/语句，各比地板多约 13.6-16.9ms，等于 1000 行扫描成本。索引下推与增量聚合后预计降到亚毫秒级。
- **单条写入不慢**：`insert-single` 本仓 10.53ms/语句、真库 5.61ms/语句，差距来自每语句地板与多行插入的逐行落盘（`insert-multi` 0.174 vs 0.094ms/行，1.86x）。批量路径改为一次提交可把每行成本压到真库同档。

## 语义差异（不计入性能比值）

整数列在 1..100 的小值区间上对比 `COUNT(*)`，真库按数值比较，本仓在边界上分叉：

| 语句 | 真实库计数 | 本仓计数 |
|-----:|-----------:|--------:|
| `WHERE id BETWEEN 6 AND 46` | 41 | 0 |
| `WHERE id BETWEEN 2 AND 10` | 9 | 2 |
| `WHERE id >= 50` | 51 | 55 |
| `WHERE id > 6` | 94 | 43 |

Regenerate: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark:mysql84`
