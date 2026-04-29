//! OpenAPI spec aggregation via utoipa.

use utoipa::OpenApi;

/// Root OpenAPI document collecting all annotated paths and schemas.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Agent Assembly API",
        version = "0.0.1",
        description = "REST API for the Agent Assembly governance gateway.\n\nThis spec is auto-generated from `aa-api` route annotations via `utoipa`. CI fails if the generated spec drifts from the committed `openapi/v1.yaml`.",
        license(name = "Apache 2.0", url = "https://www.apache.org/licenses/LICENSE-2.0.html"),
        contact(name = "Agent Assembly Contributors", url = "https://github.com/AI-agent-assembly/agent-assembly")
    ),
    servers(
        (url = "http://localhost:7700", description = "Local development gateway")
    ),
    tags(
        (name = "health", description = "Liveness and readiness probes")
    ),
    paths(
        crate::routes::health::health,
    ),
    components(schemas(
        crate::routes::health::HealthResponse,
        crate::error::ProblemDetail,
    ))
)]
pub struct ApiDoc;
