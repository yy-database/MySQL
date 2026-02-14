#![warn(missing_docs)]

use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_subscriber::prelude::*;

/// YYDS Observability Engine
///
/// Responsibility:
/// 1. Initialize Tracing & Metrics pipelines (OTLP/Prometheus)
/// 2. Collect internal kernel metrics (executor, storage, consensus)
/// 3. Provide macros/wrappers for distributed tracing
pub struct ObservabilityManager;

impl ObservabilityManager {
    pub fn init() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 1. Setup OTLP Tracing
        global::set_text_map_propagator(TraceContextPropagator::new());

        // let tracer = opentelemetry_otlp::new_pipeline()
        //     .tracing()
        //     .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        //     .install_batch(opentelemetry::runtime::Tokio)?;

        // 2. Setup Prometheus Metrics
        // let registry = prometheus::default_registry();

        // 3. Setup Tracing Subscriber
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().json())
            // .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init();

        Ok(())
    }
}
