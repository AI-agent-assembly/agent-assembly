//! Server builder wiring router, middleware, state, and graceful shutdown.

use axum::Router;
use tokio::net::TcpListener;

use crate::config::ApiConfig;
use crate::middleware::apply_middleware;
use crate::routes;
use crate::state::AppState;

/// Build the full Axum application with middleware and state.
pub fn build_app(state: AppState) -> Router {
    let api = routes::v1_router();

    let app = Router::new()
        .nest("/api/v1", api)
        .fallback(routes::fallback_404)
        .with_state(());

    let app = app.layer(axum::Extension(state));

    apply_middleware(app)
}

/// Start the HTTP server and block until shutdown.
///
/// After receiving a shutdown signal the server drains in-flight requests.
/// If draining does not complete within [`DRAIN_TIMEOUT`] the server exits
/// anyway so the process is not stuck indefinitely.
///
/// [`DRAIN_TIMEOUT`]: crate::shutdown::DRAIN_TIMEOUT
pub async fn run_server(config: ApiConfig, state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let app = build_app(state);

    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "aa-api server listening");

    let serve = axum::serve(listener, app).with_graceful_shutdown(crate::shutdown::shutdown_signal());

    match tokio::time::timeout(crate::shutdown::DRAIN_TIMEOUT, serve).await {
        Ok(Ok(())) => {
            tracing::info!("aa-api server shut down gracefully");
        }
        Ok(Err(e)) => {
            return Err(e.into());
        }
        Err(_elapsed) => {
            tracing::warn!(
                timeout_secs = crate::shutdown::DRAIN_TIMEOUT.as_secs(),
                "drain timeout exceeded, forcing shutdown"
            );
        }
    }

    Ok(())
}
