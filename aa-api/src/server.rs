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
pub async fn run_server(
    config: ApiConfig,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = build_app(state);

    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "aa-api server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(crate::shutdown::shutdown_signal())
        .await?;

    tracing::info!("aa-api server shut down");
    Ok(())
}
