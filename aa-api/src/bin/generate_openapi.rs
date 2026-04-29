//! CLI binary that prints the generated OpenAPI spec as YAML to stdout.
//!
//! Usage: `cargo run -p aa-api --bin generate_openapi > openapi/v1.yaml`

use utoipa::OpenApi;

fn main() {
    let spec = aa_api::ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&spec).expect("serialize openapi to yaml");
    print!("{yaml}");
}
