//! Telemetry setup — tracing subscriber with JSON formatting.

/// Initialize the tracing subscriber.
///
/// Uses `RUST_LOG` from the environment for filtering.
/// Produces structured JSON output (suitable for production log aggregation)
/// but falls back to human-readable formatting when desired.
pub fn init_telemetry() {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_target(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("tracing subscriber should be set exactly once");
}
