//! Telemetry module — OpenTelemetry structured logs + metrics + traces (FR-105–107).
//!
//! Every production component MUST emit all three categories.
//! Donor-privacy redaction is enforced at the emit layer (FR-106).

pub mod redaction;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Configuration for the OTLP exporter.
pub struct OtlpConfig {
    /// OTLP collector endpoint URL.
    pub endpoint: String,
    /// Service name reported to the collector.
    pub service_name: String,
}

impl OtlpConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into(), service_name: "worldcompute".into() }
    }
}

/// Initialize the telemetry stack with structured JSON logging and env-based filtering.
/// Full OpenTelemetry (OTLP export) is configured when `otel_endpoint` is provided.
pub fn init(otel_endpoint: Option<&str>) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,worldcompute=debug"));

    let fmt_layer = fmt::layer().json().with_target(true).with_thread_ids(true);

    if let Some(endpoint) = otel_endpoint {
        // Configure OTLP trace exporter
        let otlp_config = OtlpConfig::new(endpoint);
        match init_otlp_tracer(&otlp_config) {
            Ok(tracer) => {
                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                let subscriber = tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .with(otel_layer);
                subscriber.init();
                tracing::info!(endpoint = endpoint, "OTLP trace exporter initialized");
                return;
            }
            Err(e) => {
                // Fall through to non-OTLP init — don't crash if collector unreachable
                eprintln!("Warning: OTLP init failed ({e}), falling back to JSON logging only");
            }
        }
    }

    // Fallback: structured JSON logging only
    let subscriber = tracing_subscriber::registry().with(env_filter).with(fmt_layer);
    subscriber.init();
}

/// Initialize an OTLP trace exporter and return a configured tracer.
fn init_otlp_tracer(
    config: &OtlpConfig,
) -> Result<opentelemetry_sdk::trace::Tracer, Box<dyn std::error::Error>> {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::WithExportConfig;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.endpoint)
        .build()?;

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
            "service.name",
            config.service_name.clone(),
        )]))
        .build();

    let tracer = provider.tracer("worldcompute");
    opentelemetry::global::set_tracer_provider(provider);

    Ok(tracer)
}
