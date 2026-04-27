//! `aa-runtime` sidecar binary entry point.

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().json())
        .init();
}

fn main() {
    init_tracing();

    let config = aa_runtime::config::RuntimeConfig::from_env();

    tracing::info!(
        worker_threads = config.worker_threads,
        shutdown_timeout_secs = config.shutdown_timeout_secs,
        "configuration loaded"
    );

    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();

    if config.worker_threads > 0 {
        builder.worker_threads(config.worker_threads);
    }

    builder
        .build()
        .expect("failed to build Tokio runtime")
        .block_on(aa_runtime::run(config));
}
