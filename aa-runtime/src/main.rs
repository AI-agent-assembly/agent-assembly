//! `aa-runtime` sidecar binary entry point.

fn main() {
    let config = aa_runtime::config::RuntimeConfig::from_env();

    // Build the Tokio multi-thread runtime.
    // When worker_threads == 0, Builder uses one thread per logical CPU (Tokio default).
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
