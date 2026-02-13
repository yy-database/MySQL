# OpenTelemetry (OTEL) Protocol

**we-trust-otel** enables full-stack observability for YYKV, supporting standard OpenTelemetry protocols.

## 1. Monitoring Dimensions

Through the OTEL protocol, YYKV actively exposes information across the following dimensions:
- **Traces**: Records the complete lifecycle of a request, from entering the client, through compatibility layer translation, to being persisted at the storage node.
- **Metrics**: Real-time output of key indicators such as QPS, latency distribution (P99), memory utilization, and WAL write speed.
- **Logs**: Provides structured audit logs and error reports that comply with OTEL specifications.

## 2. Export Mechanism

- **Push Mode**: YYKV nodes can periodically push metric data to an OpenTelemetry Collector.
- **Otlp Codec**: Implements an efficient Protobuf codec in `we-trust-otel`, ensuring that monitoring data collection does not significantly impact the performance of core business operations.

## 3. Use Cases

- **Prometheus/Grafana**: Integration with standard monitoring dashboards.
- **Jaeger/Tempo**: Distributed troubleshooting to locate slow queries within the storage cluster.
