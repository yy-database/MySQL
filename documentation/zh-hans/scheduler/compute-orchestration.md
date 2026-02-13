# 计算调度与编排

计算调度层专注于将计算逻辑（算子）下推至最靠近数据的位置，通过优化执行路径来减少数据在网络和总线上的传输。

## 核心组件

### [yyds-compute](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-compute) / [yykv-compute](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-compute)
**分布式/单机计算算子**。
- **职责**：执行 KQL 查询中的过滤、投影和聚合。
- **调度逻辑**：根据数据分布，决定计算是在网关层执行还是下推到存储节点执行。

### [yykv-stream](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-stream)
**流式执行引擎**。
- **职责**：处理连续的数据流查询。
- **调度特性**：基于反压机制的流式调度，确保数据产生速度与消费速度匹配。

### [yykv-udf](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yykv-udf)
**用户定义函数运行时**。
- **职责**：在存储层执行沙箱化的 UDF。
- **调度相关**：通过 Wasm 运行时调度 UDF 的执行资源，确保租户代码的隔离与安全。

### [yyds-ai](file:///e:/yydb%20%E6%95%B0%E6%8D%AE%E5%BA%93/yykv/packages/yyds-ai)
**AI 协处理器调度**。
- **职责**：管理向量化（Embedding）和模型推理任务。
- **调度策略**：协调 CPU 与专用 AI 加速器之间的推理负载。
