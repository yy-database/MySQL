# Apache Arrow 协议详解

**we-trust-arrow** 为 YYKV 提供了向量化数据交换的能力，是实现高性能 OLAP（联机分析处理）的关键组件。

## 1. 为什么使用 Arrow？

传统的行式数据传输在处理海量分析查询时，CPU 会浪费大量时间在数据序列化和类型转换上。通过 Arrow 协议，YYKV 可以：
- **零拷贝交换**: 存储引擎生成的列式数据块直接以 Arrow IPC 格式发送，客户端无需解析即可直接进行计算。
- **内存对齐**: 报文结构完全符合 SIMD 指令集的要求，加速过滤和聚合运算。

## 2. 协议集成

Arrow 协议作为 We-Trust 的扩展类型存在：
- 当 `MsgType` 为 `MessageType::Query` 且指定了 Arrow 输出格式时，Payload 将包含标准的 **Arrow Schema** 和 **RecordBatches**。
- 支持 **Dictionary Encoding**（字典编码），极大压缩低基数列的数据传输量。

## 3. 应用场景

- **AI/ML 集成**: 与 Python 的 `pandas`, `pyarrow` 无缝对接，将 YYKV 中的特征数据高速喂给机器学习模型。
- **大数据处理**: 作为 Spark 或 Flink 的数据源，实现亚秒级的列式数据读取。
- **BI 报表**: 加速超大规模数据集的聚合展示。
