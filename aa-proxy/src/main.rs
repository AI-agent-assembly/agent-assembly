//! Binary entry point for the `aa-proxy` sidecar.
//!
//! This is intentionally minimal. All logic lives in the library crate.
//! `aa-runtime` spawns this binary via `tokio::process::Command::new("aa-proxy")`.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = aa_proxy::ProxyConfig::from_env()?;
    aa_proxy::run(config).await
}
