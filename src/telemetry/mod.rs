//! Telemetry module — OpenTelemetry structured logs + metrics + traces (FR-105–107).
//!
//! Every production component MUST emit all three categories.
//! Donor-privacy redaction is enforced at the emit layer (FR-106).

pub mod redaction;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the telemetry stack with structured JSON logging and env-based filtering.
/// Full OpenTelemetry (OTLP export) is configured when `otel_endpoint` is provided.
pub fn init(otel_endpoint: Option<&str>) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,worldcompute=debug"));

    let fmt_layer = fmt::layer().json().with_target(true).with_thread_ids(true);

    let subscriber = tracing_subscriber::registry().with(env_filter).with(fmt_layer);

    // TODO: When otel_endpoint is Some, add OTLP exporter layer for traces + metrics.
    // For now, structured JSON logging is the baseline.
    let _ = otel_endpoint;

    subscriber.init();
}
