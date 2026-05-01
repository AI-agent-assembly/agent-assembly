//! `aasm version` — display CLI and runtime version information.

use std::process::ExitCode;

use comfy_table::Table;
use serde::{Deserialize, Serialize};

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Subset of the gateway health response used for version extraction.
#[derive(Debug, Deserialize)]
struct HealthInfo {
    version: String,
    api_version: String,
}

/// A single row in the version output.
#[derive(Debug, Serialize)]
struct VersionRow {
    component: String,
    version: String,
    status: String,
}

/// Build version rows by probing the gateway health endpoint.
fn build_rows(ctx: &ResolvedContext) -> Vec<VersionRow> {
    let cli_row = VersionRow {
        component: "cli".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        status: "-".to_string(),
    };

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let (gateway_row, api_row) = rt.block_on(async {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/health", ctx.api_url);

        let mut req = client.get(&url);
        if let Some(ref key) = ctx.api_key {
            req = req.bearer_auth(key);
        }

        match req.send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<HealthInfo>().await {
                Ok(info) => (
                    VersionRow {
                        component: "gateway".to_string(),
                        version: info.version,
                        status: "reachable".to_string(),
                    },
                    VersionRow {
                        component: "api".to_string(),
                        version: info.api_version,
                        status: "reachable".to_string(),
                    },
                ),
                Err(_) => unreachable_rows(),
            },
            _ => unreachable_rows(),
        }
    });

    vec![cli_row, gateway_row, api_row]
}

/// Produce gateway and api rows for the unreachable case.
fn unreachable_rows() -> (VersionRow, VersionRow) {
    (
        VersionRow {
            component: "gateway".to_string(),
            version: "-".to_string(),
            status: "unreachable".to_string(),
        },
        VersionRow {
            component: "api".to_string(),
            version: "-".to_string(),
            status: "unreachable".to_string(),
        },
    )
}

/// Render version rows as a comfy-table.
fn render_table(rows: &[VersionRow]) {
    let mut table = Table::new();
    table.set_header(vec!["COMPONENT", "VERSION", "STATUS"]);
    for r in rows {
        table.add_row(vec![&r.component, &r.version, &r.status]);
    }
    println!("{table}");
}

/// Run the `aasm version` command.
pub fn run(ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    let rows = build_rows(ctx);

    match output {
        OutputFormat::Table => render_table(&rows),
        OutputFormat::Json => match serde_json::to_string_pretty(&rows) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("error serializing JSON: {e}"),
        },
        OutputFormat::Yaml => match serde_yaml::to_string(&rows) {
            Ok(yaml) => print!("{yaml}"),
            Err(e) => eprintln!("error serializing YAML: {e}"),
        },
    }

    ExitCode::SUCCESS
}
