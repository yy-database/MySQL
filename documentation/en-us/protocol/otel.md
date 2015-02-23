# OpenTelemetry (OTEL) Protocol

**we-trust-otel** implements full-link observability for YYKV, supporting standard OpenTelemetry protocols.

## 1. Monitoring Dimensions

Through the OTEL protocol, YYKV actively exposes information across several dimensions:
- **Traces**: Records the complete lifecycle of a request, from client entry through compatibility layer transformation to storage node persistence.
- **Metrics**: Outputs real-time QPS, latency distribution (P99), memory usage, WAL write speed, and other key indicators.
- **Logs**: Provides structured audit logs and error reports compliant with OTEL specifications.

## 2. Export Mechanism

- **Push Mode**: YYKV nodes can periodically push metric data to an OpenTelemetry Collector.
- **Otlp Codec**: An efficient Protobuf codec is implemented in `we-trust-otel`, ensuring that telemetry collection does not significantly impact primary business performance.

## 3. Usage Scenarios

- **Prometheus/Grafana**: Integration with standard monitoring dashboards.
- **Jaeger/Tempo**: Distributed troubleshooting to locate slow queries within the storage cluster.
