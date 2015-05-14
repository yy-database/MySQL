# OpenTelemetry (OTEL) 协议详解

**we-trust-otel** 实现了 YYKV 的全链路可观测性，支持标准的 OpenTelemetry 协议。

## 1. 监控维度

通过 OTEL 协议，YYKV 主动暴露以下维度的信息：
- **Traces (链路追踪)**: 记录一个请求从客户端进入、经过兼容层转换、到存储节点落盘的完整生命周期。
- **Metrics (指标度量)**: 实时输出 QPS、延迟分布 (P99)、内存使用率、WAL 写入速度等关键指标。
- **Logs (结构化日志)**: 提供符合 OTEL 规范的审计日志和错误报告。

## 2. 导出机制

- **Push 模式**: YYKV 节点可以定期将指标数据推送到 OpenTelemetry Collector。
- **Otlp Codec**: 在 `we-trust-otel` 中实现了高效的 Protobuf 编解码器，确保监控数据的采集不会对主业务产生显著的性能性能影响。

## 3. 使用场景

- **Prometheus/Grafana**: 接入标准监控面板。
- **Jaeger/Tempo**: 进行分布式故障排查，定位存储集群中的慢查询。
