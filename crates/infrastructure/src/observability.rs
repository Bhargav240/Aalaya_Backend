use crate::config::LogFormat;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialise the global `tracing` subscriber. JSON in prod, pretty in dev.
pub fn init(format: LogFormat) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,pg_api=debug,pg_infrastructure=debug"));

    match format {
        LogFormat::Json => tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init(),
        LogFormat::Pretty => tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().pretty())
            .init(),
    }
}
